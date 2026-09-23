//! Resolving a recipe name or path to a file on disk, and walking a whole
//! collection of them.

use crate::{parser::parse_unscaled, CoreError, Diagnostic};
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use cooklang::Recipe;
use cooklang_find::{tree::TreeError, RecipeEntry, RecipeTree};
use std::collections::BTreeSet;

/// Re-exported from [`cooklang_format`], which is where the writers that
/// depend on this spelling now live.
pub use cooklang_format::REFERENCE_SEPARATOR;

/// Whether `path`, joined to a directory, is sure to stay inside it.
///
/// Every component has to be a plain name. `..` climbs out, and a root, a
/// drive letter or a UNC share makes `join` drop the directory altogether:
/// `base.join("C:\\x")` is just `C:\x`. A leading `./` is refused as well;
/// nothing we link to starts with one. On Windows no component may hold a `:`
/// either. No file name can, so past a drive letter it could only pick an
/// NTFS alternate data stream.
///
/// Meant for paths taken from a request, before they are joined to the recipe
/// directory. The check is purely lexical on purpose: merely looking up
/// `\\host\share` on disk makes Windows connect to that host and hand it the
/// user's NTLM hash.
pub fn is_safe_relative_path(path: &str) -> bool {
    Utf8Path::new(path)
        .components()
        .all(|component| match component {
            Utf8Component::Normal(name) => !(cfg!(windows) && name.contains(':')),
            _ => false,
        })
}

/// Resolve a recipe reference to a path relative to the collection root.
///
/// `reference` is the reference as the recipe writes it — `./Shared/Sauce`,
/// `../sauces/tomato`. The parser only produces one from a name starting `./`
/// or `../`, so it always opens with a dot component and can never be an
/// absolute path of its own.
///
/// **A reference that steps up does so from the directory of the file that
/// writes it; one that does not is read from the collection root.** That is
/// not a new rule for the second half: `./` has always meant the root here,
/// and the shipped seed relies on it — `Breakfast/Mexican Style Burrito.cook`
/// writes `@./Shared/Red Beans` and means `Shared/Red Beans.cook` at the root,
/// not one beside itself. `..` had no working meaning at all before this: it
/// was resolved from the root like everything else, so `../sauces/tomato`
/// landed *outside* the collection whatever wrote it.
///
/// `from` is that writer's directory, relative to the root and empty for a
/// recipe sitting at it. It is ours, not a request's: an entry path with the
/// base directory stripped off.
///
/// Returns `None` for a reference that still climbs above the root, or that
/// normalises to something [`is_safe_relative_path`] refuses — a reference is
/// free to spell `@./C:/Windows/win.ini{}`, and joining that to the recipe
/// directory would drop the directory. Neither names a recipe in the
/// collection, so callers report them the way they report one that is missing.
pub fn resolve_reference(from: &Utf8Path, reference: &str) -> Option<Utf8PathBuf> {
    let reference = reference.replace('\\', "/");
    let parts: Vec<&str> = reference.split('/').collect();

    let mut resolved: Vec<&str> = if parts.contains(&"..") {
        from.components()
            .map(|component| match component {
                Utf8Component::Normal(name) => Some(name),
                // `from` is built from an entry path, so it holds plain names.
                // Anything else means a caller passed a path it did not strip
                // the base directory off; resolving against it would be a
                // guess, so give up rather than guess.
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?
    } else {
        Vec::new()
    };

    for part in parts {
        match part {
            "" | "." => {}
            ".." => {
                // Nothing left to step out of: this climbs above the root.
                resolved.pop()?;
            }
            name => resolved.push(name),
        }
    }

    if resolved.is_empty() {
        return None;
    }
    let path = resolved.join("/");
    is_safe_relative_path(&path).then(|| Utf8PathBuf::from(path))
}

/// Look `name` up under `base_path`, returning the file it resolves to.
///
/// `name` may be a path or a bare recipe name, with or without an extension —
/// `cooklang-find` tries `.cook` and `.menu` for the latter. A leading `./` is
/// stripped first, because `cooklang-find` does not expect it.
///
/// # Errors
///
/// - [`CoreError::RecipeNotFound`] if nothing matches. Reserved for genuine
///   absence, so that a caller told "not found" does not go looking for a file
///   that is sitting right there.
/// - [`CoreError::Io`] if a match was found but could not be opened or its
///   front matter could not be understood. The cause is in `source`.
pub fn get_recipe(base_path: &Utf8Path, name: &str) -> Result<RecipeEntry, CoreError> {
    // Remove the `./` prefix if present before passing to cooklang_find, which
    // does not expect it.
    let clean_name = name.strip_prefix("./").unwrap_or(name);

    cooklang_find::get_recipe(vec![base_path.to_path_buf()], clean_name.into())
        .map_err(|e| fetch_error(e, Utf8Path::new(clean_name)))
}

/// Map a lookup failure onto the error that describes what actually happened.
///
/// Only `FetchError::InvalidPath` means the recipe is absent;
/// the rest mean it was found and could not be opened or understood.
pub(crate) fn fetch_error(
    error: cooklang_find::fetcher::FetchError,
    lookup: &Utf8Path,
) -> CoreError {
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
pub(crate) fn entry_error(error: cooklang_find::RecipeEntryError) -> std::io::Error {
    match error {
        cooklang_find::RecipeEntryError::IoError(e) => e,
        other => std::io::Error::other(other.to_string()),
    }
}

/// Map a tree-building failure onto the error that describes what happened.
///
/// Everything except a listing failure is about the root itself: it is missing,
/// it is a file, or its name cannot be turned into a glob pattern. None of them
/// mean a recipe was read and rejected.
///
/// Shared by every command that walks a collection — `doctor::validate`,
/// `pantry::recipes` and `pantry::plan` — so that a mistyped root is reported
/// the same way whichever of them was asked.
pub(crate) fn tree_error(error: TreeError, base_dir: &Utf8Path) -> CoreError {
    let search = |message: String| CoreError::Search {
        base_dir: base_dir.to_owned(),
        message,
    };
    match error {
        // The variants' own `Display` repeats the path, which the `Search`
        // rendering already names.
        TreeError::DirectoryNotFound(_) => search("no such directory".to_string()),
        TreeError::NotADirectory(_) => search("not a directory".to_string()),
        TreeError::PatternError(source) => search(source.to_string()),
        // What a root spelled `./recipes` gets: the walk finds
        // `recipes/soup.cook`, which does not start with `./recipes`. Reached
        // by `cook doctor validate -b ./recipes`, so it is worth wording for a
        // person.
        TreeError::StripPrefixError(what) => {
            search(format!("cannot express {what} relative to it"))
        }
        // Carries the file it failed on, which is more use than the root.
        TreeError::GlobError(source) => CoreError::Io {
            path: Utf8Path::from_path(source.path())
                .map(Utf8Path::to_owned)
                .unwrap_or_else(|| base_dir.to_owned()),
            source: source.into_error(),
        },
        // Unreachable through `build_tree`, which skips an entry it cannot
        // load rather than failing. Mapped rather than ignored so that the
        // match stops compiling if that changes.
        TreeError::RecipeEntryError(source) => CoreError::Io {
            path: base_dir.to_owned(),
            source: entry_error(source),
        },
    }
}

// ---------------------------------------------------------------------------
// Walking a collection
// ---------------------------------------------------------------------------

/// Build the recipe tree under `base_dir`, in this crate's error wording.
pub(crate) fn build_tree(base_dir: &Utf8Path) -> Result<RecipeTree, CoreError> {
    tracing::trace!("walking recipes under {base_dir}");
    cooklang_find::build_tree(base_dir).map_err(|e| tree_error(e, base_dir))
}

/// Every recipe in the tree, depth first, in path order.
///
/// Sorted because `cooklang-find` holds a directory's children in a `HashMap`,
/// so the walk itself yields them differently from run to run. Several callers
/// sort their own results anyway — `pantry::recipes` does, and `pantry::plan`
/// breaks its ties alphabetically — but the diagnostics depend on this:
/// without it, a collection with two unreadable recipes reports them in a
/// different order each time.
pub(crate) fn walk(tree: &RecipeTree) -> Vec<&RecipeEntry> {
    fn collect<'a>(tree: &'a RecipeTree, out: &mut Vec<&'a RecipeEntry>) {
        if let Some(entry) = &tree.recipe {
            out.push(entry);
        }
        for subtree in tree.children.values() {
            collect(subtree, out);
        }
    }

    let mut entries = Vec::new();
    collect(tree, &mut entries);
    entries.sort_by_key(|entry| (entry.path().cloned(), entry.name().clone()));
    entries
}

/// Parse one recipe, or note that it was left out.
///
/// Nothing here fails the walk: one unreadable file in a collection must not
/// cost the caller the answer for the rest of it.
///
/// **Only the skip is reported.** A recipe that parses with warnings — the
/// deprecated `>>` metadata syntax, say — contributes its ingredients and none
/// of its warnings, because they cannot change any answer a caller of this is
/// computing. `doctor::validate` is the one command that reports them, and the
/// reason `cook doctor aisle` and `cook doctor pantry` no longer log a recipe's
/// parse warnings the way they did before they went through here.
pub(crate) fn parse_or_skip(
    entry: &RecipeEntry,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Recipe> {
    let display = entry
        .path()
        .map(ToString::to_string)
        .or_else(|| entry.name().clone())
        .unwrap_or_else(|| "unknown".to_string());

    let mut skipped = |reason: &str| {
        let diagnostic = Diagnostic::warning(format!(
            "could not {reason} {display}, so it was not considered"
        ));
        diagnostics.push(match entry.path() {
            Some(path) => diagnostic.at_file(path),
            None => diagnostic,
        });
        None
    };

    let content = match entry.content() {
        Ok(content) => content,
        Err(_) => return skipped("read"),
    };

    // Unscaled: scaling by one would only re-fit units, and no caller of this
    // reads a quantity.
    match parse_unscaled(&content, &display, entry.path().map(Utf8PathBuf::as_path)) {
        Ok(outcome) => Some(outcome.value),
        Err(_) => skipped("parse"),
    }
}

/// The ingredients a recipe asks the reader to have, as it writes them.
///
/// A set, so a recipe using flour twice wants flour once.
///
/// References to other recipes are left out: they are a recipe to make, not a
/// thing to have in. Ingredients the parser marks as not to be listed are left
/// out too, though with [`PARSER`](crate::PARSER)'s extensions there are none
/// — `@-salt{}` parses as an ingredient *named* `-salt` rather than a hidden
/// one, so it counts like any other. The filter is kept for the day that
/// changes.
pub(crate) fn listed_ingredients(recipe: &Recipe) -> BTreeSet<String> {
    recipe
        .ingredients
        .iter()
        .filter(|ingredient| ingredient.reference.is_none())
        .filter(|ingredient| ingredient.modifiers().should_be_listed())
        .map(|ingredient| ingredient.display_name().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("soup.cook"), "Boil @water{1%l}.\n").unwrap();
        std::fs::write(
            dir.path().join("sub").join("stew.cook"),
            "Boil @water{1%l}.\n",
        )
        .unwrap();
        dir
    }

    fn base(dir: &tempfile::TempDir) -> Utf8PathBuf {
        Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap()
    }

    #[test]
    fn plain_relative_paths_are_safe() {
        for path in [
            "Pancakes",
            "Breakfast/Easy Pancakes",
            "Breakfast/Pancakes.cook",
            "Crème brûlée",
        ] {
            assert!(is_safe_relative_path(path), "{path:?} must be accepted");
        }
    }

    #[test]
    fn paths_that_leave_the_directory_are_not_safe() {
        for path in [
            "..",
            "../Secret",
            "Breakfast/../../Secret",
            "/etc/passwd",
            "./Pancakes",
        ] {
            assert!(!is_safe_relative_path(path), "{path:?} must be refused");
        }
    }

    /// Drive letters, UNC shares, `\` separators and NTFS streams only mean
    /// something on Windows; anywhere else these are odd but harmless names.
    #[cfg(windows)]
    #[test]
    fn windows_prefixes_and_streams_are_not_safe() {
        for path in [
            "C:",
            "C:/",
            "C:Windows/win.ini",
            r"..\Secret",
            r"\\attacker\share\x.cook",
            "//attacker/share/x.cook",
            r"\\?\C:\Windows",
            "Breakfast/C:/Windows/win.ini",
            "Pancakes.cook::$DATA",
        ] {
            assert!(!is_safe_relative_path(path), "{path:?} must be refused");
        }
    }

    fn resolved(from: &str, reference: &str) -> Option<String> {
        resolve_reference(Utf8Path::new(from), reference).map(String::from)
    }

    /// What the shipped seed relies on: `Breakfast/Mexican Style Burrito.cook`
    /// writes `@./Shared/Red Beans` and means the one at the root.
    #[test]
    fn a_reference_that_does_not_step_up_is_read_from_the_root() {
        assert_eq!(
            resolved("Breakfast", "./Shared/Red Beans"),
            Some("Shared/Red Beans".to_string())
        );
        assert_eq!(resolved("", "./lamb-chops"), Some("lamb-chops".to_string()));
        assert_eq!(
            resolved("a/b/c", "./Risotto"),
            Some("Risotto".to_string()),
            "however deep the writer sits"
        );
    }

    #[test]
    fn a_reference_that_steps_up_does_so_from_the_writer() {
        assert_eq!(
            resolved("Breakfast", "../Shared/Vinaigrette"),
            Some("Shared/Vinaigrette".to_string())
        );
        assert_eq!(
            resolved("Menus/Autumn", "../../Risotto"),
            Some("Risotto".to_string())
        );
        assert_eq!(
            resolved("Menus/Autumn", "../Sunday"),
            Some("Menus/Sunday".to_string())
        );
    }

    /// The escape this exists to close: from the root there is nothing to step
    /// out of, and no depth of writer may be over-spent.
    #[test]
    fn a_reference_climbing_above_the_root_resolves_to_nothing() {
        assert_eq!(resolved("", "../Secret"), None);
        assert_eq!(resolved("Breakfast", "../../Secret"), None);
        assert_eq!(resolved("Menus/Autumn", "../../../Secret"), None);
        assert_eq!(resolved("", "./../Secret"), None);
    }

    /// `.` and `..` are normalised wherever they sit, not just at the front,
    /// so what comes back is always a plain relative path.
    #[test]
    fn the_result_is_normalised() {
        assert_eq!(
            resolved("", "./Shared/./Sauce"),
            Some("Shared/Sauce".to_string())
        );
        assert_eq!(
            resolved("Breakfast", "../Shared/Extra/../Sauce"),
            Some("Shared/Sauce".to_string())
        );
        assert_eq!(resolved("", "./Shared/.."), None, "normalises to the root");
    }

    /// It is the `..` anywhere in a reference that makes it writer-relative,
    /// not only one at the front, so a `./` in front of one does not re-anchor
    /// it to the root and turn a step up into an escape.
    #[test]
    fn a_dot_in_front_of_a_step_up_does_not_re_anchor_it() {
        assert_eq!(resolved("Breakfast", "./../Secret"), Some("Secret".into()));
    }

    /// A reference is written in a recipe file, which is not always ours: an
    /// imported or synced one can spell anything the parser accepts.
    #[test]
    fn a_reference_naming_something_outside_the_collection_resolves_to_nothing() {
        assert_eq!(resolved("", "./"), None);
        if cfg!(windows) {
            assert_eq!(resolved("", "./C:/Windows/win.ini"), None);
            assert_eq!(resolved("Breakfast", "../C:/Windows/win.ini"), None);
            assert_eq!(resolved("", r".\Sauce.cook::$DATA"), None);
        }
    }

    /// `\` is a separator in a reference — `parse_reference` accepts `.\` and
    /// `..\` — so it has to be normalised before the components are read, or
    /// `..\Secret` would arrive as one plain name and be joined verbatim.
    #[test]
    fn backslashes_in_a_reference_are_separators() {
        assert_eq!(
            resolved("Breakfast", r"..\Shared\Vinaigrette"),
            Some("Shared/Vinaigrette".to_string())
        );
        assert_eq!(resolved("", r"..\Secret"), None);
    }

    #[test]
    fn finds_a_recipe_by_name_path_and_extension() {
        let dir = fixture();
        for name in ["soup", "soup.cook", "./soup.cook"] {
            let entry = get_recipe(&base(&dir), name).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(entry.path().and_then(|p| p.file_name()), Some("soup.cook"));
        }
    }

    /// The `./` strip is what makes a reference like `@./sub/stew{}` resolve.
    #[test]
    fn a_leading_dot_slash_is_stripped_from_nested_paths() {
        let dir = fixture();
        let entry = get_recipe(&base(&dir), "./sub/stew.cook").expect("resolves");
        assert_eq!(entry.path().and_then(|p| p.file_name()), Some("stew.cook"));
    }

    #[test]
    fn a_missing_recipe_is_not_found() {
        let dir = fixture();
        match get_recipe(&base(&dir), "./absent.cook") {
            // The `./` must not survive into the reported name, or the message
            // names something the caller never asked for.
            Err(CoreError::RecipeNotFound { name }) => assert_eq!(name, "absent.cook"),
            other => panic!("expected RecipeNotFound, got {other:?}"),
        }
    }

    /// Present but unusable is an I/O error, not absence.
    #[test]
    fn a_directory_named_like_a_recipe_is_an_io_error() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join("adir.cook")).unwrap();
        match get_recipe(&base(&dir), "adir.cook") {
            Err(CoreError::Io { path, .. }) => assert_eq!(path.file_name(), Some("adir.cook")),
            other => panic!("expected CoreError::Io, got {other:?}"),
        }
    }

    /// Pins all three lookup outcomes, including the one `cooklang-find` does
    /// not currently produce: only genuine absence may be reported as absence.
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
}
