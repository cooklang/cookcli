use crate::util::{cli_error, split_recipe_name_and_scaling_factor};
use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use clap::Parser;
use cookcli_core::{report::RenderRequest, ConfigSource, CoreError, RecipeSource};
use std::fs;
use tracing::warn;

#[derive(Parser, Debug)]
pub struct ReportArgs {
    /// Path to the Jinja2 template file
    ///
    /// The template receives the recipe as a set of top-level variables:
    /// - metadata (title, author, tags, etc.)
    /// - ingredients (with quantities and units)
    /// - sections (the parsed steps and text)
    /// - cookware
    /// - scale (the scaling factor in effect)
    ///
    /// Example template:
    ///   {{ metadata.title }}
    ///   {% for ingredient in ingredients %}
    ///     {{ ingredient.name }}: {{ ingredient.quantity }}
    ///   {% endfor %}
    #[arg(short, long, value_hint = clap::ValueHint::FilePath)]
    template: Utf8PathBuf,

    /// Recipe to process
    ///
    /// Either a path to a .cook file or a bare recipe name, resolved
    /// against the base path like every other recipe argument.
    ///
    /// Can include an optional scaling factor using the :N syntax
    /// (e.g., "recipe.cook:2" to double the recipe). The scaling
    /// will be applied to all ingredient quantities in the template.
    #[arg(value_name = "RECIPE")]
    recipe: String,

    /// Path to the datastore directory with additional recipe data
    ///
    /// The datastore can contain nutritional information, costs,
    /// and other data that can be accessed in the template.
    #[arg(short, long, value_hint = clap::ValueHint::DirPath)]
    datastore: Option<Utf8PathBuf>,

    /// Path to the aisle configuration file
    ///
    /// Used for categorizing ingredients by store section.
    /// The template can access ingredient aisle information.
    #[arg(short, long, value_hint = clap::ValueHint::FilePath)]
    aisle: Option<Utf8PathBuf>,

    /// Path to the pantry configuration file
    ///
    /// Used for filtering out pantry items from shopping lists.
    /// Ingredients marked as pantry items can be accessed in templates.
    #[arg(short = 'p', long, value_hint = clap::ValueHint::FilePath)]
    pantry: Option<Utf8PathBuf>,

    /// Base path for resolving the recipe and its references
    ///
    /// Defaults to the current working directory. Both the RECIPE
    /// argument and any recipes it references are looked up under it.
    #[arg(short = 'b', long, value_hint = clap::ValueHint::DirPath)]
    base_path: Option<Utf8PathBuf>,
}

pub fn run(ctx: &crate::Context, args: ReportArgs) -> Result<()> {
    // Print warning about prototype feature
    warn!("⚠️  The report command is a prototype feature and will change in future versions.");

    // Split recipe name and scaling factor
    let (recipe_name, scaling_factor) =
        split_recipe_name_and_scaling_factor(&args.recipe).unwrap_or((&args.recipe, 1.0));

    // Read the template file
    let template = fs::read_to_string(&args.template)
        .with_context(|| format!("Failed to read template file: {}", args.template))?;

    // Aisle and pantry fall back to the context, which discovers them relative
    // to the working directory — not to `--base-path`, which only steers recipe
    // references inside the template.
    let mut core_ctx = ctx.clone();
    if let Some(aisle) = args.aisle {
        core_ctx = core_ctx.with_aisle(ConfigSource::Path(to_absolute(aisle)));
    }
    if let Some(pantry) = args.pantry {
        core_ctx = core_ctx.with_pantry(ConfigSource::Path(to_absolute(pantry)));
    }

    let outcome = cookcli_core::report::render(
        &core_ctx,
        RenderRequest {
            // A path *or* a bare recipe name, resolved through `cooklang-find`
            // under `base_path` — the same lookup `recipe`, `shopping-list`
            // and `doctor` use. `report` used to read the argument with a
            // plain `fs::read_to_string`, so `cook report -t t.jinja pancakes`
            // failed where `cook recipe pancakes` worked, and `--base-path`
            // had no bearing on the argument it most looked like it should
            // (#430).
            source: RecipeSource::Path(recipe_name.into()),
            template,
            scale: scaling_factor,
            datastore: args.datastore,
            base_path: args.base_path.map(to_absolute),
        },
    );

    let report = match outcome {
        Ok(outcome) => outcome.value,
        // The template engine's own report, with source location and hints,
        // reads better than anything wrapped around it — so print it as-is and
        // stop, rather than letting anyhow prefix it.
        Err(CoreError::Render { rendered, .. }) => {
            eprintln!("{rendered}");
            std::process::exit(1);
        }
        Err(other) => return Err(cli_error(other)),
    };

    // Print the report
    println!("{report}");

    Ok(())
}

/// Resolve a path against the working directory without touching the disk.
///
/// Not `util::resolve_to_absolute_path`, which canonicalises and therefore
/// fails on a path that does not exist yet; this command has always joined and
/// left it at that.
fn to_absolute(path: Utf8PathBuf) -> Utf8PathBuf {
    if path.is_absolute() {
        return path;
    }
    let cwd = std::env::current_dir()
        .ok()
        .and_then(|p| Utf8PathBuf::from_path_buf(p).ok())
        .unwrap_or_else(|| Utf8PathBuf::from("."));
    cwd.join(path)
}
