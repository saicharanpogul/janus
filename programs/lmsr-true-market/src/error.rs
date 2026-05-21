use pinocchio::program_error::ProgramError;

#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum LmsrTrueError {
    InvalidInstructionData = 0,
    InvalidAccountData = 1,
    PoolAlreadyInitialized = 2,
    InvalidPda = 3,
    InvalidMint = 4,
    InvalidVault = 5,
    InvalidCollateralMint = 6,
    ZeroAmount = 7,
    InvalidLiquidityParameter = 8,
    MathOverflow = 9,
    SlippageExceeded = 10,
    /// Resolution-related errors (post-MVP; reserved for the resolve path).
    MarketNotResolved = 11,
    PoolStillActive = 12,
    /// `Buy` / `Sell` after status != Active.
    PoolNotActive = 13,
}

impl From<LmsrTrueError> for ProgramError {
    fn from(e: LmsrTrueError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
