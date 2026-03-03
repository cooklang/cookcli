use camino::Utf8PathBuf;

pub mod endpoints;
pub mod runner;
pub mod session;

pub use runner::{start_sync, SyncHandle};
pub use session::SyncSession;

/// Resolve the sync database file path.
/// Returns an error if the global config directory cannot be determined.
pub fn sync_db_path() -> anyhow::Result<String> {
    crate::global_file_path("sync.db")
        .map(|p: Utf8PathBuf| p.to_string())
}
