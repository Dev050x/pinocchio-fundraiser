#[cfg(test)]
mod tests {

    use std::{path::PathBuf, vec};

    use litesvm::LiteSVM;
    use litesvm_token::{
        get_spl_account,
        spl_token::{
            self,
            solana_program::{msg, rent::Rent, sysvar::SysvarId},
            state::Mint,
            ID as TOKEN_PROGRAM_ID,
        },
        CreateAssociatedTokenAccount, CreateMint, MintTo,
    };

    use solana_instruction::{AccountMeta, Instruction};
    use solana_keypair::Keypair;
    use solana_message::Message;
    use solana_native_token::LAMPORTS_PER_SOL;
    use solana_pubkey::Pubkey;
    use solana_sdk_ids::system_program::ID as SYSTEM_PROGRAM_ID;
    use solana_signer::Signer;
    use solana_transaction::Transaction;
    use spl_associated_token_account::{
        get_associated_token_address_with_program_id,
        solana_program::{program_pack::Pack, sysvar::recent_blockhashes},
        ID as ASSOCIATED_TOKEN_PROGRAM_ID,
    };

    use crate::{
        constant::SECONDS_TO_DAYS,
        state::{fundraiser, FundRaiser},
    };

    // const PROGRAM_ID: Pubkey = Pubkey::from(crate::ID);
    fn program_id() -> Pubkey {
        Pubkey::from(crate::ID)
    }

    struct Helper {
        program: LiteSVM,
        payer: Keypair,
        contributor: Keypair,
        mint: Pubkey,
        fundraiser: Pubkey,
        contributor_account: Pubkey,
        contributor_ata: Pubkey,
        vault: Pubkey,
        maker_ata: Pubkey,
        system_program: Pubkey,
        token_program: Pubkey,
        associated_token_program: Pubkey,
    }

    impl Helper {
        fn new() -> Self {
            let mut svm = LiteSVM::new();
            let payer = Keypair::new();
            let contributor = Keypair::new();

            svm.airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL)
                .expect("Airdrop failed");
            svm.airdrop(&contributor.pubkey(), 10 * LAMPORTS_PER_SOL)
                .expect("Airdrop failed");

            // Load program SO file
            msg!("The path is!! {}", env!("CARGO_MANIFEST_DIR"));
            let so_path = PathBuf::from("/home/div/Desktop/Turbin3-Classwork/pinocchio-fundraising/target/sbpf-solana-solana/release/pinocchio_fundraising.so");
            msg!("The path is!! {:?}", so_path);

            let program_data = std::fs::read(so_path).expect("Failed to read program SO file");

            svm.add_program(program_id(), &program_data);

            let mint = CreateMint::new(&mut svm, &payer)
                .decimals(6)
                .authority(&payer.pubkey())
                .send()
                .unwrap();
            msg!("Mint A: {}", mint);

            let fundraiser = Pubkey::find_program_address(
                &[b"fundraiser".as_ref(), payer.pubkey().as_ref()],
                &program_id(),
            );
            msg!("Fundraiser PDA: {}", fundraiser.0);

            let vault = get_associated_token_address_with_program_id(
                &fundraiser.0,
                &mint,
                &TOKEN_PROGRAM_ID,
            );
            msg!("Vault ATA: {}", vault);

            let contributor_account = Pubkey::find_program_address(
                &[
                    b"contributor".as_ref(),
                    fundraiser.0.as_ref(),
                    contributor.pubkey().as_ref(),
                ],
                &program_id(),
            );
            msg!("Contributor account PDA: {}", contributor_account.0);

            let contributor_ata = CreateAssociatedTokenAccount::new(&mut svm, &contributor, &mint)
                .owner(&contributor.pubkey())
                .send()
                .unwrap();
            MintTo::new(&mut svm, &payer, &mint, &contributor_ata, 100_000_000)
                .send()
                .unwrap();
            msg!("Contributor ATA: {}", contributor_ata);

            let maker_ata = get_associated_token_address_with_program_id(
                &payer.pubkey(),
                &mint,
                &TOKEN_PROGRAM_ID,
            );
            msg!("Maker ATA: {}", maker_ata);

            Self {
                program: svm,
                payer,
                contributor,
                mint,
                fundraiser: fundraiser.0,
                contributor_account: contributor_account.0,
                contributor_ata,
                vault,
                maker_ata,
                system_program: SYSTEM_PROGRAM_ID,
                token_program: TOKEN_PROGRAM_ID,
                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            }
        }

        pub fn send_initialize_txn(&mut self, amount: u64, duration: u8) {
            let initialize_ix_data = [
                vec![0u8],
                amount.to_le_bytes().to_vec(),
                duration.to_le_bytes().to_vec(),
            ]
            .concat();

            let initialize_ix = Instruction {
                program_id: program_id(),
                accounts: vec![
                    AccountMeta::new(self.payer.pubkey(), true),
                    AccountMeta::new(self.mint, false),
                    AccountMeta::new(self.fundraiser, false),
                    AccountMeta::new(self.vault, false),
                    AccountMeta::new(self.system_program, false),
                    AccountMeta::new(self.token_program, false),
                    AccountMeta::new(self.associated_token_program, false),
                ],
                data: initialize_ix_data,
            };
            let message = Message::new(&[initialize_ix], Some(&self.payer.pubkey()));
            let recent_blockhashes = self.program.latest_blockhash();
            let transaction = Transaction::new(&[&self.payer], message, recent_blockhashes);

            let tx = self.program.send_transaction(transaction);
            if tx.is_err() {
                panic!("Transaction failed: {:?}", tx.err());
            }
            let tx = tx.unwrap();
            msg!(
                "Initialize Transaction succeeded with signature: {}",
                tx.signature
            );
            msg!("Compute Units Consumed: {}", tx.compute_units_consumed);
        }

        pub fn send_contribute_txn(&mut self, amount: u64) {
            let contribute_ix_data = [vec![1u8], amount.to_le_bytes().to_vec()].concat();

            let contribute_ix = Instruction {
                program_id: program_id(),
                accounts: vec![
                    AccountMeta::new(self.contributor.pubkey(), true),
                    AccountMeta::new(self.mint, false),
                    AccountMeta::new(self.fundraiser, false),
                    AccountMeta::new(self.contributor_account, false),
                    AccountMeta::new(self.contributor_ata, false),
                    AccountMeta::new(self.vault, false),
                    AccountMeta::new(self.token_program, false),
                    AccountMeta::new(self.system_program, false),
                ],
                data: contribute_ix_data,
            };

            let message = Message::new(&[contribute_ix], Some(&self.contributor.pubkey()));
            let recent_blockhashes = self.program.latest_blockhash();
            let transaction = Transaction::new(&[&self.contributor], message, recent_blockhashes);

            let tx = self.program.send_transaction(transaction);
            if tx.is_err() {
                panic!("Transaction failed: {:?}", tx.err());
            }
            let tx = tx.unwrap();
            msg!(
                "Contribute Transaction succeeded with signature: {}",
                tx.signature
            );
            msg!("Compute Units Consumed: {}", tx.compute_units_consumed);
        }

        pub fn send_check_txn(&mut self) {
            let check_ix_data = [vec![3u8]].concat();

            let check_ix = Instruction {
                program_id: program_id(),
                accounts: vec![
                    AccountMeta::new(self.payer.pubkey(), true),
                    AccountMeta::new(self.mint, false),
                    AccountMeta::new(self.fundraiser, false),
                    AccountMeta::new(self.vault, false),
                    AccountMeta::new(self.maker_ata, false),
                    AccountMeta::new(self.token_program, false),
                    AccountMeta::new(self.system_program, false),
                    AccountMeta::new(self.associated_token_program, false),
                ],
                data: check_ix_data,
            };

            let message = Message::new(&[check_ix], Some(&self.payer.pubkey()));
            let recent_blockhashes = self.program.latest_blockhash();
            let transaction = Transaction::new(&[&self.payer], message, recent_blockhashes);

            let tx = self.program.send_transaction(transaction);
            if tx.is_err() {
                panic!("Transaction failed: {:?}", tx.err());
            }
            let tx = tx.unwrap();
            msg!(
                "Check Transaction succeeded with signature: {}",
                tx.signature
            );
            msg!("Compute Units Consumed: {}", tx.compute_units_consumed);
        }

        pub fn change_contributor_and_send_txn(&mut self) {
            self.contributor = Keypair::new();
            self.program
                .airdrop(&self.contributor.pubkey(), 10 * LAMPORTS_PER_SOL)
                .expect("Airdrop failed");

            let contributor_account = Pubkey::find_program_address(
                &[
                    b"contributor".as_ref(),
                    self.fundraiser.as_ref(),
                    self.contributor.pubkey().as_ref(),
                ],
                &program_id(),
            );
            self.contributor_account = contributor_account.0;
            msg!("new Contributor account PDA: {}", self.contributor_account);

            let contributor_ata =
                CreateAssociatedTokenAccount::new(&mut self.program, &self.contributor, &self.mint)
                    .owner(&self.contributor.pubkey())
                    .send()
                    .unwrap();
            self.contributor_ata = contributor_ata;
            MintTo::new(
                &mut self.program,
                &self.payer,
                &self.mint,
                &self.contributor_ata,
                100_000_000,
            )
            .send()
            .unwrap();
            msg!("new Contributor ATA: {}", self.contributor_ata);

            self.send_contribute_txn(1_000_000);
        }
    }

    #[test]
    fn test_initialize() {
        let mut helper = Helper::new();
        helper.send_initialize_txn(10_000_000, 1);

        //assert vault
        let vault_data = helper.program.get_account(&helper.vault).unwrap();
        let vault = spl_token::state::Account::unpack(&vault_data.data).unwrap();
        assert_eq!(vault.amount, 0);
        assert_eq!(vault.owner, helper.fundraiser);
        assert_eq!(vault.mint, helper.mint);
    }

    #[test]
    fn test_contribute() {
        let mut helper = Helper::new();
        helper.send_initialize_txn(10_000_000, 1);
        helper.send_contribute_txn(1_000_000);

        //assert contributor account
        let contributor_ata_data = helper.program.get_account(&helper.contributor_ata).unwrap();
        let contributor_ata =
            spl_token::state::Account::unpack(&contributor_ata_data.data).unwrap();
        assert_eq!(contributor_ata.amount, 100_000_000 - 1_000_000);

        //assert vault
        let vault_data = helper.program.get_account(&helper.vault).unwrap();
        let vault = spl_token::state::Account::unpack(&vault_data.data).unwrap();
        assert_eq!(vault.amount, 1_000_000);
        assert_eq!(vault.owner, helper.fundraiser);
        assert_eq!(vault.mint, helper.mint);
    }

    #[test]
    fn test_check_contribution() {
        let mut helper = Helper::new();
        helper.send_initialize_txn(10_000_000, 1);

        helper.change_contributor_and_send_txn();
        helper.change_contributor_and_send_txn();
        helper.change_contributor_and_send_txn();
        helper.change_contributor_and_send_txn();
        helper.change_contributor_and_send_txn();

        helper.change_contributor_and_send_txn();
        helper.change_contributor_and_send_txn();
        helper.change_contributor_and_send_txn();
        helper.change_contributor_and_send_txn();
        helper.change_contributor_and_send_txn();

        //new clock time
        let mut clock = helper
            .program
            .get_sysvar::<spl_associated_token_account::solana_program::clock::Clock>();
        clock.unix_timestamp += SECONDS_TO_DAYS; // add 2 minutes
        helper.program.set_sysvar(&clock);
        helper.send_check_txn();
    }
}
