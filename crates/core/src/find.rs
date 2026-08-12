//! Resolving a recipe name or path to a file on disk.

use crate::CoreError;
use camino::Utf8Path;
use cooklang_find::{tree::TreeError, RecipeEntry};

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

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;

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
