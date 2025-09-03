use anyhow::Result;
use camino::Utf8PathBuf;
use clap::Args;
use cooklang_find::search;

use crate::Context;

#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Search terms to find in recipes
    ///
    /// Can be one or more words to search for in recipes.
    /// The search looks through recipe titles, ingredients, instructions,
    /// and metadata. Multiple terms are treated as AND (all must match).
    ///
    /// Examples:
    ///   cook search chicken              # Find recipes with "chicken"
    ///   cook search chicken rice         # Find recipes with both "chicken" and "rice"
    ///   cook search "olive oil"          # Search for exact phrase
    #[arg(required = true, num_args = 1.., value_name = "TERMS")]
    query: Vec<String>,

    /// Directory to search for recipes
    ///
    /// Specifies the root directory to search. The search will recursively
    /// scan for .cook files in this directory and all subdirectories.
    /// Defaults to the current directory.
    #[arg(short, long, value_hint = clap::ValueHint::DirPath)]
    base_dir: Option<Utf8PathBuf>,
}

pub fn run(ctx: &Context, args: SearchArgs) -> Result<()> {
    let base_dir = args.base_dir.unwrap_or_else(|| ctx.base_path().clone());

    // Join multiple search terms with spaces
    let query = args.query.join(" ");
    let recipes = search(&base_dir, &query)?;

    for recipe in recipes {
        if let Some(path) = recipe.path() {
            let relative_path = path.strip_prefix(&base_dir).unwrap_or(path);
            println!("\"{relative_path}\"");
        }
    }

    Ok(())
}
