//! # janus-market-factory
//!
//! On-chain registry of Janus markets.
//!
//! The factory is not a CPI orchestrator — Solana's transaction size and
//! account-list limits make a single-program-orchestrator pattern brittle
//! for stacks with as many account dependencies as Janus has. Market
//! creation is composed at the SDK / transaction layer (one tx containing
//! resolver-init + market-init + pool-init + factory-register).
//!
//! What this program *does* provide is a single canonical account per
//! market that records the full component bundle (market PDA, pool PDA,
//! resolver binding, creator, deadline) so any indexer, frontend, or
//! downstream program has a queryable index of every Janus market that
//! exists. Discovery becomes "scan the accounts owned by this program."

#![no_std]

use pinocchio::{
    account_info::AccountInfo,
    entrypoint,
    instruction::{Seed, Signer},
    nostd_panic_handler,
    program_error::ProgramError,
    pubkey::{create_program_address, Pubkey},
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;

entrypoint!(process_instruction);
nostd_panic_handler!();

// Placeholder program ID — replace before deployment.
pinocchio_pubkey::declare_id!("8ibKxXAWsdqyNG1wExRSvLhKBgXiPpqtE6ZkA277gPwC");

/// Conditional-tokens program ID (must be kept in sync with that crate's
/// `declare_id!`). Used to verify the market account ownership.
const CONDITIONAL_TOKENS_ID: Pubkey =
    pinocchio_pubkey::pubkey!("SH9ghSowHqqWR5YcXVtmkXjt8is1qERCmxHXEvf5sw1");

/// LMSR-market program ID (kept in sync with that crate). Used to verify
/// the pool account ownership.
const LMSR_MARKET_ID: Pubkey =
    pinocchio_pubkey::pubkey!("GUwcYfYGqR6WPduoB6gEEZoPG6vdAAK7gK1xP6eTJ3JK");

// ---- Byte offsets into the Market account (conditional-tokens) ----
// These mirror `janus_conditional_tokens::state::Market`. Duplicated here
// to keep the factory free of an entrypoint-conflicting library dep.
const MARKET_DEADLINE_SLOT_OFFSET: usize = 232;
const MARKET_RESOLVER_PROGRAM_OFFSET: usize = 136;
const MARKET_RESOLVER_STATE_OFFSET: usize = 168;

// ---- Byte offsets into the Pool account (lmsr-market) ----
const POOL_MARKET_OFFSET: usize = 8;

/// Persistent registration record for a market.
///
/// Total 216 bytes, `#[repr(C)]`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MarketRegistration {
    pub bump: u8,
    pub _padding: [u8; 7],
    pub market: Pubkey,
    pub pool: Pubkey,
    pub resolver_program: Pubkey,
    pub resolver_state: Pubkey,
    pub creator: Pubkey,
    pub deadline_slot: u64,
    pub created_at_slot: u64,
    /// Optional 32-byte hash of the off-chain question text. Lets a
    /// frontend pin "this market answers question Q" without forcing the
    /// text on-chain.
    pub question_hash: [u8; 32],
}

impl MarketRegistration {
    pub const LEN: usize = core::mem::size_of::<Self>();
    pub const SEED: &'static [u8] = b"registration";

    pub fn from_data_mut(d: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if d.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &mut *(d.as_mut_ptr() as *mut Self) })
    }
}

const _: () = assert!(MarketRegistration::LEN == 216);

#[repr(u8)]
pub enum InstructionTag {
    Register = 0,
}

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
        x if x == InstructionTag::Register as u8 => process_register(accounts, rest),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

/// Register a new market.
///
/// Accounts:
///   0. `[signer, writable]` payer / creator
///   1. `[writable]`         registration PDA (will be created;
///                           seeds = [b"registration", market.key()])
///   2. `[readonly]`         market account (owned by conditional-tokens)
///   3. `[readonly]`         pool account (owned by lmsr-market)
///   4. `[readonly]`         system program
///
/// Data layout: `[bump: u8, _pad: 7, question_hash: 32]`
fn process_register(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if data.len() != 1 + 7 + 32 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let bump = data[0];
    let mut question_hash = [0u8; 32];
    question_hash.copy_from_slice(&data[8..40]);

    let [payer, reg_ai, market_ai, pool_ai, _system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    if !payer.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if !reg_ai.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // Verify ownership of market and pool — only programs we trust can
    // give us a legitimate market/pool object.
    // SAFETY: see conditional-tokens::processor::read_market.
    let market_owner = unsafe { market_ai.owner() };
    if market_owner != &CONDITIONAL_TOKENS_ID {
        return Err(ProgramError::IllegalOwner);
    }
    let pool_owner = unsafe { pool_ai.owner() };
    if pool_owner != &LMSR_MARKET_ID {
        return Err(ProgramError::IllegalOwner);
    }

    // Cross-check: pool.market field must equal the provided market.
    let pool_data = pool_ai.try_borrow_data()?;
    if pool_data.len() < POOL_MARKET_OFFSET + 32 {
        return Err(ProgramError::InvalidAccountData);
    }
    if &pool_data[POOL_MARKET_OFFSET..POOL_MARKET_OFFSET + 32] != market_ai.key().as_ref() {
        return Err(ProgramError::InvalidAccountData);
    }
    drop(pool_data);

    // Read deadline + resolver binding from market.
    let market_data = market_ai.try_borrow_data()?;
    if market_data.len() < MARKET_DEADLINE_SLOT_OFFSET + 8 {
        return Err(ProgramError::InvalidAccountData);
    }
    let deadline_slot = u64::from_le_bytes(
        market_data[MARKET_DEADLINE_SLOT_OFFSET..MARKET_DEADLINE_SLOT_OFFSET + 8]
            .try_into()
            .unwrap(),
    );
    let mut resolver_program = [0u8; 32];
    resolver_program.copy_from_slice(
        &market_data[MARKET_RESOLVER_PROGRAM_OFFSET..MARKET_RESOLVER_PROGRAM_OFFSET + 32],
    );
    let mut resolver_state = [0u8; 32];
    resolver_state.copy_from_slice(
        &market_data[MARKET_RESOLVER_STATE_OFFSET..MARKET_RESOLVER_STATE_OFFSET + 32],
    );
    drop(market_data);

    // Verify registration PDA.
    let bump_arr = [bump];
    let seeds: [&[u8]; 3] = [
        MarketRegistration::SEED,
        market_ai.key().as_ref(),
        bump_arr.as_ref(),
    ];
    let derived = create_program_address(&seeds, &ID).map_err(|_| ProgramError::InvalidSeeds)?;
    if &derived != reg_ai.key() {
        return Err(ProgramError::InvalidSeeds);
    }

    // Create the registration PDA.
    let lamports = (MarketRegistration::LEN as u64 + 128) * 3_480 * 2;
    let signer_seeds: [Seed; 3] = [
        Seed::from(MarketRegistration::SEED),
        Seed::from(market_ai.key().as_ref()),
        Seed::from(bump_arr.as_ref()),
    ];
    CreateAccount {
        from: payer,
        to: reg_ai,
        lamports,
        space: MarketRegistration::LEN as u64,
        owner: &ID,
    }
    .invoke_signed(&[Signer::from(&signer_seeds[..])])?;

    // Write registration record.
    let clock = Clock::get()?;
    let mut d = reg_ai.try_borrow_mut_data()?;
    let r = MarketRegistration::from_data_mut(&mut d)?;
    r.bump = bump;
    r._padding = [0; 7];
    r.market = *market_ai.key();
    r.pool = *pool_ai.key();
    r.resolver_program = resolver_program;
    r.resolver_state = resolver_state;
    r.creator = *payer.key();
    r.deadline_slot = deadline_slot;
    r.created_at_slot = clock.slot;
    r.question_hash = question_hash;

    Ok(())
}
