use anyhow::Result;
use camino::Utf8PathBuf;
use clap::Args;

use crate::Context;

#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Search terms to find in recipes
    ///
    /// Can be one or more words to search for in recipes.
    /// The search looks through file names and the whole recipe text.
    /// Every term must match, so extra terms narrow the results; the
    /// best matches are listed first.
    ///
    /// Examples:
    ///   cook search chicken              # Find recipes with "chicken"
    ///   cook search chicken rice         # Find recipes with both "chicken" and "rice"
    ///   cook search "olive oil"          # Rank file names containing "olive oil" highest
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
    let outcome = cookcli_core::search::search(
        ctx,
        cookcli_core::search::SearchRequest {
            // The terms arrive as separate words only because a shell split
            // them; rejoining reconstructs the query the user typed.
            query: args.query.join(" "),
            base_dir: args.base_dir,
        },
    )
    .map_err(crate::util::cli_error)?;

    for hit in &outcome.value {
        println!("\"{}\"", hit.relative_path);
    }

    Ok(())
}
