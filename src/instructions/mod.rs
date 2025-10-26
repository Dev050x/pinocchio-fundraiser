pub mod check_contribution;
pub mod contribute;
pub mod intialize;
pub mod refund;

pub use check_contribution::*;
pub use contribute::*;
pub use intialize::*;
pub use refund::*;

pub enum Instruction {
    Initialize = 0,
    Contribute = 1,
    Refund = 2,
    Check = 3,
}

impl TryFrom<&u8> for Instruction {
    type Error = pinocchio::program_error::ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Instruction::Initialize),
            1 => Ok(Instruction::Contribute),
            2 => Ok(Instruction::Refund),
            3 => Ok(Instruction::Check),
            _ => Err(pinocchio::program_error::ProgramError::InvalidInstructionData),
        }
    }
}
