mod index;
mod links;
mod renderer;
mod sitemap;
mod writer;

use crate::server::language::{parse_supported_language, EN_US};
use crate::util::resolve_to_absolute_path;
use crate::Context;
use anyhow::{bail, Context as _, Result};
use camino::Utf8PathBuf;
use clap::{Args, Subcommand};
use unic_langid::LanguageIdentifier;

#[derive(Debug, Args)]
pub struct BuildArgs {
    #[command(subcommand)]
    pub command: BuildCommand,
}

#[derive(Debug, Subcommand)]
pub enum BuildCommand {
    /// Generate a self-contained static website from your recipes
    ///
    /// Renders your recipes as static HTML files browsable on any static-file
    /// host or directly from disk via file://. Excludes dynamic features
    /// (shopping list, pantry, editing).
    ///
    /// Examples:
    ///   cook build web                         # Build to ./_site
    ///   cook build web out                     # Build to ./out
    ///   cook build web --base-path ~/recipes   # Use specific source directory
    ///   cook build web --base-url /recipes/    # Absolute URL prefix for subpath hosting
    Web(WebBuildArgs),
}

#[derive(Debug, Args)]
pub struct WebBuildArgs {
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

    /// UI language for the generated site (default: en-US)
    ///
    /// Accepts a BCP-47 tag like `de-DE`, or a bare language code like `de`
    /// that matches a supported region. Supported: en-US, de-DE, nl-NL,
    /// fr-FR, es-ES, eu-ES, sv-SE.
    #[arg(long, value_parser = parse_lang_arg)]
    pub lang: Option<LanguageIdentifier>,

    /// Full base URL of the deployed site to generate a sitemap.xml
    ///
    /// When set (e.g. https://recipes.example.com or
    /// https://example.com/recipes), writes a sitemaps.org-compliant
    /// sitemap.xml at the output root listing every page with absolute URLs.
    /// This is the complete URL prefix (scheme + host + optional subpath) and
    /// is independent of --base-url. Omit it to skip sitemap generation.
    #[arg(long)]
    pub sitemap: Option<String>,
}

fn parse_lang_arg(s: &str) -> Result<LanguageIdentifier, String> {
    parse_supported_language(s).ok_or_else(|| {
        format!(
            "unsupported language '{s}'. Supported: en-US, de-DE, nl-NL, fr-FR, es-ES, eu-ES, sv-SE"
        )
    })
}

impl BuildArgs {
    pub fn get_base_path(&self) -> Option<Utf8PathBuf> {
        match &self.command {
            BuildCommand::Web(args) => args.base_path.clone(),
        }
    }
}

pub fn run(ctx: &Context, args: BuildArgs) -> Result<()> {
    match args.command {
        BuildCommand::Web(web_args) => run_web(ctx, web_args),
    }
}

fn run_web(ctx: &Context, args: WebBuildArgs) -> Result<()> {
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

    println!("Building static site from {source} into {output}");

    let lang = args.lang.clone().unwrap_or(EN_US);
    let base_url = args.base_url.as_deref();

    renderer::render_index(&source, &output, base_url, &lang)?;

    let mut tree = cooklang_find::build_tree(&source)
        .map_err(|e| anyhow::anyhow!("Failed to build recipe tree: {e}"))?;
    // If the user pointed the output directory inside the source directory
    // (the common case: `cook build web` with default `_site` next to recipes),
    // strip the output subtree so we don't re-process the previous run's
    // generated files. Without this, every run would nest `_site/recipe/...`
    // and `_site/api/static/...` one level deeper until the OS rejects the
    // path length.
    prune_output_subtree(&mut tree, &output);
    walk_directories(&tree, &source, &output, base_url, &lang, String::new())?;

    let aisle = ctx.aisle();
    let recipe_count = walk_recipes(
        &tree,
        &source,
        &output,
        aisle.as_ref(),
        base_url,
        &lang,
        String::new(),
    )?;

    let image_count = copy_all_images(&source, &output)?;
    let asset_count = writer::copy_static_assets(&output)?;

    let entries = index::build_search_index(&tree);
    let entry_count = entries.len();
    let json = serde_json::to_string(&entries)?;
    writer::write_bytes(
        &output,
        camino::Utf8Path::new("static/search-index.json"),
        json.as_bytes(),
    )?;

    let sitemap_written = if let Some(base) = args.sitemap.as_deref() {
        let parsed = url::Url::parse(base)
            .with_context(|| format!("Invalid --sitemap URL: {base}"))?;
        if parsed.host().is_none() || !matches!(parsed.scheme(), "http" | "https") {
            bail!("--sitemap must be an absolute http(s) URL with a host, e.g. https://recipes.example.com");
        }
        sitemap::write_sitemap(&output, base, &tree)?;
        true
    } else {
        false
    };

    let sitemap_note = if sitemap_written { ", sitemap.xml" } else { "" };
    println!(
        "Wrote index, directories, {recipe_count} recipe pages, {image_count} images, {asset_count} static assets, {entry_count} search entries{sitemap_note}"
    );
    Ok(())
}

fn prune_output_subtree(tree: &mut cooklang_find::RecipeTree, output: &camino::Utf8Path) {
    tree.children
        .retain(|_, child| !child.path.starts_with(output));
    for child in tree.children.values_mut() {
        prune_output_subtree(child, output);
    }
}

fn copy_all_images(source: &camino::Utf8Path, output: &camino::Utf8Path) -> Result<usize> {
    let mut count = 0;
    // `walkdir` doesn't follow symlinks by default, which prevents infinite
    // loops on symlink cycles. We also filter out dotted directories so that
    // hidden caches (.git, .cache, etc.) are skipped. Skip the output
    // subtree too — when the user builds into a directory inside the source
    // (default: `_site` next to recipes), we'd otherwise re-copy the
    // previous run's images into a deeper path each time.
    let output_std = output.as_std_path();
    let walker = walkdir::WalkDir::new(source).into_iter().filter_entry(|e| {
        if e.path().starts_with(output_std) {
            return false;
        }
        !e.file_type().is_dir()
            || e.file_name()
                .to_str()
                .map(|n| !n.starts_with('.'))
                .unwrap_or(true)
    });
    for entry in walker {
        let entry = entry.with_context(|| format!("Failed to walk {source}"))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = camino::Utf8PathBuf::try_from(entry.into_path())
            .map_err(|e| anyhow::anyhow!("Non-UTF-8 path: {e}"))?;
        if let Some("jpg" | "jpeg" | "png" | "gif" | "webp" | "avif") =
            path.extension().map(|e| e.to_ascii_lowercase()).as_deref()
        {
            writer::copy_image(output, source, &path)?;
            count += 1;
        }
    }
    Ok(count)
}

fn walk_directories(
    tree: &cooklang_find::RecipeTree,
    source: &camino::Utf8Path,
    output: &camino::Utf8Path,
    base_url: Option<&str>,
    lang: &unic_langid::LanguageIdentifier,
    prefix_path: String,
) -> Result<()> {
    for (name, child) in &tree.children {
        if child.children.is_empty() {
            continue; // it's a recipe file, handled by walk_recipes
        }
        let sub = if prefix_path.is_empty() {
            name.to_string()
        } else {
            format!("{prefix_path}/{name}")
        };
        renderer::render_directory(source, output, &sub, base_url, lang)?;
        walk_directories(child, source, output, base_url, lang, sub)?;
    }
    Ok(())
}

fn walk_recipes(
    tree: &cooklang_find::RecipeTree,
    source: &camino::Utf8Path,
    output: &camino::Utf8Path,
    aisle_path: Option<&camino::Utf8PathBuf>,
    base_url: Option<&str>,
    lang: &unic_langid::LanguageIdentifier,
    prefix_path: String,
) -> Result<usize> {
    let mut count = 0;
    for (name, child) in &tree.children {
        if child.children.is_empty() {
            // Recipe file — use the on-disk file name, not the tree key (which may be the title).
            let leaf_name = child
                .recipe
                .as_ref()
                .and_then(|r| r.file_name())
                .unwrap_or_else(|| name.to_string());
            let sub = if prefix_path.is_empty() {
                leaf_name
            } else {
                format!("{prefix_path}/{leaf_name}")
            };
            if let Err(e) =
                renderer::render_recipe(source, output, &sub, aisle_path, base_url, lang)
            {
                tracing::warn!("Skipping recipe {sub}: {e:#}");
                continue;
            }
            if sub.ends_with(".cook") {
                if let Err(e) = writer::copy_recipe_source(output, source, &sub) {
                    tracing::warn!("Skipping source copy for {sub}: {e:#}");
                }
            }
            count += 1;
        } else {
            let sub = if prefix_path.is_empty() {
                name.to_string()
            } else {
                format!("{prefix_path}/{name}")
            };
            count += walk_recipes(child, source, output, aisle_path, base_url, lang, sub)?;
        }
    }
    Ok(count)
}
