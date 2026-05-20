//! # janus-lmsr-market
//!
//! Binary market AMM that pairs with the conditional-tokens primitive.
//!
//! **v1 implementation: constant-product (CPMM) with creator subsidy.**
//!
//! A pool holds reserves of YES and NO outcome tokens. Trades preserve the
//! invariant `yes_reserves * no_reserves = k`. The creator seeds the pool
//! by depositing equal YES+NO obtained from a conditional-tokens `Split`,
//! which gives the market quotes at every price level from the moment it
//! opens — no separate LP class required. Maximum loss for the creator
//! is the subsidy they deposited.
//!
//! The reason this is named `lmsr-market` despite shipping CPMM: LMSR is
//! the target curve and the better fit for tail markets (truly bounded
//! loss for the subsidizer, no per-trade price-impact ramp). Implementing
//! LMSR cleanly on BPF requires fixed-point `exp`/`ln`; we stage that as a
//! follow-on. The instruction interface here is curve-agnostic so the
//! upgrade is invisible to integrators.

#![no_std]

mod error;
mod instruction;
mod processor;
mod state;

use pinocchio::{
    account_info::AccountInfo, entrypoint, program_error::ProgramError, pubkey::Pubkey,
    ProgramResult,
};

use crate::instruction::InstructionTag;

entrypoint!(process_instruction);

// Placeholder program ID — replace before deployment with a real keypair.
pinocchio_pubkey::declare_id!("61MLdp3QZMZ8knfBtZvi7CGhB2SgX1Usq6wVaL89SHD1");

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
    match InstructionTag::try_from(*tag)? {
        InstructionTag::InitializePool => processor::process_initialize_pool(accounts, rest),
        InstructionTag::Swap => processor::process_swap(accounts, rest),
    }
}
