#!/usr/bin/env bash
set -ex

RUST_LOG=warn,indexer=info solana-test-validator \
 --geyser-plugin-config /opt/geyser-plugin-config.json \
 --ledger /var/test-ledger \
 --limit-ledger-size 1000000 \
 --bpf-program /opt/programs/blober-keypair.json /opt/programs/blober.so \
 --account - /opt/programs/blober-account-batch.json \
 --account - /opt/programs/blober-account-proof.json \
 --account "$PAYER_PUBKEY" /opt/programs/payer-account.json \
 "$@"
