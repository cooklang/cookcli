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

/// Check if a JWT should be refreshed (less than 1 hour remaining).
pub fn should_refresh_jwt(jwt: &str) -> Result<bool> {
    let claims = decode_jwt_claims(jwt)?;
    let remaining = claims.exp - chrono::Utc::now().timestamp();
    Ok(remaining < 3600)
}
