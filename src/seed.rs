use std::fs;

use anyhow::{bail, Context as _, Result};
use camino::Utf8PathBuf;
use clap::Args;

use rust_embed::RustEmbed;

use crate::Context;

// TODO check if it's compressed
#[derive(RustEmbed)]
#[folder = "./seed/"]
struct SeedFiles;

#[derive(Debug, Args)]
#[command()]
pub struct SeedArgs {
    /// Directory where example recipes will be created
    ///
    /// The seed command will create a collection of example Cooklang
    /// recipes in this directory. These examples demonstrate various
    /// Cooklang features including ingredients, cookware, timers, and
    /// metadata. If not specified, uses the current directory.
    ///
    /// The directory will be created if it doesn't exist.
    #[arg(value_name = "DIR", value_hint = clap::ValueHint::DirPath)]
    output: Option<Utf8PathBuf>,
}

pub fn run(ctx: &Context, args: SeedArgs) -> Result<()> {
    let path = args.output.as_deref().unwrap_or(ctx.base_path());

    if path.is_file() {
        bail!("{} is not a directory", path);
    }

    fs::create_dir_all(path).with_context(|| format!("Couldn't create output path: {path}"))?;

    for seed in SeedFiles::iter() {
        let content = SeedFiles::get(seed.as_ref())
            .with_context(|| format!("Embedded seed file not found: {seed}"))?;
        let file = path.join(seed.as_ref());
        let parent = file.parent().expect("Invalid path");

        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {parent}"))?;
        fs::write(&file, content.data.as_ref())
            .with_context(|| format!("Failed to write seed file: {file}"))?;
    }

    Ok(())
}
