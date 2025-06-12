# SVM Engine

The SVM Engine is designed to run outside of a blockchain and comes with support for several SPL programs out of the box.

## Building + Testing

Copy `.env.example` to `.env` and set the API_KEY that the server should use to filter incoming requests.
Use `cargo build`, `cargo run --features="rpc-server"`, and `cargo test`. Remember to set the env vars before running.

## API Endpoints

Currently, there are currently two basic endpoints to support reading account state and sending new transactions at `127.0.0.1:8080`.
Note that JSON-RPC only supports [HTTP POST](https://www.simple-is-better.org/json-rpc/transport_http.html#get-request),
so requests should specify in the body the desired method and corresponding parameters.

### getAccountInfo

```json
{
  "jsonrpc": "2.0",
  "method": "getAccountInfo",
  "params": ["Di1NNkC7mEnsKFRRsGVzKEToGsm985JbKDvmsmC6DmNC"],
  "id": 1
}
```

### sendTransaction

```json
{
  "jsonrpc": "2.0",
  "method": "sendTransaction",
  "params": [
    {
      "signatures": ...,
      "message": ...
    }
  ],
  "id": 1
}
```
