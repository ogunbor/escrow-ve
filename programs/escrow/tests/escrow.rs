#![cfg(test)]
use {
    anchor_lang::{
        prelude::msg,
        solana_program::{instruction::Instruction, program_pack::Pack},
        system_program::ID as SYSTEM_PROGRAM_ID,
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    anchor_spl::{
        associated_token::{self, ID as ASSOCIATED_TOKEN_PROGRAM_ID},
        token::spl_token,
    },
    escrow::Escrow,
    litesvm::{types::TransactionMetadata, LiteSVM},
    litesvm_token::{
        spl_token::ID as TOKEN_PROGRAM_ID, CreateAssociatedTokenAccount, CreateMint, MintTo,
    },
    solana_keypair::Keypair,
    solana_message::Message,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_transaction::Transaction,
};

pub struct EscrowTestBuilder {
    program_id: Pubkey,
    program: LiteSVM,
    maker: Keypair,
    taker: Option<Keypair>,
    mint_a: Option<Pubkey>,
    mint_b: Option<Pubkey>,
    maker_ata_a: Option<Pubkey>,
    maker_ata_b: Option<Pubkey>,
    taker_ata_a: Option<Pubkey>,
    taker_ata_b: Option<Pubkey>,
    escrow: Option<Pubkey>,
    vault: Option<Pubkey>,
    last_tx: Option<TransactionMetadata>,
    last_tx_error: Option<String>,
}

impl EscrowTestBuilder {
    pub fn new() -> Self {
        let program_id = escrow::id();

        let mut program = LiteSVM::new();
        let maker = Keypair::new();

        program
            .airdrop(&maker.pubkey(), 30_000_000_000)
            .expect("Failed to airdrop SOL to maker");

        let program_bytes =
            include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/escrow.so"));
        program.add_program(program_id, program_bytes).unwrap();

        Self {
            program_id,
            program,
            maker,
            taker: None,
            mint_a: None,
            mint_b: None,
            maker_ata_a: None,
            maker_ata_b: None,
            taker_ata_a: None,
            taker_ata_b: None,
            escrow: None,
            vault: None,
            last_tx: None,
            last_tx_error: None,
        }
    }

    /// Create two mints (Mint A and Mint B) with 6 decimal places and the maker as the authority
    pub fn create_mints(mut self) -> Self {
        let mint_a = CreateMint::new(&mut self.program, &self.maker)
            .decimals(6)
            .authority(&self.maker.pubkey())
            .send()
            .unwrap();
        msg!("Mint A: {}\n", mint_a);

        let mint_b = CreateMint::new(&mut self.program, &self.maker)
            .decimals(6)
            .authority(&self.maker.pubkey())
            .send()
            .unwrap();
        msg!("Mint B: {}\n", mint_b);

        self.mint_a = Some(mint_a);
        self.mint_b = Some(mint_b);
        self
    }

    /// Create the maker's associated token account for Mint A
    pub fn create_maker_ata_a(mut self) -> Self {
        let mint_a = self.mint_a.expect("Mint A not created");
        let maker_ata_a =
            CreateAssociatedTokenAccount::new(&mut self.program, &self.maker, &mint_a)
                .owner(&self.maker.pubkey())
                .send()
                .unwrap();

        msg!("Maker ATA A: {}\n", maker_ata_a);

        self.maker_ata_a = Some(maker_ata_a);
        self
    }

    /// Create the maker's associated token account for Mint B
    pub fn create_maker_ata_b(mut self) -> Self {
        let mint_b = self.mint_b.expect("Mint B not created");
        let maker_ata_b =
            CreateAssociatedTokenAccount::new(&mut self.program, &self.maker, &mint_b)
                .owner(&self.maker.pubkey())
                .send()
                .unwrap();

        msg!("Maker ATA B: {}\n", maker_ata_b);

        self.maker_ata_b = Some(maker_ata_b);
        self
    }

    /// Mint tokens to the maker's associated token account for Mint A
    pub fn mint_to_maker_ata_a(mut self, amount: u64) -> Self {
        MintTo::new(
            &mut self.program,
            &self.maker,
            &self.mint_a.unwrap(),
            &self.maker_ata_a.unwrap(),
            amount,
        )
        .send()
        .unwrap();

        self
    }

    /// Mint tokens to the taker's associated token account for Mint B
    pub fn mint_to_taker_ata_b(mut self, amount: u64) -> Self {
        MintTo::new(
            &mut self.program,
            &self.maker,
            &self.mint_b.unwrap(),
            &self.taker_ata_b.unwrap(),
            amount,
        )
        .send()
        .unwrap();

        self
    }

    pub fn setup_taker(mut self) -> Self {
        let taker = Keypair::new();
        self.program
            .airdrop(&taker.pubkey(), 20_000_000_000)
            .expect("Failed to airdrop SOL to taker");

        self.taker = Some(taker);
        self
    }

    pub fn create_taker_atas(mut self) -> Self {
        let taker = self.taker.as_ref().expect("Taker not created");
        let mint_a = self.mint_a.expect("Mint A not created");
        let mint_b = self.mint_b.expect("Mint B not created");

        let taker_ata_a = CreateAssociatedTokenAccount::new(&mut self.program, taker, &mint_a)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut self.program, taker, &mint_b)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        self.taker_ata_a = Some(taker_ata_a);
        self.taker_ata_b = Some(taker_ata_b);
        self
    }

    pub fn advance_time(mut self, seconds: i64) -> Self {
        use anchor_lang::prelude::Clock;
        let mut clock = self.program.get_sysvar::<Clock>();
        clock.unix_timestamp += seconds;

        self.program.set_sysvar::<Clock>(&clock);
        self
    }

    pub fn execute_make(mut self, deposit: u64, seed: u64, receive: u64, expiration: i64) -> Self {
        let escrow = Pubkey::find_program_address(
            &[b"escrow", self.maker.pubkey().as_ref(), &seed.to_le_bytes()],
            &self.program_id,
        )
        .0;
        msg!("Escrow PDA: {}\n", escrow);

        let vault = associated_token::get_associated_token_address(
            &escrow,
            &self.mint_a.expect("Mint A not created"),
        );
        msg!("Vault PDA: {}\n", vault);

        self.escrow = Some(escrow);
        self.vault = Some(vault);

        // "initialize" is the escrow program's make instruction
        let make_ix = Instruction {
            program_id: self.program_id,
            accounts: escrow::accounts::Make {
                maker: self.maker.pubkey(),
                mint_a: self.mint_a.unwrap(),
                mint_b: self.mint_b.unwrap(),
                maker_ata_a: self.maker_ata_a.unwrap(),
                escrow: self.escrow.unwrap(),
                vault: self.vault.unwrap(),
                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: escrow::instruction::Initialize {
                seed,
                receive,
                deposit,
                expiration,
            }
            .data(),
        };

        let message = Message::new(&[make_ix], Some(&self.maker.pubkey()));
        let recent_blockhash = self.program.latest_blockhash();
        let transaction = Transaction::new(&[&self.maker], message, recent_blockhash);

        let tx = self.program.send_transaction(transaction).unwrap();

        msg!("\n\nMake transaction successful");
        msg!("CUs Consumed: {}", tx.compute_units_consumed);
        msg!("Tx Signature: {}", tx.signature);

        self.last_tx = Some(tx);
        self.last_tx_error = None;
        self
    }

    pub fn execute_take(mut self) -> Self {
        let taker = self.taker.as_ref().expect("Taker not created");

        let take_ix = Instruction {
            program_id: self.program_id,
            accounts: escrow::accounts::Take {
                taker: taker.pubkey(),
                maker: self.maker.pubkey(),
                mint_a: self.mint_a.unwrap(),
                escrow: self.escrow.unwrap(),
                vault: self.vault.unwrap(),
                mint_b: self.mint_b.unwrap(),
                taker_ata_a: self.taker_ata_a.unwrap(),
                taker_ata_b: self.taker_ata_b.unwrap(),
                maker_ata_b: self.maker_ata_b.unwrap(),
                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: escrow::instruction::Take {}.data(),
        };

        let message = Message::new(&[take_ix], Some(&taker.pubkey()));
        let recent_blockhash = self.program.latest_blockhash();
        let transaction = Transaction::new(&[&taker], message, recent_blockhash);

        let tx = self.program.send_transaction(transaction);

        match &tx {
            Ok(tx_result) => {
                msg!("\n\nTake transaction successful");
                msg!("CUs Consumed: {}", tx_result.compute_units_consumed);
                msg!("Tx Signature: {}", tx_result.signature);
                self.last_tx = Some(tx_result.clone());
                self.last_tx_error = None;
            }
            Err(err) => {
                self.last_tx = None;
                self.last_tx_error = Some(format!("{:?}", err));
            }
        }

        self
    }

    pub fn execute_refund(mut self) -> Self {
        let refund_ix = Instruction {
            program_id: self.program_id,
            accounts: escrow::accounts::Refund {
                maker: self.maker.pubkey(),
                mint_a: self.mint_a.unwrap(),
                maker_ata_a: self.maker_ata_a.unwrap(),
                escrow: self.escrow.unwrap(),
                vault: self.vault.unwrap(),
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: escrow::instruction::Refund {}.data(),
        };

        let message = Message::new(&[refund_ix], Some(&self.maker.pubkey()));
        let recent_blockhash = self.program.latest_blockhash();
        let transaction = Transaction::new(&[&self.maker], message, recent_blockhash);

        let tx = self.program.send_transaction(transaction);

        match &tx {
            Ok(tx_result) => {
                msg!("\n\nRefund transaction successful");
                msg!("CUs Consumed: {}", tx_result.compute_units_consumed);
                msg!("Tx Signature: {}", tx_result.signature);

                self.last_tx = Some(tx_result.clone());
                self.last_tx_error = None;
            }
            Err(err) => {
                self.last_tx = None;
                self.last_tx_error = Some(format!("{:?}", err));
            }
        }

        self
    }

    pub fn execute_update(mut self, seed: u64, new_expiration: i64) -> Self {
        let escrow = Pubkey::find_program_address(
            &[b"escrow", self.maker.pubkey().as_ref(), &seed.to_le_bytes()],
            &self.program_id,
        )
        .0;
        msg!("Escrow PDA: {}\n", escrow);

        self.escrow = Some(escrow);

        let update_ix = Instruction {
            program_id: self.program_id,
            accounts: escrow::accounts::Update {
                maker: self.maker.pubkey(),
                escrow: self.escrow.unwrap(),
            }
            .to_account_metas(None),
            data: escrow::instruction::Update {
                expiration: new_expiration,
            }
            .data(),
        };

        let message = Message::new(&[update_ix], Some(&self.maker.pubkey()));
        let recent_blockhash = self.program.latest_blockhash();
        let transaction = Transaction::new(&[&self.maker], message, recent_blockhash);

        let tx = self.program.send_transaction(transaction).unwrap();

        msg!("\n\nUpdate transaction successful");
        msg!("CUs Consumed: {}", tx.compute_units_consumed);
        msg!("Tx Signature: {}", tx.signature);

        self.last_tx = Some(tx);
        self.last_tx_error = None;
        self
    }

    pub fn get_vault_data(&self) -> spl_token::state::Account {
        let vault_account = self.program.get_account(&self.vault.unwrap()).unwrap();
        spl_token::state::Account::unpack(&vault_account.data).unwrap()
    }

    pub fn get_escrow_data(&self) -> Escrow {
        let escrow_account = self.program.get_account(&self.escrow.unwrap()).unwrap();
        Escrow::try_deserialize(&mut escrow_account.data.as_ref()).unwrap()
    }

    pub fn get_maker_ata_a_data(&self) -> spl_token::state::Account {
        let account = self
            .program
            .get_account(&self.maker_ata_a.unwrap())
            .unwrap();
        spl_token::state::Account::unpack(&account.data).unwrap()
    }

    pub fn get_maker_ata_b_data(&self) -> spl_token::state::Account {
        let account = self
            .program
            .get_account(&self.maker_ata_b.unwrap())
            .unwrap();
        spl_token::state::Account::unpack(&account.data).unwrap()
    }

    pub fn get_taker_ata_a_data(&self) -> spl_token::state::Account {
        let account = self
            .program
            .get_account(&self.taker_ata_a.unwrap())
            .unwrap();
        spl_token::state::Account::unpack(&account.data).unwrap()
    }

    pub fn get_taker_ata_b_data(&self) -> spl_token::state::Account {
        let account = self
            .program
            .get_account(&self.taker_ata_b.unwrap())
            .unwrap();
        spl_token::state::Account::unpack(&account.data).unwrap()
    }

    pub fn maker_pubkey(&self) -> Pubkey {
        self.maker.pubkey()
    }

    pub fn mint_a(&self) -> Pubkey {
        self.mint_a.unwrap()
    }

    pub fn mint_b(&self) -> Pubkey {
        self.mint_b.unwrap()
    }

    pub fn escrow(&self) -> Pubkey {
        self.escrow.unwrap()
    }

    pub fn last_tx_succeeded(&self) -> bool {
        self.last_tx_error.is_none()
    }

    pub fn last_tx_failed(&self) -> bool {
        self.last_tx_error.is_some()
    }
}

#[test]
fn test_make() {
    let deposit = 10u64;
    let seed = 123u64;
    let receive = 10u64;
    let expiration = 10000;

    let builder = EscrowTestBuilder::new()
        .create_mints()
        .create_maker_ata_a()
        .mint_to_maker_ata_a(deposit)
        .execute_make(deposit, seed, receive, expiration);

    let vault_data = builder.get_vault_data();
    assert_eq!(vault_data.amount, deposit);
    assert_eq!(vault_data.owner, builder.escrow());
    assert_eq!(vault_data.mint, builder.mint_a());

    let escrow_data = builder.get_escrow_data();
    assert_eq!(escrow_data.seed, seed);
    assert_eq!(escrow_data.maker, builder.maker_pubkey());
    assert_eq!(escrow_data.mint_a, builder.mint_a());
    assert_eq!(escrow_data.mint_b, builder.mint_b());
    assert_eq!(escrow_data.receive, receive);
    assert_eq!(escrow_data.expiration, expiration);
}

#[test]
fn test_update() {
    let deposit = 10u64;
    let seed = 123u64;
    let receive = 10u64;
    let expiration = 10000;
    let new_expiration = 20000;

    let builder = EscrowTestBuilder::new()
        .create_mints()
        .create_maker_ata_a()
        .mint_to_maker_ata_a(deposit)
        .execute_make(deposit, seed, receive, expiration)
        .advance_time(expiration + 1)
        .execute_update(seed, new_expiration);

    let escrow_data = builder.get_escrow_data();
    assert_eq!(escrow_data.expiration, new_expiration);
}

#[test]
fn test_take_before_unlock() {
    let deposit = 20u64;
    let seed = 123u64;
    let receive = 30u64;
    let expiration = 10000;

    let builder = EscrowTestBuilder::new()
        .create_mints()
        .create_maker_ata_a()
        .mint_to_maker_ata_a(deposit)
        .execute_make(deposit, seed, receive, expiration)
        .setup_taker()
        .create_maker_ata_b()
        .create_taker_atas()
        .mint_to_taker_ata_b(receive)
        .execute_take();

    assert!(builder.last_tx_succeeded());

    let taker_ata_a_data = builder.get_taker_ata_a_data();
    assert_eq!(taker_ata_a_data.amount, deposit);

    let taker_ata_b_data = builder.get_taker_ata_b_data();
    assert_eq!(taker_ata_b_data.amount, 0);

    let maker_ata_b_data = builder.get_maker_ata_b_data();
    assert_eq!(maker_ata_b_data.amount, receive);
}

#[test]
fn test_take_after_unlock() {
    let deposit = 20u64;
    let seed = 123u64;
    let receive = 30u64;
    let expiration = 10000;

    let builder = EscrowTestBuilder::new()
        .create_mints()
        .create_maker_ata_a()
        .mint_to_maker_ata_a(deposit)
        .execute_make(deposit, seed, receive, expiration)
        .advance_time(expiration + 1)
        .setup_taker()
        .create_maker_ata_b()
        .create_taker_atas()
        .mint_to_taker_ata_b(receive)
        .execute_take();

    assert!(
        builder.last_tx_failed(),
        "Take should fail after unlock time"
    );

    let taker_ata_a_data = builder.get_taker_ata_a_data();
    assert_eq!(taker_ata_a_data.amount, 0);

    let taker_ata_b_data = builder.get_taker_ata_b_data();
    assert_eq!(taker_ata_b_data.amount, receive);

    let maker_ata_b_data = builder.get_maker_ata_b_data();
    assert_eq!(maker_ata_b_data.amount, 0);
}

#[test]
fn test_refund_before_unlock() {
    let deposit = 20u64;
    let seed = 123u64;
    let receive = 30u64;
    let expiration = 10000;

    let builder = EscrowTestBuilder::new()
        .create_mints()
        .create_maker_ata_a()
        .mint_to_maker_ata_a(deposit)
        .execute_make(deposit, seed, receive, expiration)
        .execute_refund();

    assert!(
        builder.last_tx_failed(),
        "Refund should fail before unlock time"
    );

    let maker_ata_a_data = builder.get_maker_ata_a_data();
    assert_eq!(maker_ata_a_data.amount, 0);
    assert_eq!(maker_ata_a_data.owner, builder.maker_pubkey());
    assert_eq!(maker_ata_a_data.mint, builder.mint_a());
}

#[test]
fn test_refund_after_unlock() {
    let deposit = 20u64;
    let seed = 123u64;
    let receive = 30u64;
    let expiration = 10000;

    let builder = EscrowTestBuilder::new()
        .create_mints()
        .create_maker_ata_a()
        .mint_to_maker_ata_a(deposit)
        .execute_make(deposit, seed, receive, expiration)
        .advance_time(expiration + 1)
        .execute_refund();

    let maker_ata_a_data = builder.get_maker_ata_a_data();
    assert_eq!(maker_ata_a_data.amount, deposit);
    assert_eq!(maker_ata_a_data.owner, builder.maker_pubkey());
    assert_eq!(maker_ata_a_data.mint, builder.mint_a());
}
