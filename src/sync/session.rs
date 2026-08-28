use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSession {
    pub jwt: String,
    pub user_id: String,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum UserId {
    Integer(i64),
    String(String),
}

#[derive(Debug, Deserialize)]
struct JwtClaims {
    uid: UserId,
    exp: i64,
    email: Option<String>,
    /// Unix timestamp through which the account's sync entitlement is
    /// active. Only present on tokens minted after the sync-enforcement
    /// rollout for accounts that currently have the entitlement; absent
    /// entirely on older tokens and on free-tier accounts. Presence (not
    /// value) is what callers care about — see `jwt_lacks_sync_until_claim`.
    #[serde(default)]
    sync_until: Option<i64>,
}

impl SyncSession {
    /// Create a new session from a raw JWT string.
    pub fn from_jwt(jwt: String) -> Result<Self> {
        let claims = decode_jwt_claims(&jwt)?;
        let user_id = match claims.uid {
            UserId::Integer(id) => id.to_string(),
            UserId::String(id) => id,
        };
        Ok(SyncSession {
            jwt,
            user_id,
            email: claims.email,
        })
    }

    /// Load session from the config file, returning None if not found or expired.
    pub fn load(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path).context("Failed to read session file")?;
        let session: SyncSession =
            serde_json::from_str(&content).context("Failed to parse session file")?;
        if is_jwt_expired(&session.jwt)? {
            tracing::info!("Session expired, removing");
            let _ = std::fs::remove_file(path);
            return Ok(None);
        }
        Ok(Some(session))
    }

    /// Save session to the config file.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create config directory")?;
        }
        let content = serde_json::to_string_pretty(self)?;

        // Write with restricted permissions from the start (JWT is a bearer token)
        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(path)
                .context("Failed to create session file")?;
            file.write_all(content.as_bytes())
                .context("Failed to write session file")?;
        }

        #[cfg(not(unix))]
        {
            std::fs::write(path, &content).context("Failed to write session file")?;
        }

        Ok(())
    }

    /// Delete session file.
    pub fn delete(path: &Path) -> Result<()> {
        if path.exists() {
            std::fs::remove_file(path).context("Failed to delete session file")?;
        }
        Ok(())
    }
}

/// Decode JWT payload without signature verification.
/// This is intentional: the JWT comes directly from our auth server over HTTPS,
/// so cryptographic verification is unnecessary for client-side session management.
fn decode_jwt_claims(jwt: &str) -> Result<JwtClaims> {
    let parts: Vec<&str> = jwt.split('.').collect();
    anyhow::ensure!(parts.len() == 3, "Invalid JWT format");
    let decoded = general_purpose::STANDARD_NO_PAD
        .decode(parts[1])
        .or_else(|_| general_purpose::URL_SAFE_NO_PAD.decode(parts[1]))
        .context("Failed to base64-decode JWT payload")?;
    serde_json::from_slice(&decoded).context("Failed to parse JWT claims")
}

fn is_jwt_expired(jwt: &str) -> Result<bool> {
    let claims = decode_jwt_claims(jwt)?;
    Ok(claims.exp <= chrono::Utc::now().timestamp())
}

/// Seconds remaining until the JWT expires. Negative if it has already
/// expired.
pub fn jwt_expires_in(jwt: &str) -> Result<i64> {
    let claims = decode_jwt_claims(jwt)?;
    Ok(claims.exp - chrono::Utc::now().timestamp())
}

/// True if the JWT's payload has no `sync_until` claim at all.
///
/// The server only started minting this claim once the sync-enforcement
/// rollout shipped, and only for accounts with an active sync entitlement.
/// A token predating the rollout lacks the claim entirely, and so does a
/// perfectly healthy free-tier token — this function can't distinguish the
/// two, which is why callers must renew *at most once per process* on this
/// signal (see `runner::start_token_refresh`): a paid account's stale token
/// gets healed, and a free account's token is renewed once and then left
/// alone.
pub fn jwt_lacks_sync_until_claim(jwt: &str) -> Result<bool> {
    let claims = decode_jwt_claims(jwt)?;
    Ok(claims.sync_until.is_none())
}

/// Test-only JWT building, shared with other modules' test code (e.g.
/// `runner::tests`) that need a well-formed token but don't care about its
/// claims beyond `uid`/`exp`.
#[cfg(test)]
pub(crate) mod tests_support {
    use super::*;

    /// Builds an unsigned test JWT carrying only `uid`/`exp`/`email: null`
    /// (no `sync_until`), using the same base64url-no-pad encoding the
    /// server uses (and that `decode_jwt_claims` accepts).
    pub(crate) fn make_test_jwt(user_id: &str, exp: i64) -> String {
        let header = general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#);
        let payload = general_purpose::URL_SAFE_NO_PAD.encode(
            serde_json::json!({ "uid": user_id, "exp": exp, "email": serde_json::Value::Null })
                .to_string(),
        );
        format!("{header}.{payload}.signature")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Builds an unsigned test JWT with the given payload, using the same
    /// base64url-no-pad encoding the server uses (and that `decode_jwt_claims`
    /// accepts).
    fn make_jwt(payload: serde_json::Value) -> String {
        let header = general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#);
        let payload = general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string());
        format!("{header}.{payload}.signature")
    }

    fn future_exp() -> i64 {
        chrono::Utc::now().timestamp() + 100 * 24 * 3600 // 100 days out
    }

    #[test]
    fn jwt_without_sync_until_is_detected_as_missing() {
        let jwt = make_jwt(json!({
            "uid": 42,
            "exp": future_exp(),
            "email": "user@example.com",
        }));
        assert!(jwt_lacks_sync_until_claim(&jwt).unwrap());
    }

    #[test]
    fn jwt_with_sync_until_is_detected_as_present() {
        let jwt = make_jwt(json!({
            "uid": 42,
            "exp": future_exp(),
            "email": "user@example.com",
            "sync_until": future_exp(),
        }));
        assert!(!jwt_lacks_sync_until_claim(&jwt).unwrap());
    }

    #[test]
    fn jwt_with_null_sync_until_is_treated_as_missing() {
        let jwt = make_jwt(json!({
            "uid": 42,
            "exp": future_exp(),
            "email": "user@example.com",
            "sync_until": null,
        }));
        assert!(jwt_lacks_sync_until_claim(&jwt).unwrap());
    }

    #[test]
    fn jwt_expires_in_reflects_remaining_lifetime() {
        let exp = chrono::Utc::now().timestamp() + 3600;
        let jwt = make_jwt(json!({
            "uid": 1,
            "exp": exp,
            "email": null,
        }));
        let remaining = jwt_expires_in(&jwt).unwrap();
        // Allow a little slack for the time elapsed during the test itself.
        assert!((3595..=3600).contains(&remaining), "remaining={remaining}");
    }

    #[test]
    fn string_user_id_is_supported() {
        let jwt = make_jwt(json!({
            "uid": "abc-123",
            "exp": future_exp(),
            "email": null,
        }));
        let session = SyncSession::from_jwt(jwt).unwrap();
        assert_eq!(session.user_id, "abc-123");
        assert!(jwt_lacks_sync_until_claim(&session.jwt).unwrap());
    }
}
