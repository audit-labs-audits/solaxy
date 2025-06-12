#[derive(clap::ValueEnum, Clone, Debug, Copy)]
pub enum SupportedDaLayer {
    Celestia,
    Mock,
    Solana,
}

pub mod celestia {
    use sov_celestia_adapter::{
        types::Namespace,
        verifier::{CelestiaVerifier, RollupParams},
    };
    pub use sov_celestia_adapter::{
        verifier::CelestiaSpec as DaSpec, CelestiaService as DaService,
    };
    use sov_modules_api::{prelude::tokio::sync::watch::Receiver, Spec};
    use sov_rollup_interface::da::DaVerifier;
    use sov_stf_runner::RollupConfig;

    /// The rollup stores its data in the namespace "sov-test-b" on Celestia.
    /// You can change this constant to point your rollup at a different namespace.
    const ROLLUP_BATCH_NAMESPACE: Namespace = Namespace::const_v0(*b"sov-test-b");

    /// The rollup stores the zk proofs in the namespace "sov-test-p" on Celestia.
    const ROLLUP_PROOF_NAMESPACE: Namespace = Namespace::const_v0(*b"sov-test-p");

    pub fn new_verifier() -> CelestiaVerifier {
        CelestiaVerifier::new(RollupParams {
            rollup_batch_namespace: ROLLUP_BATCH_NAMESPACE,
            rollup_proof_namespace: ROLLUP_PROOF_NAMESPACE,
        })
    }

    pub async fn new_da_service<S: Spec>(
        rollup_config: &RollupConfig<S::Address, DaService>,
        _shutdown_receiver: Receiver<()>,
    ) -> DaService {
        DaService::new(
            rollup_config.da.clone(),
            RollupParams {
                rollup_batch_namespace: ROLLUP_BATCH_NAMESPACE,
                rollup_proof_namespace: ROLLUP_PROOF_NAMESPACE,
            },
        )
        .await
    }
}

pub mod mock {
    use sov_mock_da::MockDaVerifier;
    pub use sov_mock_da::{
        storable::service::StorableMockDaService as DaService, MockDaSpec as DaSpec,
    };
    use sov_modules_api::{prelude::tokio::sync::watch::Receiver, Spec};
    use sov_stf_runner::RollupConfig;

    pub fn new_verifier() -> MockDaVerifier {
        MockDaVerifier::default()
    }

    pub async fn new_da_service<S: Spec>(
        rollup_config: &RollupConfig<S::Address, DaService>,
        shutdown_receiver: Receiver<()>,
    ) -> DaService {
        DaService::from_config(rollup_config.da.clone(), shutdown_receiver).await
    }
}

pub mod solana {
    use std::sync::Arc;

    use nitro_da_client::BloberClient;
    use solana_cli_config::Config;
    pub use solana_da::{
        SolanaChainParams, SolanaDASpec as DaSpec, SolanaService as DaService, SolanaVerifier,
    };
    use solana_program::pubkey::Pubkey;
    use solana_sdk::{
        signature::{read_keypair_file, Keypair},
        signer::Signer,
    };
    use sov_modules_api::{prelude::tokio::sync::watch::Receiver, Spec};
    use sov_rollup_interface::da::DaVerifier;
    use sov_stf_runner::RollupConfig;

    pub fn new_verifier(program_id: Pubkey, payer: Pubkey) -> SolanaVerifier {
        SolanaVerifier::new(SolanaChainParams::new(program_id, payer))
    }

    struct SolanaDaConfig {
        cli: Config,
        payer: Arc<Keypair>,
    }

    fn read_config() -> SolanaDaConfig {
        let default_file = solana_cli_config::CONFIG_FILE
            .as_ref()
            .expect("unable to get solana config file path")
            .to_owned();

        let solana_config_file = std::env::var("SOLANA_CONFIG_FILE").unwrap_or(default_file);

        let mut cli = Config::load(&solana_config_file).expect("unable to load solana config file");

        if cli.websocket_url.is_empty() {
            cli.websocket_url = Config::compute_websocket_url(&cli.json_rpc_url);
        }

        let payer =
            Arc::new(read_keypair_file(&cli.keypair_path).expect("unable to read keypair file"));

        SolanaDaConfig { cli, payer }
    }

    pub async fn new_da_service<S: Spec>(
        rollup_config: &RollupConfig<S::Address, DaService>,
        _shutdown_receiver: Receiver<()>,
    ) -> DaService {
        let SolanaDaConfig { cli, payer } = read_config();
        tracing::warn!("cli: {cli:?}");

        let program_id = rollup_config.da.program_id.into();

        let blober_client = BloberClient::builder()
            .payer(payer.clone())
            .program_id(program_id)
            .indexer_from_url(&rollup_config.da.indexer_url)
            .await
            .expect("to build indexer client")
            .build_with_config(cli)
            .await
            .expect("to build default rpc and batch clients");

        let params = SolanaChainParams::new(program_id, payer.pubkey());

        tracing::warn!("params: {params:?}");

        let config = rollup_config.da.clone();

        DaService::builder()
            .solana_rpc_client(blober_client.rpc_client())
            .blober_client(blober_client)
            .params(params)
            .config(config)
            .build()
    }
}
