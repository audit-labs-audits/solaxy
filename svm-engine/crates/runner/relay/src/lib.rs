use std::net::SocketAddr;

#[async_trait::async_trait]
pub trait Relay {
    /// Run the relay process.
    async fn run(&self, tx_queue_addr: SocketAddr, rpc_url: String);
}
