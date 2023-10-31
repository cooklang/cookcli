use anyhow::{bail, Result};
use args::{CliArgs, Command};
use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use cooklang::Converter;
use cooklang::CooklangParser;
use cooklang::Extensions;
use cooklang_fs::FsIndex;
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

pub fn main() -> Result<()> {
    let args = CliArgs::parse();

    let ctx = configure_context()?;

    match args.command {
        Command::Recipe(args) => recipe::run(&ctx, args),
        // Command::Serve(args) => serve::run(&ctx, args),
        // Command::ShoppingList(args) => shopping_list::run(&ctx, args),
        Command::Version(args) => version::run(&ctx, args),
    }
}

pub struct Context {
    parser: OnceCell<CooklangParser>,
    base_path: Utf8PathBuf,
    recipe_index: FsIndex,
}

impl Context {
    fn parser(&self) -> Result<&CooklangParser> {
        self.parser
            .get_or_try_init(|| configure_parser(&self.base_path))
    }
}

fn configure_context() -> Result<Context> {
    let base_path = Utf8Path::new(".").to_path_buf();

    if !base_path.is_dir() {
        bail!("Base path is not a directory: {base_path}");
    }

    let mut index = FsIndex::new(&base_path, 5)?;
    index.set_config_dir(COOK_DIR.to_string());

    Ok(Context {
        parser: OnceCell::new(),
        base_path,
        recipe_index: index,
    })
}

fn configure_parser(_base_path: &Utf8Path) -> Result<CooklangParser> {
    let extensions = Extensions::empty();
    let converter = Converter::empty();

    Ok(CooklangParser::new(extensions, converter))
}
