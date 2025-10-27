use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    pubkey::find_program_address,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};
use pinocchio_associated_token_account::instructions::Create;

use crate::{
    error::{to_program_error, FundRaiserError},
    state::FundRaiser,
};

pub fn process_check_contribution(accounts: &[AccountInfo]) -> ProgramResult {
    let [maker, mint_to_raise, fundraiser, vault, maker_ata, token_program, system_program, _associated_token_program, _remainig @ ..] =
        accounts
    else {
        return Err(pinocchio::program_error::ProgramError::NotEnoughAccountKeys);
    };

    // maker should be signer
    if !maker.is_signer() {
        return Err(pinocchio::program_error::ProgramError::MissingRequiredSignature);
    }

    //verify mint_to_raise is same as fundraiser.mint_to_raise
    if fundraiser.lamports() == 0 || fundraiser.data_is_empty() {
        return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
    }

    {
        let fundraiser_state = FundRaiser::from_account_info(fundraiser)?;
        let fundraiser_mint_to_raise = fundraiser_state.mint_to_raise();
        assert_eq!(mint_to_raise.key(), &fundraiser_mint_to_raise);

        //verify fundraise pda -> close fundraiser account at the end(send to maker)
        let fundraiser_pda =
            find_program_address(&[b"fundraiser".as_ref(), maker.key().as_ref()], &crate::ID).0;
        assert_eq!(fundraiser_pda, *fundraiser.key());

        // verify vault(it's atat)
        let vault_ata = pinocchio_token::state::TokenAccount::from_account_info(&vault)?;
        assert_eq!(vault_ata.mint(), mint_to_raise.key());
        assert_eq!(vault_ata.owner(), fundraiser.key());
    }

    //check maker_ata if exists then check mint(should be mint_to_raise) & authority(should be maker)
    if maker_ata.lamports() == 0 && maker_ata.data_is_empty() {
        Create {
            funding_account: maker,
            account: maker_ata,
            wallet: maker,
            mint: mint_to_raise,
            system_program,
            token_program,
        }
        .invoke()?;
    } else {
        let maker_ata_account =
            pinocchio_token::state::TokenAccount::from_account_info(&maker_ata)?;
        assert_eq!(maker_ata_account.mint(), mint_to_raise.key());
        assert_eq!(maker_ata_account.owner(), maker.key());
    }

    {
        let fundraiser_state = FundRaiser::from_account_info(&fundraiser)?;
        let vault_state = pinocchio_token::state::TokenAccount::from_account_info(&vault)?;

        let vault_amount = vault_state.amount();
        let amount_to_raise = fundraiser_state.amount_to_raise();
        let time_started = fundraiser_state.time_started();
        let duration = fundraiser_state.duration();
        let bump = fundraiser_state.bump();

        drop(fundraiser_state);
        drop(vault_state);

        if vault_amount >= amount_to_raise
            && Clock::get()?.unix_timestamp as u64 - time_started >= duration as u64
        {
            let bump = [bump];
            let seed = [
                Seed::from(b"fundraiser"),
                Seed::from(maker.key().as_ref()),
                Seed::from(&bump),
            ];
            let signer_seeds = Signer::from(&seed);
            pinocchio_token::instructions::Transfer {
                from: vault,
                to: maker_ata,
                authority: fundraiser,
                amount: vault_amount,
            }
            .invoke_signed(&[signer_seeds.clone()])?;

            pinocchio_token::instructions::CloseAccount {
                account: vault,
                destination: maker,
                authority: fundraiser,
            }
            .invoke_signed(&[signer_seeds])?;

            unsafe {
                *maker.borrow_mut_lamports_unchecked() += fundraiser.lamports();
                *fundraiser.borrow_mut_lamports_unchecked() = 0;
            }

            let mut fundraiser_data = fundraiser.try_borrow_mut_data()?;
            fundraiser_data.fill(0);
        } else {
            return Err(to_program_error(FundRaiserError::InsufficientFundRaised));
        }
    }

    Ok(())
}
