//! Sessions signed out before they expired.
//!
//! Session cookies are stateless, so clearing one in the browser that signs
//! out would leave every copy of it valid for the rest of its 30 days: a
//! cookie copied off a shared computer, or out of a proxy log, would keep
//! working. Signing out therefore records the session's id here, and the
//! server refuses that session from then on, restarts included.
//!
//! Only genuine, unexpired sessions are recorded, and each entry is dropped
//! once its cookie would have expired anyway, so the file holds no more than
//! the sign-outs of the last 30 days.

use anyhow::{bail, Context as _, Result};
use camino::{Utf8Path, Utf8PathBuf};
use std::collections::HashMap;
use std::sync::{Mutex, RwLock};

/// Name of the file, beside the session key in the configuration directory.
pub const REVOKED_FILE_NAME: &str = "revoked-sessions";

const HEADER: &str = "# Sessions signed out before they expired, one per line: <expiry> <id>.\n\
                      # Written by `cook server`; each line is dropped once it expires.\n";

/// The signed-out sessions, by id, with the time their cookie expires.
pub struct Revoked {
    /// Where the list is kept, or `None` when it lives only in memory
    /// because the session key does too.
    path: Option<Utf8PathBuf>,
    /// Read on every request, so never held across a file write.
    ids: RwLock<HashMap<String, u64>>,
    /// Held while the file is written, so two sign-outs at once cannot save
    /// lists that each miss the other's entry.
    writing: Mutex<()>,
}

impl Revoked {
    /// A list that is not kept on disk. Used with a temporary session key,
    /// which no cookie outlives the process with anyway.
    pub fn in_memory() -> Self {
        Self {
            path: None,
            ids: RwLock::new(HashMap::new()),
            writing: Mutex::new(()),
        }
    }

    /// Reads the list at `path`, which may not exist yet.
    ///
    /// A file that cannot be read stops the server: starting with an empty
    /// list would quietly bring every signed-out session back.
    pub fn load(path: Utf8PathBuf, now: u64) -> Result<Self> {
        let ids = match std::fs::read_to_string(&path) {
            Ok(text) => parse(&text, now).with_context(|| {
                format!(
                    "{path} is not a list of signed-out sessions. Delete it together with {} \
                     and restart; everyone will have to sign in again.",
                    super::session::SECRET_FILE_NAME
                )
            })?,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => HashMap::new(),
            Err(err) => return Err(err).with_context(|| format!("could not read {path}")),
        };
        Ok(Self {
            path: Some(path),
            ids: RwLock::new(ids),
            writing: Mutex::new(()),
        })
    }

    /// Whether the session `id` was signed out.
    pub fn contains(&self, id: &str) -> bool {
        self.ids
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(id)
    }

    /// Signs out the session `id`, whose cookie expires at `expiry`, and
    /// drops the entries that have expired since.
    ///
    /// The session is refused as soon as this returns. If the file cannot be
    /// written, it stays signed out only until the server restarts.
    pub fn insert(&self, id: &str, expiry: u64, now: u64) {
        let _writing = self.writing.lock().unwrap_or_else(|e| e.into_inner());
        let contents = {
            let mut ids = self.ids.write().unwrap_or_else(|e| e.into_inner());
            ids.retain(|_, entry_expiry| *entry_expiry > now);
            ids.insert(id.to_string(), expiry);
            render(&ids)
        };

        if let Some(path) = &self.path {
            if let Err(err) = super::write_private_file(path, &contents) {
                tracing::warn!(
                    "{err:#}; the session stays signed out only until the server restarts"
                );
            }
        }
    }
}

fn parse(text: &str, now: u64) -> Result<HashMap<String, u64>> {
    let mut ids = HashMap::new();
    for (number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((expiry, id)) = line
            .split_once(' ')
            .and_then(|(expiry, id)| Some((expiry.parse::<u64>().ok()?, id)))
            .filter(|(_, id)| !id.is_empty() && id.bytes().all(|b| b.is_ascii_hexdigit()))
        else {
            bail!("line {} is not `<expiry> <id>`", number + 1);
        };
        if expiry > now {
            ids.insert(id.to_string(), expiry);
        }
    }
    Ok(ids)
}

fn render(ids: &HashMap<String, u64>) -> String {
    let mut entries: Vec<_> = ids.iter().collect();
    entries.sort_by(|a, b| a.1.cmp(b.1).then_with(|| a.0.cmp(b.0)));
    let mut text = String::from(HEADER);
    for (id, expiry) in entries {
        text.push_str(&format!("{expiry} {id}\n"));
    }
    text
}

/// The file's location beside the session key at `key_path`.
pub fn path_beside(key_path: &Utf8Path) -> Utf8PathBuf {
    key_path.with_file_name(REVOKED_FILE_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path() -> (tempfile::TempDir, Utf8PathBuf) {
        let dir = tempfile::TempDir::new().unwrap();
        let path = Utf8PathBuf::from_path_buf(dir.path().join(REVOKED_FILE_NAME)).unwrap();
        (dir, path)
    }

    #[test]
    fn a_missing_file_is_an_empty_list() {
        let (_dir, path) = temp_path();
        let revoked = Revoked::load(path.clone(), 1_000).unwrap();
        assert!(!revoked.contains("ab"));
        assert!(!path.exists());
    }

    #[test]
    fn sign_outs_survive_a_reload() {
        let (_dir, path) = temp_path();
        Revoked::load(path.clone(), 1_000)
            .unwrap()
            .insert("ab01", 2_000, 1_000);

        let reloaded = Revoked::load(path.clone(), 1_500).unwrap();
        assert!(reloaded.contains("ab01"));
        assert!(!reloaded.contains("cd02"));

        // Past its expiry the cookie is refused anyway, so the entry goes.
        assert!(!Revoked::load(path, 2_000).unwrap().contains("ab01"));
    }

    #[test]
    fn expired_entries_are_dropped_on_the_next_sign_out() {
        let (_dir, path) = temp_path();
        let revoked = Revoked::load(path.clone(), 1_000).unwrap();
        revoked.insert("ab01", 2_000, 1_000);
        revoked.insert("cd02", 5_000, 3_000);

        assert!(!revoked.contains("ab01"));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("ab01"), "{text}");
        assert!(text.contains("5000 cd02"), "{text}");
    }

    #[test]
    fn a_malformed_file_is_an_error() {
        for text in ["not a list\n", "2000\n", "2000 not-hex\n", "soon ab01\n"] {
            assert!(parse(text, 1_000).is_err(), "{text:?}");
        }
        let (_dir, path) = temp_path();
        std::fs::write(&path, "garbage\n").unwrap();
        assert!(Revoked::load(path, 1_000).is_err());
    }

    #[test]
    fn in_memory_lists_write_nothing() {
        let revoked = Revoked::in_memory();
        revoked.insert("ab01", 2_000, 1_000);
        assert!(revoked.contains("ab01"));
    }
}
