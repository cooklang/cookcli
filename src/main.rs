// This file includes a substantial portion of code from
// https://github.com/Zheoni/cooklang-chef
//
// The original code is licensed under the MIT License, a copy of which
// is provided below in addition to our project's license.
//
//

// MIT License

// Copyright (c) 2023 Francisco J. Sanchez

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use anyhow::{bail, Context as AnyhowContext, Result};
use args::{CliArgs, Command};
use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use cooklang::Converter;
use cooklang::CooklangParser;
use cooklang::Extensions;
use cooklang_fs::LazyFsIndex;
use once_cell::sync::OnceCell;
use std::path::Path;

// commands
mod recipe;
mod seed;
mod server;
mod shopping_list;

// other modules
mod args;
mod util;

const GLOBAL_CONFIG_DIR: &str = ".cooklang";
const LOCAL_CONFIG_DIR: &str = "config";
const APP_NAME: &str = "cook";
const UTF8_PATH_PANIC: &str = "cook only supports UTF-8 paths.";
const AUTO_AISLE: &str = "aisle.conf";

pub fn main() -> Result<()> {
    configure_logging();

    let args = CliArgs::parse();

    let ctx = configure_context()?;

    match args.command {
        Command::Recipe(args) => recipe::run(&ctx, args),
        Command::Server(args) => server::run(ctx, args),
        Command::ShoppingList(args) => shopping_list::run(&ctx, args),
        Command::Seed(args) => seed::run(&ctx, args),
    }
}

pub struct Context {
    parser: OnceCell<CooklangParser>,
    base_path: Utf8PathBuf,
    recipe_index: LazyFsIndex,
}

impl Context {
    fn parser(&self) -> Result<&CooklangParser> {
        self.parser.get_or_try_init(configure_parser)
    }

    fn aisle(&self) -> Option<Utf8PathBuf> {
        let auto = self.base_path.join(LOCAL_CONFIG_DIR).join(AUTO_AISLE);

        tracing::trace!("checking auto aisle file: {auto}");

        auto.is_file().then_some(auto).or_else(|| {
            let global = global_file_path(AUTO_AISLE).ok()?;
            tracing::trace!("checking global auto aisle file: {global}");
            global.is_file().then_some(global)
        })
    }
}

fn configure_context() -> Result<Context> {
    let args = CliArgs::parse();
    let base_path = match args.command {
        Command::Server(ref server_args) => server_args
            .get_base_path()
            .unwrap_or_else(|| Utf8PathBuf::from(".")),
        _ => Utf8PathBuf::from("."),
    };

    if !base_path.is_dir() {
        bail!("Base path is not a directory: {base_path}");
    }

    let index = cooklang_fs::new_index(&base_path, 5)?
        .config_dir(LOCAL_CONFIG_DIR.to_string())
        .lazy();

    Ok(Context {
        parser: OnceCell::new(),
        base_path,
        recipe_index: index,
    })
}

fn configure_parser() -> Result<CooklangParser> {
    let extensions = Extensions::empty();
    let converter = Converter::empty();

    Ok(CooklangParser::new(extensions, converter))
}

fn configure_logging() {
    tracing_subscriber::fmt()
        // Log this crate at level `trace`, but all other crates at level `info`.
        .with_env_filter("info,cooklang=info,cook=trace")
        .without_time()
        .with_target(false)
        .compact()
        .init();
}

pub fn resolve_path(base_path: &Utf8Path, path: &Path) -> Utf8PathBuf {
    let path = Utf8Path::from_path(path).expect(UTF8_PATH_PANIC);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_path.join(LOCAL_CONFIG_DIR).join(path)
    }
}

pub fn global_file_path(name: &str) -> Result<Utf8PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", APP_NAME)
        .context("Could not determine home directory path")?;
    let config = Utf8Path::from_path(dirs.config_dir()).expect(UTF8_PATH_PANIC);
    let path = config.join(name);
    Ok(path)
}
