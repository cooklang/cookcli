//! The users file: who may sign in to `cook server` and make changes.
//!
//! ```toml
//! [users]
//! alice = "$argon2id$v=19$m=19456,t=2,p=1$…"
//! ```
//!
//! Each value is an argon2 PHC string, written by `cook server user add`.
//! [`Users`] is the validated, read-only view the server checks passwords
//! against; [`UsersDocument`] is the editable one the `user` commands rewrite,
//! built on `toml_edit` so an admin's comments survive every change.

use anyhow::{bail, Context as _, Result};
use argon2::password_hash::PasswordHash;
use camino::{Utf8Path, Utf8PathBuf};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::ffi::OsStr;

/// Environment variable naming the users file, for containers that cannot
/// easily add a flag to the image's command. `--users-file` wins over it.
pub const USERS_FILE_ENV: &str = "COOK_USERS_FILE";

/// Name of the users file in the global configuration directory.
const USERS_FILE_NAME: &str = "users.toml";

/// Longest username accepted. Names end up in cookies and log lines.
const MAX_USERNAME_LEN: usize = 64;

/// Where the users file is, and whether someone asked for that path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsersFileLocation {
    pub path: Utf8PathBuf,
    /// Named by `--users-file` or `COOK_USERS_FILE`. A named file that does
    /// not exist stops the server, where the default location simply means
    /// sign-in is off.
    pub explicit: bool,
}

/// Resolves the users file from the flag, then [`USERS_FILE_ENV`], then the
/// global configuration directory.
pub fn locate(flag: Option<&Utf8Path>) -> Result<UsersFileLocation> {
    locate_in(flag, std::env::var_os(USERS_FILE_ENV).as_deref())
}

/// [`locate`] with the environment value supplied rather than read, so tests
/// need not mutate process-wide state.
fn locate_in(flag: Option<&Utf8Path>, env: Option<&OsStr>) -> Result<UsersFileLocation> {
    if let Some(path) = flag {
        return Ok(UsersFileLocation {
            path: path.to_path_buf(),
            explicit: true,
        });
    }

    // Empty means unset: `COOK_USERS_FILE=${UNDEFINED}` is what a compose file
    // expands an undefined variable to.
    if let Some(value) = env.filter(|value| !value.is_empty()) {
        let path = Utf8Path::from_path(std::path::Path::new(value)).with_context(|| {
            format!("{USERS_FILE_ENV} is not valid utf-8, and cook only supports utf-8 paths")
        })?;
        return Ok(UsersFileLocation {
            path: path.to_path_buf(),
            explicit: true,
        });
    }

    Ok(UsersFileLocation {
        path: cookcli_core::global_config_path(USERS_FILE_NAME)?,
        explicit: false,
    })
}

/// Checks a username against the characters a session cookie and a TOML key
/// can carry without escaping: ASCII letters, digits, `_`, `.`, `@` and `-`.
pub fn validate_username(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > MAX_USERNAME_LEN {
        bail!("a username must be 1 to {MAX_USERNAME_LEN} characters long");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '@' | '-'))
    {
        bail!("invalid username {name:?}: use only letters, digits, and the characters _ . @ -");
    }
    Ok(())
}

/// Checks that `phc` is an argon2 hash this server can verify against.
fn validate_hash(name: &str, phc: &str) -> Result<()> {
    let invalid = || {
        format!(
            "the password for {name:?} is not an argon2 hash; set it with \
             `cook server user passwd {name}`"
        )
    };
    let hash = PasswordHash::new(phc).map_err(|_| anyhow::anyhow!(invalid()))?;
    if !matches!(hash.algorithm.as_str(), "argon2id" | "argon2i" | "argon2d") {
        bail!(invalid());
    }
    argon2::Params::try_from(&hash).map_err(|_| anyhow::anyhow!(invalid()))?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UsersFile {
    #[serde(default)]
    users: BTreeMap<String, String>,
}

/// The users who may sign in, each with their password hash.
///
/// An empty list is valid: sign-in stays on and nobody can make changes,
/// which leaves the site read-only.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Users {
    hashes: BTreeMap<String, String>,
}

impl Users {
    /// Parses and validates the users file's contents.
    pub fn parse(text: &str) -> Result<Self> {
        let file: UsersFile = toml::from_str(text)?;
        for (name, phc) in &file.users {
            validate_username(name)?;
            validate_hash(name, phc)?;
        }
        Ok(Self { hashes: file.users })
    }

    /// Reads and validates the users file at `path`.
    pub fn load(path: &Utf8Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("could not read {path}"))?;
        Self::parse(&text).with_context(|| format!("invalid users file {path}"))
    }

    /// The password hash of `name`, if that user exists.
    pub fn hash(&self, name: &str) -> Option<&str> {
        self.hashes.get(name).map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.hashes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hashes.is_empty()
    }
}

/// The users file as an editable document, keeping everything the admin wrote
/// around the entries.
pub struct UsersDocument {
    doc: toml_edit::DocumentMut,
}

impl UsersDocument {
    /// Parses an existing file; the empty string is a new, empty one.
    ///
    /// The contents must already be a valid users file, so a command never
    /// rewrites one it does not understand.
    pub fn parse(text: &str) -> Result<Self> {
        Users::parse(text)?;
        let doc = text.parse::<toml_edit::DocumentMut>()?;
        Ok(Self { doc })
    }

    pub fn names(&self) -> Vec<String> {
        self.users()
            .map(|table| table.iter().map(|(name, _)| name.to_string()).collect())
            .unwrap_or_default()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.users().is_some_and(|table| table.contains_key(name))
    }

    /// Adds `name`, or replaces their hash if they exist.
    pub fn set(&mut self, name: &str, phc: &str) {
        let users = self.doc.entry("users").or_insert_with(|| {
            let mut table = toml_edit::Table::new();
            table.set_implicit(false);
            toml_edit::Item::Table(table)
        });
        if let Some(table) = users.as_table_like_mut() {
            table.insert(name, toml_edit::value(phc));
        }
    }

    /// Removes `name`, returning whether they were there.
    pub fn remove(&mut self, name: &str) -> bool {
        self.doc
            .get_mut("users")
            .and_then(toml_edit::Item::as_table_like_mut)
            .is_some_and(|table| table.remove(name).is_some())
    }

    fn users(&self) -> Option<&dyn toml_edit::TableLike> {
        self.doc
            .get("users")
            .and_then(toml_edit::Item::as_table_like)
    }
}

impl std::fmt::Display for UsersDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.doc.fmt(f)
    }
}

/// Replaces the users file with `contents`, creating its directory if needed.
///
/// Written to a sibling and renamed into place, so a running server's reload
/// never reads half a file. On Unix only the owner may read it: the hashes
/// are an offline password-guessing target.
pub fn write_users_file(path: &Utf8Path, contents: &str) -> Result<()> {
    use std::io::Write;

    let dir = path
        .parent()
        .filter(|dir| !dir.as_str().is_empty())
        .unwrap_or(Utf8Path::new("."));
    std::fs::create_dir_all(dir).with_context(|| format!("could not create {dir}"))?;

    let file_name = path.file_name().unwrap_or(USERS_FILE_NAME);
    let staging = dir.join(format!(".{file_name}.{}.tmp", std::process::id()));

    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = options
        .open(&staging)
        .and_then(|mut file| {
            file.write_all(contents.as_bytes())?;
            file.sync_all()
        })
        .and_then(|()| {
            crate::server::fs_atomic::rename_replace(staging.as_std_path(), path.as_std_path())
        });
    if result.is_err() {
        let _ = std::fs::remove_file(&staging);
    }
    result.with_context(|| format!("could not write {path}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tiny parameters: verification cost is irrelevant to parsing.
    const HASH: &str = "$argon2id$v=19$m=64,t=1,p=1$c29tZXNhbHQ$1Yq1Ai2xMSJ5ZB1Hm7Q5Rw";

    #[test]
    fn locate_prefers_flag_then_env_then_default() {
        let flag = Utf8Path::new("/srv/users.toml");
        let env = OsStr::new("/env/users.toml");

        let from_flag = locate_in(Some(flag), Some(env)).unwrap();
        assert_eq!(from_flag.path, flag);
        assert!(from_flag.explicit);

        let from_env = locate_in(None, Some(env)).unwrap();
        assert_eq!(from_env.path, "/env/users.toml");
        assert!(from_env.explicit);

        let default = locate_in(None, None).unwrap();
        assert_eq!(default.path.file_name(), Some("users.toml"));
        assert!(!default.explicit);
    }

    #[test]
    fn locate_treats_empty_env_as_unset() {
        let location = locate_in(None, Some(OsStr::new(""))).unwrap();
        assert!(!location.explicit);
    }

    #[test]
    fn usernames() {
        for good in ["alice", "Bob_2", "a.b-c", "me@home", &"x".repeat(64)] {
            assert!(validate_username(good).is_ok(), "{good}");
        }
        for bad in ["", "has space", "colon:name", "tab\t", "é", &"x".repeat(65)] {
            assert!(validate_username(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn parses_users() {
        let users = Users::parse(&format!("[users]\nalice = \"{HASH}\"\n")).unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users.hash("alice"), Some(HASH));
        assert_eq!(users.hash("bob"), None);
    }

    #[test]
    fn empty_file_and_empty_table_mean_no_users() {
        assert!(Users::parse("").unwrap().is_empty());
        assert!(Users::parse("[users]\n").unwrap().is_empty());
    }

    #[test]
    fn rejects_bad_entries() {
        // Not a PHC string at all.
        assert!(Users::parse("[users]\nalice = \"hunter2\"\n").is_err());
        // A PHC string, but not argon2.
        assert!(
            Users::parse("[users]\nalice = \"$pbkdf2-sha256$i=1000$c2FsdA$aGFzaA\"\n").is_err()
        );
        // A bad name.
        assert!(Users::parse(&format!("[users]\n\"a b\" = \"{HASH}\"\n")).is_err());
        // A typo'd table is an error, not a silently empty list.
        assert!(Users::parse(&format!("[user]\nalice = \"{HASH}\"\n")).is_err());
    }

    #[test]
    fn document_edits_keep_comments() {
        let original = format!("# Kitchen crew\n[users]\n# the chef\nalice = \"{HASH}\"\n");
        let mut doc = UsersDocument::parse(&original).unwrap();
        assert_eq!(doc.names(), ["alice"]);

        doc.set("bob", HASH);
        assert!(doc.contains("bob"));
        let text = doc.to_string();
        assert!(text.contains("# Kitchen crew"), "{text}");
        assert!(text.contains("# the chef"), "{text}");
        assert_eq!(Users::parse(&text).unwrap().len(), 2);

        assert!(doc.remove("bob"));
        assert!(!doc.remove("bob"));
        assert_eq!(doc.names(), ["alice"]);
    }

    #[test]
    fn document_starts_from_nothing() {
        let mut doc = UsersDocument::parse("").unwrap();
        assert!(doc.names().is_empty());
        doc.set("me@home", HASH);
        let text = doc.to_string();
        assert!(text.contains("[users]"), "{text}");
        assert_eq!(Users::parse(&text).unwrap().hash("me@home"), Some(HASH));
    }

    #[test]
    fn document_refuses_an_invalid_file() {
        assert!(UsersDocument::parse("[users]\nalice = \"plain\"\n").is_err());
    }

    #[test]
    fn writes_file_and_replaces_it() {
        let dir = tempfile::TempDir::new().unwrap();
        let path =
            Utf8PathBuf::from_path_buf(dir.path().join("nested").join("users.toml")).unwrap();

        write_users_file(&path, "[users]\n").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "[users]\n");

        let contents = format!("[users]\nalice = \"{HASH}\"\n");
        write_users_file(&path, &contents).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), contents);

        let leftovers: Vec<_> = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(
            leftovers.len(),
            1,
            "staging file left behind: {leftovers:?}"
        );
    }
}
