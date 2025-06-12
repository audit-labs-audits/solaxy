

```bash
# Sent to node 2, should fail
target/debug/sov-cli submit-transaction crates/test-data/keys/token_deployer_private_key.json Bank crates/test-data/requests/create_token.json 0 http://127.0.0.1:3000
target/debug/sov-cli publish-batch http://127.0.0.1:3000

# Registering second sequencer
target/debug/sov-cli submit-transaction crates/test-data/keys/token_deployer_private_key.json SequencerRegistry crates/test-data/requests/register_sequencer.json 0 http://127.0.0.1:8899
target/debug/sov-cli publish-batch http://127.0.0.1:8899

# Try on second sequencer again
target/debug/sov-cli submit-transaction crates/test-data/keys/token_deployer_private_key.json Bank crates/test-data/requests/transfer.json 1 http://127.0.0.1:3000
target/debug/sov-cli publish-batch http://127.0.0.1:3000
```
