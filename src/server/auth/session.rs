//! Session cookies: `cook_session=<user>:<expiry>:<mac>`.
//!
//! Stateless on purpose: the server keeps no session table, so there is
//! nothing to store, expire or lose on restart. The MAC is an HMAC-SHA256 over
//! the user, the expiry and that user's current password hash, keyed by a
//! secret kept in the configuration directory. Changing a password or removing
//! a user therefore invalidates every cookie issued to them, and deleting the
//! key file signs everyone out.

use anyhow::{Context as _, Result};
use argon2::password_hash::rand_core::{OsRng, RngCore};
use axum::http::{header, HeaderMap};
use camino::Utf8Path;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Name of the session cookie.
pub const COOKIE_NAME: &str = "cook_session";

/// How long a sign-in lasts: 30 days, in seconds.
pub const SESSION_SECONDS: u64 = 30 * 24 * 60 * 60;

/// Name of the key file in the global configuration directory.
pub const SECRET_FILE_NAME: &str = "auth-secret";

/// Domain separator, so this MAC can never be mistaken for another use of
/// the same key.
const MAC_CONTEXT: &[u8] = b"cook-session-v1";

/// The key session cookies are signed with.
pub struct SessionKey([u8; 32]);

/// Why the key file could not be used.
#[derive(Debug)]
pub enum KeyFileError {
    /// The file exists but does not hold a key. Starting with a fresh key
    /// would hide that, so the server stops instead.
    Malformed(String),
    /// The file could not be read or created, e.g. a read-only filesystem.
    Io(std::io::Error),
}

impl std::fmt::Display for KeyFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed(message) => f.write_str(message),
            Self::Io(err) => err.fmt(f),
        }
    }
}

impl SessionKey {
    /// A key that lives only as long as this process.
    pub fn random() -> Self {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        Self(key)
    }

    /// Reads the key at `path`, creating it on first use.
    ///
    /// Created with `create_new`, so two servers starting at once cannot both
    /// write one; the loser reads the winner's. On Unix only the owner may
    /// read it, since anyone holding it can sign cookies.
    pub fn load_or_create(path: &Utf8Path) -> Result<Self, KeyFileError> {
        use std::io::Write;

        if let Some(dir) = path.parent().filter(|dir| !dir.as_str().is_empty()) {
            std::fs::create_dir_all(dir).map_err(KeyFileError::Io)?;
        }

        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(path) {
            Ok(mut file) => {
                let key = Self::random();
                file.write_all(format!("{}\n", to_hex(&key.0)).as_bytes())
                    .and_then(|()| file.sync_all())
                    .map_err(KeyFileError::Io)?;
                Ok(key)
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                let text = std::fs::read_to_string(path).map_err(KeyFileError::Io)?;
                Self::from_hex(text.trim()).ok_or_else(|| {
                    KeyFileError::Malformed(format!(
                        "{path} does not hold a session key. Delete it and restart to create \
                         a new one; everyone will have to sign in again."
                    ))
                })
            }
            Err(err) => Err(KeyFileError::Io(err)),
        }
    }

    fn from_hex(text: &str) -> Option<Self> {
        from_hex(text)?.try_into().ok().map(Self)
    }

    fn mac(&self, user: &str, expiry: u64, password_hash: &str) -> HmacSha256 {
        let mut mac = HmacSha256::new_from_slice(&self.0).expect("HMAC accepts any key length");
        for part in [
            MAC_CONTEXT,
            user.as_bytes(),
            expiry.to_string().as_bytes(),
            password_hash.as_bytes(),
        ] {
            mac.update(part);
            mac.update(b"\0");
        }
        mac
    }

    /// The cookie value for `user`, valid until `expiry` (Unix seconds).
    pub fn sign(&self, user: &str, expiry: u64, password_hash: &str) -> String {
        let tag = self
            .mac(user, expiry, password_hash)
            .finalize()
            .into_bytes();
        format!("{user}:{expiry}:{}", to_hex(&tag))
    }

    /// The user a cookie value was issued to, if it is genuine, unexpired, and
    /// `password_hash` still returns the hash it was signed with.
    pub fn verify<'v, 'h>(
        &self,
        value: &'v str,
        now: u64,
        password_hash: impl FnOnce(&str) -> Option<&'h str>,
    ) -> Option<&'v str> {
        // Usernames cannot contain ':', so splitting from the right is exact.
        let mut parts = value.rsplitn(3, ':');
        let tag = from_hex(parts.next()?)?;
        let expiry_text = parts.next()?;
        let user = parts.next()?;

        let expiry: u64 = expiry_text.parse().ok()?;
        // Reject "007" and the like: the MAC covers the canonical rendering.
        if expiry.to_string() != expiry_text || expiry <= now {
            return None;
        }
        let hash = password_hash(user)?;
        self.mac(user, expiry, hash).verify_slice(&tag).ok()?;
        Some(user)
    }
}

/// Seconds since the Unix epoch.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default()
}

/// The session cookie's value among the request's cookies.
pub fn read_cookie(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .find_map(|pair| {
            let (name, value) = pair.trim().split_once('=')?;
            (name == COOKIE_NAME && !value.is_empty()).then_some(value)
        })
}

/// The `Path` for the session cookie: the URL prefix itself, or `/`.
///
/// Not `{prefix}/`: the server's home page under `--url-prefix /cook` is
/// `/cook`, which a cookie scoped to `/cook/` is not sent to.
pub fn cookie_path(url_prefix: &str) -> &str {
    if url_prefix.is_empty() {
        "/"
    } else {
        url_prefix
    }
}

/// `Set-Cookie` value that signs the browser in.
pub fn session_cookie(value: &str, url_prefix: &str, secure: bool) -> String {
    format!(
        "{COOKIE_NAME}={value}; Path={}; Max-Age={SESSION_SECONDS}; HttpOnly; SameSite=Lax{}",
        cookie_path(url_prefix),
        if secure { "; Secure" } else { "" }
    )
}

/// `Set-Cookie` value that signs the browser out.
pub fn clear_cookie(url_prefix: &str, secure: bool) -> String {
    format!(
        "{COOKIE_NAME}=; Path={}; Max-Age=0; HttpOnly; SameSite=Lax{}",
        cookie_path(url_prefix),
        if secure { "; Secure" } else { "" }
    )
}

/// The key file's location in the global configuration directory.
pub fn secret_path() -> Result<camino::Utf8PathBuf> {
    cookcli_core::global_config_path(SECRET_FILE_NAME)
        .context("could not locate the configuration directory for the session key")
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn from_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) || !text.is_ascii() {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    const HASH: &str = "$argon2id$v=19$m=64,t=1,p=1$c29tZXNhbHQ$1Yq1Ai2xMSJ5ZB1Hm7Q5Rw";
    const OTHER_HASH: &str = "$argon2id$v=19$m=64,t=1,p=1$b3RoZXJzYWx0$1Yq1Ai2xMSJ5ZB1Hm7Q5Rw";

    fn lookup(user: &str) -> Option<&'static str> {
        (user == "alice").then_some(HASH)
    }

    #[test]
    fn signed_cookie_verifies() {
        let key = SessionKey::random();
        let value = key.sign("alice", 2_000, HASH);
        assert_eq!(key.verify(&value, 1_000, lookup), Some("alice"));
    }

    #[test]
    fn rejects_expired_cookie() {
        let key = SessionKey::random();
        let value = key.sign("alice", 2_000, HASH);
        assert_eq!(key.verify(&value, 2_000, lookup), None);
    }

    #[test]
    fn rejects_tampering() {
        let key = SessionKey::random();
        let value = key.sign("alice", 2_000, HASH);

        let later = value.replacen(":2000:", ":9000:", 1);
        assert_eq!(key.verify(&later, 1_000, lookup), None);

        let padded = value.replacen(":2000:", ":02000:", 1);
        assert_eq!(key.verify(&padded, 1_000, lookup), None);

        let mut flipped = value.clone();
        let last = flipped.pop().unwrap();
        flipped.push(if last == '0' { '1' } else { '0' });
        assert_eq!(key.verify(&flipped, 1_000, lookup), None);

        for garbage in ["", "alice", "alice:2000", "alice:x:00", ":2000:00"] {
            assert_eq!(key.verify(garbage, 1_000, lookup), None, "{garbage}");
        }
    }

    #[test]
    fn rejects_other_key() {
        let value = SessionKey::random().sign("alice", 2_000, HASH);
        assert_eq!(SessionKey::random().verify(&value, 1_000, lookup), None);
    }

    #[test]
    fn password_change_and_removal_invalidate() {
        let key = SessionKey::random();
        let value = key.sign("alice", 2_000, HASH);
        assert_eq!(key.verify(&value, 1_000, |_| Some(OTHER_HASH)), None);
        assert_eq!(key.verify(&value, 1_000, |_| None), None);
    }

    #[test]
    fn key_file_round_trips() {
        let dir = tempfile::TempDir::new().unwrap();
        let path =
            camino::Utf8PathBuf::from_path_buf(dir.path().join("cfg").join("auth-secret")).unwrap();

        let created = SessionKey::load_or_create(&path).unwrap();
        let loaded = SessionKey::load_or_create(&path).unwrap();
        let value = created.sign("alice", 2_000, HASH);
        assert_eq!(loaded.verify(&value, 1_000, lookup), Some("alice"));
    }

    #[test]
    fn malformed_key_file_is_an_error() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = camino::Utf8PathBuf::from_path_buf(dir.path().join("auth-secret")).unwrap();
        std::fs::write(&path, "not a key\n").unwrap();
        assert!(matches!(
            SessionKey::load_or_create(&path),
            Err(KeyFileError::Malformed(_))
        ));
    }

    #[test]
    fn reads_cookie_among_others() {
        let mut headers = HeaderMap::new();
        headers.append(header::COOKIE, HeaderValue::from_static("lang=fr-FR"));
        headers.append(
            header::COOKIE,
            HeaderValue::from_static("show_pantry=1; cook_session=alice:1:ab"),
        );
        assert_eq!(read_cookie(&headers), Some("alice:1:ab"));

        let mut empty = HeaderMap::new();
        empty.insert(header::COOKIE, HeaderValue::from_static("cook_session="));
        assert_eq!(read_cookie(&empty), None);
        assert_eq!(read_cookie(&HeaderMap::new()), None);
    }

    #[test]
    fn cookie_attributes() {
        let cookie = session_cookie("v", "", false);
        assert!(cookie.starts_with("cook_session=v; Path=/;"), "{cookie}");
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.contains(&format!("Max-Age={SESSION_SECONDS}")));
        assert!(!cookie.contains("Secure"));

        let prefixed = session_cookie("v", "/cook", true);
        assert!(prefixed.contains("Path=/cook;"), "{prefixed}");
        assert!(prefixed.ends_with("; Secure"), "{prefixed}");

        assert!(clear_cookie("/cook", false).contains("Max-Age=0"));
    }
}
