# Rollup ![Time - ~5 mins](https://img.shields.io/badge/Time-~5_mins-informational)

<p align="center">
  <img width="50%" src="../../docs/assets/discord-banner.png">
  <br>
  <i>Stuck, facing problems, or unsure about something?</i>
  <br>
  <i>Join our <a href="https://discord.gg/kbykCcPrcA">Discord</a> and ask your questions in <code>#support</code>!</i>
</p>

#### Table of Contents

<!-- https://github.com/thlorenz/doctoc -->
<!-- $ doctoc README.md --github --notitle -->
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
    - [3. Make sure all the accounts involved have enough funds to pay for the transaction.](#3-make-sure-all-the-accounts-involved-have-enough-funds-to-pay-for-the-transaction)
    - [4. Submit the Transaction(s)](#4-submit-the-transactions)
    - [5. Verify the Token Supply](#5-verify-the-token-supply)
- [Disclaimer](#disclaimer)
- [Interacting with your Node via REST API](#interacting-with-your-node-via-rest-api)
- [Interacting with your Node via RPC](#interacting-with-your-node-via-rpc)
  - [Key Concepts](#key-concepts)
  - [RPC Methods](#rpc-methods)
    - [`ledger_getHead`](#ledger_gethead)
    - [`ledger_getSlots`](#ledger_getslots)
    - [`ledger_getBatches`](#ledger_getbatches)
    - [`ledger_getTransactions`](#ledger_gettransactions)
    - [`ledger_getEvents`](#ledger_getevents)
    - [`svm_queryModuleState`](#svm_querymodulestate)
    - [`svm_getAccountInfo`](#svm_getaccountinfo)
  - [SVM RPC Methods](#svm-rpc-methods)
    - [`svm_sendTransaction`](#svm_sendtransaction)
    - [`svm_publishBatch`](#svm_publishbatch)
- [Testing with specific DA layers](#testing-with-specific-da-layers)
- [License](#license)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

## What is This?

This demo shows how to integrate a State Transition Function (STF) with a Data Availability (DA) layer and a zkVM to
create a full
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
variable in each terminal you run. By default, rollup disables proving. If you want to enable proving, several options
are available:

- `export SOV_PROVER_MODE=skip` Skips verification logic.
- `export SOV_PROVER_MODE=simulate` Run the rollup verification logic inside the current process.
- `export SOV_PROVER_MODE=execute` Run the rollup verifier in a zkVM executor.
- `export SOV_PROVER_MODE=prove` Run the rollup verifier and create a SNARK of execution.

### Run a local DA layer instance

This setup works with an in-memory DA that is easy to set up for testing purposes.

### Start the Rollup Full Node

1. Switch to the `crates/rollup` and compile the application (skip risc0 build because it is not supported now):

```shell,test-ci
$ cd crates/rollup/
$ export SKIP_RISC0_GUEST_BUILD=1 
$ make build
```

2. Clean up the existing database.
   Makefile to simplify that process:

```sh,test-ci
$ make clean
```

3. Now run the rollup full node, as shown below (skip prover).

```sh,test-ci
$ export SOV_PROVER_MODE=skip
```

```sh,test-ci,bashtestmd:long-running,bashtestmd:wait-until=rpc_address
$ cargo run
```

Leave it running while you proceed with the rest of the demo.

### Sanity Check: Creating a Token

After switching to a new terminal tab, let's submit our first transaction by creating a token:

```sh,test-ci
$ make test-create-token
```

Once a batch is submitted the output should also contain the transaction hashes that have been submitted. For example -

```text
Your batch was submitted to the sequencer for publication. Response: "Submitted 1 transactions"
0: 0xfce2381221722b8114ba41a632c44f54384d0a31f332a64f7cbc3f667841d7f0
```

The transaction hash can be used to query the REST API endpoint to fetch events belonging to the transaction, which should in
this case have the TokenCreated Event

```sh,test-ci
$ curl -sS http://127.0.0.1:3000/ledger/txs/0xfce2381221722b8114ba41a632c44f54384d0a31f332a64f7cbc3f667841d7f0/events | jq
{
  "data": [
    {
      "type": "event",
      "number": 0,
      "key": "token_created",
      "value": {
        "TokenCreated": {
          "token_name": "sov-test-token",
          "coins": {
            "amount": 1000000,
            "token_id": "token_1zdwj8thgev2u3yyrrlekmvtsz4av4tp3m7dm5mx5peejnesga27ss0lusz"
          },
          "minter": {
            "User": "sov15vspj48hpttzyvxu8kzq5klhvaczcpyxn6z6k0hwpwtzs4a6wkvqwr57gc"
          },
          "authorized_minters": [
            {
              "User": "sov1l6n2cku82yfqld30lanm2nfw43n2auc8clw7r5u5m6s7p8jrm4zqrr8r94"
            },
            {
              "User": "sov15vspj48hpttzyvxu8kzq5klhvaczcpyxn6z6k0hwpwtzs4a6wkvqwr57gc"
            }
          ]
        }
      },
      "module": {
        "type": "moduleRef",
        "name": "Bank"
      }
    }
  ],
  "meta": {}
}
```

We can see the TokenCreated event which contains the id of the token
created - `token_1rwrh8gn2py0dl4vv65twgctmlwck6esm2as9dftumcw89kqqn3nqrduss6`

### How to Submit Transactions

The `make test-create-token` command above was useful to test if everything is running correctly. Now let's get a better
understanding of how to create and submit a transaction.

#### 1. Build `sov-cli`

You'll need the `sov-cli` binary in order to create transactions. Build it with these commands:

```bash,test-ci,bashtestmd:compare-output
# Make sure you're still in `crates/rollup`
$ SKIP_GUEST_BUILD=1 cargo build --bin sov-cli
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

Each transaction that we want to submit is a member of the `CallMessage` enum defined as part of creating a module. For
example, let's consider the `Bank` module's `CallMessage`:

```rust
use sov_bank::CallMessage::Transfer;
use sov_bank::Coins;
use sov_bank::TokenId;
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
        /// Address to mint tokens to
        mint_to_address: S::Address,
    },

    /// Freeze a token so that the supply is frozen
    Freeze {
        /// Address of the token to be frozen
        token_id: TokenId,
    },
}
```

In the above snippet, we can see that `CallMessage` in `Bank` supports five different types of calls. The `sov-cli` has
the ability to parse a JSON file that aligns with any of these calls and subsequently serialize them. The structure of
the JSON file, which represents the call, closely mirrors that of the Enum member. You can view the relevant JSON Schema
for `Bank` [here](https://github.com/Sovereign-Labs/sovereign-sdk-wip/blob/cc48cb75b58acdf02bb82847b6798fc7479b97bf/crates/module-system/module-schemas/schemas/sov-bank.json) Consider the `Transfer` message as an
example:

```rust
use sov_bank::Coins;

struct Transfer<S: sov_modules_api::Spec> {
    /// The address to which the tokens will be transferred.
    to: S::Address,
    /// The amount of tokens to transfer.
    coins: Coins,
}
```

Here's an example of a JSON representing the above call:

```json
{
  "Transfer": {
    "to": "sov1zgfpyysjzgfpyysjzgfpyysjzgfpyysjzgfpyysjzgfpyysjzgfqve8h6h",
    "coins": {
      "amount": 200,
      "token_id": "token_1rwrh8gn2py0dl4vv65twgctmlwck6esm2as9dftumcw89kqqn3nqrduss6"
    }
  }
}
```

#### 2. Generate the Transaction

The JSON above is the contents of the
file [`crates/test-data/requests/transfer.json`](../../crates/test-data/requests/transfer.json). We'll use this
transaction as our example for the rest of the tutorial. In order to send the transaction, we need to perform 2
operations:

- Import the transaction data into the wallet
- Sign and submit the transaction

Note: we're able to make a `Transfer` call here because we already created the token as part of the sanity check above,
using `make test-create-token`.

To generate transactions you can use the `transactions import from-file` subcommand, as shown below:

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
  help                 Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

Let's go ahead and import the transaction into the wallet

```bash,test-ci,bashtestmd:compare-output
$ ./../../target/debug/sov-cli transactions import from-file bank --max-fee 100000000 --path ../test-data/requests/transfer.json
Adding the following transaction to batch:
{
  "tx": {
    "Bank": {
      "Transfer": {
        "to": "sov1l6n2cku82yfqld30lanm2nfw43n2auc8clw7r5u5m6s7p8jrm4zqrr8r94",
        "coins": {
          "amount": 200,
          "token_id": "token_1rwrh8gn2py0dl4vv65twgctmlwck6esm2as9dftumcw89kqqn3nqrduss6"
        }
      }
    }
  },
  "details": {
    "max_priority_fee_bips": 0,
    "max_fee": 100000000,
    "gas_limit": null,
    "chain_id": 4321
  }
}
```

#### 3. Make sure all the accounts involved have enough funds to pay for the transaction.

For the transaction to be processed successfully, you have to ensure that the sender account has enough funds to pay for the transaction fees and the sequencer has staked enough tokens to pay for the pre-execution checks. This `README` file uses addresses from the `crates/test-data/genesis/demo/mock` folder, which are pre-populated with enough funds.

To be able to execute most simple transactions, the transaction sender should have about `1_000_000_000` tokens on their account and the sequencer should have staked `100_000_000` tokens in the registry.

More details can be found in the Sovereign book [available here](https://github.com/Sovereign-Labs/sovereign-book).


#### 4. Submit the Transaction(s)

You now have a batch with a single transaction in your wallet. If you want to submit any more transactions as part of
this
batch, you can import them now. Finally, let's submit your transaction to the rollup.

```bash,test-ci
$ ./../../target/debug/sov-cli node submit-batch --wait-for-processing by-address HjjEhif8MU9DtnXtZc5hkBu9XLAkAYe1qwzhDoxbcECv
```

#### 5. Verify the Token Supply

```bash,test-ci,bashtestmd:compare-output
$ curl -Ss http://127.0.0.1:3000/modules/bank/tokens/token_1zdwj8thgev2u3yyrrlekmvtsz4av4tp3m7dm5mx5peejnesga27ss0lusz/total-supply | jq -c -M
{"data":{"amount":1000000,"token_id":"token_1zdwj8thgev2u3yyrrlekmvtsz4av4tp3m7dm5mx5peejnesga27ss0lusz"},"meta":{}}
```

```bash,test-ci,bashtestmd:compare-output
$ curl -sS http://127.0.0.1:3000/ledger/aggregated-proofs/latest | jq 'if .data.publicData.initialSlotNumber >= 1 then true else false end'
true
```

## Disclaimer

> ⚠️ Warning! ⚠️

`rollup` is a prototype! It contains known vulnerabilities and should not be used in production under any
circumstances.

## Interacting with your Node via REST API

By default, this implementation prints the state root and the number of blobs processed for each slot. To access any
other data, you'll
want to use our REST API server. You can configure its host and port in `rollup_config.toml`.

You can get an overview of all available endpoints by reading the OpenAPI specification [here](https://github.com/Sovereign-Labs/sovereign-sdk-wip/blob/cc48cb75b58acdf02bb82847b6798fc7479b97bf/crates/full-node/sov-ledger-apis/openapi-v3.yaml). Here's just a few example queries:

- `http://localhost:3000/ledger/events/17`
- `http://localhost:3000/ledger/txs/50/events/0`
- `http://localhost:3000/ledger/txs/0/events?key=base64key`
- `http://localhost:3000/ledger/batches/10/txs/2/events/0`

## Interacting with your Node via RPC

By default, this implementation prints the state root and the number of blobs processed for each slot. To access any
other data, you'll
want to use our RPC server. You can configure its host and port in `rollup_config.toml`.

### Key Concepts

**Query Modes**

Most queries for ledger information accept an optional `QueryMode` argument. There are three QueryModes:

- `Standard`. In Standard mode, a response to a query for an outer struct will contain the full outer struct and hashes
  of inner structs. For example
  a standard `ledger_getSlots` query would return all information relating to the requested slot, but only the hashes of
  the batches contained therein.
  If no `QueryMode` is specified, a `Standard` response will be returned
- `Compact`. In Compact mode, even the hashes of child structs are omitted.
- `Full`. In Full mode, child structs are recursively expanded. So, for example, a query for a slot would return the
  slot's data, as well as data relating
  to any `batches` that occurred in that slot, any transactions in those batches, and any events that were emitted by
  those transactions.

**Identifiers**

There are several ways to uniquely identify items in the ledger DB.

- By _number_. Each family of structs (`slots`, `blocks`, `transactions`, and `events`) is numbered in order starting
  from `1`. So, for example, the
  first transaction to appear on the DA layer will be numered `1` and might emit events `1`-`5`. Or, slot `17` might
  contain batches `41` - `44`.
- By _hash_. (`slots`, `blocks`, and `transactions` only)
- By _containing item_id and offset_.
- (`Events` only) By _transaction_id and key_.

To request an item from the ledger DB, you can provide any identifier - and even mix and match different identifiers. We
recommend using item number
wherever possible, though, since resolving other identifiers may require additional database lookups.

Some examples will make this clearer. Suppose that slot number `5` contains batches `9`, `10`, and `11`, that batch `10`
contains
transactions `50`-`81`, and that transaction `52` emits event number `17`. If we want to fetch events number `17`, we
can use any of the following queries:

- `{"jsonrpc":"2.0","method":"ledger_getEvents","params":[[17]], ... }`
- `{"jsonrpc":"2.0","method":"ledger_getEvents","params":[[{"transaction_id": 50, "offset": 0}]], ... }`
- `{"jsonrpc":"2.0","method":"ledger_getEvents","params":[[{"transaction_id": 50, "key": [1, 2, 4, 2, ...]}]], ... }`
- `{"jsonrpc":"2.0","method":"ledger_getEvents","params":[[{"transaction_id": { "batch_id": 10, "offset": 2}, "offset": 0}]], ... }`

### RPC Methods

#### `ledger_getHead`

This method returns the current head of the ledger. It has no arguments.

**Example Query:**

```shell
$ curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"ledger_getHead","params":[],"id":1}' http://127.0.0.1:8899

{"jsonrpc":"2.0","result":{"number":22019,"hash":"0xe8daef0f58a558aea44632a420bb62318bff6c38bbc616ff849d0a4be0a69cd3","batch_range":{"start":2,"end":2}},"id":1}
```

This response indicates that the most recent slot processed was number `22019`, its hash, and that it contained no
batches (since the `start` and `end`
of the `batch_range` overlap). It also indicates that the next available batch to occur will be numbered `2`.

#### `ledger_getSlots`

This method retrieves slot data. It takes two arguments, a list of `SlotIdentifier`s and an optional `QueryMode`. If no
query mode is provided,
this list of identifiers may be flattened: `"params":[[7]]` and `"params":[7]` are both acceptable,
but `"params":[7, "Compact"]` is not.

**Example Query:**

```shell
$ curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"ledger_getSlots","params":[[7], "Compact"],"id":1}' http://127.0.0.1:8899

{"jsonrpc":"2.0","result":[{"number":6,"hash":"0x6a23ea92fbe3250e081b3e4c316fe52bda53d0113f9e7f8f495afa0e24b693ff","batch_range":{"start":1,"end":2}}],"id":1}
```

This response indicates that slot number `6` contained batch `1` and gives the

#### `ledger_getBatches`

This method retrieves slot data. It takes two arguments, a list of `BatchIdentifier`s and an optional `QueryMode`. If no
query mode is provided,
this list of identifiers may be flattened: `"params":[[7]]` and `"params":[7]` are both acceptable,
but `"params":[7, "Compact"]` is not.

**Example Query:**

```shell
$ curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"ledger_getBatches","params":[["0xf784a42555ed652ed045cc8675f5bc11750f1c7fb0fbc8d6a04470a88c7e1b6c"]],"id":1}' http://127.0.0.1:8899

{"jsonrpc":"2.0","result":[{"hash":"0xf784a42555ed652ed045cc8675f5bc11750f1c7fb0fbc8d6a04470a88c7e1b6c","tx_range":{"start":1,"end":2},"txs":["0x191d87a51e4e1dd13b4d89438c6717b756bd995d7108bef21a5ac0c9b6c77101"],"custom_receipt":"Rewarded"}],"id":1}%
```

#### `ledger_getTransactions`

This method retrieves transactions. It takes two arguments, a list of `TxIdentifiers`s and an optional `QueryMode`. If
no query mode is provided,
this list of identifiers may be flattened: `"params":[[7]]` and `"params":[7]` are both acceptable,
but `"params":[7, "Compact"]` is not.

**Example Query:**

```shell
$ curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"ledger_getTransactions","params":[[{ "batch_id": 1, "offset": 0}]],"id":1}' http://127.0.0.1:8899

{"jsonrpc":"2.0","result":[{"hash":"0x191d87a51e4e1dd13b4d89438c6717b756bd995d7108bef21a5ac0c9b6c77101","event_range":{"start":1,"end":1},"custom_receipt":"Successful"}],"id":1}
```

This response indicates that transaction `1` emitted no events but executed successfully.

#### `ledger_getEvents`

This method retrieves the events based on the provided event identifiers.

**Example Query:**

```shell
$ curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"ledger_getEvents","params":[1],"id":1}' http://127.0.0.1:8899

{"jsonrpc":"2.0","result":[null],"id":1}
```

This response indicates that event `1` has not been emitted yet.

#### `svm_queryModuleState`

This method returns the "Hello" String. It is a simple method to check if `SVM` module RPC endpoints were registered.

**Example Query:**

```shell
$ curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"svm_queryModuleState","params":[],"id":1}' http://127.0.0.1:8899

{"jsonrpc":"2.0","id":1,"result":"Hello"}  
```

This response indicates that the module was initialized and can process RPC requests.


#### `svm_getAccountInfo`

This method returns the AccountInfo of a solana account stored in `SVM` module

**Example Query:**

```shell
$  curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"svm_getAccountInfo","params":[[20, 8, 77, 60, 82, 228, 220, 135, 255, 156, 95, 155, 149, 250, 21, 139, 22, 249, 97, 213, 75, 3, 66, 28, 4, 46, 145, 211, 205, 64, 199, 106]],"id":1}' http://127.0.0.1:8899

{"jsonrpc":"2.0","id":1,"result":{"lamports":1000000000,"owner":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"data":[],"executable":false,"rentEpoch":123,"size":0}}
```

This response indicates that the requested account exists within `SVM` module state. In `result` we can check information of the account.

### SVM RPC Methods


#### `svm_sendTransaction`

This method returns the transaction hash of solana transaction sent

**Example Query:**

```shell
$ curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"svm_sendTransaction","params":[[1, 234, 15, 195, 212, 4, 66, 48, 146, 214, 201, 149, 72, 237, 108, 244, 30, 206, 190, 96, 14, 24, 79, 213, 5, 159, 83, 140, 162, 128, 237, 254, 160, 8, 242, 48, 125, 36, 108, 88, 201, 199, 87, 129, 23, 236, 200, 179, 65, 188, 215, 74, 254, 232, 5, 191, 84, 165, 152, 130, 116, 183, 82, 232, 10, 1, 0, 1, 3, 20, 8, 77, 60, 82, 228, 220, 135, 255, 156, 95, 155, 149, 250, 21, 139, 22, 249, 97, 213, 75, 3, 66, 28, 4, 46, 145, 211, 205, 64, 199, 106, 62, 173, 76, 169, 166, 68, 217, 141, 37, 153, 174, 112, 1, 21, 176, 108, 250, 100, 179, 23, 195, 180, 77, 47, 243, 245, 199, 135, 42, 66, 16, 248, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 126, 155, 158, 214, 61, 115, 243, 212, 6, 173, 193, 30, 15, 248, 121, 51, 135, 71, 147, 115, 26, 246, 245, 246, 7, 197, 118, 149, 225, 207, 71, 105, 1, 2, 2, 0, 1, 12, 2, 0, 0, 0, 32, 78, 0, 0, 0, 0, 0, 0]],"id":1}' http://127.0.0.1:8899

{"jsonrpc":"2.0","id":1,"result":[4,91,113,4,50,249,158,106,17,201,114,86,76,200,212,236,132,56,2,163,212,214,96,12,114,173,47,198,168,9,61,187]}
```

This response indicates that the transaction was sent and stored in `SvmNode` storage. It means that a solana transaction was added to a batch.

#### `svm_publishBatch`

This method returns the transaction hash of solana transaction sent

**Example Query:**

```shell
$ curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"svm_publishBatch","params":[],"id":1}' http://127.0.0.1:8899

{"jsonrpc":"2.0","id":1,"result":"Submitted transaction"}
```

This response indicates that the transaction batch was sent to DA layer.

**Example Query:**

```shell
$ curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"svm_publishBatch","params":[],"id":1}' http://127.0.0.1:8899

{"jsonrpc":"2.0","id":1,"error":{"code":-32001,"message":"SVM RPC Error: failed to publish batch","data":"ErrorObject { code: ServerError(-32001), message: \"SVM RPC: failed to submit a batch\", data: Some(RawValue(\"Custom error: Attempt to submit empty batch\")) }"}}
```

This response indicates that the transaction batch is empty at the moment and cannot be sent to DA layer. Batch has to contain at least one solana transaction.


## Testing with specific DA layers

Check [here](./README_CELESTIA.md) if you want to run with dockerized local Celestia instance.

## License

Licensed under the [Apache License, Version 2.0](../../LICENSE).

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this repository by you, as defined in the Apache-2.0 license, shall be
licensed as above, without any additional terms or conditions.
