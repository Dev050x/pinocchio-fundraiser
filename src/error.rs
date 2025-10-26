use pinocchio::program_error::ProgramError;

#[derive(Debug)]
pub enum FundRaiserError {
    InsufficientFundRaised,
}

pub fn to_program_error(err: FundRaiserError) -> ProgramError {
    match err {
        FundRaiserError::InsufficientFundRaised => ProgramError::Custom(0x10),
    }
}
