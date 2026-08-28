use super::endpoints;
use super::session::{self, SyncSession};
use anyhow::{Context, Result};
use cooklang_sync_client::errors::SyncError;
use cooklang_sync_client::SyncContext;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// A known, user-actionable reason the sync task stopped. Surfaced to the
/// local web UI via the sync status endpoint so it can explain why sync is
/// off instead of just showing it as stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncFailureReason {
    /// The sync server rejected the account with HTTP 402: no active plan.
    PaymentRequired,
}

impl SyncFailureReason {
    /// Stable wire value used in the sync status JSON response.
    pub fn as_str(&self) -> &'static str {
        match self {
            SyncFailureReason::PaymentRequired => "payment_required",
        }
    }
}

/// Maps a sync task's terminal error to a known, user-actionable reason, if
/// any. `None` means the error isn't one we surface specially to the UI —
/// it's still logged in full by the caller.
fn classify_sync_error(err: &SyncError) -> Option<SyncFailureReason> {
    match err {
        SyncError::PaymentRequired => Some(SyncFailureReason::PaymentRequired),
        _ => None,
    }
}

/// Holds the running sync task handle and cancellation token.
pub struct SyncHandle {
    context: Arc<SyncContext>,
    task: JoinHandle<()>,
    last_error: Arc<Mutex<Option<SyncFailureReason>>>,
}

impl SyncHandle {
    /// Check if the sync task is still running.
    pub fn is_running(&self) -> bool {
        !self.task.is_finished()
    }

    /// The reason the sync task most recently stopped, if it was a known,
    /// user-actionable condition. `None` if it's still running, finished
    /// cleanly, or failed for an unclassified reason.
    pub fn last_error_reason(&self) -> Option<SyncFailureReason> {
        *self.last_error.lock().unwrap()
    }

    /// Stop the sync task gracefully.
    pub async fn stop(self) {
        self.context.cancel();
        let timeout = tokio::time::Duration::from_secs(2);
        match tokio::time::timeout(timeout, self.task).await {
            Ok(Ok(())) => tracing::info!("Sync task stopped"),
            Ok(Err(e)) => tracing::warn!("Sync task panicked: {e:?}"),
            Err(_) => tracing::warn!("Sync task did not stop within timeout"),
        }
    }
}

/// Start the sync background task. Returns a SyncHandle for shutdown.
pub fn start_sync(
    session: &SyncSession,
    recipes_dir: String,
    db_path: String,
) -> Result<SyncHandle> {
    let context = SyncContext::new();
    let jwt = session.jwt.clone();
    let namespace_id: i32 = session
        .user_id
        .parse()
        .context("user_id is not a valid i32")?;
    let sync_ep = endpoints::sync_endpoint();

    tracing::info!("Starting sync for directory: {recipes_dir}");

    let last_error = Arc::new(Mutex::new(None));
    let last_error_for_task = Arc::clone(&last_error);

    let ctx = context.clone();
    let task = tokio::spawn(async move {
        let result = cooklang_sync_client::run_async(
            ctx,
            &recipes_dir,
            &db_path,
            &sync_ep,
            &jwt,
            namespace_id,
            false, // bidirectional sync
        )
        .await;

        match result {
            Ok(()) => tracing::info!("Sync task finished"),
            Err(e) => {
                // `run_async` does not retry internally (indexer/syncer return on
                // first error and `try_join!` propagates it), and nothing here
                // restarts the task automatically, so any error — including
                // PaymentRequired — ends the sync task for the rest of the
                // session rather than tight-looping. The user has to log in
                // again or restart `cook server` to retry.
                if let Some(reason) = classify_sync_error(&e) {
                    *last_error_for_task.lock().unwrap() = Some(reason);
                }
                match e {
                    SyncError::PaymentRequired => {
                        tracing::error!(
                            "Sync task failed: account has no sync plan (402 Payment Required)"
                        );
                        eprintln!(
                            "sync: your account has no sync plan — subscribe at https://cook.md/pricing (your files are untouched)"
                        );
                    }
                    e => tracing::error!("Sync task failed: {e:?}"),
                }
            }
        }
    });

    Ok(SyncHandle {
        context,
        task,
        last_error,
    })
}

/// Start a background token refresh task. Checks hourly, refreshes if < 1 hour remaining.
/// Returns a JoinHandle so the caller can cancel via the provided token.
pub fn start_token_refresh(
    session_state: Arc<std::sync::Mutex<Option<SyncSession>>>,
    session_path: impl AsRef<Path> + Send + 'static,
    cancel: CancellationToken,
) -> JoinHandle<()> {
    let session_path = session_path.as_ref().to_path_buf();
    let client = reqwest::Client::new();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600));
        interval.tick().await; // skip the immediate first tick
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    tracing::info!("Token refresh task stopped");
                    return;
                }
                _ = interval.tick() => {}
            }

            let jwt = {
                let guard = session_state.lock().unwrap();
                match guard.as_ref() {
                    Some(s) => s.jwt.clone(),
                    None => continue,
                }
            };

            match session::should_refresh_jwt(&jwt) {
                Ok(true) => {
                    tracing::info!("JWT needs refresh");
                    match refresh_token(&client, &jwt).await {
                        Ok(new_jwt) => match SyncSession::from_jwt(new_jwt) {
                            Ok(new_session) => {
                                if let Err(e) = new_session.save(&session_path) {
                                    tracing::error!("Failed to save refreshed session: {e}");
                                }
                                *session_state.lock().unwrap() = Some(new_session);
                                tracing::info!("JWT refreshed successfully");
                            }
                            Err(e) => tracing::error!("Failed to parse refreshed JWT: {e}"),
                        },
                        Err(e) => {
                            tracing::error!("Failed to refresh JWT: {e}");
                            let _ = SyncSession::delete(&session_path);
                            *session_state.lock().unwrap() = None;
                        }
                    }
                }
                Ok(false) => {}
                Err(e) => {
                    tracing::error!("Invalid JWT: {e}");
                    let _ = SyncSession::delete(&session_path);
                    *session_state.lock().unwrap() = None;
                }
            }
        }
    })
}

async fn refresh_token(client: &reqwest::Client, current_token: &str) -> Result<String> {
    let url = format!("{}/sessions/renew", endpoints::api_endpoint());
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {current_token}"))
        .send()
        .await?;

    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        anyhow::bail!("Authentication expired");
    }
    if !resp.status().is_success() {
        anyhow::bail!("Token refresh failed: {}", resp.status());
    }

    #[derive(serde::Deserialize)]
    struct R {
        token: String,
    }
    let data: R = resp.json().await?;
    Ok(data.token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payment_required_reason_has_stable_wire_value() {
        assert_eq!(
            SyncFailureReason::PaymentRequired.as_str(),
            "payment_required"
        );
    }

    #[test]
    fn classifies_payment_required_as_payment_required() {
        assert_eq!(
            classify_sync_error(&SyncError::PaymentRequired),
            Some(SyncFailureReason::PaymentRequired)
        );
    }

    #[test]
    fn does_not_classify_other_errors() {
        assert_eq!(classify_sync_error(&SyncError::Unauthorized), None);
        assert_eq!(
            classify_sync_error(&SyncError::Unknown("boom".to_string())),
            None
        );
    }
}
