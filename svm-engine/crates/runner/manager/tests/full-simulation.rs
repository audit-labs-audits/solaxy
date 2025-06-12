use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use example_gated_program::{Message, find_message_address};
use proxy::{find_lock_address, find_lock_config_address, find_lock_signer_address};
use solana_sdk::{
    account::Account, bpf_loader_upgradeable, commitment_config::CommitmentConfig,
    instruction::Instruction, native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signature::Keypair,
    signer::Signer, system_program, transaction::Transaction,
};
use solana_transaction_status::TransactionConfirmationStatus;
use svm_engine::programs::Program;
use svm_runner_client::RunnerClient;
use svm_runner_manager::{
    EngineConfig, ManagerConfig, ProcessorConfig, SVMManager, StorageConfig, TxQueueConfig,
    builder::Builder, queue::Queue, relay::SvmRelay, rpc::Rpc, storage::Storage, syncer::SvmSyncer,
};
use tokio::time::{Duration, sleep};

pub const DISCRIMINATOR_LENGTH: usize = 8;

#[tokio::test]
#[ignore = "Requires a local Solana cluster to be running"]
async fn full_simulation() {
    struct TestManager;

    #[async_trait::async_trait]
    impl SVMManager for TestManager {
        type Queue = Queue;
        type Storage = Storage;
        type Rpc = Rpc<Self::Storage>;
        type Builder = Builder;
        type SvmSyncer = SvmSyncer<Self::Storage>;
        type SvmRelay = SvmRelay;

        async fn create_config() -> ManagerConfig {
            let programs_to_load = vec![
                Program {
                    program_id: proxy::id(),
                    owner_program: bpf_loader_upgradeable::id(),
                    bytecode: include_bytes!("../example-gated-program/target/deploy/proxy.so"),
                },
                Program {
                    program_id: example_gated_program::id(),
                    owner_program: bpf_loader_upgradeable::id(),
                    bytecode: include_bytes!(
                        "../example-gated-program/target/deploy/example_gated_program.so"
                    ),
                },
            ];

            ManagerConfig {
                // This is the URL of the local Solana cluster
                rpc_addr: "http://localhost:8899".to_owned(),
                queue_size: 1_000,
                engine_config: EngineConfig {
                    rpc_addr: "0.0.0.0:8989".to_owned(),
                    processor_config: ProcessorConfig {
                        slot: 1,
                        epoch: 1,
                        program_ids: HashSet::new(),
                    },
                    storage_config: StorageConfig {
                        storage_file: "../../../target/storage".to_owned(),
                        persist_interval_s: 3600,
                        programs_to_load,
                        accounts_to_load: HashMap::new(),
                    },
                },
                tx_queue_config: TxQueueConfig {
                    rpc_addr: "0.0.0.0:9696".to_owned(),
                },
            }
        }
    }

    let program_id = proxy::id();
    let target = example_gated_program::id();
    let payer = Arc::new(Keypair::new());
    let config = TestManager::create_config().await;

    let pda_signer = find_lock_signer_address(program_id, target);
    let message_account = find_message_address(target, payer.pubkey());

    let mut accounts_to_load = HashMap::new();
    accounts_to_load.insert(
        payer.pubkey(),
        Account::new(100_000_000_000, 0, &system_program::id()),
    );

    let _manager_handle =
        tokio::spawn(async move { TestManager::coordinate_tasks(accounts_to_load).await });

    // Wait for the manager to start
    sleep(Duration::from_millis(1_000)).await;

    let engine_client = svm_runner_client::RunnerClient::new_with_commitment(
        config.rpc_addr.clone(),
        "ws://0.0.0.0:8989".to_owned(),
        CommitmentConfig::confirmed(),
    )
    .await;

    // Fund the payer account
    engine_client
        .request_airdrop(&payer.pubkey(), 100 * LAMPORTS_PER_SOL)
        .await
        .unwrap();

    sleep(Duration::from_millis(800)).await; // Wait for the airdrop to complete
    let balance = engine_client.get_balance(&payer.pubkey()).await.unwrap();
    assert_eq!(balance, 100 * LAMPORTS_PER_SOL);

    let lock_duration = 10;
    initialize_lock(
        &engine_client,
        program_id,
        target,
        payer.clone(),
        lock_duration,
    )
    .await;

    // Initialize the gated program via the proxy. This executes on the Engine.
    // First create the instruction and data structure for the program (including
    // PDA accounts that will be created), then attach that to the Proxy instruction.
    // Set up the Proxy structure for the proxy program (including PDA accounts).
    // Finally, send that to the engine. The Proxy program will run the user program's
    // initialization ("Hello", here) via CPI.

    // Note, there is no separate initialization step here for the user program.
    // Only one thing executes, and it creates the "Hello, World!" string inside
    // the message_account, and this test passes if that step succeeds.

    let target_program_instruction = example_gated_program::instruction::Hello {
        message: "Hello, World!".to_string(),
    };
    let target_program_accounts = example_gated_program::accounts::Hello {
        pda_signer,
        message: message_account,
        payer: payer.pubkey(),
        system_program: solana_sdk::system_program::id(),
    };

    let proxy_instruction = proxy::instruction::Proxy {
        instruction_data: target_program_instruction.data(),
        account_metas: bincode::serialize(&target_program_accounts.to_account_metas(None)).unwrap(),
    };

    let proxy_accounts = proxy::accounts::Proxy {
        lock: find_lock_address(program_id, target),
        lock_config: find_lock_config_address(program_id, target),
        pda_signer: find_lock_signer_address(program_id, target),
        payer: payer.pubkey(),
        target,
        system_program: solana_sdk::system_program::id(),
    };

    let mut target_vec = target_program_accounts.to_account_metas(None);
    target_vec.first_mut().unwrap().is_signer = false; // Make sure the PDA signer is not a signer
    let proxy_ix = Instruction {
        accounts: [proxy_accounts.to_account_metas(None), target_vec].concat(),
        data: proxy_instruction.data(),
        program_id,
    };
    let transaction = Transaction::new_signed_with_payer(
        &[proxy_ix],
        Some(&payer.pubkey()),
        &[payer.clone()],
        engine_client.get_latest_blockhash().await.unwrap(),
    );

    engine_client
        .runner_send_transaction(&transaction)
        .await
        .expect("Error sending runner transaction for the example gated program");
    sleep(Duration::from_millis(50)).await;

    // Confirm the transaction changed the state of the message account to hold "Hello, World!"
    let updated_state = engine_client
        .runner_get_account_info(&message_account, None)
        .await
        .expect("Could not get engine's account info for the message account");
    let updated_account: Account = updated_state
        .value
        .expect("Message account could not be loaded")
        .decode()
        .expect("Updated state of the message account is not decodable, likely because it's empty");
    let updated_message = Message::try_from_slice(&updated_account.data[DISCRIMINATOR_LENGTH..])
        .expect("Could not extract message from updated state of the message account");

    assert_eq!(updated_message.message, "Hello, World!");

    // Confirm the transaction succeeded using the RPC call
    let signatures = vec![transaction.signatures[0]];
    let statuses = engine_client
        .runner_get_signature_statuses(&signatures, None)
        .await
        .expect("get_signature_statuses failed")
        .value;

    assert_eq!(statuses.len(), 1);
    assert!(statuses[0].is_some());
    assert!(statuses[0].clone().unwrap().err.is_none());
    assert!(statuses[0].clone().unwrap().confirmation_status.is_some());
    assert_eq!(
        statuses[0].clone().unwrap().confirmation_status.unwrap(),
        TransactionConfirmationStatus::Finalized
    );

    // Wait for the transaction to get settled to L1
    sleep(Duration::from_millis(1_000)).await;

    let on_chain_state = engine_client
        .get_account_data(&message_account)
        .await
        .expect("Could not get validator account info for example gated program's message account");

    let on_chain_state = Message::try_from_slice(&on_chain_state[DISCRIMINATOR_LENGTH..]).unwrap();
    assert_eq!(on_chain_state.message, "Hello, World!");

    // tokio::join!(manager_handle).0.unwrap();
}

async fn initialize_lock(
    client: &RunnerClient,
    proxy_program_id: Pubkey,
    target_program_id: Pubkey,
    payer: Arc<Keypair>,
    lock_duration: u16,
) {
    // Initialize the PDA for the lock and lock-config accounts, plus any other proxy initializations
    let lock = find_lock_address(proxy_program_id, target_program_id);
    let lock_config = find_lock_config_address(proxy_program_id, target_program_id);
    let initialize_lock_ix = Instruction {
        accounts: proxy::accounts::Initialize {
            lock,
            lock_config,
            payer: payer.pubkey(),
            system_program: solana_sdk::system_program::id(),
            target: target_program_id,
        }
        .to_account_metas(None),
        data: proxy::instruction::Initialize { lock_duration }.data(),
        program_id: proxy_program_id,
    };

    let transaction = Transaction::new_signed_with_payer(
        &[initialize_lock_ix],
        Some(&payer.pubkey()),
        &[payer.clone()],
        client.get_latest_blockhash().await.unwrap(),
    );

    let _signature = client
        .runner_send_transaction(&transaction)
        .await
        .expect("Initialize lock failed to send transaction");
    sleep(Duration::from_millis(1000)).await;
}
