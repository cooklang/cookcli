use std::sync::OnceLock;

use anyhow::{bail, Context as _, Result};
use args::{CliArgs, Command};
use camino::{Utf8Path, Utf8PathBuf};
use cooklang::{Converter, CooklangParser, Extensions};
use util::resolve_to_absolute_path;

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

const LOCAL_CONFIG_DIR: &str = "config";
const APP_NAME: &str = "cook";
const UTF8_PATH_PANIC: &str = "cook only supports UTF-8 paths.";
const AUTO_AISLE: &str = "aisle.conf";
const AUTO_PANTRY: &str = "pantry.conf";

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
        let auto = self.base_path.join(LOCAL_CONFIG_DIR).join(AUTO_AISLE);

        tracing::trace!("checking auto aisle file: {auto}");

        auto.is_file().then_some(auto).or_else(|| {
            let global = global_file_path(AUTO_AISLE).ok()?;
            tracing::trace!("checking global auto aisle file: {global}");
            global.is_file().then_some(global)
        })
    }

    pub fn pantry(&self) -> Option<Utf8PathBuf> {
        let auto = self.base_path.join(LOCAL_CONFIG_DIR).join(AUTO_PANTRY);

        tracing::trace!("checking auto pantry file: {auto}");

        auto.is_file().then_some(auto).or_else(|| {
            let global = global_file_path(AUTO_PANTRY).ok()?;
            tracing::trace!("checking global auto pantry file: {global}");
            global.is_file().then_some(global)
        })
    }

    pub fn base_path(&self) -> &Utf8PathBuf {
        &self.base_path
    }
}

fn configure_parser() -> CooklangParser {
    CooklangParser::new(Extensions::empty(), Converter::default())
}

pub fn configure_context(args: &CliArgs) -> Result<Context> {
    let base_path = match args.command {
        Command::Server(ref server_args) => server_args
            .get_base_path()
            .unwrap_or_else(|| Utf8PathBuf::from(".")),
        Command::ShoppingList(ref shopping_list_args) => shopping_list_args
            .get_base_path()
            .unwrap_or_else(|| Utf8PathBuf::from(".")),
        _ => Utf8PathBuf::from("."),
    };

    let absolute_base_path = resolve_to_absolute_path(&base_path)?;

    if !absolute_base_path.is_dir() {
        bail!("Base path is not a directory: {}", absolute_base_path);
    }

    Ok(Context {
        base_path: absolute_base_path,
        parser: OnceLock::new(),
    })
}

pub fn configure_logging(verbosity: u8) {
    let env_filter = match verbosity {
        0 => "warn,cook=warn",   // Default: warnings and errors only
        1 => "info,cook=info",   // -v: info level
        2 => "debug,cook=debug", // -vv: debug level
        _ => "trace,cook=trace", // -vvv or more: trace level
    };

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .without_time()
        .with_target(false)
        .compact()
        .with_writer(std::io::stderr)
        .init();
}

/// Resolve a global configuration file path (e.g. `~/.config/cook/{name}`).
fn global_file_path(name: &str) -> Result<Utf8PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", APP_NAME)
        .context("Could not determine home directory path")?;
    let config = Utf8Path::from_path(dirs.config_dir()).expect(UTF8_PATH_PANIC);
    let path = config.join(name);
    Ok(path)
}
