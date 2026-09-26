use axum::{http::StatusCode, Json};
use camino::{Utf8Path, Utf8PathBuf};

pub type ApiError = (StatusCode, Json<serde_json::Value>);

pub fn json_error(msg: impl std::fmt::Display) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "error": msg.to_string() }))
}

/// Whether `path`, taken from a request, may be joined to the recipe
/// directory: it stays inside it ([`is_safe_relative_path`]) and names nothing
/// hidden.
///
/// A component starting with `.` is refused wherever it sits. The recipe
/// directory is often a git checkout or a home directory, and its dot-files and
/// dot-directories (`.git/`, `.ssh/`, `.config/`, `.env`) are never part of
/// the collection. They are not harmless to reach either: `cooklang-find` opens
/// any existing file whose name has an extension, so without this
/// `GET /api/recipes/.config/gh/hosts.yml` read a token file and handed its
/// lines back as recipe steps.
///
/// [`is_safe_relative_path`]: crate::util::is_safe_relative_path
pub fn is_request_path(path: &str) -> bool {
    crate::util::is_safe_relative_path(path)
        && Utf8Path::new(path)
            .components()
            .all(|component| !component.as_str().starts_with('.'))
}

pub fn check_path(p: &str) -> Result<(), ApiError> {
    if !is_request_path(p) {
        tracing::error!("Invalid path: {p}");
        return Err((
            StatusCode::BAD_REQUEST,
            json_error(format!("Invalid path: {p}")),
        ));
    }
    Ok(())
}

/// The only kinds of file the raw, save and delete endpoints, and the editor
/// page in front of them, will touch.
const RECIPE_FILE_EXTENSIONS: [&str; 2] = ["cook", "menu"];

/// Where a request path lands among the recipe and menu files.
#[derive(Debug, PartialEq, Eq)]
pub enum RecipeFile {
    /// An existing `.cook` or `.menu` file.
    Existing(Utf8PathBuf),
    /// Nothing there yet: the file a save would create.
    Missing(Utf8PathBuf),
}

/// Resolves `path` from a request to a `.cook` or `.menu` file under `base`,
/// after [`check_path`].
///
/// The extension is optional. `Pasta.cook` and `Plan.menu` name those files;
/// `Pasta` is looked up as `Pasta.cook`, then `Pasta.menu`, and is created as
/// `Pasta.cook` when neither exists. Nothing else is ever returned, whatever
/// sits at the path: `config/aisle.conf` resolves to `config/aisle.conf.cook`.
///
/// The handlers used to take the joined path as it was whenever something
/// existed there, so `PUT /api/recipes/config/aisle.conf` overwrote the aisle
/// configuration and `DELETE` removed any file in the directory (#545). They
/// all go through here now, so they cannot drift apart again.
pub fn recipe_file(base: &Utf8Path, path: &str) -> Result<RecipeFile, ApiError> {
    check_path(path)?;

    let named = base.join(path);
    if named
        .extension()
        .is_some_and(|ext| RECIPE_FILE_EXTENSIONS.contains(&ext))
    {
        return Ok(if named.is_file() {
            RecipeFile::Existing(named)
        } else {
            RecipeFile::Missing(named)
        });
    }

    let [cook, menu] =
        RECIPE_FILE_EXTENSIONS.map(|ext| Utf8PathBuf::from(format!("{named}.{ext}")));
    Ok(if cook.is_file() {
        RecipeFile::Existing(cook)
    } else if menu.is_file() {
        RecipeFile::Existing(menu)
    } else {
        RecipeFile::Missing(cook)
    })
}

/// Rewrites the `tags` entry of a serialised metadata map into an array.
///
/// YAML frontmatter takes `tags: a, b` as well as `tags: [a, b]`, and the
/// parser keeps whichever type was written, so serialising the value verbatim
/// hands clients two different shapes for the same key. Every endpoint that
/// reports metadata runs it through here: a string is split the way
/// `cooklang`'s own `Metadata::tags` splits it — on commas, trimmed, with empty
/// entries and duplicates dropped — and any other type is left alone.
pub fn normalize_tags(metadata: &mut serde_json::Value) {
    let Some(raw) = metadata.get("tags").and_then(|v| v.as_str()) else {
        return;
    };

    let mut tags: Vec<&str> = Vec::new();
    for tag in raw.split(',').map(str::trim) {
        if tag.is_empty() || tags.contains(&tag) {
            continue;
        }
        tags.push(tag);
    }

    metadata["tags"] = serde_json::json!(tags);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn comma_separated_tags_become_an_array() {
        let mut metadata = json!({ "tags": "tag1, tag2" });
        normalize_tags(&mut metadata);
        assert_eq!(metadata, json!({ "tags": ["tag1", "tag2"] }));
    }

    #[test]
    fn a_single_tag_becomes_a_one_element_array() {
        let mut metadata = json!({ "tags": "tag1" });
        normalize_tags(&mut metadata);
        assert_eq!(metadata, json!({ "tags": ["tag1"] }));
    }

    #[test]
    fn blank_and_duplicate_entries_are_dropped() {
        let mut metadata = json!({ "tags": " a ,, b , a" });
        normalize_tags(&mut metadata);
        assert_eq!(metadata, json!({ "tags": ["a", "b"] }));
    }

    #[test]
    fn other_types_are_left_alone() {
        let mut metadata = json!({ "tags": ["a", "b"], "title": "Stew" });
        normalize_tags(&mut metadata);
        assert_eq!(metadata, json!({ "tags": ["a", "b"], "title": "Stew" }));

        let mut without_tags = json!({ "title": "Stew" });
        normalize_tags(&mut without_tags);
        assert_eq!(without_tags, json!({ "title": "Stew" }));
    }

    #[test]
    fn hidden_components_are_refused_wherever_they_sit() {
        for path in [
            ".env",
            ".git/config",
            "Sub/.hidden.cook",
            ".config/gh/hosts.yml",
        ] {
            assert!(!is_request_path(path), "{path:?} must be refused");
        }
        for path in [
            "Pasta.cook",
            "Sub/Pasta",
            "Mr. Smith's Stew.cook",
            "a.b/c.menu",
        ] {
            assert!(is_request_path(path), "{path:?} must be accepted");
        }
    }

    /// A recipe directory holding one of each kind of file the endpoints meet.
    fn collection() -> (tempfile::TempDir, Utf8PathBuf) {
        let dir = tempfile::TempDir::new().unwrap();
        let base = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        std::fs::create_dir_all(base.join("config")).unwrap();
        std::fs::create_dir_all(base.join("Folder")).unwrap();
        for file in ["Pasta.cook", "Plan.menu", "config/aisle.conf", "notes.txt"] {
            std::fs::write(base.join(file), "x").unwrap();
        }
        (dir, base)
    }

    fn resolve(base: &Utf8Path, path: &str) -> RecipeFile {
        recipe_file(base, path)
            .unwrap_or_else(|(status, body)| panic!("{path:?} was refused with {status}: {body:?}"))
    }

    #[test]
    fn recipe_files_resolve_with_or_without_their_extension() {
        let (_dir, base) = collection();
        let existing = |name: &str| RecipeFile::Existing(base.join(name));

        assert_eq!(resolve(&base, "Pasta.cook"), existing("Pasta.cook"));
        assert_eq!(resolve(&base, "Pasta"), existing("Pasta.cook"));
        assert_eq!(resolve(&base, "Plan.menu"), existing("Plan.menu"));
        assert_eq!(resolve(&base, "Plan"), existing("Plan.menu"));
    }

    #[test]
    fn a_new_recipe_keeps_the_extension_it_was_given() {
        let (_dir, base) = collection();
        let missing = |name: &str| RecipeFile::Missing(base.join(name));

        assert_eq!(resolve(&base, "Soup"), missing("Soup.cook"));
        assert_eq!(resolve(&base, "Soup.cook"), missing("Soup.cook"));
        assert_eq!(resolve(&base, "Week.menu"), missing("Week.menu"));
    }

    #[test]
    fn other_files_are_never_resolved() {
        let (_dir, base) = collection();
        let missing = |name: &str| RecipeFile::Missing(base.join(name));

        assert_eq!(
            resolve(&base, "config/aisle.conf"),
            missing("config/aisle.conf.cook")
        );
        assert_eq!(resolve(&base, "notes.txt"), missing("notes.txt.cook"));
        // A directory is not a recipe either, even under a recipe's name.
        assert_eq!(resolve(&base, "Folder"), missing("Folder.cook"));
    }

    #[test]
    fn hidden_and_escaping_paths_are_refused_before_any_lookup() {
        let (_dir, base) = collection();
        for path in [
            ".git/config",
            ".shopping-list",
            "../Pasta.cook",
            "/etc/passwd",
        ] {
            let (status, _) = recipe_file(&base, path).expect_err(path);
            assert_eq!(status, StatusCode::BAD_REQUEST, "{path:?}");
        }
    }
}
