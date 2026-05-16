use camino::Utf8PathBuf;

pub mod device_flow;
pub mod endpoints;
pub mod runner;
pub mod session;

pub use runner::{start_sync, SyncHandle};
pub use session::SyncSession;

use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct PendingDeviceFlow {
    #[allow(dead_code)]
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_at: std::time::Instant,
    #[allow(dead_code)]
    pub interval: std::time::Duration,
    pub cancel: CancellationToken,
}

/// Resolve the sync database file path.
/// Returns an error if the global config directory cannot be determined.
pub fn sync_db_path() -> anyhow::Result<String> {
    crate::global_file_path("sync.db").map(|p: Utf8PathBuf| p.to_string())
}
