use super::endpoints;
use super::session::{self, SyncSession};
use anyhow::{Context, Result};
use cooklang_sync_client::errors::SyncError;
use cooklang_sync_client::SyncContext;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
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

/// A JWT should be renewed if it's close to expiring, if it's been a while
/// since the last successful renewal in this process, or if it needs the
/// one-time "heal" renewal for a token that predates the sync-enforcement
/// rollout (see `jwt_lacks_sync_until_claim`'s doc comment). Pure so the
/// decision table is exercised directly in tests below.
///
/// - `expires_in_secs`: seconds remaining until the JWT expires (may be
///   negative if it has already expired).
/// - `since_last_renew_secs`: seconds since the last successful renewal —
///   either in this process, or (when this process hasn't renewed yet)
///   derived from the session file's mtime. `None` when neither is
///   available (fresh process, no session file to stat).
/// - `claim_missing_and_not_yet_healed`: true when the JWT has no
///   `sync_until` claim and the once-per-boot heal renewal for that hasn't
///   run yet.
fn should_renew(
    expires_in_secs: i64,
    since_last_renew_secs: Option<i64>,
    claim_missing_and_not_yet_healed: bool,
) -> bool {
    /// Must be larger than the check interval (1 hour) to ensure the token
    /// is always refreshed before it expires.
    const RENEW_THRESHOLD_SECS: i64 = 7200; // 2 hours
    const RENEW_CADENCE_SECS: i64 = 86400; // 24 hours

    claim_missing_and_not_yet_healed
        || expires_in_secs < RENEW_THRESHOLD_SECS
        || matches!(since_last_renew_secs, Some(secs) if secs >= RENEW_CADENCE_SECS)
}

/// Seconds since `path` was last modified, or `None` if its metadata can't
/// be read (missing file, unsupported platform, clock skew, etc).
fn file_age_secs(path: &Path) -> Option<i64> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(modified.elapsed().ok()?.as_secs() as i64)
}

/// Check the current session's JWT and renew it if `should_renew` says so.
/// Shared by the boot-time heal check and the periodic loop in
/// `start_token_refresh` so both go through the exact same decision logic.
async fn check_and_maybe_renew(
    client: &reqwest::Client,
    session_state: &Arc<std::sync::Mutex<Option<SyncSession>>>,
    session_path: &Path,
    last_renewed_at: &mut Option<Instant>,
    claim_heal_attempted: &mut bool,
) {
    let jwt = {
        let guard = session_state.lock().unwrap();
        match guard.as_ref() {
            Some(s) => s.jwt.clone(),
            None => return,
        }
    };

    let expires_in = match session::jwt_expires_in(&jwt) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("Invalid JWT: {e}");
            let _ = SyncSession::delete(session_path);
            *session_state.lock().unwrap() = None;
            return;
        }
    };

    // Only ever force a renewal on the missing-claim signal once per boot:
    // a free-tier token lacks `sync_until` permanently (not just until
    // healed), so re-checking this every tick would renew forever for
    // free users.
    let claim_missing_and_not_yet_healed =
        !*claim_heal_attempted && session::jwt_lacks_sync_until_claim(&jwt).unwrap_or(false);

    let since_last_renew = last_renewed_at
        .map(|t| t.elapsed().as_secs() as i64)
        .or_else(|| file_age_secs(session_path));

    if !should_renew(
        expires_in,
        since_last_renew,
        claim_missing_and_not_yet_healed,
    ) {
        return;
    }

    tracing::info!("JWT needs refresh");
    if claim_missing_and_not_yet_healed {
        // One attempt, win or lose — see the comment above.
        *claim_heal_attempted = true;
    }

    apply_renew_outcome(
        refresh_token(client, &jwt).await,
        session_state,
        session_path,
        last_renewed_at,
    );
}

/// Applies the outcome of a `refresh_token` call to the in-memory session
/// and the session file. Split out from `check_and_maybe_renew` so the two
/// meaningfully different failure modes can be exercised directly in tests
/// without a real HTTP round trip:
///
/// - a rejected auth (`RenewError::AuthRejected`, HTTP 401) means the
///   account was logged out, deleted, or otherwise had its session revoked
///   server-side — retrying with the same token will never succeed, so this
///   restores the old blanket behavior: delete the session file, clear the
///   in-memory session, and let the existing "no session" pathway (the same
///   one `SyncSession::load` uses for a naturally-expired token) tell the
///   local UI a re-login is needed.
/// - anything else (`RenewError::Transient`: network failure, timeout,
///   non-401 non-2xx, unexpected body) keeps the old token and logs —
///   worth retrying next tick, not a reason to log the user out.
fn apply_renew_outcome(
    outcome: Result<String, RenewError>,
    session_state: &Arc<std::sync::Mutex<Option<SyncSession>>>,
    session_path: &Path,
    last_renewed_at: &mut Option<Instant>,
) {
    match outcome {
        Ok(new_jwt) => match SyncSession::from_jwt(new_jwt) {
            Ok(new_session) => {
                if let Err(e) = new_session.save(session_path) {
                    tracing::error!("Failed to save refreshed session: {e}");
                }
                *session_state.lock().unwrap() = Some(new_session);
                *last_renewed_at = Some(Instant::now());
                tracing::info!("JWT refreshed successfully");
            }
            Err(e) => tracing::error!("Failed to parse refreshed JWT: {e}"),
        },
        Err(RenewError::AuthRejected) => {
            tracing::warn!(
                "Session rejected by the server (401 Unauthorized); clearing session, re-login required"
            );
            let _ = SyncSession::delete(session_path);
            *session_state.lock().unwrap() = None;
        }
        Err(RenewError::Transient(e)) => {
            tracing::error!("Failed to refresh JWT: {e}");
        }
    }
}

/// Start a background token refresh task. Checks immediately (this is what
/// heals a session whose JWT predates the sync-enforcement rollout — see
/// `jwt_lacks_sync_until_claim`), then hourly; renews when < 2 hours of
/// token life remain or when ≥ 24 hours have passed since the last
/// successful renewal. Returns a JoinHandle so the caller can cancel via the
/// provided token.
pub fn start_token_refresh(
    session_state: Arc<std::sync::Mutex<Option<SyncSession>>>,
    session_path: impl AsRef<Path> + Send + 'static,
    cancel: CancellationToken,
) -> JoinHandle<()> {
    let session_path = session_path.as_ref().to_path_buf();
    let client = reqwest::Client::new();
    tokio::spawn(async move {
        let mut last_renewed_at: Option<Instant> = None;
        let mut claim_heal_attempted = false;

        // Immediate check at boot, before waiting for the first hourly tick.
        check_and_maybe_renew(
            &client,
            &session_state,
            &session_path,
            &mut last_renewed_at,
            &mut claim_heal_attempted,
        )
        .await;

        let mut interval = tokio::time::interval(Duration::from_secs(3600));
        interval.tick().await; // consume the interval's own immediate first tick; we already checked above
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    tracing::info!("Token refresh task stopped");
                    return;
                }
                _ = interval.tick() => {}
            }

            check_and_maybe_renew(
                &client,
                &session_state,
                &session_path,
                &mut last_renewed_at,
                &mut claim_heal_attempted,
            )
            .await;
        }
    })
}

/// Why a `/api/sessions/renew` call failed.
#[derive(Debug)]
enum RenewError {
    /// HTTP 401: the server rejected the token itself — the account was
    /// logged out, deleted, or otherwise had its session revoked
    /// server-side. Retrying with the same token will never succeed.
    AuthRejected,
    /// Anything else: a network failure, a timeout, a non-401 non-2xx
    /// status, or an unexpected response body. The token itself may still
    /// be fine — worth retrying on the next tick.
    Transient(anyhow::Error),
}

impl std::fmt::Display for RenewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenewError::AuthRejected => write!(f, "authentication rejected (401 Unauthorized)"),
            RenewError::Transient(e) => write!(f, "{e}"),
        }
    }
}

/// Pure: classifies a non-2xx `/api/sessions/renew` response status as an
/// auth rejection (401 only) or a transient failure. Never called with a
/// success status.
fn classify_renew_failure(status: reqwest::StatusCode) -> RenewError {
    if status == reqwest::StatusCode::UNAUTHORIZED {
        RenewError::AuthRejected
    } else {
        RenewError::Transient(anyhow::anyhow!("Token refresh failed: {status}"))
    }
}

async fn refresh_token(
    client: &reqwest::Client,
    current_token: &str,
) -> Result<String, RenewError> {
    let url = format!("{}/sessions/renew", endpoints::api_endpoint());
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {current_token}"))
        .send()
        .await
        .map_err(|e| RenewError::Transient(e.into()))?;

    if !resp.status().is_success() {
        return Err(classify_renew_failure(resp.status()));
    }

    #[derive(serde::Deserialize)]
    struct R {
        token: String,
    }
    let data: R = resp
        .json()
        .await
        .map_err(|e| RenewError::Transient(e.into()))?;
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

    // should_renew truth table. Columns: expires_in_secs, since_last_renew_secs,
    // claim_missing_and_not_yet_healed -> expected.
    const ONE_HOUR: i64 = 3600;
    const THREE_HOURS: i64 = 3 * 3600;
    const TWENTY_THREE_HOURS: i64 = 23 * 3600;
    const TWENTY_FIVE_HOURS: i64 = 25 * 3600;

    #[test]
    fn healthy_token_far_from_expiry_recently_renewed_does_not_renew() {
        assert!(!should_renew(THREE_HOURS, Some(ONE_HOUR), false));
    }

    #[test]
    fn renews_when_less_than_two_hours_remain() {
        assert!(should_renew(ONE_HOUR, Some(ONE_HOUR), false));
    }

    #[test]
    fn does_not_renew_at_exactly_two_hours_remaining() {
        // The threshold is a strict "<", not "<=".
        assert!(!should_renew(7200, Some(ONE_HOUR), false));
    }

    #[test]
    fn renews_when_already_expired() {
        assert!(should_renew(-1, Some(ONE_HOUR), false));
    }

    #[test]
    fn renews_when_24h_have_passed_since_last_renewal() {
        assert!(should_renew(THREE_HOURS, Some(TWENTY_FIVE_HOURS), false));
    }

    #[test]
    fn does_not_renew_before_24h_cadence_elapses() {
        assert!(!should_renew(THREE_HOURS, Some(TWENTY_THREE_HOURS), false));
    }

    #[test]
    fn renews_at_exactly_24h_cadence() {
        assert!(should_renew(THREE_HOURS, Some(86400), false));
    }

    #[test]
    fn no_prior_renewal_and_healthy_token_does_not_force_renewal() {
        // `None` means "no successful renewal yet this process, and no file
        // mtime to fall back on" — absent any other reason, that alone must
        // not force a renewal (a process restart shouldn't hammer the
        // renew endpoint every time `cook server` starts for a healthy
        // long-lived token).
        assert!(!should_renew(THREE_HOURS, None, false));
    }

    #[test]
    fn missing_claim_and_not_yet_healed_forces_renewal_even_when_otherwise_healthy() {
        assert!(should_renew(THREE_HOURS, Some(ONE_HOUR), true));
    }

    #[test]
    fn missing_claim_forces_renewal_even_with_no_prior_renewal_recorded() {
        assert!(should_renew(THREE_HOURS, None, true));
    }

    #[test]
    fn all_three_conditions_true_still_renews() {
        assert!(should_renew(-1, Some(TWENTY_FIVE_HOURS), true));
    }

    #[test]
    fn file_age_secs_is_none_for_missing_file() {
        assert_eq!(
            file_age_secs(Path::new("/nonexistent/path/session.json")),
            None
        );
    }

    #[test]
    fn file_age_secs_reports_recent_write_as_near_zero() {
        let dir = std::env::temp_dir().join(format!(
            "cookcli-runner-test-{}-{:?}",
            std::process::id(),
            Instant::now()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("session.json");
        std::fs::write(&path, "{}").unwrap();

        let age = file_age_secs(&path).expect("metadata should be readable");
        assert!(
            age < 5,
            "expected a freshly written file to be young, got {age}s"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- 401 vs transient renew failures --------------------------------

    #[test]
    fn classifies_401_as_auth_rejected() {
        assert!(matches!(
            classify_renew_failure(reqwest::StatusCode::UNAUTHORIZED),
            RenewError::AuthRejected
        ));
    }

    #[test]
    fn classifies_503_as_transient() {
        assert!(matches!(
            classify_renew_failure(reqwest::StatusCode::SERVICE_UNAVAILABLE),
            RenewError::Transient(_)
        ));
    }

    #[test]
    fn classifies_other_4xx_as_transient_not_auth_rejected() {
        // Only a 401 means the *token* was rejected; a 404/500/etc is not
        // grounds for logging the user out.
        assert!(matches!(
            classify_renew_failure(reqwest::StatusCode::NOT_FOUND),
            RenewError::Transient(_)
        ));
    }

    fn sample_session() -> SyncSession {
        SyncSession {
            jwt: "header.payload.signature".to_string(),
            user_id: "1".to_string(),
            email: Some("user@example.com".to_string()),
        }
    }

    fn temp_session_path(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cookcli-runner-test-{label}-{}-{:?}",
            std::process::id(),
            Instant::now()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("session.json")
    }

    #[test]
    fn auth_rejected_clears_the_session_and_deletes_the_file() {
        let path = temp_session_path("auth-rejected");
        let session = sample_session();
        session.save(&path).unwrap();
        let session_state = Arc::new(Mutex::new(Some(session)));
        let mut last_renewed_at: Option<Instant> = None;

        apply_renew_outcome(
            Err(RenewError::AuthRejected),
            &session_state,
            &path,
            &mut last_renewed_at,
        );

        assert!(
            session_state.lock().unwrap().is_none(),
            "a 401 must clear the in-memory session"
        );
        assert!(!path.exists(), "a 401 must delete the session file");
        assert!(last_renewed_at.is_none());

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn transient_failure_keeps_the_session_and_the_file() {
        let path = temp_session_path("transient");
        let session = sample_session();
        session.save(&path).unwrap();
        let session_state = Arc::new(Mutex::new(Some(session.clone())));
        let mut last_renewed_at: Option<Instant> = None;

        apply_renew_outcome(
            Err(RenewError::Transient(anyhow::anyhow!(
                "Token refresh failed: 503 Service Unavailable"
            ))),
            &session_state,
            &path,
            &mut last_renewed_at,
        );

        let guard = session_state.lock().unwrap();
        assert_eq!(
            guard.as_ref().map(|s| s.jwt.as_str()),
            Some(session.jwt.as_str()),
            "a transient failure must keep the old token for the next tick"
        );
        drop(guard);
        assert!(
            path.exists(),
            "a transient failure must not delete the session file"
        );
        assert!(last_renewed_at.is_none());

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn successful_renewal_replaces_the_session_and_records_the_time() {
        let path = temp_session_path("success");
        let old_session = sample_session();
        old_session.save(&path).unwrap();
        let session_state = Arc::new(Mutex::new(Some(old_session)));
        let mut last_renewed_at: Option<Instant> = None;

        let new_jwt = crate::sync::session::tests_support::make_test_jwt(
            "2",
            chrono::Utc::now().timestamp() + 100 * 24 * 3600,
        );

        apply_renew_outcome(
            Ok(new_jwt.clone()),
            &session_state,
            &path,
            &mut last_renewed_at,
        );

        let guard = session_state.lock().unwrap();
        assert_eq!(guard.as_ref().unwrap().jwt, new_jwt);
        drop(guard);
        assert!(last_renewed_at.is_some());

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
