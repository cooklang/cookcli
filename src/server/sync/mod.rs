use camino::Utf8PathBuf;

pub mod endpoints;
pub mod runner;
pub mod session;

pub use runner::{start_sync, SyncHandle};
pub use session::SyncSession;

/// Resolve the sync database file path.
pub fn sync_db_path() -> String {
    crate::global_file_path("sync.db")
        .map(|p: Utf8PathBuf| p.to_string())
        .unwrap_or_else(|_| ".cook-sync.db".to_string())
}
