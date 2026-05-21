use pinocchio::program_error::ProgramError;

use crate::error::LmsrTrueError;

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum InstructionTag {
    /// Create the pool, mints, vault, and pull initial subsidy from
    /// the creator. q_yes = q_no = 0 at start.
    InitializePool = 0,
    /// Pay collateral, receive newly-minted outcome tokens. The number
    /// of tokens minted is `delta`; the collateral cost is computed by
    /// the LMSR cost-difference and must not exceed `max_collateral_in`.
    Buy = 1,
    /// Burn `delta` outcome tokens; receive collateral computed by the
    /// LMSR cost-difference. Must produce at least `min_collateral_out`.
    Sell = 2,
    /// After the bound resolver fires, the authority can sweep any
    /// residual collateral in excess of outstanding winning-side claims.
    WithdrawResidual = 3,
}

impl TryFrom<u8> for InstructionTag {
    type Error = ProgramError;
    fn try_from(b: u8) -> Result<Self, Self::Error> {
        match b {
            0 => Ok(Self::InitializePool),
            1 => Ok(Self::Buy),
            2 => Ok(Self::Sell),
            3 => Ok(Self::WithdrawResidual),
            _ => Err(LmsrTrueError::InvalidInstructionData.into()),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct InitializePoolData {
    /// Liquidity parameter in outcome-token units.
    pub b: u64,
    /// Subsidy in collateral units the creator commits up front. Must
    /// be `>= b · ln(2)` for the bounded-loss invariant to hold (the
    /// program rejects smaller subsidies).
    pub initial_subsidy: u64,
    pub pool_bump: u8,
    pub collateral_vault_bump: u8,
    pub _padding: [u8; 6],
}

impl InitializePoolData {
    pub const LEN: usize = core::mem::size_of::<Self>();
    pub fn from_bytes(d: &[u8]) -> Result<&Self, ProgramError> {
        if d.len() != Self::LEN {
            return Err(LmsrTrueError::InvalidInstructionData.into());
        }
        Ok(unsafe { &*(d.as_ptr() as *const Self) })
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BuyData {
    /// Number of outcome tokens to receive.
    pub delta: u64,
    /// Max collateral the user is willing to pay (slippage protection).
    pub max_collateral_in: u64,
    /// `0` = YES, `1` = NO.
    pub side: u8,
    pub _padding: [u8; 7],
}

impl BuyData {
    pub const LEN: usize = core::mem::size_of::<Self>();
    pub fn from_bytes(d: &[u8]) -> Result<&Self, ProgramError> {
        if d.len() != Self::LEN {
            return Err(LmsrTrueError::InvalidInstructionData.into());
        }
        Ok(unsafe { &*(d.as_ptr() as *const Self) })
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SellData {
    /// Number of outcome tokens to burn.
    pub delta: u64,
    /// Min collateral the user requires (slippage protection).
    pub min_collateral_out: u64,
    /// `0` = YES, `1` = NO.
    pub side: u8,
    pub _padding: [u8; 7],
}

impl SellData {
    pub const LEN: usize = core::mem::size_of::<Self>();
    pub fn from_bytes(d: &[u8]) -> Result<&Self, ProgramError> {
        if d.len() != Self::LEN {
            return Err(LmsrTrueError::InvalidInstructionData.into());
        }
        Ok(unsafe { &*(d.as_ptr() as *const Self) })
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct WithdrawResidualData {
    pub amount: u64,
}

impl WithdrawResidualData {
    pub const LEN: usize = core::mem::size_of::<Self>();
    pub fn from_bytes(d: &[u8]) -> Result<&Self, ProgramError> {
        if d.len() != Self::LEN {
            return Err(LmsrTrueError::InvalidInstructionData.into());
        }
        Ok(unsafe { &*(d.as_ptr() as *const Self) })
    }
}

const _: () = assert!(InitializePoolData::LEN == 24);
const _: () = assert!(BuyData::LEN == 24);
const _: () = assert!(SellData::LEN == 24);
const _: () = assert!(WithdrawResidualData::LEN == 8);
