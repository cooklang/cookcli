//! The users file: who may sign in to `cook server` and make changes.
//!
//! ```toml
//! [users]
//! alice = "$argon2id$v=19$m=19456,t=2,p=1$…"
//! bob = { hash = "$argon2id$…", role = "editor" }
//! ```
//!
//! Each value is an argon2 PHC string, written by `cook server user add`,
//! or a table pairing one with a [`Role`]. A bare hash is an admin, which is
//! what every user was before roles existed.
//! [`Users`] is the validated, read-only view the server checks passwords
//! against; [`UsersDocument`] is the editable one the `user` commands rewrite,
//! built on `toml_edit` so an admin's comments survive every change.

use anyhow::{bail, Context as _, Result};
use argon2::password_hash::PasswordHash;
use camino::{Utf8Path, Utf8PathBuf};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::ffi::OsStr;

pub use crate::web::viewer::Role;

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
    users: BTreeMap<String, toml::Value>,
}

/// The table form of an entry, `bob = { hash = "…", role = "editor" }`.
///
/// Unknown keys are refused, so a typo'd `role` does not quietly leave the
/// user an admin.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryTable {
    hash: String,
    role: Option<String>,
}

/// One user's line in the users file.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    hash: String,
    role: Role,
}

impl Entry {
    /// Reads either form of an entry: a bare hash, which is an admin as every
    /// user was before roles existed, or a table with a hash and a role.
    fn parse(name: &str, value: toml::Value) -> Result<Self> {
        let (hash, role) = match value {
            toml::Value::String(hash) => (hash, None),
            value @ toml::Value::Table(_) => {
                let table: EntryTable = value
                    .try_into()
                    .with_context(|| format!("invalid entry for {name:?}"))?;
                (table.hash, table.role)
            }
            _ => bail!(
                "the entry for {name:?} must be a password hash, or a table with a `hash` \
                 and a `role`"
            ),
        };
        let role = match role {
            Some(role) => role
                .parse()
                .map_err(|err| anyhow::anyhow!("{name:?} has an {err}"))?,
            None => Role::Admin,
        };
        validate_hash(name, &hash)?;
        Ok(Self { hash, role })
    }
}

/// The users who may sign in, each with their password hash and role.
///
/// An empty list is valid: sign-in stays on and nobody can make changes,
/// which leaves the site read-only.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Users {
    entries: BTreeMap<String, Entry>,
}

impl Users {
    /// Parses and validates the users file's contents.
    ///
    /// Anything it does not understand, down to an unknown role, is an error:
    /// the server refuses to start rather than guess what someone meant.
    pub fn parse(text: &str) -> Result<Self> {
        let file: UsersFile = toml::from_str(text)?;
        let mut entries = BTreeMap::new();
        for (name, value) in file.users {
            validate_username(&name)?;
            let entry = Entry::parse(&name, value)?;
            entries.insert(name, entry);
        }
        Ok(Self { entries })
    }

    /// Reads and validates the users file at `path`.
    pub fn load(path: &Utf8Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("could not read {path}"))?;
        Self::parse(&text).with_context(|| format!("invalid users file {path}"))
    }

    /// The password hash of `name`, if that user exists.
    pub fn hash(&self, name: &str) -> Option<&str> {
        self.entries.get(name).map(|entry| entry.hash.as_str())
    }

    /// The role of `name`, if that user exists.
    pub fn role(&self, name: &str) -> Option<Role> {
        self.entries.get(name).map(|entry| entry.role)
    }

    /// Every user and their role, by name.
    pub fn roles(&self) -> impl Iterator<Item = (&str, Role)> {
        self.entries
            .iter()
            .map(|(name, entry)| (name.as_str(), entry.role))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
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
        self.users_table()
            .map(|table| table.iter().map(|(name, _)| name.to_string()).collect())
            .unwrap_or_default()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.users_table()
            .is_some_and(|table| table.contains_key(name))
    }

    /// The users as the server would read them.
    pub fn users(&self) -> Result<Users> {
        Users::parse(&self.doc.to_string())
    }

    /// Adds `name` with `role`, replacing them if they exist.
    ///
    /// An admin is written as a bare hash, which every version of the server
    /// reads; anyone else as `{ hash = "…", role = "…" }`, which a server
    /// from before roles refuses rather than make them an admin.
    pub fn add(&mut self, name: &str, phc: &str, role: Role) {
        let value = if role == Role::Admin {
            toml_edit::Value::from(phc)
        } else {
            let mut table = toml_edit::InlineTable::new();
            table.insert("hash", phc.into());
            table.insert("role", role.as_str().into());
            table.into()
        };
        self.replace(name, value);
    }

    /// Changes the password hash of `name`, who must exist, keeping their
    /// role.
    pub fn set_password(&mut self, name: &str, phc: &str) {
        match self.entry_table_mut(name) {
            Some(table) => {
                table.insert("hash", toml_edit::value(phc));
            }
            None => self.replace(name, phc.into()),
        }
    }

    /// Gives `name`, who must exist, `role`, keeping their password.
    pub fn set_role(&mut self, name: &str, role: Role) -> Result<()> {
        if let Some(table) = self.entry_table_mut(name) {
            table.insert("role", toml_edit::value(role.as_str()));
            return Ok(());
        }
        let hash = self
            .users()?
            .hash(name)
            .with_context(|| format!("there is no user {name}"))?
            .to_owned();
        self.add(name, &hash, role);
        Ok(())
    }

    /// Removes `name`, returning whether they were there.
    pub fn remove(&mut self, name: &str) -> bool {
        self.doc
            .get_mut("users")
            .and_then(toml_edit::Item::as_table_like_mut)
            .is_some_and(|table| table.remove(name).is_some())
    }

    fn users_table(&self) -> Option<&dyn toml_edit::TableLike> {
        self.doc
            .get("users")
            .and_then(toml_edit::Item::as_table_like)
    }

    fn users_table_mut(&mut self) -> Option<&mut dyn toml_edit::TableLike> {
        self.doc
            .entry("users")
            .or_insert_with(|| {
                let mut table = toml_edit::Table::new();
                table.set_implicit(false);
                toml_edit::Item::Table(table)
            })
            .as_table_like_mut()
    }

    /// The entry of `name` when it is written as a table, inline or not.
    fn entry_table_mut(&mut self, name: &str) -> Option<&mut dyn toml_edit::TableLike> {
        self.users_table_mut()?.get_mut(name)?.as_table_like_mut()
    }

    /// Sets the entry of `name` to `value`, keeping any comment beside the
    /// value it replaces.
    fn replace(&mut self, name: &str, mut value: toml_edit::Value) {
        let Some(table) = self.users_table_mut() else {
            return;
        };
        match table.get_mut(name) {
            Some(item) => {
                if let Some(old) = item.as_value() {
                    *value.decor_mut() = old.decor().clone();
                }
                *item = toml_edit::Item::Value(value);
            }
            None => {
                table.insert(name, toml_edit::Item::Value(value));
            }
        }
    }
}

impl std::fmt::Display for UsersDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.doc.fmt(f)
    }
}

/// Replaces the users file with `contents`, creating its directory if needed.
///
/// Atomic, so a running server's reload never reads half a file, and on Unix
/// readable by the owner only: the hashes are an offline password-guessing
/// target.
pub fn write_users_file(path: &Utf8Path, contents: &str) -> Result<()> {
    super::write_private_file(path, contents)
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

        doc.add("bob", HASH, Role::Admin);
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
        doc.add("me@home", HASH, Role::Admin);
        let text = doc.to_string();
        assert!(text.contains("[users]"), "{text}");
        assert_eq!(Users::parse(&text).unwrap().hash("me@home"), Some(HASH));
    }

    #[test]
    fn a_bare_hash_is_an_admin_and_a_table_names_the_role() {
        let users = Users::parse(&format!(
            "[users]\n\
             alice = \"{HASH}\"\n\
             bob = {{ hash = \"{HASH}\", role = \"editor\" }}\n\
             carol = {{ hash = \"{HASH}\" }}\n\
             [users.dave]\nhash = \"{HASH}\"\nrole = \"reader\"\n"
        ))
        .unwrap();
        assert_eq!(users.role("alice"), Some(Role::Admin));
        assert_eq!(users.role("bob"), Some(Role::Editor));
        assert_eq!(users.hash("bob"), Some(HASH));
        assert_eq!(users.role("carol"), Some(Role::Admin));
        assert_eq!(users.role("dave"), Some(Role::Reader));
        assert_eq!(users.role("erin"), None);
        assert_eq!(
            users.roles().collect::<Vec<_>>(),
            [
                ("alice", Role::Admin),
                ("bob", Role::Editor),
                ("carol", Role::Admin),
                ("dave", Role::Reader),
            ]
        );
    }

    #[test]
    fn rejects_entries_it_does_not_understand() {
        let parse = |entry: String| Users::parse(&format!("[users]\nbob = {entry}\n"));

        // An unknown role fails closed, naming the user and the choices.
        let err = parse(format!("{{ hash = \"{HASH}\", role = \"chef\" }}")).unwrap_err();
        let message = format!("{err:#}");
        assert!(message.contains("bob"), "{message}");
        assert!(message.contains("chef"), "{message}");
        assert!(
            message.contains("reader, shopper, editor, admin"),
            "{message}"
        );
        // Role names are exact.
        assert!(parse(format!("{{ hash = \"{HASH}\", role = \"Editor\" }}")).is_err());
        // A typo'd key is not ignored, which would leave bob an admin.
        assert!(parse(format!("{{ hash = \"{HASH}\", rol = \"reader\" }}")).is_err());
        // A table still needs a real hash.
        assert!(parse("{ role = \"reader\" }".to_string()).is_err());
        assert!(parse("{ hash = \"hunter2\", role = \"reader\" }".to_string()).is_err());
        // Neither a string nor a table.
        assert!(parse("42".to_string()).is_err());
    }

    #[test]
    fn document_writes_roles_as_the_server_reads_them() {
        let mut doc = UsersDocument::parse("").unwrap();
        doc.add("alice", HASH, Role::Admin);
        doc.add("bob", HASH, Role::Shopper);
        let text = doc.to_string();
        // An admin stays a bare hash, which older servers read too.
        assert!(text.contains(&format!("alice = \"{HASH}\"")), "{text}");
        let users = doc.users().unwrap();
        assert_eq!(users.role("alice"), Some(Role::Admin));
        assert_eq!(users.role("bob"), Some(Role::Shopper));
    }

    #[test]
    fn document_changes_a_role_and_keeps_the_rest() {
        const OTHER: &str = "$argon2id$v=19$m=64,t=1,p=1$b3RoZXJzYWx0$AAAAAAAAAAAAAAAAAAAAAA";
        let original = format!(
            "[users]\n\
             # the chef\n\
             alice = \"{HASH}\" # owner\n\
             [users.dave]\nhash = \"{HASH}\"\nrole = \"reader\"\n"
        );
        let mut doc = UsersDocument::parse(&original).unwrap();

        // A bare hash becomes a table, keeping its comments and password.
        doc.set_role("alice", Role::Reader).unwrap();
        let text = doc.to_string();
        assert!(text.contains("# the chef"), "{text}");
        assert!(text.contains("# owner"), "{text}");
        let users = doc.users().unwrap();
        assert_eq!(users.role("alice"), Some(Role::Reader));
        assert_eq!(users.hash("alice"), Some(HASH));

        // A new password keeps the role, in either form.
        doc.set_password("alice", OTHER);
        doc.set_password("dave", OTHER);
        let users = doc.users().unwrap();
        assert_eq!(users.hash("alice"), Some(OTHER));
        assert_eq!(users.role("alice"), Some(Role::Reader));
        assert_eq!(users.hash("dave"), Some(OTHER));
        assert_eq!(users.role("dave"), Some(Role::Reader));

        // A table entry keeps its form.
        doc.set_role("dave", Role::Admin).unwrap();
        let text = doc.to_string();
        assert!(text.contains("[users.dave]"), "{text}");
        assert_eq!(doc.users().unwrap().role("dave"), Some(Role::Admin));

        assert!(doc.set_role("erin", Role::Editor).is_err());
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
