use anyhow::Result;
use clap::{Args, ValueEnum};
use cooklang_import::{text_to_cooklang, url_to_recipe};

use crate::Context;

#[derive(Debug, Clone, ValueEnum)]
pub enum MetadataFormat {
    /// Include metadata as YAML frontmatter (default for Cooklang output)
    Frontmatter,
    /// Output metadata as JSON
    Json,
    /// Output metadata as YAML
    Yaml,
    /// Don't include metadata
    None,
}

#[derive(Debug, Args)]
pub struct ImportArgs {
    /// URL of the recipe webpage to import
    ///
    /// The importer supports many popular recipe websites and will
    /// automatically extract ingredients, instructions, and metadata.
    /// The recipe will be converted to Cooklang format unless
    /// --skip-conversion is used.
    ///
    /// Example URLs:
    ///   https://www.allrecipes.com/recipe/...
    ///   https://www.bbcgoodfood.com/recipes/...
    ///   https://cooking.nytimes.com/recipes/...
    #[arg(value_name = "URL")]
    url: String,

    /// Output the original recipe data without converting to Cooklang
    ///
    /// By default, imported recipes are converted to Cooklang format.
    /// Use this flag to get the raw recipe data as extracted from
    /// the website (useful for debugging or custom processing).
    #[arg(short, long)]
    skip_conversion: bool,

    /// How to include metadata in the output
    ///
    /// When using --skip-conversion, metadata can be output separately
    /// in different formats. With Cooklang conversion, metadata is
    /// automatically included as frontmatter.
    #[arg(long, value_enum, default_value = "frontmatter")]
    metadata: MetadataFormat,

    /// Output only the metadata (no recipe content)
    ///
    /// Useful for extracting just the metadata from a recipe webpage
    /// for analysis or processing.
    #[arg(long)]
    metadata_only: bool,
}

/// Generate YAML frontmatter from metadata string
fn generate_frontmatter(name: &str, metadata: &str) -> String {
    let has_name = !name.is_empty();
    let has_metadata = !metadata.is_empty();

    if !has_name && !has_metadata {
        return String::new();
    }

    let mut frontmatter = String::from("---\n");
    if has_name {
        frontmatter.push_str(&format!("title: {}\n", name));
    }
    if has_metadata {
        frontmatter.push_str(metadata);
        if !metadata.ends_with('\n') {
            frontmatter.push('\n');
        }
    }
    frontmatter.push_str("---\n\n");
    frontmatter
}

pub fn run(_ctx: &Context, args: ImportArgs) -> Result<()> {
    let output = tokio::runtime::Runtime::new()?.block_on(async {
        let recipe = url_to_recipe(&args.url)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // Handle metadata-only output
        if args.metadata_only {
            return match args.metadata {
                MetadataFormat::Json => {
                    // Parse YAML metadata to JSON
                    if recipe.metadata.is_empty() {
                        Ok("{}".to_string())
                    } else {
                        let value: serde_json::Value = serde_yaml::from_str(&recipe.metadata)
                            .map_err(|e| anyhow::anyhow!("Failed to parse metadata: {}", e))?;
                        serde_json::to_string_pretty(&value)
                            .map_err(|e| anyhow::anyhow!("Failed to serialize metadata to JSON: {}", e))
                    }
                }
                MetadataFormat::Yaml => Ok(recipe.metadata.clone()),
                MetadataFormat::Frontmatter => Ok(generate_frontmatter(&recipe.name, &recipe.metadata)),
                MetadataFormat::None => Ok(String::new()),
            };
        }

        // Handle full recipe output
        if args.skip_conversion {
            let mut output = String::new();

            // Add metadata based on format
            match args.metadata {
                MetadataFormat::Frontmatter => {
                    output.push_str(&generate_frontmatter(&recipe.name, &recipe.metadata));
                }
                MetadataFormat::Json => {
                    if !recipe.metadata.is_empty() {
                        let value: serde_json::Value = serde_yaml::from_str(&recipe.metadata)
                            .map_err(|e| anyhow::anyhow!("Failed to parse metadata: {}", e))?;
                        output.push_str(&format!(
                            "[Metadata]\n{}\n\n",
                            serde_json::to_string_pretty(&value)?
                        ));
                    }
                }
                MetadataFormat::Yaml => {
                    if !recipe.metadata.is_empty() {
                        output.push_str(&format!("[Metadata]\n{}\n\n", recipe.metadata));
                    }
                }
                MetadataFormat::None => {}
            }

            // Add recipe content
            if !recipe.name.is_empty() {
                output.push_str(&format!("{}\n\n", recipe.name));
            }

            output.push_str(&recipe.text);

            Ok(output)
        } else {
            // Convert to Cooklang (includes metadata as frontmatter by default)
            text_to_cooklang(&recipe)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))
        }
    })?;

    println!("{output}");
    Ok(())
}
