use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    pubkey::find_program_address,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};
use pinocchio_pubkey::derive_address;
use pinocchio_token::instructions::{CloseAccount, Transfer};

use crate::{
    error::{to_program_error, FundRaiserError},
    state::{Contributor, FundRaiser},
};

pub fn process_refund(accounts: &[AccountInfo]) -> ProgramResult {
    let [contributor, maker, mint_to_raise, fundraiser, contributor_account, contributor_ata, vault, token_program, system_program, _remaining @ ..] =
        accounts
    else {
        return Err(pinocchio::program_error::ProgramError::NotEnoughAccountKeys);
    };

    //contributor should be signer
    if !contributor.is_signer() {
        return Err(pinocchio::program_error::ProgramError::MissingRequiredSignature);
    }
    {
        //verify fundraiser pda and it's mint
        if fundraiser.lamports() == 0 || fundraiser.data_is_empty() {
            return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
        }
        let fundraiser_state = FundRaiser::from_account_info(fundraiser)?;
        let bump = fundraiser_state.bump();
        let seed = [b"fundraiser".as_ref(), maker.key().as_ref(), &[bump]];
        let fundraiser_pda = derive_address(&seed, None, &crate::ID);
        assert_eq!(fundraiser_pda, *fundraiser.key());

        let fundraiser_mint_to_raise = fundraiser_state.mint_to_raise();
        assert_eq!(mint_to_raise.key(), &fundraiser_mint_to_raise);

        //verify contributor_account pda  -> close the end of the program
        if contributor_account.lamports() == 0 || contributor_account.data_is_empty() {
            return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
        }
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

        if fundraiser_state.duration() as u64
            > (Clock::get()?.unix_timestamp as u64 - fundraiser_state.time_started())
        {
            return Err(to_program_error(FundRaiserError::DurationNotReached));
        }

        if vault_ata_account.amount() >= fundraiser_state.amount_to_raise() {
            return Err(to_program_error(FundRaiserError::TargetMet));
        }
    }

    let fundraiser_state = FundRaiser::from_account_info(fundraiser)?;
    let bump = [fundraiser_state.bump()];
    let seed = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.key().as_ref()),
        Seed::from(&bump),
    ];
    let signer_seeds = Signer::from(&seed);
    drop(fundraiser_state);
    {
        let amount = Contributor::from_account_info(contributor_account)?.amount();
        Transfer {
            from: vault,
            to: contributor_ata,
            authority: fundraiser,
            amount,
        }
        .invoke_signed(&[signer_seeds])?;

        let fundraiser_state = FundRaiser::from_mut_account_info(fundraiser)?;
        fundraiser_state.subtract_current_amount(amount);
    }

    // CloseAccount {
    //     account: contributor_account,
    //     destination: contributor,
    //     authority: contributor,
    // }
    // .invoke()?;

    Ok(())
}
