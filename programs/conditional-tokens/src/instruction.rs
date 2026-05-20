use pinocchio::program_error::ProgramError;

use crate::error::ConditionalTokensError;

/// Top-level instruction dispatcher.
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum InstructionTag {
    /// Create a new market: allocates the market PDA, the YES/NO mints,
    /// and the collateral vault.
    InitializeMarket = 0,
    /// Deposit collateral, mint matching YES + NO outcome tokens.
    Split = 1,
    /// Burn matching YES + NO outcome tokens, withdraw collateral.
    Merge = 2,
    /// After resolution, exchange winning outcome tokens for collateral.
    Redeem = 3,
    /// Query the bound resolver and write the result into the market.
    Resolve = 4,
}

impl TryFrom<u8> for InstructionTag {
    type Error = ProgramError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0 => Ok(Self::InitializeMarket),
            1 => Ok(Self::Split),
            2 => Ok(Self::Merge),
            3 => Ok(Self::Redeem),
            4 => Ok(Self::Resolve),
            _ => Err(ConditionalTokensError::InvalidInstructionData.into()),
        }
    }
}

/// Instruction data for [`InstructionTag::InitializeMarket`].
///
/// `#[repr(C)]` so the on-wire bytes match the in-memory layout 1:1.
/// Total size: 16 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct InitializeMarketData {
    pub deadline_slot: u64,
    pub market_bump: u8,
    pub yes_mint_bump: u8,
    pub no_mint_bump: u8,
    pub vault_bump: u8,
    pub _padding: [u8; 4],
}

impl InitializeMarketData {
    pub const LEN: usize = core::mem::size_of::<Self>();

    pub fn from_bytes(data: &[u8]) -> Result<&Self, ProgramError> {
        if data.len() != Self::LEN {
            return Err(ConditionalTokensError::InvalidInstructionData.into());
        }
        // SAFETY: size is checked; layout is `#[repr(C)]` and alignment is 8.
        Ok(unsafe { &*(data.as_ptr() as *const Self) })
    }
}

/// Instruction data carrying a single `u64` amount (Split / Merge / Redeem).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AmountData {
    pub amount: u64,
}

impl AmountData {
    pub const LEN: usize = core::mem::size_of::<Self>();

    pub fn from_bytes(data: &[u8]) -> Result<&Self, ProgramError> {
        if data.len() != Self::LEN {
            return Err(ConditionalTokensError::InvalidInstructionData.into());
        }
        // SAFETY: size is checked; layout is `#[repr(C)]` and alignment is 8.
        Ok(unsafe { &*(data.as_ptr() as *const Self) })
    }
}

const _: () = assert!(InitializeMarketData::LEN == 16);
const _: () = assert!(AmountData::LEN == 8);
