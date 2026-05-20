//! # janus-conditional-tokens
//!
//! The asset layer of the Janus binary-markets primitive.
//!
//! A market splits collateral (e.g. USDC) into matching `YES` and `NO`
//! outcome tokens. The pair can always be recombined back into collateral
//! (the `merge` invariant). After the bound resolver determines the outcome,
//! holders of the winning token redeem 1:1; the losing token settles to zero.

#![no_std]

use pinocchio::{
    account_info::AccountInfo, entrypoint, program_error::ProgramError, pubkey::Pubkey,
    ProgramResult,
};

pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

use crate::instruction::InstructionTag;

entrypoint!(process_instruction);

// Placeholder program ID — replace before deployment by running
// `solana-keygen new -o target/deploy/janus_conditional_tokens-keypair.json`
// and pasting the resulting pubkey here.
pinocchio_pubkey::declare_id!("61MLdp3EEExnhh6W9BYT8Jj52ZoXYoQn6PHKmxtrsc7y");

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if program_id != &ID {
        return Err(ProgramError::IncorrectProgramId);
    }

    let (tag, data) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    match InstructionTag::try_from(*tag)? {
        InstructionTag::InitializeMarket => processor::process_initialize_market(accounts, data),
        InstructionTag::Split => processor::process_split(accounts, data),
        InstructionTag::Merge => processor::process_merge(accounts, data),
        InstructionTag::Redeem => processor::process_redeem(accounts, data),
        InstructionTag::Resolve => processor::process_resolve(accounts, data),
    }
}
