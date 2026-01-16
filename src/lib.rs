// Re-export modules for testing
use camino::Utf8PathBuf;

// Commands - make them available as public modules
pub mod doctor;
pub mod import;
pub mod lsp;
pub mod pantry;
pub mod recipe;
pub mod report;
pub mod search;
pub mod seed;
pub mod server;
pub mod shopping_list;
#[cfg(feature = "self-update")]
pub mod update;

// Other modules
pub mod args;
pub mod util;

// Context struct for testing - matches the one in main.rs
pub struct Context {
    base_path: Utf8PathBuf,
}

impl Context {
    pub fn new(base_path: Utf8PathBuf) -> Self {
        Self { base_path }
    }

    pub fn aisle(&self) -> Option<Utf8PathBuf> {
        let local_config = self.base_path.join("config").join("aisle.conf");
        if local_config.is_file() {
            Some(local_config)
        } else {
            None
        }
    }

    pub fn pantry(&self) -> Option<Utf8PathBuf> {
        let local_config = self.base_path.join("config").join("pantry.conf");
        if local_config.is_file() {
            Some(local_config)
        } else {
            None
        }
    }

    pub fn base_path(&self) -> &Utf8PathBuf {
        &self.base_path
    }
}
