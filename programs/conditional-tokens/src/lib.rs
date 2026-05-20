#![no_std]

use pinocchio::{
    account_info::AccountInfo,
    entrypoint,
    program_error::ProgramError,
    pubkey::Pubkey,
    ProgramResult,
};

entrypoint!(process_instruction);

pinocchio_pubkey::declare_id!("Janus11111111111111111111111111111111111111");

#[repr(u8)]
pub enum Instruction {
    InitializeMarket = 0,
    Split = 1,
    Merge = 2,
    Redeem = 3,
}

impl TryFrom<&u8> for Instruction {
    type Error = ProgramError;

    fn try_from(byte: &u8) -> Result<Self, Self::Error> {
        match *byte {
            0 => Ok(Self::InitializeMarket),
            1 => Ok(Self::Split),
            2 => Ok(Self::Merge),
            3 => Ok(Self::Redeem),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let (discriminator, data) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    match Instruction::try_from(discriminator)? {
        Instruction::InitializeMarket => process_initialize_market(accounts, data),
        Instruction::Split => process_split(accounts, data),
        Instruction::Merge => process_merge(accounts, data),
        Instruction::Redeem => process_redeem(accounts, data),
    }
}

fn process_initialize_market(_accounts: &[AccountInfo], _data: &[u8]) -> ProgramResult {
    // TODO: create market PDA
    //   - collateral mint
    //   - resolver program + params
    //   - resolution deadline (slot)
    //   - YES and NO outcome token mints (PDAs of market)
    //   - vault PDA holding collateral
    Ok(())
}

fn process_split(_accounts: &[AccountInfo], _data: &[u8]) -> ProgramResult {
    // TODO:
    //   - take `amount` of collateral from user into vault
    //   - mint `amount` of YES to user
    //   - mint `amount` of NO to user
    Ok(())
}

fn process_merge(_accounts: &[AccountInfo], _data: &[u8]) -> ProgramResult {
    // TODO:
    //   - burn `amount` of YES from user
    //   - burn `amount` of NO from user
    //   - return `amount` of collateral from vault to user
    Ok(())
}

fn process_redeem(_accounts: &[AccountInfo], _data: &[u8]) -> ProgramResult {
    // TODO:
    //   - read resolver state via CPI; require resolved
    //   - if YES: burn user's YES, return collateral 1:1
    //   - if NO: burn user's NO, return collateral 1:1
    //   - if INVALID: allow split/merge unwinding instead
    Ok(())
}
