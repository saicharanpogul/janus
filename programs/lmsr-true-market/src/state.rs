use pinocchio::{program_error::ProgramError, pubkey::Pubkey};

use crate::error::LmsrTrueError;

/// Pool status.
///
/// In `Active`, buys/sells run against the LMSR curve. After the resolver
/// reports an outcome, status flips to `ResolvedYes`/`ResolvedNo` and only
/// redeems on the winning side are allowed. `Invalid` means the resolver
/// reported "no outcome" — both YES and NO are redeemable 1:1 against
/// collateral split 50/50 (TODO: a fairer policy may want a frozen
/// quote).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoolStatus {
    Active = 0,
    ResolvedYes = 1,
    ResolvedNo = 2,
    Invalid = 3,
}

impl TryFrom<u8> for PoolStatus {
    type Error = ProgramError;
    fn try_from(b: u8) -> Result<Self, Self::Error> {
        match b {
            0 => Ok(Self::Active),
            1 => Ok(Self::ResolvedYes),
            2 => Ok(Self::ResolvedNo),
            3 => Ok(Self::Invalid),
            _ => Err(LmsrTrueError::InvalidAccountData.into()),
        }
    }
}

/// True-LMSR pool.
///
/// `#[repr(C)]`, 248 bytes. The pool mints YES + NO on demand (the pool
/// PDA is the mint authority for both) and holds collateral in a single
/// vault. Pricing follows the LMSR cost function with liquidity
/// parameter `b`. The subsidizer's maximum loss is `b · ln(2)` (this is
/// the bounded-loss theorem we'll prove in Lean).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Pool {
    pub bump: u8,
    pub status: u8,
    pub _padding: [u8; 6],
    /// Subsidizer / pool creator.
    pub authority: Pubkey,
    /// Resolver program ID; called via CPI during `Resolve`.
    pub resolver_program: Pubkey,
    /// Resolver state account that resolves to a `ResolutionOutcome`.
    pub resolver_state: Pubkey,
    /// SPL Token mint of the collateral (e.g., USDC).
    pub collateral_mint: Pubkey,
    /// Pool's collateral vault token account.
    pub collateral_vault: Pubkey,
    /// YES mint (pool PDA is mint authority).
    pub yes_mint: Pubkey,
    /// NO mint (pool PDA is mint authority).
    pub no_mint: Pubkey,
    /// Liquidity parameter `b`, denominated in raw outcome-token units
    /// (same scale as `q_yes` / `q_no`). For Q32.32 math we lift via
    /// `Q32_32::from_int(b_truncated)`.
    pub b: u64,
    /// Outstanding YES shares (cumulative buys minus cumulative sells).
    pub q_yes: u64,
    /// Outstanding NO shares.
    pub q_no: u64,
    /// Initial subsidy deposited at init (`b · ln(2)` rounded up); the
    /// invariant `vault.amount + winning_side_outstanding ≤ initial +
    /// b·ln(2)` is the bounded-loss theorem.
    pub initial_subsidy: u64,
}

impl Pool {
    pub const LEN: usize = core::mem::size_of::<Self>();
    pub const SEED: &'static [u8] = b"pool";
    pub const COLLATERAL_VAULT_SEED: &'static [u8] = b"coll-vault";

    pub fn from_data(d: &[u8]) -> Result<&Self, ProgramError> {
        if d.len() != Self::LEN {
            return Err(LmsrTrueError::InvalidAccountData.into());
        }
        Ok(unsafe { &*(d.as_ptr() as *const Self) })
    }

    pub fn from_data_mut(d: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if d.len() != Self::LEN {
            return Err(LmsrTrueError::InvalidAccountData.into());
        }
        Ok(unsafe { &mut *(d.as_mut_ptr() as *mut Self) })
    }
}

// 8 (bump+status+pad) + 7·32 (pubkeys) + 4·8 (u64) = 264.
const _: () = assert!(Pool::LEN == 264);
