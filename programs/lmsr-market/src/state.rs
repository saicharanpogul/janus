use pinocchio::{program_error::ProgramError, pubkey::Pubkey};

use crate::error::LmsrError;

/// Binary-market AMM pool.
///
/// `#[repr(C)]`, total 224 bytes. Holds reserves of YES + NO outcome
/// tokens for a single bound conditional-tokens market.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Pool {
    pub bump: u8,
    pub _padding: [u8; 7],
    /// The conditional-tokens market this pool trades on.
    pub market: Pubkey,
    pub yes_mint: Pubkey,
    pub no_mint: Pubkey,
    /// Pool's YES token account (PDA, authority = pool).
    pub yes_vault: Pubkey,
    /// Pool's NO token account (PDA, authority = pool).
    pub no_vault: Pubkey,
    /// Creator / subsidizer; can withdraw remaining subsidy after market
    /// is resolved if the pool's winning-side balance exceeds outstanding
    /// claims. (Withdrawal flow is a v1.1 addition.)
    pub authority: Pubkey,
    /// Current YES reserves (mirror of `yes_vault.amount`, cached for
    /// O(1) curve maths).
    pub yes_reserves: u64,
    /// Current NO reserves.
    pub no_reserves: u64,
    /// Swap fee in basis points (max 1000 = 10%).
    pub fee_bps: u16,
    pub _padding2: [u8; 6],
}

impl Pool {
    pub const LEN: usize = core::mem::size_of::<Self>();
    pub const SEED: &'static [u8] = b"pool";
    pub const YES_VAULT_SEED: &'static [u8] = b"yes-vault";
    pub const NO_VAULT_SEED: &'static [u8] = b"no-vault";

    pub fn from_data(d: &[u8]) -> Result<&Self, ProgramError> {
        if d.len() != Self::LEN {
            return Err(LmsrError::InvalidAccountData.into());
        }
        Ok(unsafe { &*(d.as_ptr() as *const Self) })
    }

    pub fn from_data_mut(d: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if d.len() != Self::LEN {
            return Err(LmsrError::InvalidAccountData.into());
        }
        Ok(unsafe { &mut *(d.as_mut_ptr() as *mut Self) })
    }
}

const _: () = assert!(Pool::LEN == 224);
