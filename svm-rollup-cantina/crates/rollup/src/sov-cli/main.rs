use sov_modules_api::cli::{FileNameArg, JsonStringArg};
use sov_modules_rollup_blueprint::WalletBlueprint;
use stf::runtime::RuntimeSubcommand;
use svm_rollup::solana_rollup::SolanaRollup;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    SolanaRollup::run_wallet::<
        RuntimeSubcommand<FileNameArg, _>,
        RuntimeSubcommand<JsonStringArg, _>,
    >()
    .await
}
