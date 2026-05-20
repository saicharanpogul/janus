//! # janus-pyth-price-resolver
//!
//! Resolves a binary market YES/NO depending on whether a Pyth price feed
//! satisfies a configured threshold comparison at or after a target slot.
//!
//! Pyth's `PriceUpdateV2` account layout (from `pyth-solana-receiver-sdk`):
//!
//! ```text
//! offset  field
//!     0   anchor discriminator (8 bytes)
//!     8   write_authority   (32)
//!    40   verification_level (1)
//!    41   feed_id           (32)
//!    73   price             (i64)
//!    81   conf              (u64)
//!    89   exponent          (i32)
//!    93   publish_time      (i64)
//!   101   prev_publish_time (i64)
//!   109   ema_price         (i64)
//!   117   ema_conf          (u64)
//!   125   posted_slot       (u64)
//! ```
//!
//! We read `price` and `exponent` directly at their known offsets. In
//! production you would prefer to depend on `pyth-solana-receiver-sdk`
//! for safe parsing; this implementation is intentionally minimal so the
//! Janus stack stays dependency-light during prototyping.

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
pinocchio_pubkey::declare_id!("61MLdp3R75WtTNjW4MfvbCDU43uAB2uUNZhD53kX9hyq");

const INSTRUCTION_INITIALIZE: u8 = 1;

// Pyth PriceUpdateV2 byte offsets (see module-level comment).
const PYTH_PRICE_OFFSET: usize = 73;
const PYTH_EXPONENT_OFFSET: usize = 89;
const PYTH_MIN_LEN: usize = 133;

/// Comparison applied to the (price, exponent) pulled from the feed
/// against the configured (`threshold_price`, `threshold_expo`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Comparison {
    /// YES iff `feed_price >= threshold_price` (after exponent alignment).
    GreaterThanOrEqual = 0,
    /// YES iff `feed_price < threshold_price` (after exponent alignment).
    LessThan = 1,
}

impl TryFrom<u8> for Comparison {
    type Error = ProgramError;
    fn try_from(b: u8) -> Result<Self, Self::Error> {
        match b {
            0 => Ok(Self::GreaterThanOrEqual),
            1 => Ok(Self::LessThan),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

/// 104-byte resolver-state account.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PythPriceResolverState {
    pub bump: u8,
    pub comparison: u8,
    pub _padding: [u8; 6],
    pub authority: Pubkey,
    pub price_feed: Pubkey,
    pub earliest_slot: u64,
    pub threshold_price: i64,
    pub threshold_expo: i32,
    pub _padding2: [u8; 4],
}

impl PythPriceResolverState {
    pub const LEN: usize = core::mem::size_of::<Self>();
    pub const SEED: &'static [u8] = b"pyth-resolver";

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

const _: () = assert!(PythPriceResolverState::LEN == 96);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    if program_id != &ID {
        return Err(ProgramError::IncorrectProgramId);
    }
    let (tag, rest) = data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;
    match *tag {
        RESOLVE_INSTRUCTION_TAG => process_resolve(accounts),
        INSTRUCTION_INITIALIZE => process_initialize(accounts, rest),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

/// Initialize accounts:
///   0. `[signer, writable]` payer
///   1. `[writable]`         state PDA
///   2. `[signer]`           authority
///   3. `[readonly]`         system program
///
/// Data: `[bump:u8, comparison:u8, pad:6, price_feed:32, earliest_slot:u64,
///         threshold_price:i64, threshold_expo:i32, pad:4, seed_key:32]`
fn process_initialize(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if data.len() != PythPriceResolverState::LEN - 32 + 32 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let bump = data[0];
    let comparison = data[1];
    // bytes [2..8] padding
    let mut price_feed = [0u8; 32];
    price_feed.copy_from_slice(&data[8..40]);
    let earliest_slot = u64::from_le_bytes(data[40..48].try_into().unwrap());
    let threshold_price = i64::from_le_bytes(data[48..56].try_into().unwrap());
    let threshold_expo = i32::from_le_bytes(data[56..60].try_into().unwrap());
    // bytes [60..64] padding
    let seed_key: &[u8] = &data[64..96];

    // Sanity: parse comparison so an invalid byte is rejected up-front.
    let _ = Comparison::try_from(comparison)?;

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
        PythPriceResolverState::SEED,
        seed_key,
        bump_arr.as_ref(),
    ];
    let expected =
        create_program_address(&seeds, &ID).map_err(|_| ProgramError::InvalidSeeds)?;
    if &expected != state_ai.key() {
        return Err(ProgramError::InvalidSeeds);
    }

    let signer_seeds: [Seed; 3] = [
        Seed::from(PythPriceResolverState::SEED),
        Seed::from(seed_key),
        Seed::from(bump_arr.as_ref()),
    ];
    let lamports = (PythPriceResolverState::LEN as u64 + 128) * 3_480 * 2;
    CreateAccount {
        from: payer,
        to: state_ai,
        lamports,
        space: PythPriceResolverState::LEN as u64,
        owner: &ID,
    }
    .invoke_signed(&[Signer::from(&signer_seeds[..])])?;

    {
        let mut d = state_ai.try_borrow_mut_data()?;
        let s = PythPriceResolverState::from_data_mut(&mut d)?;
        s.bump = bump;
        s.comparison = comparison;
        s._padding = [0; 6];
        s.authority = *authority_ai.key();
        s.price_feed = price_feed;
        s.earliest_slot = earliest_slot;
        s.threshold_price = threshold_price;
        s.threshold_expo = threshold_expo;
        s._padding2 = [0; 4];
    }
    Ok(())
}

/// Resolve accounts:
///   0. `[readonly]` resolver state
///   1. `[readonly]` Pyth price feed account (must match state.price_feed)
fn process_resolve(accounts: &[AccountInfo]) -> ProgramResult {
    let [state_ai, feed_ai] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    // SAFETY: pinocchio marks AccountInfo::owner unsafe; in the runtime,
    // the loader guarantees account headers are aligned and present.
    let owner = unsafe { state_ai.owner() };
    if owner != &ID {
        return Err(ProgramError::IllegalOwner);
    }

    let state_data = state_ai.try_borrow_data()?;
    let state = PythPriceResolverState::from_data(&state_data)?;

    if &state.price_feed != feed_ai.key() {
        return Err(ProgramError::InvalidAccountData);
    }

    let outcome = compute_outcome(state, feed_ai)?;
    set_return_data(&[outcome.as_byte()]);
    Ok(())
}

fn compute_outcome(
    state: &PythPriceResolverState,
    feed_ai: &AccountInfo,
) -> Result<ResolutionOutcome, ProgramError> {
    let clock = Clock::get()?;
    if clock.slot < state.earliest_slot {
        return Ok(ResolutionOutcome::Unresolved);
    }

    let feed_data = feed_ai.try_borrow_data()?;
    if feed_data.len() < PYTH_MIN_LEN {
        // Likely not a valid PriceUpdateV2 account; the market is undecidable.
        return Ok(ResolutionOutcome::Invalid);
    }

    let price = i64::from_le_bytes(
        feed_data[PYTH_PRICE_OFFSET..PYTH_PRICE_OFFSET + 8]
            .try_into()
            .unwrap(),
    );
    let expo = i32::from_le_bytes(
        feed_data[PYTH_EXPONENT_OFFSET..PYTH_EXPONENT_OFFSET + 4]
            .try_into()
            .unwrap(),
    );

    // For now require matching exponents; full scaling can be added once
    // the rest of the stack lands. Markets created with mis-matched
    // exponents resolve as Invalid so collateral can be unwound.
    if expo != state.threshold_expo {
        return Ok(ResolutionOutcome::Invalid);
    }

    let comparison = Comparison::try_from(state.comparison)?;
    let yes = match comparison {
        Comparison::GreaterThanOrEqual => price >= state.threshold_price,
        Comparison::LessThan => price < state.threshold_price,
    };
    Ok(if yes {
        ResolutionOutcome::Yes
    } else {
        ResolutionOutcome::No
    })
}
