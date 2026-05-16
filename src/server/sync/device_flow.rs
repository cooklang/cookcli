#![allow(dead_code)]

use std::time::{Duration, Instant};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use super::endpoints;

const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Serialize)]
struct DeviceCodeRequest<'a> {
    client_name: &'a str,
}

#[derive(Debug, Serialize)]
struct TokenRequest<'a> {
    grant_type: &'a str,
    device_code: &'a str,
}

#[derive(Debug, Deserialize)]
struct TokenSuccess {
    token: String,
}

#[derive(Debug, Deserialize)]
struct TokenError {
    error: String,
}

#[derive(Debug, thiserror::Error)]
pub enum DeviceFlowError {
    #[error("user denied authorization")]
    AccessDenied,
    #[error("device code expired")]
    Expired,
    #[error("flow cancelled")]
    Cancelled,
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("bad response from cook.md: {0}")]
    BadResponse(String),
}

pub async fn request_device_code(
    client: &reqwest::Client,
    client_name: &str,
) -> anyhow::Result<DeviceCodeResponse> {
    let url = format!("{}/oauth/device/code", endpoints::base_url());
    let resp = client
        .post(&url)
        .json(&DeviceCodeRequest { client_name })
        .send()
        .await
        .context("calling /oauth/device/code")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("device code request failed: HTTP {status}: {body}");
    }

    resp.json::<DeviceCodeResponse>()
        .await
        .context("parsing device code response")
}

/// Polls /oauth/device/token until approved, denied, expired, or cancelled.
/// Respects `slow_down` (bumps interval by 5 s) and the `expires_at` deadline.
pub async fn poll_for_token(
    client: &reqwest::Client,
    device_code: &str,
    mut interval: Duration,
    expires_at: Instant,
    cancel: CancellationToken,
) -> Result<String, DeviceFlowError> {
    let url = format!("{}/oauth/device/token", endpoints::base_url());

    loop {
        if Instant::now() >= expires_at {
            return Err(DeviceFlowError::Expired);
        }

        tokio::select! {
            _ = cancel.cancelled() => return Err(DeviceFlowError::Cancelled),
            _ = tokio::time::sleep(interval) => {}
        }

        let resp = client
            .post(&url)
            .json(&TokenRequest {
                grant_type: GRANT_TYPE,
                device_code,
            })
            .send()
            .await?;

        let status = resp.status();

        if status.is_success() {
            let body: TokenSuccess = resp.json().await.map_err(DeviceFlowError::Network)?;
            return Ok(body.token);
        }

        // 400 → parse {"error": "..."} per RFC 8628
        let body: TokenError = resp
            .json()
            .await
            .map_err(|e| DeviceFlowError::BadResponse(format!("unparseable error body: {e}")))?;

        match body.error.as_str() {
            "authorization_pending" => continue,
            "slow_down" => {
                interval += Duration::from_secs(5);
            }
            "access_denied" => return Err(DeviceFlowError::AccessDenied),
            "expired_token" => return Err(DeviceFlowError::Expired),
            other => {
                return Err(DeviceFlowError::BadResponse(format!(
                    "unexpected error code: {other}"
                )))
            }
        }
    }
}

/// Builds the client_name string sent to cook.md. Includes OS and a
/// best-effort label ("docker" / "cli" / hostname).
pub fn client_name(suffix: &str) -> String {
    format!(
        "CookCLI {} ({}/{})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        suffix
    )
}

/// Returns "docker" if /.dockerenv exists, else "server".
pub fn server_host_label() -> &'static str {
    if std::path::Path::new("/.dockerenv").exists() {
        "docker"
    } else {
        "server"
    }
}
