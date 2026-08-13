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

    /// Recipe file to process
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

    /// Base path for resolving recipe references
    ///
    /// Defaults to the current working directory. This is used when
    /// recipes reference other recipes using relative paths.
    #[arg(short = 'b', long, value_hint = clap::ValueHint::DirPath)]
    base_path: Option<Utf8PathBuf>,
}

pub fn run(ctx: &crate::Context, args: ReportArgs) -> Result<()> {
    // Print warning about prototype feature
    warn!("⚠️  The report command is a prototype feature and will change in future versions.");

    // Split recipe name and scaling factor
    let (recipe_name, scaling_factor) =
        split_recipe_name_and_scaling_factor(&args.recipe).unwrap_or((&args.recipe, 1.0));

    // Read the recipe file.
    //
    // Deliberately a plain read rather than the `cooklang-find` lookup every
    // other command uses, because that is what this command has always done:
    // `cook report -t x.jinja pancakes` does not resolve a bare recipe name the
    // way `cook recipe pancakes` does, and the path is interpreted against the
    // working directory rather than `--base-path`. Core handles a
    // `RecipeSource::Path` the usual way, so switching this to one is all it
    // would take — but that is a user-visible change and not this refactor's to
    // make.
    let recipe = fs::read_to_string(recipe_name)
        .with_context(|| format!("Failed to read recipe file: {recipe_name}"))?;

    // Read the template file
    let template = fs::read_to_string(&args.template)
        .with_context(|| format!("Failed to read template file: {}", args.template))?;

    // Aisle and pantry fall back to the context, which discovers them relative
    // to the working directory — not to `--base-path`, which only steers recipe
    // references inside the template.
    let mut core_ctx = ctx.to_core();
    if let Some(aisle) = args.aisle {
        core_ctx = core_ctx.with_aisle(ConfigSource::Path(to_absolute(aisle)));
    }
    if let Some(pantry) = args.pantry {
        core_ctx = core_ctx.with_pantry(ConfigSource::Path(to_absolute(pantry)));
    }

    let outcome = cookcli_core::report::render(
        &core_ctx,
        RenderRequest {
            source: RecipeSource::Content {
                text: recipe,
                name: recipe_name.to_string(),
            },
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
