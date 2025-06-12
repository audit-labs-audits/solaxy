# Svm Rollup ![Time - ~5 mins](https://img.shields.io/badge/Time-~5_mins-informational)

This is a demo full node running a simple Sovereign SDK rollup on [SOLANA](https://solana.com/).

<p align="center">
  <img width="50%" src="../../docs/assets/discord-banner.png">
  <br>
  <i>Stuck, facing problems, or unsure about something?</i>
  <br>
  <i>Join our <a href="https://discord.gg/kbykCcPrcA">Discord</a> and ask your questions in <code>#support</code>!</i>
</p>

You can follow the steps below to run the demo rollup on a local Solana devnet instance. However, due to numerous users encountering failures because of basic local setup or Docker issues, we strongly suggest using the plain demo rollup with mock Data Availability (DA) for testing.
We are developing more robust tooling to enable seamless deployment of rollups on any DA layer. Until this tooling is available, we will only support our early partners in deploying on devnets.

#### Table of Contents

<!-- https://github.com/thlorenz/doctoc -->
<!-- $ doctoc README_SOLANA.md --github --notitle -->
<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [What is This?](#what-is-this)
- [Getting Started](#getting-started)
    - [Run a local DA layer instance](#run-a-local-da-layer-instance)
    - [Start the Rollup Full Node](#start-the-rollup-full-node)
    - [Sanity Check: Creating a Token](#sanity-check-creating-a-token)
    - [How to Submit Transactions](#how-to-submit-transactions)
        - [1. Build `sov-cli`](#1-build-sov-cli)
        - [2. Generate the Transaction](#2-generate-the-transaction)
        - [3. Submit the Transaction(s)](#3-submit-the-transactions)
        - [4. Verify the Token Supply](#4-verify-the-token-supply)
    - [Makefile](#makefile)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

## What is This?

This demo shows how to integrate a State Transition Function (STF) with a Data Availability (DA) layer and a zkVM to create a full
zk-rollup. The code in this repository corresponds to running a full-node of the rollup, which executes
every transaction.

By swapping out or modifying the imported state transition function, you can customize
this example full-node to run arbitrary logic.
This particular example relies on the state transition exported by [`stf`](../stf/). If you want to
understand how to build your own state transition function, check out at the docs in that package.

## Getting Started

If you are looking for a simple rollup with minimal dependencies as a starting point, please have a look here:
[sov-rollup-starter](https://github.com/Sovereign-Labs/sov-rollup-starter/)

If you don't need ZK guest to be compiled, for faster compilation time you can export `export SKIP_GUEST_BUILD=1`
environment
variable in each terminal you run. By default, demo-rollup disables proving. If you want to enable proving, several options
are available:

- `export SOV_PROVER_MODE=skip` Skips verification logic.
- `export SOV_PROVER_MODE=simulate` Run the rollup verification logic inside the current process.
- `export SOV_PROVER_MODE=execute` Run the rollup verifier in a zkVM executor.
- `export SOV_PROVER_MODE=prove` Run the rollup verifier and create a SNARK of execution.

### Run a local DA layer instance

1. Install Docker: <https://www.docker.com>.

2. Follow [this guide](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry#authenticating-with-a-personal-access-token-classic)
   to authorize yourself in github's container registry.

```shell
# this has to be ran only once, unless your token expires
$ echo $MY_PERSONAL_GITHUB_TOKEN | docker login ghcr.io -u $MY_GITHUB_USERNAME --password-stdin
```

3. Switch to the `svm-rollup` directory (which is the parent directory of where this `README.md` is located!), and compile the application:

```shell,test-ci
$ cd ../
$ make build
```

4. Spin up a local Solana instance as your DA layer. We've built a small Makefile to simplify that process:

```sh,test-ci
$ export SOV_PROVER_MODE=execute
```

```sh,test-ci
$ export SOLANA_CONFIG_FILE=$(make solana-config)
```

```sh,test-ci,bashtestmd:wait-until=genesis.json
$ make clean
# Make sure to run `make stop` or `make clean` when you're done with the rollup!
$ make start-solana
```

### Start the Rollup Full Node

Now run the rollup full node, as shown below. You will see it consuming blocks from the Solana node running inside Docker:

```sh,test-ci,bashtestmd:long-running,bashtestmd:wait-until=rpc_address
# Make sure you're still in the examples/demo-rollup directory and `make build` has been executed before
$ ../../target/debug/svm-rollup --da-layer solana --rollup-config-path solana_rollup_config.toml --genesis-config-dir ../test-data/genesis/demo/solana
2024-12-02T18:07:18.701356Z  INFO sov_demo_rollup: Running demo rollup with prover config prover_config=None
2024-12-02T18:07:18.701428Z  INFO prometheus_exporter: exporting metrics to http://127.0.0.1:9845/metrics    
2024-12-02T18:07:18.701461Z DEBUG sov_demo_rollup: Starting Solana rollup config_path="solana_rollup_config.toml"
2024-12-02T18:07:18.701604Z DEBUG sov_stf_runner::config: Parsing config file size_in_bytes=873 contents="[da]\nindexer_url = \"ws://localhost:9696\"\npriority = \"Medium\"\n\n[storage]\n# The path to the rollup's data directory. Paths that do not begin with `/` are interpreted as relative paths.\npath = \"demo_data\"\n\n# We define the rollup's genesis to occur at block number `genesis_height`. The rollup will ignore\n# any blocks before this height, and any blobs at this height will not be processed\n[runner]\ngenesis_height = 3\nda_polling_interval_ms = 10000\n\n[runner.rpc_config]\n# the host and port to bind the rpc server for\nbind_host = \"127.0.0.1\"\nbind_port = 12345\n[runner.axum_config]\nbind_host = \"127.0.0.1\"\nbind_port = 12346\n\n[proof_manager]\naggregated_proof_block_jump = 1\nprover_address = \"sov1l6n2cku82yfqld30lanm2nfw43n2auc8clw7r5u5m6s7p8jrm4zqrr8r94\"\n\n[sequencer]\nmax_allowed_blocks_behind = 5\nda_address = \"7VkXP2q6G2DdEXnkwTR9eqpdfRN17i4TJpiUA8xdXkaJ\"\n[sequencer.standard]\n"
2024-12-02T18:07:18.741395Z  INFO rockbound: Opened RocksDB rocksdb_name="state-db"
2024-12-02T18:07:18.754898Z  INFO rockbound: Opened RocksDB rocksdb_name="accessory-db"
2024-12-02T18:07:18.788028Z  INFO rockbound: Opened RocksDB rocksdb_name="ledger-db"
2024-12-02T18:07:18.895767Z  INFO rockbound: Opened RocksDB rocksdb_name="sequencer-db"
2024-12-02T18:07:18.934415Z  INFO sov_stf_runner::runner: No history detected. Initializing chain on the block header... header=sov_solana_adapter::spec::block_header::SolanaBlockHeader prev_hash=FgKJqLuLrNTUNEDZrK3z12GQ8Ktgy7oQ57tGbjzE5cto hash=2tg9xmR2YKUQvahLMA4GEYYTdkhn6ry8TTcgeaSh6Mgm height=3
2024-12-02T18:07:18.935036Z  INFO sov_chain_state::genesis: Starting chain state genesis... current_time=Time { secs: 0, nanos: 0 } operating_mode=Zk genesis_da_height=3 inner_code_commitment=Risc0MethodId([0, 0, 0, 0, 0, 0, 0, 0]) outer_code_commitment=MockCodeCommitment([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
2024-12-02T18:07:18.935313Z DEBUG sov_bank::genesis: Gas token token_id=token_1rwrh8gn2py0dl4vv65twgctmlwck6esm2as9dftumcw89kqqn3nqrduss6 token_name=sov-token
2024-12-02T18:07:18.935369Z DEBUG sov_bank::genesis: Genesis of the token token_config=TokenConfig { token_name: "sov-token", token_id: TokenIdBech32("token_1rwrh8gn2py0dl4vv65twgctmlwck6esm2as9dftumcw89kqqn3nqrduss6"), address_and_balances: [(AddressBech32("sov1l6n2cku82yfqld30lanm2nfw43n2auc8clw7r5u5m6s7p8jrm4zqrr8r94"), 1000000000), (AddressBech32("sov1x3jtvq0zwhj2ucsc4hqugskvralrulxvf53vwtkred93s2x9gmzs04jvyr"), 1000000000), (AddressBech32("sov15vspj48hpttzyvxu8kzq5klhvaczcpyxn6z6k0hwpwtzs4a6wkvqwr57gc"), 1000000000), (AddressBech32("sov1dnhqk4mdsj2kwv4xymt8a624xuahfx8906j9usdkx7ensfghndkq8p33f7"), 1000000000), (AddressBech32("sov1qypqxpq9qcrsszg2pvxq6rs0zqg3yyc5z5tpwxqergd3c8g7rusqqsn6hm"), 1000000000), (AddressBech32("sov1nc27sae83wj0prl0en4y9lldk6e7v9qz63k50tus95zpd8h84rfsfzczmv"), 1000000000), (AddressBech32("sov16wxfprgt9lhvd097kes7e2v4juz5rn8jrzt7477cp9dw0qldydas769lvu"), 1000000000)], authorized_minters: [AddressBech32("sov1l6n2cku82yfqld30lanm2nfw43n2auc8clw7r5u5m6s7p8jrm4zqrr8r94")] } token_id=token_1rwrh8gn2py0dl4vv65twgctmlwck6esm2as9dftumcw89kqqn3nqrduss6
2024-12-02T18:07:18.935796Z DEBUG sov_bank::genesis: Token has been created token_name=sov-token token_id=token_1rwrh8gn2py0dl4vv65twgctmlwck6esm2as9dftumcw89kqqn3nqrduss6
2024-12-02T18:07:18.935862Z  INFO sov_sequencer_registry::genesis: Starting sequencer registry genesis... sequencer_rollup_address=sov1l6n2cku82yfqld30lanm2nfw43n2auc8clw7r5u5m6s7p8jrm4zqrr8r94 sequencer_da_address=7VkXP2q6G2DdEXnkwTR9eqpdfRN17i4TJpiUA8xdXkaJ sequencer_bond=10000000 is_preferred_sequencer=true minimum_bond=GasUnit[5000, 5000]
2024-12-02T18:07:18.944916Z DEBUG sov_db::storage_manager: Finalizing changes block_header=sov_solana_adapter::spec::block_header::SolanaBlockHeader prev_hash=FgKJqLuLrNTUNEDZrK3z12GQ8Ktgy7oQ57tGbjzE5cto hash=2tg9xmR2YKUQvahLMA4GEYYTdkhn6ry8TTcgeaSh6Mgm height=3
2024-12-02T18:07:18.971097Z  INFO sov_stf_runner::runner: Chain initialization is done genesis_state_root="098e3059e4821fee439340a81461cdec0db3754a223e7fad005641a966971e8c097af137dd5cd44a0dc4744351651a05fba2aefe7b8c3085678a5484e3ffd4c5"
2024-12-02T18:07:18.971362Z DEBUG sov_stf_runner::runner: Initializing StfRunner last_slot_processed_before_shutdown=0 runner_config.genesis_height=3 first_unprocessed_height_at_startup=4
2024-12-02T18:07:18.971441Z  INFO sov_solana_adapter::service::service: get_last_finalized_block_header START
2024-12-02T18:07:18.974732Z  INFO sov_solana_adapter::service::service: get_last_finalized_block_header DONE
2024-12-02T18:07:18.975087Z  INFO sov_solana_adapter::service::service: get_head_block_header START
2024-12-02T18:07:18.975080Z  INFO sov_stf_runner::da_pre_fetcher: BlockFetcher synced all finalized headers
2024-12-02T18:07:18.975153Z DEBUG sov_stf_runner::da_pre_fetcher: BlockFetcher task has completed
2024-12-02T18:07:18.975119Z  INFO sov_stf_runner::runner: Starting RPC server rpc_address=127.0.0.1:12345
2024-12-02T18:07:18.975165Z  INFO sov_stf_runner::runner: Starting REST API server rest_address=127.0.0.1:12346
```

Leave it running while you proceed with the rest of the demo.

### Sanity Check: Creating a Token

After switching to a new terminal tab, let's submit our first transaction by creating a token:

```sh,test-ci
$ make test-create-token
```

...wait a few seconds and you will see the transaction receipt in the output:

```sh
Submitting a batch
/home/lucas/repos/rebased/sov-sdk2/target/debug/sov-cli node submit-batch --wait-for-processing by-nickname DANGER__DO_NOT_USE_WITH_REAL_MONEY
Submitting tx: 0: 0xa6ba39e5fc6041007310fd55cf91dfdd9f04d6730736c65067ec6dac35d9ae52
Your batch was submitted to the sequencer for publication. Response: SubmittedBatchInfo { da_height: 41, num_txs: 1 }
Going to wait for target rollup height 41 to be processed, up to 300s
Rollup has processed target DA height=41!
```

### How to Submit Transactions

The `make test-create-token` command above was useful to test if everything is running correctly. Now let's get a better understanding of how to create and submit a transaction.

#### 1. Build `sov-cli`

You'll need the `sov-cli` binary in order to create transactions. Build it with these commands:

```bash,test-ci,bashtestmd:compare-output
# Make sure you're still in `examples/demo-rollup` and `make build` has been executed previously
$ make check-sov-cli
$ ./../../target/debug/sov-cli --help
Usage: sov-cli <COMMAND>

Commands:
  transactions  Generate, sign, list and remove transactions
  keys          View and manage keys associated with this wallet
  node          Query the current state of the rollup and send transactions
  help          Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

Each transaction that we want to submit is a member of the `CallMessage` enum defined as part of creating a module. For example, let's consider the `Bank` module's `CallMessage`:

```rust
use sov_bank::CallMessage::Transfer;
use sov_bank::Coins;
use sov_bank::Amount;

pub enum CallMessage<S: sov_modules_api::Spec> {
    /// Creates a new token with the specified name and initial balance.
    CreateToken {
        /// Random value used to create a unique token ID.
        salt: u64,
        /// The name of the new token.
        token_name: String,
        /// The initial balance of the new token.
        initial_balance: Amount,
        /// The address of the account that the new tokens are minted to.
        mint_to_address: S::Address,
        /// Authorized minter list.
        authorized_minters: Vec<S::Address>,
    },

    /// Transfers a specified amount of tokens to the specified address.
    Transfer {
        /// The address to which the tokens will be transferred.
        to: S::Address,
        /// The amount of tokens to transfer.
        coins: Coins,
    },

    /// Burns a specified amount of tokens.
    Burn {
        /// The amount of tokens to burn.
        coins: Coins,
    },

    /// Mints a specified amount of tokens.
    Mint {
        /// The amount of tokens to mint.
        coins: Coins,
        /// Address to mint tokens to.
        mint_to_address: S::Address,
    },

    /// Freeze a token so that the supply is frozen.
    Freeze {
        /// Address of the token to be frozen.
        token_id: TokenId,
    },
}
```

In the above snippet, we can see that `CallMessage` in `Bank` supports five different types of calls. The `sov-cli` has the ability to parse a JSON file that aligns with any of these calls and subsequently serialize them. The structure of the JSON file, which represents the call, closely mirrors that of the Enum member. You can view the relevant JSON Schema for `Bank` [here](../../module-system/module-schemas/schemas/sov-bank.json) Consider the `Transfer` message as an example:

```rust
use sov_bank::Coins;

struct Transfer<S: sov_modules_api::Spec>  {
    /// The address to which the tokens will be transferred.
    to: S::Address,
    /// The amount of tokens to transfer.
    coins: Coins,
}
```

Here's an example of a JSON representing the above call:

```json
{
  "transfer": {
    "to": "sov1zgfpyysjzgfpyysjzgfpyysjzgfpyysjzgfpyysjzgfpyysjzgfqve8h6h",
    "coins": {
      "amount": 200,
      "token_id": "token_1rwrh8gn2py0dl4vv65twgctmlwck6esm2as9dftumcw89kqqn3nqrduss6"
    }
  }
}
```

#### 2. Generate the Transaction

The JSON above is the contents of the file [`examples/test-data/requests/transfer.json`](../../examples/test-data/requests/transfer.json). We'll use this transaction as our example for the rest of the tutorial. In order to send the transaction, we need to perform 2 operations:

- Import the transaction data into the wallet
- Sign and submit the transaction

Note: we're able to make a `Transfer` call here because we already created the token as part of the sanity check above, using `make test-create-token`.

To generate transactions, you can use the `transactions import from-file` subcommand, as shown below:

```bash,test-ci,bashtestmd:compare-output
$ ./../../target/debug/sov-cli transactions import from-file -h
Import a transaction from a JSON file at the provided path

Usage: sov-cli transactions import from-file <COMMAND>

Commands:
  bank                 A subcommand for the `Bank` module
  sequencer-registry   A subcommand for the `SequencerRegistry` module
  value-setter         A subcommand for the `ValueSetter` module
  attester-incentives  A subcommand for the `AttesterIncentives` module
  prover-incentives    A subcommand for the `ProverIncentives` module
  accounts             A subcommand for the `Accounts` module
  nonces               A subcommand for the `Nonces` module
  nft                  A subcommand for the `Nft` module
  chain-state          A subcommand for the `ChainState` module
  blob-storage         A subcommand for the `BlobStorage` module
  paymaster            A subcommand for the `Paymaster` module
  help                 Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

Let's go ahead and import the transaction into the wallet

```bash,test-ci,bashtestmd:compare-output
$ ./../../target/debug/sov-cli transactions import from-file bank --chain-id 4321 --max-fee 100000000 --path ../test-data/requests/transfer.json
Adding the following transaction to batch:
{
  "tx": {
    "bank": {
      "transfer": {
        "to": "sov1l6n2cku82yfqld30lanm2nfw43n2auc8clw7r5u5m6s7p8jrm4zqrr8r94",
        "coins": {
          "amount": 200,
          "token_id": "token_1nyl0e0yweragfsatygt24zmd8jrr2vqtvdfptzjhxkguz2xxx3vs0y07u7"
        }
      }
    }
  },
  "chain_hash": "0x44187785252131f9a1ccd1bda83ac148d2cd3c8c6de1c8b9f0791a8df83870f6",
  "details": {
    "max_priority_fee_bips": 0,
    "max_fee": 100000000,
    "gas_limit": null,
    "chain_id": 4321
  }
}
```

This output indicates that the wallet has saved the transaction details for later signing.

#### 3. Submit the Transaction(s)

You now have a batch with a single transaction in your wallet. If you want to submit any more transactions as part of this
batch, you can import them now. Finally, let's submit your transaction to the rollup.
'true' parameter after `submit-batch` indicates, that command will wait for batch to be processed by the node.

```bash,test-ci
$ ./../../target/debug/sov-cli node submit-batch --wait-for-processing by-address sov1l6n2cku82yfqld30lanm2nfw43n2auc8clw7r5u5m6s7p8jrm4zqrr8r94 
```

This command will use your default private key.

#### 4. Verify the Token Supply

```bash,test-ci,bashtestmd:compare-output
$ curl -Ss http://127.0.0.1:12346/modules/bank/tokens/token_17zrpsyv06x7wmf2hg878gg5szwurckr3e2u77fvrdmanjhve8r2sj4jy42/total-supply | jq -c -M
{"data":{"amount":1000000,"token_id":"token_17zrpsyv06x7wmf2hg878gg5szwurckr3e2u77fvrdmanjhve8r2sj4jy42"},"meta":{}}
```

### Makefile

`demo-rollup/Makefile` automates a number of things for convenience:

- Starts docker compose with a Solana network for a local setup
- `make solana-config`:
    - creates a temporary directory with Solana config files to avoid overriding user's configs
    - copies solana config and demo key from `docker/solana` directory to this temporary one
- `make start-solana`:
    - Performs a number of checks to ensure services are not already running
    - Starts the docker compose setup
    - Exposes the RPC port `26658`
    - Waits until the container is started
- `make stop`:
    - Shuts down the docker compose setup if running.
- `make clean`:
    - Removes `demo-data` (or the configured path of the rollup database from rollup_config.toml)
    - Removes pending transactions from `~/.sov_cli_wallet`. Keys are not touched.