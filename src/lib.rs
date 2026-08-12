// Re-export modules for testing
use anyhow::{Context as _, Result};
use camino::{Utf8Path, Utf8PathBuf};

// Commands - make them available as public modules
pub mod build;
pub mod doctor;
#[cfg(feature = "import")]
pub mod import;
#[cfg(feature = "sync")]
pub mod login;
#[cfg(feature = "sync")]
pub mod logout;
#[cfg(feature = "lsp")]
pub mod lsp;
pub mod pantry;
pub mod recipe;
pub mod report;
pub mod search;
pub mod seed;
#[cfg(feature = "server")]
pub mod server;
pub mod shopping_list;
#[cfg(feature = "sync")]
pub mod sync;
#[cfg(feature = "self-update")]
pub mod update;
pub mod web;

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

    /// The `cookcli-core` view of this context, for commands that delegate.
    ///
    /// Unlike `main.rs`'s copy this cannot delegate to `Context::discover`:
    /// the `aisle` and `pantry` above look only in the local `config/`
    /// directory and never at the global one, so discovery would give the
    /// integration tests that use this `Context` different configuration from
    /// the rest of the crate. That divergence is
    /// <https://github.com/cooklang/cookcli/issues/417>; until it is resolved
    /// this reproduces it deliberately.
    pub fn to_core(&self) -> cookcli_core::Context {
        let source = |path: Option<Utf8PathBuf>| match path {
            Some(path) => cookcli_core::ConfigSource::Path(path),
            None => cookcli_core::ConfigSource::None,
        };
        cookcli_core::Context::new(self.base_path.clone())
            .with_aisle(source(self.aisle()))
            .with_pantry(source(self.pantry()))
    }
}

const APP_NAME: &str = "cook";
const UTF8_PATH_PANIC: &str = "cook only supports UTF-8 paths.";

/// Resolve a global configuration file path (e.g. `~/.config/cook/{name}`).
pub fn global_file_path(name: &str) -> Result<Utf8PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", APP_NAME)
        .context("Could not determine home directory path")?;
    let config = Utf8Path::from_path(dirs.config_dir()).expect(UTF8_PATH_PANIC);
    let path = config.join(name);
    Ok(path)
}
