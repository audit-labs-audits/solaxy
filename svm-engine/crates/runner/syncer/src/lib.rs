use svm_engine::storage::SVMStorage;

/// A trait that defines the syncer interface.
#[async_trait::async_trait]
pub trait Syncer<Storage>
where
    Storage: SVMStorage + Clone + Send + Sync + 'static,
{
    /// Setup the syncer with the given storage and URL.
    fn new(storage: Storage, url: String) -> Self;

    /// Run the syncing process.
    async fn run(&self);
}
