//! Reading and scaling a single recipe.

use crate::{parse_recipe, parse_recipe_at, Context, CoreError, Outcome, RecipeSource};
use camino::{Utf8Path, Utf8PathBuf};
use cooklang::Recipe;

/// The character separating a recipe name from an inline scaling factor.
const SCALING_DELIMITER: char = ':';

/// What to read, and at what scale.
#[derive(Debug, Clone)]
pub struct ReadRequest {
    /// The recipe to read.
    pub source: RecipeSource,
    /// Scaling factor applied to all quantities. Pass `1.0` to leave
    /// quantities alone.
    ///
    /// This is the only scaling channel. CookCLI's `name:factor` argument
    /// convention is a *command-line* spelling, not a property of a path, so
    /// callers split it themselves with [`split_name_and_scale`] — otherwise
    /// a path chosen from a file picker could pick up a scaling factor from
    /// any directory that happens to end in `:2`.
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
    /// The file the recipe was read from, once resolved. `None` for
    /// [`RecipeSource::Content`].
    ///
    /// A bare name like `pancakes` can resolve to any of several directories
    /// and either extension, so the caller cannot reconstruct this. Callers
    /// that watch, reveal or write back the file they just read need it, and
    /// [`Diagnostic::location`](crate::Diagnostic::location) does not serve:
    /// a clean recipe produces no diagnostics at all.
    pub path: Option<Utf8PathBuf>,
}

/// Split a `name:factor` query into its parts.
///
/// `"pasta.cook:2"` becomes `("pasta.cook", 2.0)`. Returns `None` when there is
/// no colon, or when what follows the last one is not a number — so a Windows
/// path like `C:\recipes\pasta.cook` is left alone.
///
/// This is CookCLI's command-line convention for naming a recipe and a scaling
/// factor in one argument. [`read`] deliberately does not apply it: callers
/// that accept arguments in that form split them here and fill in
/// [`ReadRequest::scale`] themselves.
pub fn split_name_and_scale(query: &str) -> Option<(&str, f64)> {
    let (name, factor) = query.trim().rsplit_once(SCALING_DELIMITER)?;
    let factor = factor.parse::<f64>().ok()?;
    Some((name, factor))
}

/// Read a recipe, scale it, and report anything the parser had to say.
///
/// A [`RecipeSource::Path`] is resolved against [`Context::base_path`], trying
/// both `.cook` and `.menu` when the name carries no extension. A
/// [`RecipeSource::Content`] is parsed as given and never touches the
/// filesystem; its `name` is only a fallback for [`ReadResult::title`], used
/// when the recipe declares no title of its own.
///
/// # Errors
///
/// - [`CoreError::RecipeNotFound`] if no file matches the given path or name.
///   Reserved for genuine absence — a file that exists but cannot be opened is
///   an [`CoreError::Io`], since telling a caller a file it can see is "not
///   found" sends it looking for the wrong problem.
/// - [`CoreError::Io`] if the file is found but cannot be read or its front
///   matter cannot be understood. The underlying cause is in `source`.
/// - [`CoreError::Parse`] if the recipe has parse errors. Its `name` is the
///   recipe's path, matching the file the `rendered` report points at.
/// - [`CoreError::InvalidScale`] if the scale is not finite.
pub fn read(ctx: &Context, req: ReadRequest) -> Result<Outcome<ReadResult>, CoreError> {
    match req.source {
        RecipeSource::Content { text, name } => {
            let outcome = parse_recipe(&text, &name, req.scale)?;
            let title = title_for(&outcome.value, || name);
            Ok(Outcome::with_diagnostics(
                ReadResult {
                    recipe: outcome.value,
                    title,
                    path: None,
                },
                outcome.diagnostics,
            ))
        }
        RecipeSource::Path(lookup) => {
            let entry =
                cooklang_find::get_recipe(vec![ctx.base_path().to_path_buf()], lookup.clone())
                    .map_err(|e| fetch_error(e, &lookup))?;

            // `get_recipe` only ever returns path-backed entries, but the type
            // permits otherwise; fall back to what we looked up rather than
            // inventing a placeholder path.
            let path = entry.path().cloned();
            let display_path = path.clone().unwrap_or(lookup);

            let content = entry.content().map_err(|source| CoreError::Io {
                path: display_path.clone(),
                source: entry_error(source),
            })?;

            // Diagnostics and the parse report name the file, not the title, so
            // that the caller can open what they point at.
            let outcome =
                parse_recipe_at(&content, display_path.as_str(), req.scale, path.as_deref())?;

            let title = title_for(&outcome.value, || entry.name().clone().unwrap_or_default());
            Ok(Outcome::with_diagnostics(
                ReadResult {
                    recipe: outcome.value,
                    title,
                    path,
                },
                outcome.diagnostics,
            ))
        }
    }
}

/// The recipe's own declared title, or `fallback` when it declares none.
///
/// The one place the rule lives. It used to be applied twice — once from the
/// parsed recipe and once from `cooklang-find`'s `entry.name()`, which reads
/// YAML front matter only — so the same bytes produced different titles
/// depending on whether they arrived by path or in memory.
fn title_for(recipe: &Recipe, fallback: impl FnOnce() -> String) -> String {
    recipe
        .metadata
        .title()
        .map_or_else(fallback, ToOwned::to_owned)
}

/// Map a lookup failure onto the error that describes what actually happened.
///
/// Only `FetchError::InvalidPath` means the recipe is absent;
/// the rest mean it was found and could not be opened or understood.
fn fetch_error(error: cooklang_find::fetcher::FetchError, lookup: &Utf8Path) -> CoreError {
    use cooklang_find::fetcher::FetchError;
    match error {
        FetchError::InvalidPath(name) => CoreError::RecipeNotFound {
            name: name.to_string(),
        },
        FetchError::IoError(source) => CoreError::Io {
            path: lookup.to_owned(),
            source,
        },
        FetchError::RecipeEntryError(source) => CoreError::Io {
            path: lookup.to_owned(),
            source: entry_error(source),
        },
    }
}

/// Unwrap a `cooklang-find` entry error to the underlying [`std::io::Error`].
///
/// Its other variants are front matter problems rather than I/O; they keep
/// their message as the source so nothing is lost, and travel as
/// [`CoreError::Io`] because they too mean "found, but unusable".
fn entry_error(error: cooklang_find::RecipeEntryError) -> std::io::Error {
    match error {
        cooklang_find::RecipeEntryError::IoError(e) => e,
        other => std::io::Error::other(other.to_string()),
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
            path,
        } = outcome.value;
        assert_eq!(title, "simple", "title falls back to the file stem");
        assert_eq!(
            path.as_deref(),
            Some(
                camino::Utf8PathBuf::from_path_buf(dir.path().join("simple.cook"))
                    .unwrap()
                    .as_path()
            ),
            "the resolved file must be reported back"
        );
        assert_eq!(recipe.ingredients.len(), 2);
        assert_eq!(recipe.ingredients[0].name, "water");
        assert_eq!(recipe.ingredients[1].name, "salt");
        assert_eq!(quantity_value(&recipe, 0), 2.0);
        assert_eq!(quantity_value(&recipe, 1), 1.0);
        assert!(outcome.diagnostics.is_empty());
    }

    /// The extension is optional, exactly as in `cook recipe simple`.
    ///
    /// This is why [`ReadResult::path`] has to exist: `"simple"` alone does not
    /// tell the caller which file was opened.
    #[test]
    fn a_bare_name_resolves_to_the_cook_file() {
        let dir = fixture_dir();
        let outcome = read(&ctx_for(&dir), path_request("simple", 1.0)).expect("reads");
        assert_eq!(outcome.value.title, "simple");
        assert_eq!(outcome.value.recipe.ingredients.len(), 2);
        assert_eq!(
            outcome.value.path.as_ref().and_then(|p| p.file_name()),
            Some("simple.cook"),
            "the extension the lookup chose must be reported back"
        );
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
        assert_eq!(
            outcome.value.path, None,
            "in-memory text has no file to report"
        );
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

    /// The same bytes must produce the same title whichever way they arrive.
    ///
    /// The two routes used to run through different metadata parsers — the
    /// cooklang parser for `Content`, `cooklang-find`'s front-matter reader for
    /// `Path` — so anything the two disagreed on silently forked. The `>>`
    /// spelling is one such disagreement and stands in for the rest.
    #[test]
    fn a_path_and_a_buffer_of_the_same_bytes_agree_on_the_title() {
        for text in [
            "---\ntitle: Agreed\n---\nBoil @water{1%cup}.\n",
            ">> title: Agreed\n\nBoil @water{1%cup}.\n",
            "Boil @water{1%cup}.\n",
        ] {
            let dir = tempfile::TempDir::new().unwrap();
            std::fs::write(dir.path().join("same.cook"), text).unwrap();
            let from_path = read(&ctx_for(&dir), path_request("same.cook", 1.0))
                .expect("reads")
                .value
                .title;

            let from_memory = read(
                &Context::new(Utf8PathBuf::from("/nonexistent")),
                request(
                    RecipeSource::Content {
                        text: text.to_string(),
                        // The same fallback the path route uses, so only a real
                        // disagreement can make these differ.
                        name: "same".to_string(),
                    },
                    1.0,
                ),
            )
            .expect("reads")
            .value
            .title;

            assert_eq!(from_path, from_memory, "titles diverged for {text:?}");
        }
    }

    #[test]
    fn missing_recipe_is_recipe_not_found() {
        let dir = fixture_dir();
        match read(&ctx_for(&dir), path_request("absent.cook", 1.0)) {
            Err(CoreError::RecipeNotFound { name }) => assert_eq!(name, "absent.cook"),
            other => panic!("expected RecipeNotFound, got {other:?}"),
        }
    }

    /// A file that exists but cannot be opened is *not* "recipe not found".
    ///
    /// Reporting absence for a file the caller can see in its own tree sends it
    /// looking for the wrong problem, and hides the permission error that
    /// actually needs fixing. This also covers the only other route into
    /// `CoreError::Io` here, since `get_recipe` fails before the entry exists.
    #[test]
    #[cfg(unix)]
    fn an_unreadable_recipe_is_an_io_error_not_a_missing_one() {
        use std::os::unix::fs::PermissionsExt;

        // Root ignores the permission bits, so there would be nothing to test.
        if unsafe { libc_geteuid() } == 0 {
            return;
        }

        let dir = fixture_dir();
        let path = dir.path().join("simple.cook");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();

        let result = read(&ctx_for(&dir), path_request("simple.cook", 1.0));

        // Restore before asserting, so a failure does not leave an
        // undeletable temporary directory behind.
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        match result {
            Err(CoreError::Io {
                path: reported,
                source,
            }) => {
                assert_eq!(reported.file_name(), Some("simple.cook"));
                assert_eq!(source.kind(), std::io::ErrorKind::PermissionDenied);
            }
            other => panic!("expected CoreError::Io, got {other:?}"),
        }
    }

    // `geteuid` without taking a dependency on `libc` for one call.
    #[cfg(unix)]
    extern "C" {
        #[link_name = "geteuid"]
        fn libc_geteuid() -> u32;
    }

    /// The other way a recipe can be present but unusable, and the one that
    /// needs no permission bits — so it runs everywhere, including as root and
    /// on Windows. Covers `fetch_error`'s `RecipeEntryError` arm.
    #[test]
    fn a_directory_named_like_a_recipe_is_an_io_error() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join("adir.cook")).unwrap();

        match read(&ctx_for(&dir), path_request("adir.cook", 1.0)) {
            Err(CoreError::Io { path, .. }) => {
                assert_eq!(path.file_name(), Some("adir.cook"));
            }
            other => panic!("expected CoreError::Io, got {other:?}"),
        }
    }

    /// Pins all three lookup outcomes, including the one `get_recipe` does not
    /// currently produce: only genuine absence may be reported as absence.
    /// Going through `read` alone would leave `FetchError::IoError` unpinned,
    /// so any future `cooklang-find` that starts returning it would silently
    /// take the wrong branch.
    #[test]
    fn only_a_missing_file_maps_to_recipe_not_found() {
        use cooklang_find::fetcher::FetchError;
        let lookup = Utf8Path::new("recipes/pancakes.cook");

        let absent = fetch_error(
            FetchError::InvalidPath(Utf8PathBuf::from("pancakes.cook")),
            lookup,
        );
        assert!(
            matches!(absent, CoreError::RecipeNotFound { ref name } if name == "pancakes.cook"),
            "an absent file is not found, got {absent:?}"
        );

        let unreadable = fetch_error(
            FetchError::IoError(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "denied",
            )),
            lookup,
        );
        match unreadable {
            CoreError::Io { path, source } => {
                assert_eq!(path, lookup);
                assert_eq!(source.kind(), std::io::ErrorKind::PermissionDenied);
            }
            other => panic!("an unreadable file is an I/O error, got {other:?}"),
        }

        let unusable = fetch_error(
            FetchError::RecipeEntryError(cooklang_find::RecipeEntryError::MetadataError(
                "bad front matter".to_string(),
            )),
            lookup,
        );
        match unusable {
            CoreError::Io { path, source } => {
                assert_eq!(path, lookup);
                assert!(source.to_string().contains("bad front matter"));
            }
            other => panic!("an unusable file is an I/O error, got {other:?}"),
        }
    }

    /// `entry_error` is the shared mapping used both when the lookup fails and
    /// when a later read does. Reaching the latter needs the file to become
    /// unreadable *between* two reads, which cannot be arranged without a race,
    /// so pin the mapping directly instead.
    #[test]
    fn entry_errors_keep_their_io_kind_and_never_lose_their_message() {
        let io = entry_error(cooklang_find::RecipeEntryError::IoError(
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        ));
        assert_eq!(
            io.kind(),
            std::io::ErrorKind::PermissionDenied,
            "an I/O cause must keep its kind, so callers can match on it"
        );

        let other = entry_error(cooklang_find::RecipeEntryError::MetadataError(
            "bad front matter".to_string(),
        ));
        assert!(
            other.to_string().contains("bad front matter"),
            "a non-I/O cause must keep its message: {other}"
        );
    }

    #[test]
    fn the_request_scale_is_applied() {
        let dir = fixture_dir();
        // `simple.cook` declares `@water{2%cups}`.
        let outcome = read(&ctx_for(&dir), path_request("simple.cook", 3.0)).expect("reads");
        assert_eq!(quantity_value(&outcome.value.recipe, 0), 6.0);
        let outcome = read(&ctx_for(&dir), path_request("simple.cook", 0.5)).expect("reads");
        assert_eq!(quantity_value(&outcome.value.recipe, 0), 1.0);
    }

    /// `read` takes a path, not a query string: a `:factor` suffix is part of
    /// the name it looks for, so the file simply is not there.
    ///
    /// Splitting inside `read` would mean a path from a file picker could pick
    /// up a scaling factor from any segment ending in `:<number>`. Callers that
    /// accept CookCLI's argument spelling call [`split_name_and_scale`] first.
    #[test]
    fn an_inline_factor_is_not_interpreted_as_scaling() {
        let dir = fixture_dir();
        match read(&ctx_for(&dir), path_request("simple.cook:3", 1.0)) {
            Err(CoreError::RecipeNotFound { name }) => assert_eq!(name, "simple.cook:3"),
            other => panic!("expected RecipeNotFound, got {other:?}"),
        }

        // And the split, applied by the caller, gets the scaling it asked for.
        let (name, factor) = split_name_and_scale("simple.cook:3").expect("splits");
        let outcome = read(&ctx_for(&dir), path_request(name, factor)).expect("reads");
        assert_eq!(quantity_value(&outcome.value.recipe, 0), 6.0);
    }

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
    }

    /// A colon in a filename is just a character, and reaches the lookup intact.
    #[test]
    fn a_colon_in_a_filename_is_part_of_the_name() {
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
        // The declared title, even in the deprecated `>>` spelling that
        // `cooklang-find`'s front-matter reader does not understand. Reading
        // the same bytes from memory must give the same answer.
        assert_eq!(outcome.value.title, "Old Style");
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
