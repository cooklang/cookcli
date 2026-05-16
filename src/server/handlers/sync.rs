use crate::server::sync::{self, device_flow, PendingDeviceFlow, SyncSession};
use crate::server::AppState;
use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

#[derive(Serialize)]
pub struct PendingLogin {
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in_secs: u64,
}

#[derive(Serialize)]
pub struct SyncStatusResponse {
    pub logged_in: bool,
    pub email: Option<String>,
    pub syncing: bool,
    pub pending_login: Option<PendingLogin>,
}

pub async fn sync_status(State(state): State<Arc<AppState>>) -> Json<SyncStatusResponse> {
    let (logged_in, email, syncing) = state.sync_status().await;

    let pending_login = {
        let guard = state.pending_device_flow.lock().await;
        guard.as_ref().map(|p| PendingLogin {
            user_code: p.user_code.clone(),
            verification_uri: p.verification_uri.clone(),
            verification_uri_complete: p.verification_uri_complete.clone(),
            expires_in_secs: p
                .expires_at
                .saturating_duration_since(Instant::now())
                .as_secs(),
        })
    };

    Json(SyncStatusResponse {
        logged_in,
        email,
        syncing,
        pending_login,
    })
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
}

pub async fn sync_login(
    State(state): State<Arc<AppState>>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<serde_json::Value>)> {
    if state.sync_session.lock().unwrap().is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Already logged in" })),
        ));
    }

    {
        let guard = state.pending_device_flow.lock().await;
        if guard.is_some() {
            return Err((
                StatusCode::CONFLICT,
                Json(serde_json::json!({ "error": "Login already in progress" })),
            ));
        }
    }

    let client = reqwest::Client::new();
    let name = device_flow::client_name(device_flow::server_host_label());
    let dc = device_flow::request_device_code(&client, &name)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({ "error": format!("cook.md unreachable: {e}") })),
            )
        })?;

    let cancel = CancellationToken::new();
    let expires_at = Instant::now() + Duration::from_secs(dc.expires_in);
    let interval = Duration::from_secs(dc.interval);

    let pending = PendingDeviceFlow {
        device_code: dc.device_code.clone(),
        user_code: dc.user_code.clone(),
        verification_uri: dc.verification_uri.clone(),
        verification_uri_complete: dc.verification_uri_complete.clone(),
        expires_at,
        interval,
        cancel: cancel.clone(),
    };

    *state.pending_device_flow.lock().await = Some(pending);

    let state_clone = state.clone();
    let device_code = dc.device_code.clone();
    tokio::spawn(async move {
        let result =
            device_flow::poll_for_token(&client, &device_code, interval, expires_at, cancel).await;

        *state_clone.pending_device_flow.lock().await = None;

        match result {
            Ok(jwt) => match SyncSession::from_jwt(jwt) {
                Ok(session) => {
                    if let Err(e) = session.save(&state_clone.session_path) {
                        tracing::error!("Failed to save session: {e}");
                        return;
                    }
                    *state_clone.sync_session.lock().unwrap() = Some(session.clone());

                    match sync::sync_db_path() {
                        Ok(db_path) => match sync::start_sync(
                            &session,
                            state_clone.base_path.to_string(),
                            db_path,
                        ) {
                            Ok(handle) => {
                                *state_clone.sync_handle.lock().await = Some(handle);
                                tracing::info!("Sync started after login");
                            }
                            Err(e) => tracing::warn!("Failed to start sync after login: {e}"),
                        },
                        Err(e) => tracing::error!("Failed to resolve sync db path: {e}"),
                    }
                }
                Err(e) => tracing::error!("Failed to build SyncSession from JWT: {e}"),
            },
            Err(device_flow::DeviceFlowError::Cancelled) => {
                tracing::info!("Login cancelled by user");
            }
            Err(e) => tracing::error!("Login failed: {e}"),
        }
    });

    Ok(Json(LoginResponse {
        user_code: dc.user_code,
        verification_uri: dc.verification_uri,
        verification_uri_complete: dc.verification_uri_complete,
        expires_in: dc.expires_in,
    }))
}

pub async fn sync_cancel_login(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let mut guard = state.pending_device_flow.lock().await;
    if let Some(p) = guard.take() {
        p.cancel.cancel();
        Json(serde_json::json!({ "cancelled": true }))
    } else {
        Json(serde_json::json!({ "cancelled": false }))
    }
}

pub async fn sync_logout(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(handle) = state.sync_handle.lock().await.take() {
        handle.stop().await;
    }

    *state.sync_session.lock().unwrap() = None;
    if let Err(e) = SyncSession::delete(&state.session_path) {
        tracing::warn!("Failed to delete session file: {e}");
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}
