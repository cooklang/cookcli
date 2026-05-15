use crate::Context;
use anyhow::Result;
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

pub fn run(_ctx: &Context, _args: BuildArgs) -> Result<()> {
    println!("cook build: not yet implemented");
    Ok(())
}
