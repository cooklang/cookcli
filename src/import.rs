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
        // The title is a scraped string, not trusted YAML: a name containing
        // a newline or a `key: ` break used to be written out raw, letting a
        // hostile page inject its own frontmatter keys. Serializing the
        // `title` entry through serde_yaml quotes, escapes or block-scalars
        // the value however it needs to stay one string.
        let mut entry = serde_yaml::Mapping::new();
        entry.insert("title".into(), name.into());
        let line =
            serde_yaml::to_string(&entry).expect("a string-keyed string map always serializes");
        frontmatter.push_str(&line);
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
                        serde_json::to_string_pretty(&value).map_err(|e| {
                            anyhow::anyhow!("Failed to serialize metadata to JSON: {}", e)
                        })
                    }
                }
                MetadataFormat::Yaml => Ok(recipe.metadata.clone()),
                MetadataFormat::Frontmatter => {
                    Ok(generate_frontmatter(&recipe.name, &recipe.metadata))
                }
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

#[cfg(test)]
mod tests {
    use super::generate_frontmatter;

    /// The YAML inside the `---` fences.
    fn inner_yaml(frontmatter: &str) -> serde_yaml::Value {
        let body = frontmatter
            .strip_prefix("---\n")
            .and_then(|rest| rest.split("\n---").next())
            .expect("fenced frontmatter");
        serde_yaml::from_str(body).expect("emitted frontmatter is valid YAML")
    }

    /// The frontmatter must parse back and `title` must still be exactly the
    /// scraped name — whatever YAML-significant characters it carries.
    fn title_of(frontmatter: &str) -> String {
        inner_yaml(frontmatter)["title"]
            .as_str()
            .expect("a title")
            .to_string()
    }

    #[test]
    fn a_plain_title_is_emitted_unquoted() {
        let fm = generate_frontmatter("Pancakes", "");
        assert_eq!(title_of(&fm), "Pancakes");
    }

    #[test]
    fn a_title_with_a_newline_stays_one_value() {
        let hostile = "Pancakes\nservings: 99\nevil: ";
        let fm = generate_frontmatter(hostile, "");
        let yaml = inner_yaml(&fm);
        assert_eq!(yaml["title"].as_str().unwrap(), hostile);
        assert!(
            yaml.get("servings").is_none() && yaml.get("evil").is_none(),
            "the title must not inject frontmatter keys: {fm}"
        );
    }

    #[test]
    fn a_title_looking_like_yaml_is_quoted() {
        for hostile in ["has: colon", "true", "#hash", "123", "- item"] {
            let fm = generate_frontmatter(hostile, "");
            assert_eq!(title_of(&fm), hostile, "round-trips: {fm}");
        }
    }
}
