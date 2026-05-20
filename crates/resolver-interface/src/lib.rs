//! # janus-resolver-interface
//!
//! Standardized interface every Janus resolver program implements.
//!
//! A resolver is any Solana program that exposes a single `Resolve`
//! instruction at discriminator [`RESOLVE_INSTRUCTION_TAG`] which:
//!
//! 1. Reads its own state plus whatever oracle/source accounts it needs.
//! 2. Computes a [`ResolutionOutcome`] (or returns `Unresolved` if it
//!    cannot yet).
//! 3. Writes the outcome as a single byte to Solana's return-data buffer
//!    via `set_return_data`.
//!
//! The conditional-tokens program reads this byte back with
//! `get_return_data` after CPIing into the resolver and updates the
//! market's status accordingly. This keeps resolvers fully pluggable —
//! a market only knows *which* program it's bound to, never *how* that
//! program determines truth.

#![no_std]

use pinocchio::program_error::ProgramError;

/// Instruction discriminator every resolver program must use for its
/// `Resolve` entrypoint.
pub const RESOLVE_INSTRUCTION_TAG: u8 = 0;

/// Outcome a resolver reports back via `set_return_data`.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolutionOutcome {
    /// Resolver cannot yet determine an outcome (e.g. data not posted yet).
    Unresolved = 0,
    /// YES side wins.
    Yes = 1,
    /// NO side wins.
    No = 2,
    /// Outcome is permanently undecidable (oracle missing, parameters bad).
    /// Market should unwind via merge.
    Invalid = 3,
}

impl TryFrom<u8> for ResolutionOutcome {
    type Error = ProgramError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0 => Ok(Self::Unresolved),
            1 => Ok(Self::Yes),
            2 => Ok(Self::No),
            3 => Ok(Self::Invalid),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

impl ResolutionOutcome {
    #[inline(always)]
    pub fn as_byte(self) -> u8 {
        self as u8
    }
}
