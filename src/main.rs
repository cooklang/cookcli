use anyhow::{bail, Result};
use args::{CliArgs, Command};
use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use cooklang::{CooklangParser};
use once_cell::sync::OnceCell;

// commands
mod recipe;
mod serve;
mod shopping_list;
mod version;

// other modules
mod args;
mod util;

const COOK_DIR: &str = ".cooklang";
const APP_NAME: &str = "cooklang-chef";
const UTF8_PATH_PANIC: &str = "chef currently only supports UTF-8 paths. If this is problem for you, file an issue in the cooklang-chef github repository";

pub fn main() -> Result<()> {
    let args = CliArgs::parse();

    let ctx = configure_context()?;

    match args.command {
        // Command::Recipe(args) => recipe::run(&ctx, args),
        // Command::Serve(args) => serve::run(&ctx, args),
        // Command::ShoppingList(args) => shopping_list::run(&ctx, args),
        Command::Version(args) => version::run(&ctx, args),
    }
}

pub struct Context {
    parser: OnceCell<CooklangParser>,
    base_path: Utf8PathBuf,
}

fn configure_context() -> Result<Context> {
    let base_path = Utf8Path::new(".").to_path_buf();

    if !base_path.is_dir() {
        bail!("Base path is not a directory: {base_path}");
    }

    Ok(Context {
        parser: OnceCell::new(),
        base_path,
    })
}
