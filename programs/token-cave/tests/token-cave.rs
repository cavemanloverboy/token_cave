use std::rc::Rc;
use std::time::Duration;

use anchor_client::anchor_lang::solana_program::native_token::LAMPORTS_PER_SOL;
use anchor_client::anchor_lang::solana_program::sysvar::SysvarId;
use anchor_client::anchor_lang::system_program;
use anchor_client::solana_sdk::instruction::Instruction;
use anchor_client::solana_sdk::program_pack::Pack;
use anchor_client::solana_sdk::transaction::Transaction;
use anchor_client::solana_client::rpc_client::RpcClient;
use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::{read_keypair_file, Signature};
use anchor_client::solana_sdk::signature::{Keypair, Signer};
use anchor_client::solana_sdk::system_transaction;
use anchor_spl::token::spl_token::state::{Mint, Account};
use anchor_client::{Client, Cluster, Program};
use anchor_spl::token::ID as TOKEN_PROGRAM_ID;
use rand::rngs::OsRng;
// Get token_cave
use token_cave::instructions::initialize::{CaveInfo, CAVE_INFO_SIZE};
use anyhow::Result;

const PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    218,7,92,178,255,94,198,129,
    118,19,222,83,11,105,42,135,
    53,71,119,105,218,71,67,12,
    189,129,84,51,92,74,131,39
]);
const DEMO_TOKEN_DECIMALS: u8 = 6;
const ONE_DEMO_TOKEN: u64 = 10_u64.pow(DEMO_TOKEN_DECIMALS as u32);
const TEST_TIMELOCK_DURATION: u32 = 5;

#[test]
fn test_deposit_unlock_withdraw() {

    println!("{CAVE_INFO_SIZE}");

    // Get dev and mint key.
    let dev_key: Keypair = read_keypair_file(&*shellexpand::tilde("../../dev_key.json"))
        .expect("Example requires a keypair file");
    let dev_key_for_client: Keypair = read_keypair_file(&*shellexpand::tilde("../../dev_key.json"))
        .expect("Example requires a keypair file");
    let mint_key: Keypair = read_keypair_file(&*shellexpand::tilde("../../mint_key.json"))
            .expect("Example requires a keypair file");
    
    // Get client, program, and rpc client
    let url: Cluster = Cluster::Localnet;
    let client: Client = Client::new_with_options(url, Rc::new(dev_key_for_client), CommitmentConfig::processed());
    let program: Program = client.program(PROGRAM_ID);
    let solana_client: RpcClient = program.rpc();

    // Initialize mint account
    println!(
        "initialize token mint tx signature: {}",
        initialize_mint_account(&dev_key, &mint_key, &solana_client)
            .unwrap_or("FAILED TO INITIALIZE MINT ACCOUNT".to_string())
    );

    // Get funded user and backup
    let user: User = get_funded_user(&dev_key, &mint_key, &solana_client)
        .expect("failed to get funded user");
    let backup: User = get_funded_user(&dev_key, &mint_key, &solana_client)
        .expect("failed to get funded user");

    // Get PDAs
    let cave = {
        let mut bump: u8 = 255;
        loop {
            if let Ok(pda) = Pubkey::create_program_address(
                &[user.ata.as_ref(), &[bump]],
                &program.id(),
            ) {
                break pda
            } else { bump -= 1; }
        }
    };
    let cave_info = {
        let mut bump: u8 = 255;
        loop {
            if let Ok(pda) = Pubkey::create_program_address(
                &[cave.as_ref(), &[bump]],
                &program.id(),
            ) {
                break pda
            } else { bump -= 1; }
        }
    };

    // Construct and send deposit instruction
    match program
        .request()
        .accounts(token_cave::accounts::Initialize {
            cave_info,
            cave,
            mint: mint_key.pubkey(),
            depositor: user.keypair.pubkey(),
            depositor_token_account: user.ata,
            token_program: TOKEN_PROGRAM_ID,
            system_program: system_program::ID,
            rent: anchor_client::solana_sdk::rent::Rent::id(),
        })
        .args(token_cave::instruction::Initialize {
            backup_address: Some(backup.keypair.pubkey()),
            deposit_amount: 10 * ONE_DEMO_TOKEN,
            timelock_duration: TEST_TIMELOCK_DURATION,
        })
        .signer(&*user.keypair)
        .payer(user.keypair.clone())
        .send() {
            Ok(sig) => println!("deposit tx signature: {sig}"),
            Err(e) => panic!("{e:#?}"),
    };

    // Verify deposit and info
    assert_eq!(
        10 * ONE_DEMO_TOKEN,
        solana_client.get_token_account_balance(&cave)
            .expect("failed to get cave balance")
            .amount
            .parse::<u64>()
            .unwrap(),
        "incorrect balance"
    );
    assert_eq!(
        90 * ONE_DEMO_TOKEN,
        solana_client.get_token_account_balance(&user.ata)
            .expect("failed to get ata balance")
            .amount
            .parse::<u64>()
            .unwrap(),
    );
    let cave_info_account: CaveInfo = program
        .account(cave_info)
        .unwrap();
    assert_eq!(
        cave_info_account.backup_address,
        Some(backup.keypair.pubkey()),
    );
    assert_eq!(
        cave_info_account.timelock_duration,
        TEST_TIMELOCK_DURATION,
    );
    assert_eq!(
        cave_info_account.unlock_request_time,
        i64::MIN,
    );
    assert!(!cave_info_account.unlocking);

    // Construct and send unlock instruction
    match program
        .request()
        .accounts(token_cave::accounts::Unlock {
            cave_info,
            cave,
            mint: mint_key.pubkey(),
            depositor: user.keypair.pubkey(),
            depositor_token_account: user.ata,
        })
        .args(token_cave::instruction::Unlock)
        .signer(&*user.keypair)
        .payer(user.keypair.clone())
        .send() {
            Ok(sig) => println!("cave unlock tx signature: {sig}"),
            Err(e) => panic!("{e:#?}"),
    };
    
    // Verify unlock has begun
    let cave_info_account_post_unlock: CaveInfo = program
        .account(cave_info)
        .unwrap();
    assert!(cave_info_account_post_unlock.unlock_request_time > 0);
    assert!(cave_info_account_post_unlock.unlocking);

    // Construct and send withdraw instruction (too early, and then on time)
    program
        .request()
        .accounts(token_cave::accounts::Withdraw {
            cave_info,
            cave,
            mint: mint_key.pubkey(),
            depositor: user.keypair.pubkey(),
            depositor_token_account: user.ata,
            token_program: TOKEN_PROGRAM_ID,
        })
        .args(token_cave::instruction::Withdraw)
        .signer(&*user.keypair)
        .payer(user.keypair.clone())
        .send()
        .expect_err("should have failed");
    std::thread::sleep(Duration::from_secs(1 + TEST_TIMELOCK_DURATION as u64));
    match program
        .request()
        .accounts(token_cave::accounts::Withdraw {
            cave_info,
            cave,
            mint: mint_key.pubkey(),
            depositor: user.keypair.pubkey(),
            depositor_token_account: user.ata,
            token_program: TOKEN_PROGRAM_ID,
        })
        .args(token_cave::instruction::Withdraw)
        .payer(user.keypair.clone())
        .send() {
            Ok(sig) => println!("withdraw tx signature: {sig}"),
            Err(e) => panic!("{e:#?}"),
    };
    
    // Verify abort occurred
    assert_eq!(
        100 * ONE_DEMO_TOKEN,
        solana_client.get_token_account_balance(&user.ata)
            .expect("failed to get ata balance")
            .amount
            .parse::<u64>()
            .unwrap(),
    );

}


#[test]
fn test_deposit_unlock_abort() {

    println!("{CAVE_INFO_SIZE}");

    // Get dev and mint key.
    let dev_key: Keypair = read_keypair_file(&*shellexpand::tilde("../../dev_key.json"))
        .expect("Example requires a keypair file");
    let dev_key_for_client: Keypair = read_keypair_file(&*shellexpand::tilde("../../dev_key.json"))
        .expect("Example requires a keypair file");
    let mint_key: Keypair = read_keypair_file(&*shellexpand::tilde("../../mint_key.json"))
            .expect("Example requires a keypair file");
    
    // Get client, program, and rpc client
    let url: Cluster = Cluster::Localnet;
    let client: Client = Client::new_with_options(url, Rc::new(dev_key_for_client), CommitmentConfig::processed());
    let program: Program = client.program(PROGRAM_ID);
    let solana_client: RpcClient = program.rpc();

    // Initialize mint account
    println!(
        "initialize token mint tx signature: {}",
        initialize_mint_account(&dev_key, &mint_key, &solana_client)
            .unwrap_or("FAILED TO INITIALIZE MINT ACCOUNT".to_string())
    );

    // Get funded user and backup
    let user: User = get_funded_user(&dev_key, &mint_key, &solana_client)
        .expect("failed to get funded user");
    let backup: User = get_funded_user(&dev_key, &mint_key, &solana_client)
        .expect("failed to get funded user");

    // Get PDAs
    let cave = {
        let mut bump: u8 = 255;
        loop {
            if let Ok(pda) = Pubkey::create_program_address(
                &[user.ata.as_ref(), &[bump]],
                &program.id(),
            ) {
                break pda
            } else { bump -= 1; }
        }
    };
    let cave_info = {
        let mut bump: u8 = 255;
        loop {
            if let Ok(pda) = Pubkey::create_program_address(
                &[cave.as_ref(), &[bump]],
                &program.id(),
            ) {
                break pda
            } else { bump -= 1; }
        }
    };

    // Construct and send deposit instruction
    match program
        .request()
        .accounts(token_cave::accounts::Initialize {
            cave_info,
            cave,
            mint: mint_key.pubkey(),
            depositor: user.keypair.pubkey(),
            depositor_token_account: user.ata,
            token_program: TOKEN_PROGRAM_ID,
            system_program: system_program::ID,
            rent: anchor_client::solana_sdk::rent::Rent::id(),
        })
        .args(token_cave::instruction::Initialize {
            backup_address: Some(backup.keypair.pubkey()),
            deposit_amount: 10 * ONE_DEMO_TOKEN,
            timelock_duration: TEST_TIMELOCK_DURATION,
        })
        .signer(&*user.keypair)
        .payer(user.keypair.clone())
        .send() {
            Ok(sig) => println!("deposit tx signature: {sig}"),
            Err(e) => panic!("{e:#?}"),
    };
    

    // Verify deposit and info
    assert_eq!(
        10 * ONE_DEMO_TOKEN,
        solana_client.get_token_account_balance(&cave)
            .expect("failed to get cave balance")
            .amount
            .parse::<u64>()
            .unwrap(),
        "incorrect balance"
    );
    assert_eq!(
        90 * ONE_DEMO_TOKEN,
        solana_client.get_token_account_balance(&user.ata)
            .expect("failed to get ata balance")
            .amount
            .parse::<u64>()
            .unwrap(),
    );
    let cave_info_account: CaveInfo = program
        .account(cave_info)
        .unwrap();
    assert_eq!(
        cave_info_account.backup_address,
        Some(backup.keypair.pubkey()),
    );
    assert_eq!(
        cave_info_account.timelock_duration,
        TEST_TIMELOCK_DURATION,
    );
    assert_eq!(
        cave_info_account.unlock_request_time,
        i64::MIN,
    );
    assert!(!cave_info_account.unlocking);

    // Construct and send unlock instruction
    match program
        .request()
        .accounts(token_cave::accounts::Unlock {
            cave_info,
            cave,
            mint: mint_key.pubkey(),
            depositor: user.keypair.pubkey(),
            depositor_token_account: user.ata,
        })
        .args(token_cave::instruction::Unlock)
        .signer(&*user.keypair)
        .payer(user.keypair.clone())
        .send() {
            Ok(sig) => println!("unlock tx signature: {sig}"),
            Err(e) => panic!("{e:#?}"),
    };
    
    // Verify unlock has begun
    let cave_info_account_post_unlock: CaveInfo = program
        .account(cave_info)
        .unwrap();
    assert!(cave_info_account_post_unlock.unlock_request_time > 0);
    assert!(cave_info_account_post_unlock.unlocking);

    // Construct and send withdraw instruction (too early, and then on time)
    match program
        .request()
        .accounts(token_cave::accounts::Abort {
            cave_info,
            cave,
            mint: mint_key.pubkey(),
            depositor: user.keypair.pubkey(),
            depositor_token_account: user.ata,
            token_program: TOKEN_PROGRAM_ID,
            backup: backup.keypair.pubkey(),
            backup_spl_account: backup.ata,
        })
        .args(token_cave::instruction::Abort)
        .signer(&*user.keypair)
        .payer(user.keypair.clone())
        .send() {
            Ok(sig) => println!("abort unlock tx signature: {sig}"),
            Err(e) => panic!("{e:#?}"),
    };
    
    // Verify withdraw occurred
    assert_eq!(
        90 * ONE_DEMO_TOKEN,
        solana_client.get_token_account_balance(&user.ata)
            .expect("failed to get ata balance")
            .amount
            .parse::<u64>()
            .unwrap(),
    );
    assert_eq!(
        110 * ONE_DEMO_TOKEN,
        solana_client.get_token_account_balance(&backup.ata)
            .expect("failed to get ata balance")
            .amount
            .parse::<u64>()
            .unwrap(),
    );

}

fn get_funded_user(
    dev_key: &Keypair,
    mint_key: &Keypair,
    solana_client: &RpcClient,
 ) -> Result<User> {
    
    // Generate a new keypair
    let user = Keypair::generate(&mut OsRng);

    // Fund the keypair from the dev wallet with sol
    let fund_with_sol_tx: Transaction = system_transaction::transfer(
        dev_key,
        &user.pubkey(),
        LAMPORTS_PER_SOL,
        solana_client.get_latest_blockhash().expect("failed to get lastest blockhash")
    );
    println!(
        "fund_with_sol_tx signature: {}",
        solana_client.send_and_confirm_transaction(&fund_with_sol_tx)
            .expect("failed to fund user with sol")
    );
    assert_eq!(
        solana_client.get_balance(&user.pubkey()).expect("failed to get balance"),
        LAMPORTS_PER_SOL,
    );
    drop(fund_with_sol_tx);

    // Create user token account
    let user_ata: Pubkey = spl_associated_token_account::get_associated_token_address(&user.pubkey(), &mint_key.pubkey());
    let spl_create_account_ix: Instruction = spl_associated_token_account::instruction::create_associated_token_account(
        &user.pubkey(),
        &user.pubkey(),
        &mint_key.pubkey(),
    );
    let create_spl_account_tx: Transaction = Transaction::new_signed_with_payer(
        &[spl_create_account_ix],
        Some(&user.pubkey()),
        &[&user],
        solana_client.get_latest_blockhash().expect("failed to get lastest blockhash")
    );
    println!(
        "create_spl_account_tx signature: {}",
        solana_client.send_and_confirm_transaction(&create_spl_account_tx)
            .expect("failed to create spl account ")
    );
    drop(create_spl_account_tx);

    // Ensure account properties are okay
    let user_token_account = solana_client.get_token_account(&user_ata)
        .expect("failed to retrieve user token account")
        .expect("expecting user account to exist");
    assert_eq!(&user.pubkey().to_string(), &user_token_account.owner, "incorrect ata owner");

    // Fund token account by minting tokens
    let token_mint_ix: Instruction = anchor_spl::token::spl_token::instruction::mint_to(
        &TOKEN_PROGRAM_ID,
        &mint_key.pubkey(),
        &user_ata,
        &dev_key.pubkey(),
        &[&dev_key.pubkey()],
        100 * ONE_DEMO_TOKEN,
    ).expect("unable to create mint transaction");
    let fund_with_spl_tx: Transaction = Transaction::new_signed_with_payer(
        &[token_mint_ix],
        Some(&dev_key.pubkey()),
        &[dev_key],
        solana_client.get_latest_blockhash().expect("failed to get lastest blockhash")
    );
    println!(
        "fund_with_spl_tx signature: {}",
        solana_client.send_and_confirm_transaction(&fund_with_spl_tx)
            .expect("failed to create spl account ")
    );
    drop(fund_with_spl_tx);

    Ok(
        User { keypair: Rc::new(user), ata: user_ata }
    )
}


/// This allow(unused_must_use) makes this function idempotent & infallible with a valid dev environment
#[allow(unused_must_use)]
fn initialize_mint_account(
    dev_key: &Keypair,
    mint_key: &Keypair,
    solana_client: &RpcClient,
) -> Result<String, anchor_client::solana_client::client_error::ClientError> {

    // Create transaction with single spl mint instruction
    let pay_rent_and_create_account_ix: Instruction = anchor_client::solana_sdk::system_instruction::create_account(
        &dev_key.pubkey(),
        &mint_key.pubkey(),
        solana_client.get_minimum_balance_for_rent_exemption(Mint::LEN)?,
        Mint::LEN as u64,
        &TOKEN_PROGRAM_ID,
    );
    let initialize_mint_account_ix: Instruction = anchor_spl::token::spl_token::instruction::initialize_mint(
        &TOKEN_PROGRAM_ID,
        &mint_key.pubkey(),
        &dev_key.pubkey(),
        None,
        DEMO_TOKEN_DECIMALS
    ).expect("failed to create initialize mint account instruction");
    let spl_mint_tx = Transaction::new_signed_with_payer(
        &[pay_rent_and_create_account_ix, initialize_mint_account_ix],
        Some(&dev_key.pubkey()),
        &[dev_key, mint_key],
        solana_client.get_latest_blockhash()?,
    );

    // Send and confirm transaction, and get signature
    let signature = solana_client.send_and_confirm_transaction(&spl_mint_tx);
    signature.map(|s| s.to_string())
}



struct User {
    keypair: Rc<Keypair>,
    ata: Pubkey,
}