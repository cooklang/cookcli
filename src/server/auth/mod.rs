//! Opt-in sign-in for `cook server`.
//!
//! With no users file the server is open, as it always was. Once a users file
//! exists (see [`users`]), anyone can still browse, but every request that
//! changes something needs a signed-in user: recipes, the pantry, the
//! shopping list, the cook.md sync binding, and the editor's language server.
//! [`middleware::middleware`] enforces that for every route in one place.
//!
//! Users are managed on the server only, with `cook server user …`; the
//! browser can do no more than sign in and out.

pub mod cli;
pub mod handlers;
pub mod middleware;
pub mod session;
pub mod users;
mod watcher;

use crate::web::viewer::Viewer;
use anyhow::{bail, Context as _, Result};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::http::HeaderMap;
use camino::{Utf8Path, Utf8PathBuf};
use session::{KeyFileError, SessionKey};
use std::sync::{Arc, OnceLock, RwLock};
use users::Users;

/// Longest password accepted, so a sign-in attempt cannot make the server
/// hash megabytes.
pub const MAX_PASSWORD_LEN: usize = 1024;

/// How many password checks may run at once. Each argon2 verification holds
/// about 19 MiB for a noticeable fraction of a second; unbounded, a burst of
/// sign-in attempts would exhaust memory rather than merely queue.
const CONCURRENT_VERIFICATIONS: usize = 2;

/// Sign-in state for a server that has a users file.
pub struct Auth {
    users_path: Utf8PathBuf,
    users: RwLock<Arc<Users>>,
    key: SessionKey,
    verifications: tokio::sync::Semaphore,
}

impl Auth {
    fn new(users_path: Utf8PathBuf, users: Users, key: SessionKey) -> Self {
        Self {
            users_path,
            users: RwLock::new(Arc::new(users)),
            key,
            verifications: tokio::sync::Semaphore::new(CONCURRENT_VERIFICATIONS),
        }
    }

    /// The users file this server reads.
    pub fn users_path(&self) -> &Utf8Path {
        &self.users_path
    }

    /// The current list of users.
    pub fn users(&self) -> Arc<Users> {
        let users = self.users.read().unwrap_or_else(|e| e.into_inner());
        Arc::clone(&*users)
    }

    /// Swaps in a reloaded list. Cookies of removed users, and of users whose
    /// password changed, stop verifying from this point on.
    fn replace_users(&self, users: Users) {
        *self.users.write().unwrap_or_else(|e| e.into_inner()) = Arc::new(users);
    }

    /// Who sent this request: a signed-in user if it carries a valid session
    /// cookie, a guest otherwise.
    pub fn viewer(&self, headers: &HeaderMap) -> Viewer {
        let users = self.users();
        session::read_cookie(headers)
            .and_then(|value| {
                self.key
                    .verify(value, session::now(), |name| users.hash(name))
            })
            .map_or_else(Viewer::guest, Viewer::signed_in)
    }

    /// Whether `password` is `name`'s password.
    ///
    /// An unknown name is checked against a stand-in hash, so a wrong
    /// username takes as long to refuse as a wrong password and the timing
    /// does not reveal who has an account.
    pub async fn check_password(&self, name: &str, password: String) -> bool {
        if password.len() > MAX_PASSWORD_LEN {
            return false;
        }
        let hash = self.users().hash(name).map(str::to_owned);
        let Ok(_permit) = self.verifications.acquire().await else {
            return false;
        };
        tokio::task::spawn_blocking(move || match hash {
            Some(hash) => verify_password(&password, &hash),
            None => {
                verify_password(&password, stand_in_hash());
                false
            }
        })
        .await
        .unwrap_or(false)
    }

    /// A session cookie value for `name`, or `None` if they no longer exist.
    pub fn issue_session(&self, name: &str) -> Option<String> {
        let users = self.users();
        let hash = users.hash(name)?;
        Some(
            self.key
                .sign(name, session::now() + session::SESSION_SECONDS, hash),
        )
    }
}

/// Hashes `password` for the users file, with argon2's default parameters
/// and a fresh random salt.
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| anyhow::anyhow!("could not hash the password: {e}"))
}

fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).is_ok_and(|hash| {
        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
    })
}

/// A hash no password matches in practice, made with the same default
/// parameters as the ones `cook server user add` writes.
fn stand_in_hash() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| {
        let mut secret = [0u8; 32];
        argon2::password_hash::rand_core::RngCore::fill_bytes(
            &mut argon2::password_hash::rand_core::OsRng,
            &mut secret,
        );
        hash_password(&format!("{secret:?}")).unwrap_or_default()
    })
}

/// Sets up sign-in for a server rooted at `base_path`, or returns `None` when
/// there is no users file and the server stays open.
///
/// Anything wrong with a users file that does exist stops the server: an
/// administrator who asked for sign-in must not end up with an open server
/// because of a typo.
pub fn setup(users_file: Option<&Utf8Path>, base_path: &Utf8Path) -> Result<Option<Arc<Auth>>> {
    let location = users::locate(users_file)?;
    if !location.path.exists() {
        if location.explicit {
            bail!(
                "the users file {} does not exist. Create it with `cook server user add <name>`, \
                 or unset {} / --users-file to run without sign-in.",
                location.path,
                users::USERS_FILE_ENV
            );
        }
        return Ok(None);
    }

    // Absolute, so the watcher has a directory to watch, but not canonical:
    // on Windows that would print as `\\?\C:\…` in every message.
    let users_path = std::path::absolute(location.path.as_std_path())
        .ok()
        .and_then(|path| Utf8PathBuf::from_path_buf(path).ok())
        .with_context(|| format!("could not resolve {}", location.path))?;
    refuse_inside(base_path, &users_path, "The users file")?;
    let users = Users::load(&users_path)?;

    let key = match session::secret_path() {
        Ok(path) => {
            refuse_inside(base_path, &path, "The session key")?;
            match SessionKey::load_or_create(&path) {
                Ok(key) => key,
                Err(KeyFileError::Malformed(message)) => bail!(message),
                Err(KeyFileError::Io(err)) => {
                    tracing::warn!(
                        "could not read or create the session key {path}: {err}. \
                         Using a temporary one: everyone will have to sign in again \
                         after a restart."
                    );
                    SessionKey::random()
                }
            }
        }
        Err(err) => {
            tracing::warn!("{err:#}. Using a temporary session key.");
            SessionKey::random()
        }
    };

    let auth = Arc::new(Auth::new(users_path, users, key));
    if let Err(err) = watcher::spawn(Arc::clone(&auth)) {
        tracing::warn!(
            "could not watch {} for changes ({err:#}); restart the server after editing it",
            auth.users_path
        );
    }
    Ok(Some(auth))
}

/// Refuses a file that `/api/static` would hand to anyone who asks.
///
/// That route serves everything under the recipe directory, dotfiles
/// included, so password hashes or the session key stored there would be
/// public, and the key would let anyone forge a sign-in.
fn refuse_inside(base_path: &Utf8Path, path: &Utf8Path, what: &str) -> Result<()> {
    let base = base_path
        .canonicalize_utf8()
        .unwrap_or_else(|_| base_path.to_path_buf());
    if canonical_or_nearest(path).starts_with(&base) {
        bail!(
            "{what} {path} is inside the recipe directory {base_path}, which the server \
             publishes. Keep it elsewhere: set {} to another directory, or pass --users-file \
             for the users file.",
            cookcli_core::CONFIG_DIR_ENV
        );
    }
    Ok(())
}

/// `path` canonicalized, or, when it does not exist yet, its nearest existing
/// ancestor canonicalized with the rest appended.
fn canonical_or_nearest(path: &Utf8Path) -> Utf8PathBuf {
    let mut missing = Vec::new();
    let mut current = path;
    loop {
        if let Ok(canonical) = current.canonicalize_utf8() {
            return missing
                .iter()
                .rev()
                .fold(canonical, |acc: Utf8PathBuf, part| acc.join(part));
        }
        match (current.parent(), current.file_name()) {
            (Some(parent), Some(name)) => {
                missing.push(name);
                current = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_verify() {
        let hash = hash_password("correct horse").unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password("correct horse", &hash));
        assert!(!verify_password("wrong", &hash));
        assert!(!verify_password("correct horse", "not a hash"));
    }

    #[test]
    fn refuses_files_inside_the_recipe_directory() {
        let dir = tempfile::TempDir::new().unwrap();
        let base = Utf8PathBuf::from_path_buf(dir.path().join("recipes")).unwrap();
        std::fs::create_dir_all(base.join(".cook-config")).unwrap();
        let outside = Utf8PathBuf::from_path_buf(dir.path().join("config")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();

        // Existing and not-yet-created files alike.
        assert!(refuse_inside(&base, &base.join(".cook-config").join("auth-secret"), "x").is_err());
        assert!(refuse_inside(&base, &base.join("new").join("users.toml"), "x").is_err());
        assert!(refuse_inside(&base, &outside.join("users.toml"), "x").is_ok());
    }
}
