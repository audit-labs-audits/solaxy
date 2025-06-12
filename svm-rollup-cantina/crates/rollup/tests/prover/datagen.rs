use std::env;

use sov_cli::wallet_state::PrivateKeyAndAddress;
use sov_mock_da::{MockAddress, MockBlock, MockDaService};
use sov_modules_api::execution_mode::{Native, WitnessGeneration};
use sov_rollup_interface::node::da::DaService;
use sov_test_utils::{
    generators::{bank::BankMessageGenerator, BlobBuildingCtx},
    MessageGenerator,
};
use svm_rollup::{
    mock_rollup::MockSvmRollup,
    zk::{InnerCryptoSpec, InnerZkvm},
};

use crate::{prover::MockDaSpec, test_helpers::read_private_keys};

type S = svm_types::SolanaSpec<
    MockDaSpec,
    InnerZkvm,
    sov_mock_zkvm::MockZkvm,
    InnerCryptoSpec,
    WitnessGeneration,
    svm_types::NativeStorage,
>;

const DEFAULT_BLOCKS: u64 = 10;
const DEFAULT_TXNS_PER_BLOCK: u64 = 100;

pub async fn get_blocks_from_da(mode: BlobBuildingCtx) -> anyhow::Result<Vec<MockBlock>> {
    let txns_per_block = match env::var("TXNS_PER_BLOCK") {
        Ok(txns_per_block) => txns_per_block.parse::<u64>()?,
        Err(_) => {
            println!("TXNS_PER_BLOCK not set, using default");
            DEFAULT_TXNS_PER_BLOCK
        }
    };

    let _block_cnt = match env::var("BLOCKS") {
        Ok(block_cnt_str) => block_cnt_str.parse::<u64>()?,
        Err(_) => {
            println!("BLOCKS not set, using default");
            DEFAULT_BLOCKS
        }
    };

    let da_service = MockDaService::new(MockAddress::default());
    let mut blocks = Vec::new();

    let ka1: PrivateKeyAndAddress<S> = read_private_keys::<S>("minter_private_key.json");
    let ka2: PrivateKeyAndAddress<S> = read_private_keys::<S>("tx_signer_private_key.json");
    let ka3: PrivateKeyAndAddress<S> = read_private_keys::<S>("token_deployer_private_key.json");

    for (i, key_addr) in [ka1, ka2, ka3].iter().enumerate() {
        let msg_gen = {
            let (create_token_message_gen, transfer_message_gen) =
                BankMessageGenerator::generate_token_and_random_transfers(
                    txns_per_block,
                    key_addr.private_key.clone(),
                );
            BankMessageGenerator {
                token_create_txs: create_token_message_gen.token_create_txs,
                transfer_txs: transfer_message_gen.transfer_txs,
            }
        };

        let blob = msg_gen
            .create_blobs::<<MockSvmRollup<Native> as sov_modules_rollup_blueprint::RollupBlueprint<
            Native,
        >>::Runtime>(&mode);
        da_service
            .send_transaction(&blob)
            .await
            .await
            .expect("The transaction sender should not fail")
            .unwrap();
        let blocki = da_service.get_block_at((1 + i) as u64).await.unwrap();
        blocks.push(blocki);
    }

    Ok(blocks)
}
