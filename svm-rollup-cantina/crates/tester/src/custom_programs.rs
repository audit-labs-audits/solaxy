use std::time::Duration;

use anyhow::anyhow;
use borsh::{to_vec, BorshDeserialize};
use jsonrpsee::tracing::debug;
use solana_program::{
    bpf_loader_upgradeable::{get_program_data_address, UpgradeableLoaderState},
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_instruction,
};
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use svm::test_utils::{create_deploy_program_transactions, Counter};
use tokio::time::sleep;

use crate::{client::TestClient, SLEEP_DURATION_SECONDS};

pub struct Program {
    pub name: &'static str,
    pub bytecode: &'static [u8],
}

static PROGRAMS: &[Program] = &[
    Program {
        name: "hello_world",
        bytecode: include_bytes!(
            "../../test-data/custom_bytecode/hello_world/hello_world_program.so"
        ),
    },
    Program {
        name: "counter",
        bytecode: include_bytes!("../../test-data/custom_bytecode/counter/counter_program.so"),
    },
];

pub(crate) async fn deploy_program(
    client: &TestClient,
    payer: &Keypair,
    bytecode: &[u8],
) -> anyhow::Result<Keypair> {
    let program_keypair = Keypair::new();
    let program_pubkey = program_keypair.pubkey();
    debug!("Program pubkey {program_pubkey}");

    let program_data_pubkey = get_program_data_address(&program_pubkey);
    debug!("Program data pubkey {program_data_pubkey}");

    let latest_blockhash = client
        .svm_get_latest_blockhash()
        .await
        .map_err(|e| anyhow!("Failed to retrieve the latest blockhash: {e}"))?;

    let buffer_lamports = client
        .get_minimum_balance_for_rent_exemption(
            bytecode.len() + UpgradeableLoaderState::size_of_buffer_metadata(),
        )
        .await
        .map_err(|e| anyhow!("Failed to get a rent exempt balance: {e}"))?;

    let deploy_txs = create_deploy_program_transactions(
        payer,
        &program_keypair,
        payer,
        bytecode.to_vec(),
        buffer_lamports,
        latest_blockhash,
    )
    .map_err(|e| anyhow!("Failed to initialize transactions for program deployment: {e}"))?;

    for tx in deploy_txs {
        let tx_hash = client
            .svm_send_transaction(&tx)
            .await
            .map_err(|e| anyhow!("Failed to send a transaction: {e}"))?
            .to_string();
        debug!("Send Transaction Hash: {tx_hash}");
    }

    sleep(Duration::from_secs(SLEEP_DURATION_SECONDS)).await;

    let program_account = client
        .svm_get_account_info(program_pubkey)
        .await
        .map_err(|e| anyhow!("Failed to retrieve a program account: {e}"))?;
    debug!("Program account: {program_account:?}");

    let program_account_data: UpgradeableLoaderState = bincode::deserialize(&program_account.data)
        .map_err(|e| anyhow!("Failed to deserialize program account data: {e}"))?;

    if let UpgradeableLoaderState::Program {
        programdata_address,
    } = program_account_data
    {
        assert_eq!(
            programdata_address, program_data_pubkey,
            "Expected programdata_address from program account's metadata to match \
            program_data_pubkey derived for program's pubkey"
        );
    }

    let program_data_account = client
        .svm_get_account_info(program_data_pubkey)
        .await
        .map_err(|e| anyhow!("Failed to retrieve a program data account: {e}"))?;
    debug!("Program Data account: {program_data_account:?}");

    let program_data_account_metadata: UpgradeableLoaderState = bincode::deserialize(
        &program_data_account.data[..UpgradeableLoaderState::size_of_programdata_metadata()],
    )
    .map_err(|e| anyhow!("Failed to deserialize program data account metadata: {e}"))?;

    if let UpgradeableLoaderState::ProgramData {
        upgrade_authority_address,
        ..
    } = program_data_account_metadata
    {
        let authority = upgrade_authority_address
            .expect("Expected program data account to have a program authority");
        assert_eq!(
            authority,
            payer.pubkey(),
            "Expected program authority from program data account to match\
            payer's address"
        );
    }

    Ok(program_keypair)
}

pub(crate) async fn call_hello_world(
    client: &TestClient,
    payer: &Keypair,
    program_pubkey: &Pubkey,
) -> anyhow::Result<()> {
    let latest_blockhash = client
        .svm_get_latest_blockhash()
        .await
        .map_err(|e| anyhow!("Failed to retrieve the latest blockhash: {e}"))?;

    let hello_world_instruction = Instruction::new_with_bincode(
        *program_pubkey,
        &(), // To trigger msg!(hello world) line in the program no instruction is required
        vec![AccountMeta::new(payer.pubkey(), true)],
    );
    let hello_world_tx = Transaction::new_signed_with_payer(
        &[hello_world_instruction],
        Some(&payer.pubkey()),
        &[&payer],
        latest_blockhash,
    );

    let tx_hash = client
        .svm_send_transaction(&hello_world_tx)
        .await
        .map_err(|e| anyhow!("Failed to send a transaction: {e}"))?
        .to_string();
    debug!("Send Transaction Hash: {tx_hash}");

    Ok(())
}

pub(crate) async fn create_counter_account(
    client: &TestClient,
    payer: &Keypair,
    counter_program_id: &Pubkey,
) -> anyhow::Result<Keypair> {
    debug!("Create counter account");
    let counter_account = Keypair::new();

    let counter = Counter { count: 0 };
    let data =
        to_vec(&counter).map_err(|e| anyhow!("Failed to serialize counter into data: {e}"))?;

    let rent_balance = client
        .get_minimum_balance_for_rent_exemption(data.len())
        .await
        .map_err(|e| anyhow!("Failed to get a rent exempt balance: {e}"))?;

    // We do not need to write Counter struct into the account, as it is simple enough
    // so vector of zeros can be deserialized inside the counter program as a counter object
    let create_account_instruction = system_instruction::create_account(
        &payer.pubkey(),
        &counter_account.pubkey(),
        rent_balance,
        data.len() as u64,
        counter_program_id,
    );

    let latest_blockhash = client
        .svm_get_latest_blockhash()
        .await
        .map_err(|e| anyhow!("Failed to retrieve the latest blockhash: {e}"))?;

    let create_acc_tx = Transaction::new_signed_with_payer(
        &[create_account_instruction],
        Some(&payer.pubkey()),
        &[payer, &counter_account],
        latest_blockhash,
    );

    let tx_hash = client
        .svm_send_transaction(&create_acc_tx)
        .await
        .map_err(|e| anyhow!("Failed to send a transaction: {e}"))?
        .to_string();
    debug!("Send Transaction Hash: {tx_hash}");

    sleep(Duration::from_secs(SLEEP_DURATION_SECONDS)).await;

    let counter_acc = client
        .svm_get_account_info(counter_account.pubkey())
        .await
        .map_err(|e| anyhow!("Failed to retrieve a counter account: {e}"))?;

    debug!("Counter Account created and retrieved: {counter_acc:?}");

    Ok(counter_account)
}

pub(crate) async fn increment_counter(
    client: &TestClient,
    payer: &Keypair,
    counter_pubkey: &Pubkey,
    program_pubkey: &Pubkey,
) -> anyhow::Result<()> {
    debug!("Interact and increment Counter");

    let counter_acc = client
        .svm_get_account_info(*counter_pubkey)
        .await
        .map_err(|e| anyhow!("Failed to retrieve a counter account: {e}"))?;
    let initial_counter_value = Counter::try_from_slice(&counter_acc.data)
        .map_err(|e| anyhow!("Failed to deserialize a counter account data: {e}"))?
        .count;

    let latest_blockhash = client
        .svm_get_latest_blockhash()
        .await
        .map_err(|e| anyhow!("Failed to retrieve the latest blockhash: {e}"))?;

    let counter_instruction = Instruction::new_with_bincode(
        *program_pubkey,
        &[0], // Instruction to increase counter by one
        vec![AccountMeta::new(*counter_pubkey, false)],
    );
    let counter_transaction = Transaction::new_signed_with_payer(
        &[counter_instruction],
        Some(&payer.pubkey()),
        &[&payer],
        latest_blockhash,
    );

    let tx_hash = client
        .svm_send_transaction(&counter_transaction)
        .await
        .map_err(|e| anyhow!("Failed to send a transaction: {e}"))?
        .to_string();
    debug!("Send Transaction Hash: {tx_hash}");

    sleep(Duration::from_secs(SLEEP_DURATION_SECONDS)).await;

    let counter_acc = client
        .svm_get_account_info(*counter_pubkey)
        .await
        .map_err(|e| anyhow!("{e}"))?;

    debug!("Counter retrieved: {counter_acc:?}");

    let post_counter = Counter::try_from_slice(&counter_acc.data)
        .map_err(|e| anyhow!("Failed to deserialize a counter account data: {e}"))?
        .count;
    let expected_counter = initial_counter_value + 1;
    assert_eq!(
        post_counter, expected_counter,
        "Counter is expected to be {expected_counter}"
    );
    debug!("Counter successfully incremented");

    Ok(())
}

pub(crate) async fn hello_world_program_scenario(
    client: &TestClient,
    payer: &Keypair,
) -> anyhow::Result<()> {
    let program_name = PROGRAMS[0].name;
    debug!("Program {program_name} scenario");

    let hello_world_program_bytecode = PROGRAMS[0].bytecode;

    let program_keypair = deploy_program(client, payer, hello_world_program_bytecode)
        .await
        .map_err(|e| anyhow!("Failed to deploy hello world program: {e}"))?;

    call_hello_world(client, payer, &program_keypair.pubkey())
        .await
        .map_err(|e| anyhow!("Failed to call hello world program: {e}"))?;
    debug!("Hello World program deployed successfully");

    Ok(())
}

pub(crate) async fn counter_program_scenario(
    client: &TestClient,
    payer: &Keypair,
) -> anyhow::Result<()> {
    let program_name = PROGRAMS[1].name;
    debug!("Program {program_name} scenario");

    let counter_program_bytecode = PROGRAMS[1].bytecode;

    let program_keypair = deploy_program(client, payer, counter_program_bytecode)
        .await
        .map_err(|e| anyhow!("Failed to deploy counter program: {e}"))?;

    let counter = create_counter_account(client, payer, &program_keypair.pubkey())
        .await
        .map_err(|e| anyhow!("Failed to create a counter program: {e}"))?;

    increment_counter(client, payer, &counter.pubkey(), &program_keypair.pubkey())
        .await
        .map_err(|e| anyhow!("Failed to increment a counter: {e}"))?;
    debug!("Counter program deployed successfully");

    Ok(())
}
