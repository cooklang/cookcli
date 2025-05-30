use anyhow::Result;
use camino::Utf8PathBuf;
use clap::Args;
use cooklang_find::search;

use crate::Context;

#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Search query
    #[arg(required = true)]
    query: String,

    /// Base directory to search in
    #[arg(short, long)]
    base_dir: Option<Utf8PathBuf>,
}

pub fn run(ctx: &Context, args: SearchArgs) -> Result<()> {
    let base_dir = args.base_dir.unwrap_or_else(|| ctx.base_path.clone());

    let recipes = search(&base_dir, &args.query)?;

    for recipe in recipes {
        if let Some(path) = recipe.path() {
            let relative_path = path.strip_prefix(&base_dir).unwrap_or(path);
            println!("{}", relative_path);
        }
    }

    Ok(())
}
