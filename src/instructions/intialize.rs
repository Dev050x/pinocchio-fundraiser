use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    pubkey::find_program_address,
    sysvars::{clock::Clock, rent::Rent, Sysvar},
    ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;

use crate::state::FundRaiser;

pub fn process_initialize(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [maker, mint_to_raise, fundraiser, vault, system_program, token_program, _associated_token_program, _remaining @ ..] =
        accounts
    else {
        return Err(pinocchio::program_error::ProgramError::NotEnoughAccountKeys);
    };

    let amount_to_raise = unsafe { *(data.as_ptr() as *const u64) };
    let duration = unsafe { *(data.as_ptr().add(8) as *const u8) };

    if amount_to_raise < FundRaiser::MIN_AMOUNT_TO_RAISE {
        return Err(pinocchio::program_error::ProgramError::InvalidArgument);
    }

    if duration == 0 {
        return Err(pinocchio::program_error::ProgramError::InvalidArgument);
    }

    // Verify Signer
    if !maker.is_signer() {
        return Err(pinocchio::program_error::ProgramError::MissingRequiredSignature);
    }

    {
        // Verify mint_to_raise weather mint is intialized or not
        let mint = pinocchio_token::state::Mint::from_account_info(&mint_to_raise)?;
        if !mint.is_initialized() {
            return Err(pinocchio::program_error::ProgramError::UninitializedAccount);
        }

        // verify vault address (if address is wrong then Create will fail)
        if vault.lamports() != 0 || !vault.data_is_empty() {
            return Err(pinocchio::program_error::ProgramError::AccountAlreadyInitialized);
        }
    }

    // verify fundraiser address with PDA
    if fundraiser.lamports() != 0 || !fundraiser.data_is_empty() {
        return Err(pinocchio::program_error::ProgramError::AccountAlreadyInitialized);
    }
    let (fundraiser_pda, bump) =
        find_program_address(&[b"fundraiser", maker.key().as_ref()], &crate::ID);
    assert_eq!(fundraiser_pda, *fundraiser.key()); //bcz we're creating if using address not like vault

    let bumps = [bump.to_le()];
    let seed = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.key().as_ref()),
        Seed::from(&bumps),
    ];
    let seeds = Signer::from(&seed);

    // create fundraiser account(onchain)

    CreateAccount {
        from: maker,
        to: fundraiser,
        lamports: Rent::get()?.minimum_balance(FundRaiser::LEN),
        space: FundRaiser::LEN as u64,
        owner: &crate::ID,
    }
    .invoke_signed(&[seeds])?;

    // create vault (onchain)
    pinocchio_associated_token_account::instructions::Create {
        funding_account: maker,
        account: vault,
        wallet: fundraiser,
        mint: mint_to_raise,
        system_program,
        token_program,
    }
    .invoke()?;

    {
        // initialize fundraiser account(onchain) check mininum threashold
        let fundraiser_state = FundRaiser::from_account_info(fundraiser)?;
        fundraiser_state.set_maker(maker.key());
        fundraiser_state.set_mint_to_raise(mint_to_raise.key());
        fundraiser_state.set_amount_to_raise(amount_to_raise);
        fundraiser_state.set_current_amount(0);
        fundraiser_state.set_time_started(Clock::get()?.unix_timestamp);
        fundraiser_state.set_duration(duration);
        fundraiser_state.set_bump(bump);
    }
    Ok(())
}
