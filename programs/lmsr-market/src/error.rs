use pinocchio::program_error::ProgramError;

#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum LmsrError {
    InvalidInstructionData = 0,
    InvalidAccountData = 1,
    PoolAlreadyInitialized = 2,
    InvalidPda = 3,
    InvalidMint = 4,
    InvalidVault = 5,
    InvalidMarket = 6,
    InsufficientLiquidity = 7,
    SlippageExceeded = 8,
    MathOverflow = 9,
    ZeroAmount = 10,
    InvalidFee = 11,
}

impl From<LmsrError> for ProgramError {
    fn from(e: LmsrError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
