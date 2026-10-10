use axum::{http::StatusCode, Json};
use camino::{Utf8Path, Utf8PathBuf};

pub type ApiError = (StatusCode, Json<serde_json::Value>);

/// The JSON body of an error response. The message goes through
/// [`private_paths::hide`](crate::server::private_paths::hide), so a path in it
/// does not say where the server keeps its files.
pub fn json_error(msg: impl std::fmt::Display) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "error": crate::server::private_paths::hide(&msg.to_string()) }))
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

/// Whether `path` from a request may name a recipe or menu to read: it is a
/// [`is_request_path`], and its extension, if any, is `.cook` or `.menu`.
///
/// `cooklang-find` resolves a name without an extension to its `.cook` or
/// `.menu` file, but opens a name with any other extension as it is, so
/// without the second rule `/recipe/config/pantry.conf` showed the pantry
/// configuration as recipe steps, to `--recipes-only` guests too (#658).
pub fn is_recipe_request(path: &str) -> bool {
    is_request_path(path)
        && Utf8Path::new(path)
            .extension()
            .is_none_or(|ext| RECIPE_FILE_EXTENSIONS.contains(&ext))
}

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
///
/// A file that does not exist yet is refused when a folder or file name in
/// `path` starts or ends with whitespace, as `POST /new` never makes one: a
/// save would otherwise create ` Soup .cook` beside `Soup.cook`. One already
/// on disk under such a name still resolves, so it can be opened and removed.
pub fn recipe_file(base: &Utf8Path, path: &str) -> Result<RecipeFile, ApiError> {
    check_path(path)?;

    let named = base.join(path);
    let file = if named
        .extension()
        .is_some_and(|ext| RECIPE_FILE_EXTENSIONS.contains(&ext))
    {
        if named.is_file() {
            RecipeFile::Existing(named)
        } else {
            RecipeFile::Missing(named)
        }
    } else {
        let [cook, menu] =
            RECIPE_FILE_EXTENSIONS.map(|ext| Utf8PathBuf::from(format!("{named}.{ext}")));
        if cook.is_file() {
            RecipeFile::Existing(cook)
        } else if menu.is_file() {
            RecipeFile::Existing(menu)
        } else {
            RecipeFile::Missing(cook)
        }
    };

    if matches!(file, RecipeFile::Missing(_)) && has_padded_name(path) {
        tracing::error!("Refused to create a file with spaces around a name: {path:?}");
        return Err((
            StatusCode::BAD_REQUEST,
            json_error(format!(
                "Invalid path: {path:?}: a name cannot start or end with a space"
            )),
        ));
    }
    Ok(file)
}

/// Whether a folder or file name in `path` starts or ends with whitespace,
/// leaving out a `.cook` or `.menu` extension: `Soup .cook` is padded too.
fn has_padded_name(path: &str) -> bool {
    path.split('/').any(|segment| {
        let name = RECIPE_FILE_EXTENSIONS
            .iter()
            .find_map(|ext| segment.strip_suffix(ext)?.strip_suffix('.'))
            .unwrap_or(segment);
        name.trim() != name
    })
}

/// Longest new name a rename accepts, in bytes. File systems stop at 255, and
/// the step pictures that move along add up to `.99.99.jpeg` to it.
const MAX_NAME_BYTES: usize = 200;

/// Names Windows keeps for devices, whatever follows the first dot:
/// `CON.cook` opens the console there, not a file. Collections are synced to
/// Windows machines, so they are refused everywhere.
const WINDOWS_DEVICE_NAMES: [&str; 6] = ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"];

/// The stem a `.cook` or `.menu` file of extension `ext` is renamed to, from
/// the name typed for it.
///
/// The name is one file name in the file's own folder, never a path. Typing
/// the file's own extension is allowed and dropped; the other one is refused,
/// since a rename cannot turn a recipe into a menu. Refused besides:
///
/// - path separators, `:` and the other characters Windows does not allow in a
///   file name (`* ? " < > |`), control and invisible formatting characters;
/// - a leading `.`, which would hide the file (see [`is_request_path`]), and a
///   trailing `.` or space, which Windows drops;
/// - Windows device names (`CON`, `COM1`, …);
/// - what could not be written in a recipe reference, since the references to
///   the file are rewritten to the new name: `@ # ~ { } [ ]` would end or
///   split it, `--` would start a comment, and two spaces in a row would be
///   read back as one.
pub fn new_file_name(name: &str, ext: &str) -> Result<String, ApiError> {
    let refuse = |why: &str| {
        Err((
            StatusCode::BAD_REQUEST,
            json_error(format!("Invalid name: {why}")),
        ))
    };

    let name = name.trim();
    let stem = name
        .strip_suffix(ext)
        .and_then(|rest| rest.strip_suffix('.'))
        .unwrap_or(name)
        .trim_end();

    if stem.is_empty() {
        return refuse("it is empty");
    }
    if let Some(other) = RECIPE_FILE_EXTENSIONS
        .iter()
        .find(|other| **other != ext && stem.ends_with(&format!(".{other}")))
    {
        return refuse(&format!("a .{ext} file cannot be renamed to .{other}"));
    }
    if stem.len() > MAX_NAME_BYTES {
        return refuse(&format!("it is longer than {MAX_NAME_BYTES} bytes"));
    }
    if let Some(c) = stem.chars().find(|c| {
        matches!(
            c,
            '/' | '\\'
                | ':'
                | '*'
                | '?'
                | '"'
                | '<'
                | '>'
                | '|'
                | '@'
                | '#'
                | '~'
                | '{'
                | '}'
                | '['
                | ']'
        ) || c.is_control()
            || is_invisible(*c)
    }) {
        return refuse(&format!("it contains {:?}", c));
    }
    if stem.contains("--") {
        return refuse("it contains \"--\"");
    }
    if stem.contains("  ") {
        return refuse("it contains two spaces in a row");
    }
    if stem.starts_with('.') {
        return refuse("it starts with a dot");
    }
    if stem.ends_with('.') {
        return refuse("it ends with a dot");
    }
    if is_windows_device_name(stem) {
        return refuse("it is a name Windows reserves for a device");
    }

    Ok(stem.to_string())
}

/// Zero-width and bidirectional formatting characters: they do not show, so a
/// name holding one looks like another name.
fn is_invisible(c: char) -> bool {
    matches!(
        c,
        '\u{00AD}'
            | '\u{034F}'
            | '\u{200B}'..='\u{200F}'
            | '\u{2028}'..='\u{202E}'
            | '\u{2060}'..='\u{2069}'
            | '\u{FE00}'..='\u{FE0F}'
            | '\u{FEFF}'
    )
}

fn is_windows_device_name(stem: &str) -> bool {
    let device = stem.split('.').next().unwrap_or(stem).trim_end();
    // `COM1` to `COM9`, and `COM¹` to `COM³`, which Windows reserves too.
    let numbered = |prefix: &str| {
        let mut chars = device.chars();
        let start: String = chars.by_ref().take(3).collect();
        start.eq_ignore_ascii_case(prefix)
            && matches!(
                (chars.next(), chars.next()),
                (Some('0'..='9' | '¹' | '²' | '³'), None)
            )
    };
    WINDOWS_DEVICE_NAMES
        .iter()
        .any(|reserved| device.eq_ignore_ascii_case(reserved))
        || numbered("COM")
        || numbered("LPT")
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
    fn a_new_file_cannot_have_spaces_around_a_name() {
        let (_dir, base) = collection();
        for path in [
            " Soup",
            "Soup ",
            "Soup .cook",
            " Week.menu",
            "Folder /Soup",
            " Mains/Soup.cook",
        ] {
            let (status, _) = recipe_file(&base, path).expect_err(path);
            assert_eq!(status, StatusCode::BAD_REQUEST, "{path:?}");
        }
        // Spaces inside a name are fine.
        assert_eq!(
            resolve(&base, "Beef and Beer Stew"),
            RecipeFile::Missing(base.join("Beef and Beer Stew.cook"))
        );
    }

    /// A file made with spaces around its name before they were refused can
    /// still be opened, saved and deleted.
    ///
    /// Leading spaces only: Windows drops trailing ones from a file or folder
    /// name, so ` Stew .cook` cannot be made there to begin with.
    #[test]
    fn an_existing_file_with_spaces_around_its_name_still_resolves() {
        let (_dir, base) = collection();
        std::fs::create_dir_all(base.join(" Mains")).unwrap();
        std::fs::write(base.join(" Mains/ Stew.cook"), "x").unwrap();

        assert_eq!(
            resolve(&base, " Mains/ Stew"),
            RecipeFile::Existing(base.join(" Mains/ Stew.cook"))
        );
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

    fn renamed(name: &str, ext: &str) -> Result<String, String> {
        new_file_name(name, ext).map_err(|(status, body)| {
            assert_eq!(status, StatusCode::BAD_REQUEST, "{name:?}");
            body["error"].as_str().unwrap().to_string()
        })
    }

    #[test]
    fn a_new_name_is_a_plain_file_name() {
        for (name, ext, stem) in [
            ("Noodles", "cook", "Noodles"),
            ("  Noodles  ", "cook", "Noodles"),
            ("Noodles.cook", "cook", "Noodles"),
            ("Week 42.menu", "menu", "Week 42"),
            ("Mr. Smith's Stew", "cook", "Mr. Smith's Stew"),
            ("Crème brûlée (v2)", "cook", "Crème brûlée (v2)"),
            ("Pasta - quick", "cook", "Pasta - quick"),
            ("Console", "cook", "Console"),
            ("COM10", "cook", "COM10"),
            ("COé1", "cook", "COé1"),
        ] {
            assert_eq!(renamed(name, ext).as_deref(), Ok(stem), "{name:?}");
        }
    }

    #[test]
    fn a_new_name_that_could_reach_another_file_is_refused() {
        for (name, ext) in [
            ("", "cook"),
            ("   ", "cook"),
            (".cook", "cook"),
            ("../Pasta", "cook"),
            ("Sub/Pasta", "cook"),
            ("Sub\\Pasta", "cook"),
            ("..", "cook"),
            (".hidden", "cook"),
            ("C:Pasta", "cook"),
            ("Pasta.", "cook"),
            ("Week.menu", "cook"),
            ("Pasta.cook", "menu"),
            ("CON", "cook"),
            ("con.cook", "cook"),
            ("Nul.backup", "cook"),
            ("COM1", "cook"),
            ("lpt9", "menu"),
            ("COM¹", "cook"),
            ("lpt³.notes", "cook"),
            ("Pas\u{2028}ta", "cook"),
            ("Pas\u{FE0F}ta", "cook"),
            ("Pas\nta", "cook"),
            ("Pas\u{202E}ta", "cook"),
            ("Pas\u{200B}ta", "cook"),
            ("Pasta?", "cook"),
            ("Pasta*", "cook"),
            ("Pasta|Alias", "cook"),
            ("Pasta@home", "cook"),
            ("Pasta{2}", "cook"),
            ("Pasta #1", "cook"),
            ("Pasta~", "cook"),
            ("Pasta [v2]", "cook"),
            ("Pasta -- quick", "cook"),
            ("Red  Beans", "cook"),
        ] {
            assert!(renamed(name, ext).is_err(), "{name:?} must be refused");
        }
        assert!(renamed(&"a".repeat(MAX_NAME_BYTES), "cook").is_ok());
        assert!(renamed(&"a".repeat(MAX_NAME_BYTES + 1), "cook").is_err());
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
