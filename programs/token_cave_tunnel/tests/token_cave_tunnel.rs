use std::rc::Rc;
use std::str::FromStr;
use std::time::Duration;

use anchor_client::anchor_lang::solana_program::{
    native_token::LAMPORTS_PER_SOL, system_program, sysvar::SysvarId,
};
use anchor_client::solana_client::rpc_client::RpcClient;
use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::solana_sdk::instruction::Instruction;
use anchor_client::solana_sdk::program_pack::Pack;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::read_keypair_file;
use anchor_client::solana_sdk::signature::{Keypair, Signer};
use anchor_client::solana_sdk::system_transaction;
use anchor_client::solana_sdk::transaction::Transaction;
use anchor_client::{Client, Cluster, Program};
use anchor_lang::ToAccountMetas;
use anchor_spl::token::spl_token::state::Mint;
use anchor_spl::token::ID as TOKEN_PROGRAM_ID;
// Get token_cave
use token_cave_tunnel::instructions::payment::{
    CaveTunnelInfo, CAVE_TUNNEL_INFO_SIZE, COST_OF_SERVICE_PER_SECOND, TIMELOCK_DURATION,
};

use anyhow::Result;

#[allow(unused)]
const PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    218, 7, 92, 178, 255, 94, 198, 129, 118, 19, 222, 83, 11, 105, 42, 135, 53, 71, 119, 105, 218,
    71, 67, 12, 189, 129, 84, 51, 92, 74, 131, 39,
]);
#[allow(unused)]
const PROGRAM_ID_TESTNET: Pubkey = Pubkey::new_from_array([
    42, 56, 57, 130, 71, 199, 23, 186, 50, 28, 77, 241, 222, 64, 89, 243, 90, 247, 81, 92, 19, 240,
    147, 246, 56, 32, 95, 12, 66, 171, 183, 15,
]);

const DEMO_TOKEN_DECIMALS: u8 = 6;
const ONE_DEMO_TOKEN: u64 = 10_u64.pow(DEMO_TOKEN_DECIMALS as u32);
const DEMO_REQUEST_SERVICE_TIME: u32 = TIMELOCK_DURATION as u32;
const TEST_TIMELOCK_DURATION: u32 = TIMELOCK_DURATION as u32;

#[test]
fn test_payment_payout() {
    println!("{CAVE_TUNNEL_INFO_SIZE}");

    // Get dev and mint key.
    let dev_key: Keypair = read_keypair_file(&*shellexpand::tilde("../../dev_key.json"))
        .expect("Example requires a keypair file");
    let dev_key_for_client: Keypair = read_keypair_file(&*shellexpand::tilde("../../dev_key.json"))
        .expect("Example requires a keypair file");
    let mint_key: Keypair = read_keypair_file(&*shellexpand::tilde("../../mint_key.json"))
        .expect("Example requires a keypair file");

    // Get client, program, and rpc client
    let url: Cluster = Cluster::Localnet;
    let client: Client = Client::new_with_options(
        url,
        Rc::new(dev_key_for_client),
        CommitmentConfig::processed(),
    );
    let program: Program = client.program(PROGRAM_ID_TESTNET);
    let solana_client: RpcClient = program.rpc();

    // Initialize mint account
    println!(
        "initialize token mint tx signature: {}",
        initialize_mint_account(&dev_key, &mint_key, &solana_client)
            .unwrap_or("FAILED TO INITIALIZE MINT ACCOUNT".to_string())
    );

    // Get funded user and operator
    let user: User =
        get_funded_user(&dev_key, &mint_key, &solana_client).expect("failed to get funded user");
    let operator: User =
        get_funded_user(&dev_key, &mint_key, &solana_client).expect("failed to get funded user");

    // Get tss placeholder (NOTE: this account is currently unchecked)
    let tss_placeholder: User =
        get_funded_user(&dev_key, &mint_key, &solana_client).expect("failed to get funded user");

    println!("user pubkey: {}; ata {}", user.keypair.pubkey(), user.ata);
    println!(
        "operator pubkey: {}; ata {}",
        operator.keypair.pubkey(),
        operator.ata
    );
    println!(
        "tss_placeholder pubkey: {}; ata {}",
        tss_placeholder.keypair.pubkey(),
        tss_placeholder.ata
    );

    // Get PDAs
    let cave_tunnel: Pubkey = {
        let mut bump: u8 = 255;
        loop {
            if let Ok(pda) = Pubkey::create_program_address(
                &[user.keypair.pubkey().as_ref(), &[bump]],
                &program.id(),
            ) {
                break pda;
            } else {
                bump -= 1;
            }
        }
    };
    let cave_tunnel_info: Pubkey = {
        let mut bump: u8 = 255;
        loop {
            if let Ok(pda) =
                Pubkey::create_program_address(&[cave_tunnel.as_ref(), &[bump]], &program.id())
            {
                break pda;
            } else {
                bump -= 1;
            }
        }
    };

    // Construct and send payment instruction
    match program
        .request()
        .accounts(token_cave_tunnel::accounts::Payment {
            cave_tunnel_info: cave_tunnel_info,
            cave_tunnel: cave_tunnel,
            mint: mint_key.pubkey(),
            payer: user.keypair.pubkey(),
            payer_token_account: user.ata,
            token_program: TOKEN_PROGRAM_ID,
            system_program: system_program::ID,
            rent: anchor_client::solana_sdk::rent::Rent::id(),
        })
        .args(token_cave_tunnel::instruction::Payment {
            service_time: DEMO_REQUEST_SERVICE_TIME,
        })
        .signer(&*user.keypair)
        .payer(user.keypair.clone())
        .send()
    {
        Ok(sig) => println!("payment tx signature: {sig}"),
        Err(e) => panic!("{e:#?}"),
    };

    // Verify payment and info
    assert_eq!(
        DEMO_REQUEST_SERVICE_TIME as u64 * COST_OF_SERVICE_PER_SECOND,
        solana_client
            .get_token_account_balance(&cave_tunnel)
            .expect("failed to get cave balance")
            .amount
            .parse::<u64>()
            .unwrap(),
        "incorrect balance"
    );
    assert_eq!(
        100 * ONE_DEMO_TOKEN - COST_OF_SERVICE_PER_SECOND * DEMO_REQUEST_SERVICE_TIME as u64,
        solana_client
            .get_token_account_balance(&user.ata)
            .expect("failed to get ata balance")
            .amount
            .parse::<u64>()
            .unwrap(),
    );
    let cave_info_account: CaveTunnelInfo = program.account(cave_tunnel_info).unwrap();
    assert_eq!(cave_info_account.service_time, DEMO_REQUEST_SERVICE_TIME,);

    // Construct and send payout instruction
    std::thread::sleep(Duration::from_secs(1 + TEST_TIMELOCK_DURATION as u64));
    match program
        .request()
        .accounts(token_cave_tunnel::accounts::Payout {
            cave_tunnel_info: cave_tunnel_info,
            cave_tunnel: cave_tunnel,
            mint: mint_key.pubkey(),
            payee: operator.keypair.pubkey(),
            payee_token_account: operator.ata,
            placeholder_for_threshold_signature: tss_placeholder.keypair.pubkey(),
            token_program: TOKEN_PROGRAM_ID,
        })
        .args(token_cave_tunnel::instruction::Payout {
            payee_pubkey: operator.keypair.pubkey(),
        })
        .signer(&*tss_placeholder.keypair)
        .payer(tss_placeholder.keypair.clone())
        .send()
    {
        Ok(sig) => println!("payout tx signature: {sig}"),
        Err(e) => panic!("{e:#?}"),
    };

    // Verify payout occurred
    assert_eq!(
        100 * ONE_DEMO_TOKEN + COST_OF_SERVICE_PER_SECOND * DEMO_REQUEST_SERVICE_TIME as u64,
        solana_client
            .get_token_account_balance(&operator.ata)
            .expect("expecting user account to exist (account doesn't exist)")
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
    let user = Keypair::new();

    // Fund the keypair from the dev wallet with sol
    let fund_with_sol_tx: Transaction = system_transaction::transfer(
        dev_key,
        &user.pubkey(),
        LAMPORTS_PER_SOL / 100,
        solana_client
            .get_latest_blockhash()
            .expect("failed to get lastest blockhash"),
    );
    println!(
        "fund_with_sol_tx signature: {}",
        solana_client
            .send_and_confirm_transaction(&fund_with_sol_tx)
            .expect("failed to fund user with sol")
    );
    assert_eq!(
        solana_client
            .get_balance(&user.pubkey())
            .expect("failed to get balance"),
        LAMPORTS_PER_SOL / 100,
    );
    drop(fund_with_sol_tx);

    // Create user token account
    let user_ata: Pubkey = spl_associated_token_account::get_associated_token_address(
        &user.pubkey(),
        &mint_key.pubkey(),
    );
    let spl_create_account_ix: Instruction =
        spl_associated_token_account::instruction::create_associated_token_account(
            &user.pubkey(),
            &user.pubkey(),
            &mint_key.pubkey(),
        );
    let create_spl_account_tx: Transaction = Transaction::new_signed_with_payer(
        &[spl_create_account_ix],
        Some(&user.pubkey()),
        &[&user],
        solana_client
            .get_latest_blockhash()
            .expect("failed to get lastest blockhash"),
    );
    println!(
        "create_spl_account_tx signature: {}",
        solana_client
            .send_and_confirm_transaction(&create_spl_account_tx)
            .expect("failed to create spl account ")
    );
    drop(create_spl_account_tx);

    // Ensure account properties are okay
    let user_token_account = solana_client
        .get_token_account(&user_ata)
        .expect("failed to retrieve user token account (network error)")
        .expect("expecting user account to exist (account doesn't exist)");
    assert_eq!(
        &user.pubkey().to_string(),
        &user_token_account.owner,
        "incorrect ata owner"
    );

    // Fund token account by minting tokens
    let token_mint_ix: Instruction = anchor_spl::token::spl_token::instruction::mint_to(
        &TOKEN_PROGRAM_ID,
        &mint_key.pubkey(),
        &user_ata,
        &dev_key.pubkey(),
        &[&dev_key.pubkey()],
        100 * ONE_DEMO_TOKEN,
    )
    .expect("unable to create mint transaction");
    let fund_with_spl_tx: Transaction = Transaction::new_signed_with_payer(
        &[token_mint_ix],
        Some(&dev_key.pubkey()),
        &[dev_key],
        solana_client
            .get_latest_blockhash()
            .expect("failed to get lastest blockhash"),
    );
    println!(
        "fund_with_spl_tx signature: {}",
        solana_client
            .send_and_confirm_transaction(&fund_with_spl_tx)
            .expect("failed to create spl account ")
    );
    drop(fund_with_spl_tx);

    Ok(User {
        keypair: Rc::new(user),
        ata: user_ata,
    })
}

/// This allow(unused_must_use) makes this function idempotent & infallible with a valid dev environment
#[allow(unused_must_use)]
fn initialize_mint_account(
    dev_key: &Keypair,
    mint_key: &Keypair,
    solana_client: &RpcClient,
) -> Result<String, anchor_client::solana_client::client_error::ClientError> {
    // Create transaction with single spl mint instruction
    let pay_rent_and_create_account_ix: Instruction =
        anchor_client::solana_sdk::system_instruction::create_account(
            &dev_key.pubkey(),
            &mint_key.pubkey(),
            solana_client.get_minimum_balance_for_rent_exemption(Mint::LEN)?,
            Mint::LEN as u64,
            &TOKEN_PROGRAM_ID,
        );
    let initialize_mint_account_ix: Instruction =
        anchor_spl::token::spl_token::instruction::initialize_mint(
            &TOKEN_PROGRAM_ID,
            &mint_key.pubkey(),
            &dev_key.pubkey(),
            None,
            DEMO_TOKEN_DECIMALS,
        )
        .expect("failed to create initialize mint account instruction");
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
