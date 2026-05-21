//! # janus-lmsr-true-market
//!
//! True LMSR (Logarithmic Market Scoring Rule, Hanson 2003) binary market
//! on Solana.
//!
//! **Model**: collateral in, outcome out. The pool itself is the AMM and
//! the mint authority for both YES and NO. Pricing uses the LMSR cost
//! function with liquidity parameter `b`:
//!
//! ```text
//!     C(q_yes, q_no) = b · ln(exp(q_yes / b) + exp(q_no / b))
//! ```
//!
//! - **Buy** of `δ` shares of YES costs `C(q_yes + δ, q_no) − C(q_yes, q_no)`
//!   collateral, paid into the pool's collateral vault. The pool mints δ
//!   YES to the user.
//! - **Sell** of `δ` YES burns δ from the user and pays out
//!   `C(q_yes, q_no) − C(q_yes − δ, q_no)` collateral.
//! - **Bounded loss**: the subsidizer's maximum exposure is `b · ln(2)`,
//!   regardless of trader activity. This is the property we prove in
//!   `formal_verification/lmsr_true_market/`.
//!
//! All math runs on `janus-lmsr-math`'s Q32.32 fixed-point primitives so
//! that every validator computes the same price for the same state. See
//! that crate for the precision envelope (~1e-4 relative error vs f64).
//!
//! This program is intentionally standalone — it does *not* depend on
//! `janus-conditional-tokens`. Resolution is delegated to a resolver
//! program via the same `resolver-interface` contract.

#![no_std]

mod error;
mod instruction;
mod processor;
mod state;

use pinocchio::{
    account_info::AccountInfo, entrypoint, nostd_panic_handler, program_error::ProgramError,
    pubkey::Pubkey, ProgramResult,
};

use crate::instruction::InstructionTag;

entrypoint!(process_instruction);
nostd_panic_handler!();

// Placeholder program ID — replace before deployment via
// `scripts/sync-program-ids.sh`.
pinocchio_pubkey::declare_id!("HrFV8Nfncv2gekc9jZPC6rXxnVVaUQi75BmwVFzd5fjQ");

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
        InstructionTag::Buy => processor::process_buy(accounts, rest),
        InstructionTag::Sell => processor::process_sell(accounts, rest),
        InstructionTag::WithdrawResidual => {
            processor::process_withdraw_residual(accounts, rest)
        }
    }
}
