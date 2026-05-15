mod index;
mod links;
mod renderer;
mod writer;

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

    let lang: unic_langid::LanguageIdentifier = "en-US".parse().unwrap();
    let base_url = args.base_url.as_deref();

    renderer::render_index(&source, &output, base_url, &lang)?;

    let tree = cooklang_find::build_tree(&source)
        .map_err(|e| anyhow::anyhow!("Failed to build recipe tree: {e}"))?;
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

    println!(
        "Wrote index, directories, {recipe_count} recipe pages, {image_count} images, {asset_count} static assets, {entry_count} search entries"
    );
    Ok(())
}

fn copy_all_images(source: &camino::Utf8Path, output: &camino::Utf8Path) -> Result<usize> {
    let mut count = 0;
    for path in walkdir_utf8(source)? {
        if path.is_file() {
            if let Some("jpg" | "jpeg" | "png" | "gif" | "webp" | "avif") =
                path.extension().map(|e| e.to_ascii_lowercase()).as_deref()
            {
                writer::copy_image(output, source, &path)?;
                count += 1;
            }
        }
    }
    Ok(count)
}

fn walkdir_utf8(root: &camino::Utf8Path) -> Result<Vec<camino::Utf8PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = camino::Utf8PathBuf::try_from(entry.path())
                .map_err(|e| anyhow::anyhow!("Non-UTF-8 path: {e}"))?;
            if path.is_dir() {
                if let Some(name) = path.file_name() {
                    if name.starts_with('.') {
                        continue;
                    }
                }
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    Ok(out)
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
