use pinocchio::{program_error::ProgramError, pubkey::Pubkey};

use crate::error::ConditionalTokensError;

/// Market lifecycle status.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarketStatus {
    /// Market is open; users can split, merge, and trade outcome tokens.
    Active = 0,
    /// Resolver reported YES; YES holders may redeem 1:1.
    ResolvedYes = 1,
    /// Resolver reported NO; NO holders may redeem 1:1.
    ResolvedNo = 2,
    /// Resolver reported INVALID; users unwind via merge instead of redeem.
    ResolvedInvalid = 3,
}

impl TryFrom<u8> for MarketStatus {
    type Error = ProgramError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0 => Ok(Self::Active),
            1 => Ok(Self::ResolvedYes),
            2 => Ok(Self::ResolvedNo),
            3 => Ok(Self::ResolvedInvalid),
            _ => Err(ConditionalTokensError::InvalidAccountData.into()),
        }
    }
}

/// Persistent on-chain Market state.
///
/// Layout is fixed `#[repr(C)]` so it can be safely cast over raw account
/// data. Total size: 240 bytes.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Market {
    /// PDA bump for the market account itself.
    pub bump: u8,
    /// Lifecycle status; see [`MarketStatus`].
    pub status: u8,
    /// Reserved for future use; keeps the following pubkeys 8-byte aligned.
    pub _padding: [u8; 6],
    /// SPL mint accepted as collateral (e.g. USDC).
    pub collateral_mint: Pubkey,
    /// Mint of the YES outcome token (PDA owned by this program).
    pub yes_mint: Pubkey,
    /// Mint of the NO outcome token (PDA owned by this program).
    pub no_mint: Pubkey,
    /// Token account holding the collateral backing the market (PDA).
    pub vault: Pubkey,
    /// Program ID of the bound resolver implementation.
    pub resolver_program: Pubkey,
    /// State account the resolver reads to determine the outcome.
    pub resolver_state: Pubkey,
    /// Authority allowed to call the `Resolve` instruction.
    pub authority: Pubkey,
    /// Earliest slot at which the market may be resolved.
    pub deadline_slot: u64,
    /// Slot the market was initialized at; useful for indexing.
    pub created_at_slot: u64,
}

impl Market {
    pub const LEN: usize = core::mem::size_of::<Self>();

    pub const SEED: &'static [u8] = b"market";
    pub const YES_MINT_SEED: &'static [u8] = b"yes";
    pub const NO_MINT_SEED: &'static [u8] = b"no";
    pub const VAULT_SEED: &'static [u8] = b"vault";

    /// View an immutable reference to a `Market` over raw account data.
    ///
    /// # Safety
    /// Caller must guarantee `data.len() == Market::LEN` and that the bytes
    /// were written by this program.
    #[inline(always)]
    pub fn from_account_data(data: &[u8]) -> Result<&Self, ProgramError> {
        if data.len() != Self::LEN {
            return Err(ConditionalTokensError::InvalidAccountData.into());
        }
        // SAFETY: layout is `#[repr(C)]` and the size check above guarantees
        // sufficient bytes; alignment is satisfied because Solana account data
        // is always 8-byte aligned and `Market`'s alignment is 1 (no >u8
        // fields with stricter alignment than 8).
        Ok(unsafe { &*(data.as_ptr() as *const Self) })
    }

    #[inline(always)]
    pub fn from_account_data_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if data.len() != Self::LEN {
            return Err(ConditionalTokensError::InvalidAccountData.into());
        }
        // SAFETY: see `from_account_data`.
        Ok(unsafe { &mut *(data.as_mut_ptr() as *mut Self) })
    }
}

// Compile-time assertion that the on-chain layout is exactly 248 bytes.
const _: () = assert!(Market::LEN == 248);
