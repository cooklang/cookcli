use crate::util::resolve_to_absolute_path;
use crate::Context;
use anyhow::{bail, Context as _, Result};
use camino::Utf8PathBuf;
use clap::Args;

#[derive(Debug, Args)]
pub struct BuildArgs {
    /// Output directory for the generated static site
    ///
    /// Defaults to ./_site if not specified. The directory is created if
    /// missing. Existing files in the directory are overwritten as needed
    /// but not wiped wholesale.
    #[arg(value_hint = clap::ValueHint::DirPath)]
    pub output_dir: Option<Utf8PathBuf>,

    /// Root directory containing your recipe files
    #[arg(long, value_hint = clap::ValueHint::DirPath)]
    pub base_path: Option<Utf8PathBuf>,

    /// Absolute URL prefix for hosting under a subpath (e.g. /recipes/)
    ///
    /// When set, internal links use this absolute prefix instead of
    /// page-relative paths. Useful when you know the deployed subpath.
    #[arg(long)]
    pub base_url: Option<String>,
}

impl BuildArgs {
    pub fn get_base_path(&self) -> Option<Utf8PathBuf> {
        self.base_path.clone()
    }
}

pub fn run(ctx: &Context, args: BuildArgs) -> Result<()> {
    let source = resolve_to_absolute_path(ctx.base_path())?;
    if !source.is_dir() {
        bail!("Source base path is not a directory: {source}");
    }

    let output_raw = args
        .output_dir
        .clone()
        .unwrap_or_else(|| Utf8PathBuf::from("_site"));

    // Create the output directory before canonicalizing (canonicalize requires existence).
    std::fs::create_dir_all(&output_raw)
        .with_context(|| format!("Failed to create output directory: {output_raw}"))?;

    let output = resolve_to_absolute_path(&output_raw)?;

    tracing::info!("Building static site from {source} into {output}");
    println!("Building static site from {source} into {output}");
    Ok(())
}
