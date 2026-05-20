use pinocchio::program_error::ProgramError;

use crate::error::LmsrError;

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum InstructionTag {
    /// Create a pool and seed it with matching YES + NO reserves.
    InitializePool = 0,
    /// Swap one outcome token for the other along the constant-product curve.
    Swap = 1,
    /// After the bound market has resolved, the pool's authority sweeps
    /// the winning side's remaining vault balance to their own token
    /// account so it can be redeemed for collateral via
    /// conditional-tokens::redeem.
    WithdrawPoolTokens = 2,
}

impl TryFrom<u8> for InstructionTag {
    type Error = ProgramError;
    fn try_from(b: u8) -> Result<Self, Self::Error> {
        match b {
            0 => Ok(Self::InitializePool),
            1 => Ok(Self::Swap),
            2 => Ok(Self::WithdrawPoolTokens),
            _ => Err(LmsrError::InvalidInstructionData.into()),
        }
    }
}

/// Instruction data for `InitializePool`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct InitializePoolData {
    /// Subsidy in YES tokens (must equal `subsidy_no` for a fair start).
    pub subsidy_yes: u64,
    /// Subsidy in NO tokens.
    pub subsidy_no: u64,
    /// Swap fee in basis points; capped at 1000 (10%).
    pub fee_bps: u16,
    /// PDA bump for the pool itself.
    pub pool_bump: u8,
    /// PDA bump for the YES vault token account.
    pub yes_vault_bump: u8,
    /// PDA bump for the NO vault token account.
    pub no_vault_bump: u8,
    pub _padding: [u8; 3],
}

impl InitializePoolData {
    pub const LEN: usize = core::mem::size_of::<Self>();
    pub fn from_bytes(d: &[u8]) -> Result<&Self, ProgramError> {
        if d.len() != Self::LEN {
            return Err(LmsrError::InvalidInstructionData.into());
        }
        Ok(unsafe { &*(d.as_ptr() as *const Self) })
    }
}

/// Instruction data for `Swap`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SwapData {
    /// Amount of the input outcome token the user is depositing.
    pub amount_in: u64,
    /// Minimum amount of the output outcome token the user will accept
    /// (slippage protection).
    pub min_amount_out: u64,
    /// `0` = input is YES, output is NO. `1` = input is NO, output is YES.
    pub direction: u8,
    pub _padding: [u8; 7],
}

impl SwapData {
    pub const LEN: usize = core::mem::size_of::<Self>();
    pub fn from_bytes(d: &[u8]) -> Result<&Self, ProgramError> {
        if d.len() != Self::LEN {
            return Err(LmsrError::InvalidInstructionData.into());
        }
        Ok(unsafe { &*(d.as_ptr() as *const Self) })
    }
}

/// Instruction data for `WithdrawPoolTokens`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct WithdrawPoolTokensData {
    /// Amount of winning-side tokens to withdraw.
    pub amount: u64,
    /// `0` = withdraw YES (use when market resolved YES), `1` = NO.
    pub side: u8,
    pub _padding: [u8; 7],
}

impl WithdrawPoolTokensData {
    pub const LEN: usize = core::mem::size_of::<Self>();
    pub fn from_bytes(d: &[u8]) -> Result<&Self, ProgramError> {
        if d.len() != Self::LEN {
            return Err(LmsrError::InvalidInstructionData.into());
        }
        Ok(unsafe { &*(d.as_ptr() as *const Self) })
    }
}

const _: () = assert!(InitializePoolData::LEN == 24);
const _: () = assert!(SwapData::LEN == 24);
const _: () = assert!(WithdrawPoolTokensData::LEN == 16);
