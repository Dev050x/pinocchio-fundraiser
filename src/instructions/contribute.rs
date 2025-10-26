use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    pubkey::find_program_address,
    sysvars::{clock::Clock, rent::Rent, Sysvar},
    ProgramResult,
};
use pinocchio_pubkey::derive_address;
use pinocchio_system::instructions::CreateAccount;

use crate::{
    constant::{MAX_CONTRIBUTION_PERCENTAGE, PERCENTAGE_SCALER},
    state::{Contributor, FundRaiser},
};

pub fn process_contribute(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [contributor, mint_to_raise, fundraiser, contributor_account, contributor_ata, vault, token_program, system_program, remaining @ ..] =
        accounts
    else {
        return Err(pinocchio::program_error::ProgramError::NotEnoughAccountKeys);
    };

    let amount_to_contribute = unsafe { *(data.as_ptr() as *const u64) };

    //contributor should be signer
    if !contributor.is_signer() {
        return Err(pinocchio::program_error::ProgramError::MissingRequiredSignature);
    }

    //verify mint_to_raise is same as fundraiser.mint_to_raise
    if fundraiser.lamports() == 0 || fundraiser.data_is_empty() {
        return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
    }
    let fundraiser_state = FundRaiser::from_account_info(fundraiser)?;
    let fundraiser_mint_to_raise = fundraiser_state.mint_to_raise();
    assert_eq!(mint_to_raise.key(), &fundraiser_mint_to_raise);

    // Check if the amount to contribute is less than the maximum allowed contribution
    if amount_to_contribute
        > (fundraiser_state.amount_to_raise() * MAX_CONTRIBUTION_PERCENTAGE) / PERCENTAGE_SCALER
    {
        return Err(pinocchio::program_error::ProgramError::InvalidInstructionData);
    }

    // Check if the fundraising duration has been reached
    if (Clock::get()?.unix_timestamp as u64 - fundraiser_state.time_started())
        > fundraiser_state.duration() as u64
    {
        return Err(pinocchio::program_error::ProgramError::InvalidInstructionData);
    }

    {
        //verify fundraiser pda
        let bump = fundraiser_state.bump();
        let fundraiser_maker = fundraiser_state.maker();
        let seed = [b"fundraiser".as_ref(), fundraiser_maker.as_ref(), &[bump]];
        let fundraiser_pda = derive_address(&seed, None, &crate::ID);
        assert_eq!(fundraiser_pda, *fundraiser.key());

        //verify contributor account init_if_needed (b"contributor", fundraiser.key(), contributor.key())
        let (contributor_account_pda, contributor_bump) = find_program_address(
            &[
                b"contributor",
                fundraiser.key().as_ref(),
                contributor.key().as_ref(),
            ],
            &crate::ID,
        );
        assert_eq!(contributor_account_pda, *contributor_account.key());

        //verify contributor ata (mint , authority-contributor)
        let contributor_ata_account =
            pinocchio_token::state::TokenAccount::from_account_info(contributor_ata)?;
        assert_eq!(contributor_ata_account.mint(), &fundraiser_mint_to_raise);
        assert_eq!(contributor_ata_account.owner(), contributor.key());

        //verify vault ata (mint , authority-fundraiser)
        let vault_ata_account = pinocchio_token::state::TokenAccount::from_account_info(vault)?;
        assert_eq!(vault_ata_account.mint(), &fundraiser_mint_to_raise);
        assert_eq!(vault_ata_account.owner(), fundraiser.key());

        // Check if the amount to contribute meets the minimum amount required
        let mint_account = pinocchio_token::state::Mint::from_account_info(&mint_to_raise)?;
        let decimals = mint_account.decimals();
        if amount_to_contribute < 10_u8.pow(decimals as u32) as u64 {
            return Err(pinocchio::program_error::ProgramError::InvalidInstructionData);
        }

        let contributor_bump_array = [contributor_bump.to_le()];
        let contributor_seed = [
            Seed::from(b"contributor"),
            Seed::from(fundraiser.key().as_ref()),
            Seed::from(contributor.key().as_ref()),
            Seed::from(&contributor_bump_array),
        ];

        let contributor_singers = Signer::from(&contributor_seed);

        //create contributor account init_if_needed (b"contributor", fundraiser.key(), contributor.key())
        if contributor_account.lamports() == 0 && contributor_account.data_is_empty() {
            //create account
            CreateAccount {
                from: contributor,
                to: contributor_account,
                lamports: Rent::get()?.minimum_balance(Contributor::LEN),
                owner: &crate::ID,
                space: Contributor::LEN as u64,
            }
            .invoke_signed(&[contributor_singers])?;

            //initialize account
            let contributor_account_state = Contributor::from_account_info(contributor_account)?;
            contributor_account_state.set_amount(0);
        }
    }

    // Check if the maximum contributions per contributor have been reached
    {
        let contributor_account_state = Contributor::from_account_info(contributor_account)?;
        let total_contribution = contributor_account_state.amount() + amount_to_contribute;
        let total_contribution_cap =
            (fundraiser_state.amount_to_raise() * MAX_CONTRIBUTION_PERCENTAGE) / PERCENTAGE_SCALER;
        if total_contribution > total_contribution_cap {
            return Err(pinocchio::program_error::ProgramError::InvalidInstructionData);
        }
    }

    //transfer fund to contributor ata to vault
    pinocchio_token::instructions::Transfer {
        from: contributor_ata,
        to: vault,
        authority: contributor,
        amount: amount_to_contribute,
    }
    .invoke()?;

    //update fundraiser account
    {
        let fundraiser_state = FundRaiser::from_mut_account_info(&fundraiser)?;
        fundraiser_state.update_current_amount(amount_to_contribute);
    }

    //update contributor account
    {
        let contributor_account_state = Contributor::from_account_info(contributor_account)?;
        contributor_account_state.update_amount(amount_to_contribute);
    }

    Ok(())
}
