use anyhow::{bail, Context as AnyhowContext, Result};
use args::{CliArgs, Command};
use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use cooklang::Converter;
use cooklang::CooklangParser;
use cooklang::Extensions;
use cooklang_fs::FsIndex;
use once_cell::sync::OnceCell;
use std::path::Path;

// commands
mod recipe;
mod seed;
mod serve;
mod shopping_list;
mod version;

// other modules
mod args;
mod util;

const COOK_DIR: &str = ".cooklang";
const APP_NAME: &str = "cook";
const UTF8_PATH_PANIC: &str = "chef currently only supports UTF-8 paths. If this is problem for you, file an issue in the cooklang-chef github repository";
const AUTO_AISLE: &str = "aisle.conf";

pub fn main() -> Result<()> {
    let args = CliArgs::parse();

    let ctx = configure_context()?;

    match args.command {
        Command::Recipe(args) => recipe::run(&ctx, args),
        // Command::Serve(args) => serve::run(&ctx, args),
        Command::ShoppingList(args) => shopping_list::run(&ctx, args),
        Command::Seed(args) => seed::run(&ctx, args),
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

    fn aisle(&self) -> Option<Utf8PathBuf> {
        let auto = self.base_path.join(COOK_DIR).join(AUTO_AISLE);

        tracing::trace!("checking auto aisle file: {auto}");

        auto.is_file().then_some(auto).or_else(|| {
            let global = global_file_path(AUTO_AISLE).ok()?;
            tracing::trace!("checking global auto aisle file: {global}");
            global.is_file().then_some(global)
        })
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

pub fn resolve_path(base_path: &Utf8Path, path: &Path) -> Utf8PathBuf {
    let path = Utf8Path::from_path(path).expect(UTF8_PATH_PANIC);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_path.join(COOK_DIR).join(path)
    }
}

pub fn global_file_path(name: &str) -> Result<Utf8PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", APP_NAME)
        .context("Could not determine home directory path")?;
    let config = Utf8Path::from_path(dirs.config_dir()).expect(UTF8_PATH_PANIC);
    let path = config.join(name);
    Ok(path)
}
