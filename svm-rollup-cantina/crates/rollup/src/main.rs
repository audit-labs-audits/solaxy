use std::{path::PathBuf, process::exit};

use anyhow::Context;
use clap::Parser;
use sov_modules_rollup_blueprint::{logging::initialize_logging, FullNodeBlueprint, Rollup};
use sov_rollup_interface::execution_mode::Native;
use sov_stf_runner::{
    from_toml_path,
    processes::{RollupProverConfig, RollupProverConfigDiscriminants},
    RollupConfig,
};
use stf::genesis_config::GenesisPaths;
use svm_rollup::{
    celestia_rollup::CelestiaSvmRollup,
    da::{celestia, mock, solana, SupportedDaLayer},
    mock_rollup::MockSvmRollup,
    solana_rollup::SolanaRollup,
    zk::{rollup_host_args, InnerZkvm},
};
use svm_types::MultiAddressSvm;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The data layer type.
    #[arg(long, default_value = "mock")]
    da_layer: SupportedDaLayer,

    /// The path to the rollup config.
    #[arg(long, default_value = "mock_rollup_config.toml")]
    rollup_config_path: PathBuf,

    /// The path to the genesis configs.
    #[arg(long, default_value = "../test-data/genesis/demo/mock")]
    genesis_config_dir: PathBuf,

    /// Listen address for Prometheus exporter.
    #[arg(long, default_value = "127.0.0.1:9845")]
    prometheus_exporter_bind: String,
}

#[tokio::main]
// Not returning result here, so error could be logged properly.
async fn main() {
    initialize_logging();

    match run().await {
        Ok(_) => {
            tracing::debug!("Rollup execution complete. Shutting down.");
        }
        Err(e) => {
            tracing::error!(error = ?e, backtrace= e.backtrace().to_string(), "Rollup execution failed");
            exit(1);
        }
    }
}

async fn run() -> anyhow::Result<()> {
    let args = Args::parse();
    prometheus_exporter::start(args.prometheus_exporter_bind.parse()?)
        .context("Prometheus exporter start failed")?;

    let prover_config_disc = parse_prover_config().expect("Malformed prover_config");
    tracing::info!(?prover_config_disc, "Running svm rollup with prover config");

    let prover_config = prover_config_disc
        .map(|config_disc| config_disc.into_config(rollup_host_args(args.da_layer)));

    match args.da_layer {
        SupportedDaLayer::Mock => {
            let rollup = new_rollup_with_mock_da(
                args.genesis_config_dir,
                args.rollup_config_path,
                prover_config,
            )
            .await
            .context("Failed to initialize MockDa rollup")?;
            rollup.run().await
        }
        SupportedDaLayer::Celestia => {
            let rollup = new_rollup_with_celestia_da(
                args.genesis_config_dir,
                args.rollup_config_path,
                prover_config,
            )
            .await
            .context("Failed to initialize Celestia rollup")?;
            rollup.run().await
        }
        SupportedDaLayer::Solana => {
            let rollup = new_rollup_with_solana_da(
                args.genesis_config_dir,
                args.rollup_config_path,
                prover_config,
            )
            .await
            .context("Failed to initialize Solana rollup")?;
            rollup.run().await
        }
    }
}

fn parse_prover_config() -> anyhow::Result<Option<RollupProverConfigDiscriminants>> {
    if let Some(value) = option_env!("SOV_PROVER_MODE") {
        let config = std::str::FromStr::from_str(value).inspect_err(|error| {
            tracing::error!(value, ?error, "Unknown `SOV_PROVER_MODE` value; aborting");
        })?;
        Ok(Some(config))
    } else {
        Ok(None)
    }
}

async fn new_rollup_with_celestia_da(
    genesis_path: PathBuf,
    rollup_config_path: PathBuf,
    prover_config: Option<RollupProverConfig<InnerZkvm>>,
) -> anyhow::Result<Rollup<CelestiaSvmRollup<Native>, Native>> {
    tracing::info!(?rollup_config_path, "Starting Celestia rollup with config");

    let rollup_config: RollupConfig<MultiAddressSvm, celestia::DaService> =
        from_toml_path(&rollup_config_path).with_context(|| {
            format!(
                "Failed to read rollup configuration from {}",
                rollup_config_path.to_str().unwrap()
            )
        })?;

    let genesis_paths = &GenesisPaths::from_dir(&genesis_path);
    let rollup = CelestiaSvmRollup::new(genesis_paths.clone());

    rollup
        .create_new_rollup(genesis_paths, rollup_config, prover_config)
        .await
}

async fn new_rollup_with_mock_da(
    genesis_path: PathBuf,
    rollup_config_path: PathBuf,
    prover_config: Option<RollupProverConfig<InnerZkvm>>,
) -> anyhow::Result<Rollup<MockSvmRollup<Native>, Native>> {
    tracing::info!(?rollup_config_path, "Starting mock rollup with config");

    let rollup_config: RollupConfig<MultiAddressSvm, mock::DaService> =
        from_toml_path(&rollup_config_path).with_context(|| {
            format!(
                "Failed to read rollup configuration from {}",
                rollup_config_path.to_str().unwrap()
            )
        })?;

    let genesis_paths = &GenesisPaths::from_dir(&genesis_path);
    let rollup = MockSvmRollup::new(genesis_paths.clone());

    rollup
        .create_new_rollup(genesis_paths, rollup_config, prover_config)
        .await
}

async fn new_rollup_with_solana_da(
    genesis_path: PathBuf,
    rollup_config_path: PathBuf,
    prover_config: Option<RollupProverConfig<InnerZkvm>>,
) -> anyhow::Result<Rollup<SolanaRollup<Native>, Native>> {
    tracing::info!(?rollup_config_path, "Starting Solana rollup");

    let rollup_config: RollupConfig<MultiAddressSvm, solana::DaService> =
        from_toml_path(&rollup_config_path).with_context(|| {
            format!(
                "Failed to read rollup configuration from {}",
                rollup_config_path.to_str().unwrap()
            )
        })?;

    let genesis_paths = &GenesisPaths::from_dir(&genesis_path);
    let solana_rollup = SolanaRollup::new(genesis_paths.clone());

    solana_rollup
        .create_new_rollup(genesis_paths, rollup_config, prover_config)
        .await
}
