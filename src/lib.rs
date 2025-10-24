#![no_std]

use pinocchio::{
    account_info::AccountInfo, default_panic_handler, no_allocator, program_entrypoint,
    pubkey::Pubkey, ProgramResult,
};

use crate::instructions::Instruction;

mod constant;
mod instructions;
mod state;
mod tests;

program_entrypoint!(process_instruction);
no_allocator!();
default_panic_handler!();

// Currently Random Program ID
pinocchio_pubkey::declare_id!("Fg6PaFpoGXkYsidMpWxTWqfQRyQ4aW5n5g5g5g5g5g5g");

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    assert_eq!(program_id, &ID);
    let (discriminator, data) = instruction_data
        .split_first()
        .ok_or(pinocchio::program_error::ProgramError::InvalidInstructionData)?;

    match Instruction::try_from(discriminator)? {
        Instruction::Initialize => instructions::intialize::process_initialize(accounts, data)?,
        Instruction::Contribute => instructions::contribute::process_contribute(accounts, data)?,
        Instruction::Refund => instructions::refund::process_refund()?,
        _ => return Err(pinocchio::program_error::ProgramError::InvalidInstructionData),
    }

    Ok(())
}
