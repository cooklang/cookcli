use crate::server::sync::{self, SyncSession};
use crate::server::AppState;
use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use std::sync::Arc;

/// CORS origin for the OAuth callback server.
fn cors_origin() -> String {
    sync::endpoints::base_url()
}

#[derive(Serialize)]
pub struct SyncStatusResponse {
    pub logged_in: bool,
    pub email: Option<String>,
    pub syncing: bool,
}

pub async fn sync_status(State(state): State<Arc<AppState>>) -> Json<SyncStatusResponse> {
    let (logged_in, email, syncing) = state.sync_status().await;
    Json(SyncStatusResponse {
        logged_in,
        email,
        syncing,
    })
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub login_url: String,
}

pub async fn sync_login(
    State(state): State<Arc<AppState>>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<serde_json::Value>)> {
    use std::sync::atomic::Ordering;

    // Already logged in?
    if state.sync_session.lock().unwrap().is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Already logged in" })),
        ));
    }

    // Prevent concurrent login attempts
    if state
        .login_in_progress
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": "Login already in progress" })),
        ));
    }

    let state_clone = state.clone();

    // Spawn login flow in background (it blocks waiting for callback)
    tokio::spawn(async move {
        match browser_login_flow(&state_clone).await {
            Ok(()) => tracing::info!("Login completed successfully"),
            Err(e) => tracing::error!("Login failed: {e}"),
        }
        state_clone.login_in_progress.store(false, Ordering::SeqCst);
    });

    // Return the base URL so the frontend knows login was initiated
    let base_url = sync::endpoints::base_url();
    Ok(Json(LoginResponse {
        login_url: format!("{base_url}/auth/desktops"),
    }))
}

async fn browser_login_flow(state: &Arc<AppState>) -> anyhow::Result<()> {
    use tokio::net::TcpListener;

    // Bind to random port
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    // CSRF state
    let csrf_state = uuid::Uuid::new_v4().to_string();

    // Build login URL
    let callback_url = format!("http://localhost:{port}/auth/callback");
    let encoded_callback = urlencoding::encode(&callback_url);
    let base_url = sync::endpoints::base_url();
    let login_url =
        format!("{base_url}/auth/desktops?callback={encoded_callback}&state={csrf_state}");

    tracing::info!("Opening browser for CookCloud login");
    if let Err(e) = open::that(&login_url) {
        tracing::error!("Failed to open browser: {e}");
        return Err(e.into());
    }

    // Wait for callback (5 min timeout)
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(300),
        wait_for_callback(&listener, &csrf_state),
    )
    .await;

    let jwt = match result {
        Ok(Ok(token)) => token,
        Ok(Err(e)) => return Err(e),
        Err(_) => anyhow::bail!("Login timed out after 5 minutes"),
    };

    // Create and save session
    let session = SyncSession::from_jwt(jwt)?;
    session.save(&state.session_path)?;

    // Update state
    *state.sync_session.lock().unwrap() = Some(session.clone());

    // Start sync
    let db_path = sync::sync_db_path();

    match sync::start_sync(&session, state.base_path.to_string(), db_path) {
        Ok(handle) => {
            *state.sync_handle.lock().await = Some(handle);
            tracing::info!("Sync started after login");
        }
        Err(e) => tracing::warn!("Failed to start sync after login: {e}"),
    }

    Ok(())
}

async fn wait_for_callback(
    listener: &tokio::net::TcpListener,
    expected_state: &str,
) -> anyhow::Result<String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let (mut socket, _) = listener.accept().await?;
    let mut buffer = [0u8; 4096];
    let n = socket.read(&mut buffer).await?;
    let request = String::from_utf8_lossy(&buffer[..n]);

    // Handle CORS preflight
    if request.starts_with("OPTIONS ") {
        let origin = cors_origin();
        let cors_response = format!(
            "HTTP/1.1 200 OK\r\n\
            Access-Control-Allow-Origin: {origin}\r\n\
            Access-Control-Allow-Methods: GET, OPTIONS\r\n\
            Access-Control-Allow-Headers: x-csrf-token, x-turbo-request-id\r\n\
            Access-Control-Max-Age: 86400\r\n\
            Content-Length: 0\r\n\r\n"
        );
        socket.write_all(cors_response.as_bytes()).await?;

        // Try same connection, then new connection
        let mut buffer = [0u8; 4096];
        match tokio::time::timeout(std::time::Duration::from_secs(5), socket.read(&mut buffer))
            .await
        {
            Ok(Ok(n)) if n > 0 => {
                let req = String::from_utf8_lossy(&buffer[..n]).to_string();
                handle_get_callback(&mut socket, &req, expected_state).await
            }
            _ => {
                let (mut new_socket, _) = listener.accept().await?;
                let mut buffer = [0u8; 4096];
                let n = new_socket.read(&mut buffer).await?;
                let req = String::from_utf8_lossy(&buffer[..n]).to_string();
                handle_get_callback(&mut new_socket, &req, expected_state).await
            }
        }
    } else {
        // Regular GET
        handle_get_callback(&mut socket, &request, expected_state).await
    }
}

async fn handle_get_callback(
    socket: &mut tokio::net::TcpStream,
    request: &str,
    expected_state: &str,
) -> anyhow::Result<String> {
    use tokio::io::AsyncWriteExt;

    let origin = cors_origin();
    if let Some(token) = extract_token(request, expected_state) {
        let response = format!(
            "HTTP/1.1 200 OK\r\n\
            Content-Type: text/html; charset=utf-8\r\n\
            Access-Control-Allow-Origin: {origin}\r\n\r\n\
            <!DOCTYPE html><html><head><title>Login Complete</title>\
            <style>\
            body{{font-family:system-ui;display:flex;align-items:center;\
            justify-content:center;min-height:100vh;margin:0;background:#f5f5f5}}\
            .c{{text-align:center;background:white;padding:2rem 3rem;\
            border-radius:8px;box-shadow:0 2px 10px rgba(0,0,0,.1)}}\
            .ok{{width:64px;height:64px;margin:0 auto 1rem;background:#4CAF50;\
            border-radius:50%;display:flex;align-items:center;\
            justify-content:center;font-size:32px;color:white}}\
            </style></head><body><div class=\"c\"><div class=\"ok\">&#10003;</div>\
            <h1>All Done!</h1>\
            <p>You can close this tab and return to CookCLI.</p></div>\
            <script>setTimeout(()=>{{if(window.opener)window.close()}},2000)</script>\
            </body></html>"
        );
        socket.write_all(response.as_bytes()).await?;
        Ok(token)
    } else {
        let response = format!(
            "HTTP/1.1 400 Bad Request\r\n\
            Content-Type: text/html; charset=utf-8\r\n\
            Access-Control-Allow-Origin: {origin}\r\n\r\n\
            <!DOCTYPE html><html><head><title>Login Failed</title>\
            <style>\
            body{{font-family:system-ui;display:flex;align-items:center;\
            justify-content:center;min-height:100vh;margin:0;background:#f5f5f5}}\
            .c{{text-align:center;background:white;padding:2rem 3rem;\
            border-radius:8px;box-shadow:0 2px 10px rgba(0,0,0,.1)}}\
            .err{{width:64px;height:64px;margin:0 auto 1rem;background:#f44336;\
            border-radius:50%;display:flex;align-items:center;\
            justify-content:center;font-size:32px;color:white}}\
            </style></head><body><div class=\"c\"><div class=\"err\">&#10005;</div>\
            <h1>Login Failed</h1>\
            <p>Please close this tab and try again.</p></div>\
            </body></html>"
        );
        socket.write_all(response.as_bytes()).await?;
        anyhow::bail!("Failed to extract token from callback")
    }
}

fn extract_token(request: &str, expected_state: &str) -> Option<String> {
    let first_line = request.lines().next()?;
    if !first_line.starts_with("GET ") {
        return None;
    }
    let path = first_line.split(' ').nth(1)?;

    // Use url crate for robust query string parsing (handles %26-encoded values, etc.)
    let full_url = format!("http://localhost{path}");
    let parsed = url::Url::parse(&full_url).ok()?;

    let mut token = None;
    let mut state = None;

    for (key, value) in parsed.query_pairs() {
        match key.as_ref() {
            "token" => token = Some(value.into_owned()),
            "state" => state = Some(value.into_owned()),
            _ => {}
        }
    }

    if state.as_deref() == Some(expected_state) {
        token
    } else {
        None
    }
}

pub async fn sync_logout(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Stop sync task
    if let Some(handle) = state.sync_handle.lock().await.take() {
        handle.stop().await;
    }

    // Clear session
    *state.sync_session.lock().unwrap() = None;
    if let Err(e) = SyncSession::delete(&state.session_path) {
        tracing::warn!("Failed to delete session file: {e}");
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}
