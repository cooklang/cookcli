//! Reading and scaling a single recipe.

use crate::{parse_recipe, parse_recipe_at, Context, CoreError, Outcome, RecipeSource};
use camino::Utf8PathBuf;
use cooklang::Recipe;

/// The character separating a recipe name from an inline scaling factor.
const SCALING_DELIMITER: char = ':';

/// What a path-backed recipe with no resolvable path is called in messages.
///
/// Only reachable in principle: [`cooklang_find::get_recipe`] always returns a
/// path-backed entry. Kept so that the rendering never says "recipe ''".
const UNKNOWN_PATH: &str = "unknown";

/// What to read, and at what scale.
#[derive(Debug, Clone)]
pub struct ReadRequest {
    /// The recipe to read.
    pub source: RecipeSource,
    /// Scaling factor applied to all quantities. An inline `name:factor`
    /// suffix on a [`RecipeSource::Path`] takes precedence over this.
    pub scale: f64,
}

/// A parsed recipe together with the title to display for it.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ReadResult {
    /// The parsed, scaled recipe.
    pub recipe: Recipe,
    /// The title to display for the recipe: its metadata `title` when it
    /// declares one, otherwise the file stem for a path or the caller-supplied
    /// name for in-memory text. Empty when none of those are known.
    ///
    /// Formatters put this in markdown headings and in the `name` of
    /// schema.org output, so it is the recipe's identity rather than a debug
    /// label — see [`CoreError::Parse`]'s `name` for the latter.
    pub title: String,
    /// The factor the recipe was actually scaled by, which is
    /// [`ReadRequest::scale`] unless an inline `name:factor` suffix overrode
    /// it. Formatters need it to label the output, and it is the only place
    /// the resolved value is visible.
    pub scale: f64,
}

/// Split a `name:factor` query into its parts.
///
/// `"pasta.cook:2"` becomes `("pasta.cook", 2.0)`. Returns `None` when there is
/// no colon, or when what follows the last one is not a number — so a Windows
/// path like `C:\recipes\pasta.cook` is left alone.
pub fn split_name_and_scale(query: &str) -> Option<(&str, f64)> {
    let (name, factor) = query.trim().rsplit_once(SCALING_DELIMITER)?;
    let factor = factor.parse::<f64>().ok()?;
    Some((name, factor))
}

/// Read a recipe, scale it, and report anything the parser had to say.
///
/// For a [`RecipeSource::Path`], the name is resolved against
/// [`Context::base_path`] and may carry an inline `:factor` suffix, which
/// overrides [`ReadRequest::scale`]. A [`RecipeSource::Content`] is parsed as
/// given and never touches the filesystem; its `name` is only a fallback for
/// [`ReadResult::title`], used when the recipe declares no title of its own.
///
/// # Errors
///
/// - [`CoreError::RecipeNotFound`] if no file matches the given path or name.
/// - [`CoreError::Io`] if the file is found but cannot be read.
/// - [`CoreError::Parse`] if the recipe has parse errors. Its `name` is the
///   recipe's path, matching the file the `rendered` report points at.
/// - [`CoreError::InvalidScale`] if the effective scale is not finite.
pub fn read(ctx: &Context, req: ReadRequest) -> Result<Outcome<ReadResult>, CoreError> {
    match req.source {
        RecipeSource::Content { text, name } => {
            let outcome = parse_recipe(&text, &name, req.scale)?;
            // The recipe's own `title` wins over the caller's name, which is
            // only a label for a buffer that may not have one. Skipping this
            // would put "stdin" into `-f markdown` headings and the `name` of
            // schema.org output.
            let title = outcome
                .value
                .metadata
                .title()
                .map_or_else(|| name, ToOwned::to_owned);
            Ok(Outcome::with_diagnostics(
                ReadResult {
                    recipe: outcome.value,
                    title,
                    scale: req.scale,
                },
                outcome.diagnostics,
            ))
        }
        RecipeSource::Path(query) => {
            // An inline factor is the more specific instruction, so it wins.
            let (name, scale) =
                split_name_and_scale(query.as_str()).unwrap_or((query.as_str(), req.scale));

            let entry = cooklang_find::get_recipe(
                vec![ctx.base_path().to_path_buf()],
                Utf8PathBuf::from(name),
            )
            .map_err(|_| CoreError::RecipeNotFound {
                name: name.to_string(),
            })?;

            let path = entry.path().cloned();
            let content = entry.content().map_err(|source| CoreError::Io {
                path: path.clone().unwrap_or_else(|| UNKNOWN_PATH.into()),
                source: match source {
                    cooklang_find::RecipeEntryError::IoError(e) => e,
                    other => std::io::Error::other(other.to_string()),
                },
            })?;

            // Diagnostics and the parse report name the file, not the title, so
            // that the caller can open what they point at.
            let display_path = path
                .as_ref()
                .map_or_else(|| UNKNOWN_PATH.to_string(), |p| p.to_string());
            let outcome = parse_recipe_at(&content, &display_path, scale, path.as_deref())?;

            Ok(Outcome::with_diagnostics(
                ReadResult {
                    recipe: outcome.value,
                    title: entry.name().clone().unwrap_or_default(),
                    scale,
                },
                outcome.diagnostics,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Context, RecipeSource};
    use cooklang::quantity::Value;

    fn fixture_dir() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("simple.cook"),
            "Boil @water{2%cups} for ~{5%minutes}.\nAdd @salt{1%tsp}.\n",
        )
        .unwrap();
        dir
    }

    fn ctx_for(dir: &tempfile::TempDir) -> Context {
        Context::new(camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap())
    }

    /// The numeric value of an ingredient's quantity, ignoring its unit.
    ///
    /// Comparing numbers rather than formatted quantities matters: cooklang
    /// re-fits units when scaling, so `2 cups` can render as `4 c` and a string
    /// comparison would be testing the formatter, not the scaling.
    fn quantity_value(recipe: &Recipe, index: usize) -> f64 {
        match recipe.ingredients[index]
            .quantity
            .as_ref()
            .expect("ingredient has a quantity")
            .value()
        {
            Value::Number(n) => n.value(),
            other => panic!("expected a numeric quantity, got {other:?}"),
        }
    }

    fn request(source: RecipeSource, scale: f64) -> ReadRequest {
        ReadRequest { source, scale }
    }

    fn path_request(name: &str, scale: f64) -> ReadRequest {
        request(RecipeSource::Path(Utf8PathBuf::from(name)), scale)
    }

    #[test]
    fn reads_a_recipe_from_a_path() {
        let dir = fixture_dir();
        let outcome = read(&ctx_for(&dir), path_request("simple.cook", 1.0)).expect("reads");

        let ReadResult {
            recipe,
            title,
            scale,
        } = outcome.value;
        assert_eq!(title, "simple", "title falls back to the file stem");
        assert_eq!(scale, 1.0);
        assert_eq!(recipe.ingredients.len(), 2);
        assert_eq!(recipe.ingredients[0].name, "water");
        assert_eq!(recipe.ingredients[1].name, "salt");
        assert_eq!(quantity_value(&recipe, 0), 2.0);
        assert_eq!(quantity_value(&recipe, 1), 1.0);
        assert!(outcome.diagnostics.is_empty());
    }

    /// The extension is optional, exactly as in `cook recipe simple`.
    #[test]
    fn a_bare_name_resolves_to_the_cook_file() {
        let dir = fixture_dir();
        let outcome = read(&ctx_for(&dir), path_request("simple", 1.0)).expect("reads");
        assert_eq!(outcome.value.title, "simple");
        assert_eq!(outcome.value.recipe.ingredients.len(), 2);
    }

    #[test]
    fn reads_a_recipe_from_memory() {
        // A base path that does not exist: reading in-memory text must not go
        // near the filesystem, so this context is never consulted.
        let ctx = Context::new(Utf8PathBuf::from("/nonexistent"));
        let outcome = read(
            &ctx,
            request(
                RecipeSource::Content {
                    text: "Boil @water{2%cups}.\nAdd @salt{1%tsp}.\n".to_string(),
                    name: "buffer".to_string(),
                },
                1.0,
            ),
        )
        .expect("reads in-memory text");

        assert_eq!(
            outcome.value.title, "buffer",
            "with no title of its own, the recipe falls back to the caller's name"
        );
        assert_eq!(outcome.value.recipe.ingredients.len(), 2);
        assert_eq!(quantity_value(&outcome.value.recipe, 0), 2.0);
    }

    /// The metadata title beats the caller's name and the file stem alike.
    ///
    /// This is what reaches `-f markdown` headings and the `name` of
    /// schema.org output, so a fallback leaking through here corrupts them.
    #[test]
    fn a_metadata_title_wins_over_the_fallback_name() {
        let ctx = Context::new(Utf8PathBuf::from("/nonexistent"));
        let outcome = read(
            &ctx,
            request(
                RecipeSource::Content {
                    text: "---\ntitle: Proper Title\n---\nBoil @water{1%cup}.\n".to_string(),
                    name: "buffer".to_string(),
                },
                1.0,
            ),
        )
        .expect("reads");
        assert_eq!(
            outcome.value.title, "Proper Title",
            "the recipe's own title must beat the caller's label"
        );

        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("stem.cook"),
            "---\ntitle: Proper Title\n---\nBoil @water{1%cup}.\n",
        )
        .unwrap();
        let outcome = read(&ctx_for(&dir), path_request("stem.cook", 1.0)).expect("reads");
        assert_eq!(outcome.value.title, "Proper Title");
    }

    #[test]
    fn missing_recipe_is_recipe_not_found() {
        let dir = fixture_dir();
        match read(&ctx_for(&dir), path_request("absent.cook", 1.0)) {
            Err(CoreError::RecipeNotFound { name }) => assert_eq!(name, "absent.cook"),
            other => panic!("expected RecipeNotFound, got {other:?}"),
        }
    }

    #[test]
    fn the_request_scale_is_applied_when_there_is_no_inline_factor() {
        let dir = fixture_dir();
        // `simple.cook` declares `@water{2%cups}`.
        let outcome = read(&ctx_for(&dir), path_request("simple.cook", 3.0)).expect("reads");
        assert_eq!(quantity_value(&outcome.value.recipe, 0), 6.0);
        let outcome = read(&ctx_for(&dir), path_request("simple.cook", 0.5)).expect("reads");
        assert_eq!(quantity_value(&outcome.value.recipe, 0), 1.0);
    }

    #[test]
    fn inline_scale_overrides_the_request() {
        let dir = fixture_dir();
        // Request 1.0, inline 3: the inline factor must win, so `@water{2%cups}`
        // becomes 6 cups. The `:3` must also come off the name, or the lookup
        // would not find the file at all.
        let outcome = read(&ctx_for(&dir), path_request("simple.cook:3", 1.0)).expect("reads");
        assert_eq!(quantity_value(&outcome.value.recipe, 0), 6.0);
        assert_eq!(outcome.value.title, "simple");
        // Reported back, because formatters label the output with it.
        assert_eq!(outcome.value.scale, 3.0);

        // And the other way round, so that a test asserting only "not 2.0"
        // cannot pass by accident: request 3.0, inline 1.
        let outcome = read(&ctx_for(&dir), path_request("simple.cook:1", 3.0)).expect("reads");
        assert_eq!(quantity_value(&outcome.value.recipe, 0), 2.0);
        assert_eq!(outcome.value.scale, 1.0);
    }

    /// The request scale is ignored for in-memory text only if we forget it,
    /// so pin it: `Content` has no inline suffix to fall back on.
    #[test]
    fn content_is_scaled_by_the_request() {
        let ctx = Context::new(Utf8PathBuf::from("/nonexistent"));
        let outcome = read(
            &ctx,
            request(
                RecipeSource::Content {
                    text: "Boil @water{2%cups}.\n".to_string(),
                    name: "buffer".to_string(),
                },
                2.5,
            ),
        )
        .expect("reads");
        assert_eq!(quantity_value(&outcome.value.recipe, 0), 5.0);
        assert_eq!(outcome.value.scale, 2.5);
    }

    /// A colon in a path is not a scaling factor unless a number follows it.
    #[test]
    fn a_non_numeric_suffix_is_part_of_the_name() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("odd:name.cook"), "Boil @water{2%cups}.\n").unwrap();
        let outcome = read(&ctx_for(&dir), path_request("odd:name.cook", 2.0)).expect("reads");
        assert_eq!(quantity_value(&outcome.value.recipe, 0), 4.0);
    }

    #[test]
    fn parse_errors_name_the_path_not_the_title() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = camino::Utf8PathBuf::from_path_buf(dir.path().join("broken.cook")).unwrap();
        // A metadata title, so a report naming the title would be visibly wrong.
        std::fs::write(&path, "---\ntitle: Fancy\n---\nAdd @{1%tsp}.\n").unwrap();

        match read(&ctx_for(&dir), path_request("broken.cook", 1.0)) {
            Err(CoreError::Parse {
                name,
                diagnostics,
                rendered,
            }) => {
                assert_eq!(name, path.as_str());
                assert!(
                    rendered.contains(path.as_str()),
                    "report should name the file: {rendered}"
                );
                assert_eq!(diagnostics.len(), 1);
                assert_eq!(
                    diagnostics[0].location.as_ref().unwrap().file.as_deref(),
                    Some(path.as_path())
                );
            }
            other => panic!("expected CoreError::Parse, got {other:?}"),
        }
    }

    /// Warnings come back rather than being logged and dropped.
    #[test]
    fn warnings_reach_the_caller() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("old.cook"),
            ">> title: Old Style\n\nBoil @water{1%cup}.\n",
        )
        .unwrap();

        let outcome = read(&ctx_for(&dir), path_request("old.cook", 1.0)).expect("parses");
        // The title falls back to the file stem: `cooklang-find` reads titles
        // from YAML frontmatter only, and this recipe uses the deprecated
        // `>>` syntax. That is pre-existing CLI behaviour, pinned here.
        assert_eq!(outcome.value.title, "old");
        assert!(!outcome.diagnostics.is_empty(), "expected a diagnostic");
        for d in &outcome.diagnostics {
            assert_eq!(d.severity, crate::Severity::Warning, "got {d:?}");
        }
    }

    #[test]
    fn non_finite_scale_is_rejected() {
        let dir = fixture_dir();
        match read(&ctx_for(&dir), path_request("simple.cook", f64::NAN)) {
            Err(CoreError::InvalidScale { scale }) => assert!(scale.is_nan()),
            other => panic!("expected InvalidScale, got {other:?}"),
        }
    }

    #[test]
    fn splits_a_name_from_its_scaling_factor() {
        assert_eq!(
            split_name_and_scale("recipe.cook:2"),
            Some(("recipe.cook", 2.0))
        );
        assert_eq!(
            split_name_and_scale("recipe.cook:1.5"),
            Some(("recipe.cook", 1.5))
        );
        assert_eq!(split_name_and_scale("recipe.cook"), None);
        assert_eq!(split_name_and_scale("recipe.cook:abc"), None);
        // Regression for https://github.com/cooklang/cookcli/issues/335: a
        // Windows drive letter must not be read as a scaling factor.
        assert_eq!(split_name_and_scale(r"C:\test\recipe.cook"), None);
        // The last colon wins, so a directory with a colon in it still scales.
        assert_eq!(
            split_name_and_scale("odd:dir/recipe.cook:2"),
            Some(("odd:dir/recipe.cook", 2.0))
        );
    }
}
