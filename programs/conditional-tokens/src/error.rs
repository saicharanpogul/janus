use pinocchio::program_error::ProgramError;

#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum ConditionalTokensError {
    InvalidInstructionData = 0,
    InvalidAccountData = 1,
    MarketAlreadyInitialized = 2,
    MarketNotInitialized = 3,
    MarketNotResolved = 4,
    MarketAlreadyResolved = 5,
    MarketResolvedInvalid = 6,
    InvalidPda = 7,
    InvalidMint = 8,
    InvalidVault = 9,
    InvalidResolver = 10,
    DeadlineNotReached = 11,
    DeadlineInPast = 12,
    Unauthorized = 13,
    MathOverflow = 14,
}

impl From<ConditionalTokensError> for ProgramError {
    fn from(e: ConditionalTokensError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
