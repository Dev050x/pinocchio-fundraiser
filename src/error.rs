use pinocchio::program_error::ProgramError;

#[derive(Debug)]
pub enum FundRaiserError {
    InsufficientFundRaised,
    DurationNotReached,
    TargetMet,
}

pub fn to_program_error(err: FundRaiserError) -> ProgramError {
    match err {
        FundRaiserError::InsufficientFundRaised => ProgramError::Custom(0x10),
        FundRaiserError::DurationNotReached => ProgramError::Custom(0x11),
        FundRaiserError::TargetMet => ProgramError::Custom(0x12),
    }
}
