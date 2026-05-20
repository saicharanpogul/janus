//! # janus-slot-height-resolver
//!
//! Trivial reference resolver: once the current slot reaches
//! `target_slot`, return the outcome chosen at initialization time.
//!
//! Useful for tests, for time-based markets where the answer is known at
//! creation ("does this period end at slot X?"), and as the canonical
//! example of how to satisfy the [`janus_resolver_interface`] contract.

#![no_std]

use janus_resolver_interface::{ResolutionOutcome, RESOLVE_INSTRUCTION_TAG};
use pinocchio::{
    account_info::AccountInfo,
    entrypoint,
    instruction::{Seed, Signer},
    program::set_return_data,
    program_error::ProgramError,
    pubkey::{create_program_address, Pubkey},
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;

entrypoint!(process_instruction);

// Placeholder program ID — replace before deployment by running
// `solana-keygen new` and pasting the resulting pubkey here.
pinocchio_pubkey::declare_id!("61MLdp3RWCNin5N3CPGTdCHoSA4EyZYLqegkaDus9nFZ");

const INSTRUCTION_INITIALIZE: u8 = 1;

/// On-chain state describing how this resolver should answer.
///
/// 48 bytes; `#[repr(C)]` for direct casting.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SlotHeightResolverState {
    pub bump: u8,
    /// The outcome to return once `target_slot` has been reached. Must be
    /// one of `Yes`, `No`, or `Invalid` — never `Unresolved`.
    pub outcome_at_or_after: u8,
    pub _padding: [u8; 6],
    pub authority: Pubkey,
    pub target_slot: u64,
}

impl SlotHeightResolverState {
    pub const LEN: usize = core::mem::size_of::<Self>();
    pub const SEED: &'static [u8] = b"slot-resolver";

    pub fn from_data(d: &[u8]) -> Result<&Self, ProgramError> {
        if d.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &*(d.as_ptr() as *const Self) })
    }

    pub fn from_data_mut(d: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if d.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &mut *(d.as_mut_ptr() as *mut Self) })
    }
}

const _: () = assert!(SlotHeightResolverState::LEN == 48);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    if program_id != &ID {
        return Err(ProgramError::IncorrectProgramId);
    }
    let (tag, rest) = data.split_first().ok_or(ProgramError::InvalidInstructionData)?;
    match *tag {
        RESOLVE_INSTRUCTION_TAG => process_resolve(accounts),
        INSTRUCTION_INITIALIZE => process_initialize(accounts, rest),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

/// Initialize accounts:
///   0. `[signer, writable]` payer
///   1. `[writable]`         resolver state PDA (will be created)
///   2. `[signer]`           authority
///   3. `[readonly]`         system program
///
/// Data layout: `[outcome:u8, bump:u8, _pad:6, target_slot:u64, seed_key:32]`
/// where `seed_key` is the public bytes used as the resolver state seed
/// (typically the market's predicted PDA or any caller-chosen 32-byte tag,
/// allowing one authority to operate many independent resolver instances).
fn process_initialize(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if data.len() != 1 + 1 + 6 + 8 + 32 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let outcome = data[0];
    let bump = data[1];
    // bytes [2..8] are padding
    let mut slot_bytes = [0u8; 8];
    slot_bytes.copy_from_slice(&data[8..16]);
    let target_slot = u64::from_le_bytes(slot_bytes);
    let seed_key: &[u8] = &data[16..48];

    // Outcome must be a terminal state.
    let outcome_enum = ResolutionOutcome::try_from(outcome)?;
    if matches!(outcome_enum, ResolutionOutcome::Unresolved) {
        return Err(ProgramError::InvalidInstructionData);
    }

    let [payer, state_ai, authority_ai, _system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    if !payer.is_signer() || !authority_ai.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if !state_ai.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let bump_arr = [bump];
    let seeds: [&[u8]; 3] = [
        SlotHeightResolverState::SEED,
        seed_key,
        bump_arr.as_ref(),
    ];
    let expected = create_program_address(&seeds, &ID).map_err(|_| ProgramError::InvalidSeeds)?;
    if &expected != state_ai.key() {
        return Err(ProgramError::InvalidSeeds);
    }

    let signer_seeds: [Seed; 3] = [
        Seed::from(SlotHeightResolverState::SEED),
        Seed::from(seed_key),
        Seed::from(bump_arr.as_ref()),
    ];
    let lamports = (SlotHeightResolverState::LEN as u64 + 128) * 3_480 * 2;
    CreateAccount {
        from: payer,
        to: state_ai,
        lamports,
        space: SlotHeightResolverState::LEN as u64,
        owner: &ID,
    }
    .invoke_signed(&[Signer::from(&signer_seeds[..])])?;

    {
        let mut data_ref = state_ai.try_borrow_mut_data()?;
        let s = SlotHeightResolverState::from_data_mut(&mut data_ref)?;
        s.bump = bump;
        s.outcome_at_or_after = outcome;
        s._padding = [0; 6];
        s.authority = *authority_ai.key();
        s.target_slot = target_slot;
    }
    Ok(())
}

/// Resolve accounts:
///   0. `[readonly]` resolver state account (owned by this program)
fn process_resolve(accounts: &[AccountInfo]) -> ProgramResult {
    let [state_ai] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    // SAFETY: see `read_market` in conditional-tokens.
    let owner = unsafe { state_ai.owner() };
    if owner != &ID {
        return Err(ProgramError::IllegalOwner);
    }
    let data = state_ai.try_borrow_data()?;
    let state = SlotHeightResolverState::from_data(&data)?;

    let clock = Clock::get()?;
    let outcome = if clock.slot >= state.target_slot {
        state.outcome_at_or_after
    } else {
        ResolutionOutcome::Unresolved.as_byte()
    };
    set_return_data(&[outcome]);
    Ok(())
}
