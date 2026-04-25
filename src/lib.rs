use std::sync::OnceLock;

// Re-export modules for testing
use anyhow::{Context as _, Result};
use camino::{Utf8Path, Utf8PathBuf};
use cooklang::{Converter, CooklangParser, Extensions};

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
    parser: OnceLock<CooklangParser>,
}

impl Context {
    pub fn new(base_path: Utf8PathBuf) -> Self {
        Self {
            base_path,
            parser: OnceLock::new(),
        }
    }

    pub fn parser(&self) -> &CooklangParser {
        self.parser.get_or_init(configure_parser)
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

fn configure_parser() -> CooklangParser {
    CooklangParser::new(Extensions::empty(), Converter::default())
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
