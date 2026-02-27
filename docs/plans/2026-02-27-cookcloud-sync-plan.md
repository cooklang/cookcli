# CookCloud Sync Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Embed CookCloud recipe sync into the CookCLI server, with browser-based login from the Preferences page.

**Architecture:** The sync module lives inside `src/server/` and runs as a background tokio task alongside the Axum server. Auth uses a browser-redirect flow (user logs in on cook.md, JWT comes back to a local callback). Session is stored in a JSON config file. The `cooklang-sync-client` crate does the actual continuous sync (directory watching + remote polling).

**Tech Stack:** Rust, Axum, tokio, cooklang-sync-client, reqwest, base64, uuid

---

### Task 1: Add new dependencies to Cargo.toml

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add dependencies**

Add these to the `[dependencies]` section in `Cargo.toml`:

```toml
cooklang-sync-client = "0.3.0"
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
base64 = "0.22"
uuid = { version = "1", features = ["v4"] }
tokio-util = "0.7"
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Successful compilation (dependencies resolve)

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "feat: add cookcloud sync dependencies"
```

---

### Task 2: Create the sync session module

This handles JWT parsing, session load/save from a JSON config file, and endpoint constants.

**Files:**
- Create: `src/server/sync/mod.rs`
- Create: `src/server/sync/session.rs`
- Create: `src/server/sync/endpoints.rs`

**Step 1: Create `src/server/sync/endpoints.rs`**

```rust
/// API endpoint for authentication and user management
#[cfg(debug_assertions)]
pub const API: &str = "http://localhost:3000/api";
#[cfg(not(debug_assertions))]
pub const API: &str = "https://cook.md/api";

/// Sync server endpoint for recipe synchronization
#[cfg(debug_assertions)]
pub const SYNC: &str = "http://localhost:8000";
#[cfg(not(debug_assertions))]
pub const SYNC: &str = "https://cook.md/api";

/// Get the API endpoint, with env var override for development
pub fn api_endpoint() -> String {
    if let Ok(ep) = std::env::var("COOK_API_ENDPOINT") {
        return ep.trim_end_matches('/').to_string();
    }
    if let Ok(base) = std::env::var("COOK_ENDPOINT") {
        return format!("{}/api", base.trim_end_matches('/'));
    }
    API.to_string()
}

/// Get the sync endpoint, with env var override for development
pub fn sync_endpoint() -> String {
    if let Ok(ep) = std::env::var("COOK_SYNC_ENDPOINT") {
        return ep.trim_end_matches('/').to_string();
    }
    if let Ok(base) = std::env::var("COOK_ENDPOINT") {
        let base = base.trim_end_matches('/');
        if base.contains("localhost") || base.contains("127.0.0.1") {
            return "http://127.0.0.1:8000".to_string();
        }
        return format!("{base}/api");
    }
    SYNC.to_string()
}

/// Get base URL (strip /api suffix) for auth redirects
pub fn base_url() -> String {
    let api = api_endpoint();
    api.strip_suffix("/api").unwrap_or(&api).to_string()
}
```

**Step 2: Create `src/server/sync/session.rs`**

Session struct with file-based persistence. Adapted from sync-agent's `SecureSession` + `JwtToken` but simplified (no keyring, uses JSON file).

```rust
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    /// Parses claims from the JWT payload (no cryptographic verification).
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
    pub fn load(path: &PathBuf) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path)
            .context("Failed to read session file")?;
        let session: SyncSession = serde_json::from_str(&content)
            .context("Failed to parse session file")?;

        // Check expiry
        if is_jwt_expired(&session.jwt)? {
            tracing::info!("Session expired, removing");
            let _ = std::fs::remove_file(path);
            return Ok(None);
        }

        Ok(Some(session))
    }

    /// Save session to the config file.
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)
            .context("Failed to write session file")?;
        Ok(())
    }

    /// Delete session file.
    pub fn delete(path: &PathBuf) -> Result<()> {
        if path.exists() {
            std::fs::remove_file(path)
                .context("Failed to delete session file")?;
        }
        Ok(())
    }
}

fn decode_jwt_claims(jwt: &str) -> Result<JwtClaims> {
    let parts: Vec<&str> = jwt.split('.').collect();
    anyhow::ensure!(parts.len() == 3, "Invalid JWT format");

    // JWT payload may use standard or URL-safe base64
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
```

**Step 3: Create `src/server/sync/mod.rs`**

```rust
pub mod endpoints;
pub mod session;

pub use session::SyncSession;
```

**Step 4: Register the module in `src/server/mod.rs`**

Add `mod sync;` to the module declarations (after `mod ui;`, line 54).

**Step 5: Verify it compiles**

Run: `cargo check`
Expected: Successful compilation

**Step 6: Commit**

```bash
git add src/server/sync/
git commit -m "feat: add sync session and endpoint modules"
```

---

### Task 3: Add sync state to AppState and start sync on server boot

This wires the sync session into AppState, starts the sync task on boot if already logged in, and handles graceful shutdown.

**Files:**
- Modify: `src/server/mod.rs`
- Create: `src/server/sync/runner.rs`

**Step 1: Create `src/server/sync/runner.rs`**

This module manages the sync task lifecycle: start, stop, and token refresh.

```rust
use super::endpoints;
use super::session::{self, SyncSession};
use anyhow::Result;
use cooklang_sync_client::extract_uid_from_jwt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// Holds the running sync task handle and cancellation token.
pub struct SyncHandle {
    cancel: CancellationToken,
    task: JoinHandle<()>,
}

impl SyncHandle {
    /// Stop the sync task gracefully.
    pub async fn stop(self) {
        self.cancel.cancel();
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
    let token = CancellationToken::new();
    let child_token = token.child_token();
    let jwt = session.jwt.clone();
    let namespace_id = extract_uid_from_jwt(&jwt);
    let sync_ep = endpoints::sync_endpoint();

    tracing::info!("Starting sync for directory: {recipes_dir}");

    let task = tokio::spawn(async move {
        let result = cooklang_sync_client::run_async(
            child_token,
            None,
            &recipes_dir,
            &db_path,
            &sync_ep,
            &jwt,
            namespace_id,
            false, // bidirectional
        )
        .await;

        match result {
            Ok(()) => tracing::info!("Sync task finished"),
            Err(e) => tracing::error!("Sync task failed: {e:?}"),
        }
    });

    Ok(SyncHandle { cancel: token, task })
}

/// Start a background token refresh task. Checks hourly, refreshes if < 1 hour remaining.
pub fn start_token_refresh(
    session_state: Arc<Mutex<Option<SyncSession>>>,
    session_path: PathBuf,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;

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
                    match refresh_token(&jwt).await {
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
                            // Clear invalid session
                            let _ = SyncSession::delete(&session_path);
                            *session_state.lock().unwrap() = None;
                        }
                    }
                }
                Ok(false) => {} // Token still valid
                Err(e) => {
                    tracing::error!("Invalid JWT: {e}");
                    let _ = SyncSession::delete(&session_path);
                    *session_state.lock().unwrap() = None;
                }
            }
        }
    });
}

async fn refresh_token(current_token: &str) -> Result<String> {
    let url = format!("{}/sessions/renew", endpoints::api_endpoint());
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {current_token}"))
        .json(&serde_json::json!({ "token": current_token }))
        .send()
        .await?;

    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        anyhow::bail!("Authentication expired");
    }
    if !resp.status().is_success() {
        anyhow::bail!("Token refresh failed: {}", resp.status());
    }

    #[derive(serde::Deserialize)]
    struct R { token: String }
    let data: R = resp.json().await?;
    Ok(data.token)
}
```

**Step 2: Export runner in `src/server/sync/mod.rs`**

Update to:

```rust
pub mod endpoints;
pub mod runner;
pub mod session;

pub use runner::{start_sync, SyncHandle};
pub use session::SyncSession;
```

**Step 3: Modify `src/server/mod.rs` — AppState**

Update the `AppState` struct (line 225) to:

```rust
pub struct AppState {
    pub base_path: Utf8PathBuf,
    pub aisle_path: Option<Utf8PathBuf>,
    pub pantry_path: Option<Utf8PathBuf>,
    pub sync_session: Arc<Mutex<Option<sync::SyncSession>>>,
    pub sync_handle: Arc<tokio::sync::Mutex<Option<sync::SyncHandle>>>,
    pub session_path: std::path::PathBuf,
}
```

Add `use std::sync::Mutex;` to the imports at the top.

**Step 4: Modify `build_state()` in `src/server/mod.rs`**

Update `build_state()` (line 172) to load session and configure sync paths:

```rust
fn build_state(ctx: Context, args: ServerArgs) -> Result<Arc<AppState>> {
    let Context { base_path } = ctx;

    let path = args.base_path.as_ref().unwrap_or(&base_path);
    let absolute_path = resolve_to_absolute_path(path)?;

    if absolute_path.is_file() {
        bail!("Base path {} is not a directory", absolute_path);
    }

    tracing::info!("Using absolute base path: {:?}", absolute_path);

    let server_ctx = Context::new(absolute_path.clone());
    let aisle_path = server_ctx.aisle();
    let pantry_path = server_ctx.pantry();

    tracing::info!("Aisle configuration: {:?}", aisle_path);
    tracing::info!("Pantry configuration: {:?}", pantry_path);

    // Session file path
    let session_path = crate::global_file_path("session.json")
        .map(|p| p.into_std_path_buf())
        .unwrap_or_else(|_| std::path::PathBuf::from(".cook-session.json"));

    // Load existing session
    let session = match sync::SyncSession::load(&session_path) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Failed to load sync session: {e}");
            None
        }
    };

    Ok(Arc::new(AppState {
        base_path: absolute_path,
        aisle_path,
        pantry_path,
        sync_session: Arc::new(Mutex::new(session)),
        sync_handle: Arc::new(tokio::sync::Mutex::new(None)),
        session_path,
    }))
}
```

**Step 5: Modify `run()` in `src/server/mod.rs`**

After building state and before starting the Axum server, start sync if session exists. Also stop sync on shutdown.

After `let state = build_state(ctx, args)?;` (line 126), add:

```rust
    // Start sync if already logged in
    {
        let session_guard = state.sync_session.lock().unwrap();
        if let Some(ref session) = *session_guard {
            let db_path = crate::global_file_path("sync.db")
                .map(|p| p.to_string())
                .unwrap_or_else(|_| ".cook-sync.db".to_string());
            match sync::start_sync(session, state.base_path.to_string(), db_path) {
                Ok(handle) => {
                    let state_clone = state.clone();
                    tokio::spawn(async move {
                        *state_clone.sync_handle.lock().await = Some(handle);
                    });
                    tracing::info!("Sync started on server boot");
                }
                Err(e) => tracing::warn!("Failed to start sync: {e}"),
            }
        }

        // Start token refresh task
        sync::runner::start_token_refresh(
            Arc::clone(&state.sync_session),
            state.session_path.clone(),
        );
    }
```

Before the `Ok(())` at the end of `run()`, after the server stops, stop sync:

```rust
    // Stop sync on shutdown
    if let Some(handle) = state.sync_handle.lock().await.take() {
        handle.stop().await;
    }
```

Note: since `run()` uses `#[tokio::main]`, we need to handle the fact that the last section after `axum::serve` is async. The shutdown signal will fire, the server stops, then we clean up sync.

**Step 6: Verify it compiles**

Run: `cargo check`
Expected: Successful compilation

**Step 7: Commit**

```bash
git add src/server/sync/runner.rs src/server/mod.rs
git commit -m "feat: start sync on server boot with graceful shutdown"
```

---

### Task 4: Add sync API handlers (status, login, logout)

**Files:**
- Create: `src/server/handlers/sync.rs`
- Modify: `src/server/handlers/mod.rs`
- Modify: `src/server/mod.rs` (add routes)

**Step 1: Create `src/server/handlers/sync.rs`**

```rust
use crate::server::sync::{self, SyncSession};
use crate::server::AppState;
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize)]
pub struct SyncStatusResponse {
    pub logged_in: bool,
    pub email: Option<String>,
    pub syncing: bool,
}

pub async fn sync_status(
    State(state): State<Arc<AppState>>,
) -> Json<SyncStatusResponse> {
    let session = state.sync_session.lock().unwrap();
    let syncing = state.sync_handle.lock().await.is_some();

    Json(SyncStatusResponse {
        logged_in: session.is_some(),
        email: session.as_ref().and_then(|s| s.email.clone()),
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
    // Already logged in?
    if state.sync_session.lock().unwrap().is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Already logged in" })),
        ));
    }

    let state_clone = state.clone();

    // Spawn login flow in background (it blocks waiting for callback)
    tokio::spawn(async move {
        match browser_login_flow(&state_clone).await {
            Ok(()) => tracing::info!("Login completed successfully"),
            Err(e) => tracing::error!("Login failed: {e}"),
        }
    });

    // Build the URL we'll open in the browser
    // The actual flow opens the browser from the spawned task,
    // but we also return it so the frontend can open it directly
    let base_url = sync::endpoints::base_url();
    // We can't know the port yet (random), so the frontend should just
    // call this endpoint and then poll /api/sync/status for completion
    Ok(Json(LoginResponse {
        login_url: format!("{base_url}/auth/desktops"),
    }))
}

async fn browser_login_flow(state: &Arc<AppState>) -> anyhow::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
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
    let login_url = format!(
        "{base_url}/auth/desktops?callback={encoded_callback}&state={csrf_state}"
    );

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
    let db_path = crate::global_file_path("sync.db")
        .map(|p| p.to_string())
        .unwrap_or_else(|_| ".cook-sync.db".to_string());

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
        let cors_response = "HTTP/1.1 200 OK\r\n\
            Access-Control-Allow-Origin: *\r\n\
            Access-Control-Allow-Methods: GET, OPTIONS\r\n\
            Access-Control-Allow-Headers: x-csrf-token, x-turbo-request-id\r\n\
            Access-Control-Max-Age: 86400\r\n\
            Content-Length: 0\r\n\r\n";
        socket.write_all(cors_response.as_bytes()).await?;

        // Try same connection, then new connection
        let mut buffer = [0u8; 4096];
        let request = match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            socket.read(&mut buffer),
        )
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
        };
        return request;
    }

    // Regular GET
    handle_get_callback(&mut socket, &request, expected_state).await
}

async fn handle_get_callback(
    socket: &mut tokio::net::TcpStream,
    request: &str,
    expected_state: &str,
) -> anyhow::Result<String> {
    use tokio::io::AsyncWriteExt;

    if let Some(token) = extract_token(request, expected_state) {
        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nAccess-Control-Allow-Origin: *\r\n\r\n\
            <!DOCTYPE html><html><head><title>Login Complete</title>\
            <style>body{font-family:system-ui;display:flex;align-items:center;justify-content:center;min-height:100vh;margin:0;background:#f5f5f5}\
            .c{text-align:center;background:white;padding:2rem 3rem;border-radius:8px;box-shadow:0 2px 10px rgba(0,0,0,.1)}\
            .ok{width:64px;height:64px;margin:0 auto 1rem;background:#4CAF50;border-radius:50%;display:flex;align-items:center;justify-content:center;font-size:32px;color:white}\
            </style></head><body><div class=\"c\"><div class=\"ok\">&#10003;</div>\
            <h1>All Done!</h1><p>You can close this tab and return to CookCLI.</p></div>\
            <script>setTimeout(()=>{if(window.opener)window.close()},2000)</script>\
            </body></html>";
        socket.write_all(response.as_bytes()).await?;
        Ok(token)
    } else {
        let response = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\nAccess-Control-Allow-Origin: *\r\n\r\n\
            <!DOCTYPE html><html><head><title>Login Failed</title>\
            <style>body{font-family:system-ui;display:flex;align-items:center;justify-content:center;min-height:100vh;margin:0;background:#f5f5f5}\
            .c{text-align:center;background:white;padding:2rem 3rem;border-radius:8px;box-shadow:0 2px 10px rgba(0,0,0,.1)}\
            .err{width:64px;height:64px;margin:0 auto 1rem;background:#f44336;border-radius:50%;display:flex;align-items:center;justify-content:center;font-size:32px;color:white}\
            </style></head><body><div class=\"c\"><div class=\"err\">&#10005;</div>\
            <h1>Login Failed</h1><p>Please close this tab and try again.</p></div>\
            </body></html>";
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
    let query_start = path.find('?')?;
    let query = &path[query_start + 1..];

    let mut token = None;
    let mut state = None;

    for param in query.split('&') {
        if let Some(eq) = param.find('=') {
            let key = &param[..eq];
            let value = &param[eq + 1..];
            match key {
                "token" => token = Some(urlencoding::decode(value).ok()?.into_owned()),
                "state" => state = Some(urlencoding::decode(value).ok()?.into_owned()),
                _ => {}
            }
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
    let _ = SyncSession::delete(&state.session_path);

    Ok(Json(serde_json::json!({ "ok": true })))
}
```

**Step 2: Update `src/server/handlers/mod.rs`**

Add the sync module and re-exports:

```rust
pub mod pantry;
pub mod recipes;
pub mod shopping_list;
pub mod sync;

pub use pantry::{
    add_item as add_pantry_item, get_pantry, remove_item as remove_pantry_item,
    update_item as update_pantry_item,
};
pub use recipes::{all_recipes, recipe, recipe_delete, recipe_raw, recipe_save, reload, search};
pub use shopping_list::{
    add_to_shopping_list, clear_shopping_list, get_shopping_list_items, remove_from_shopping_list,
    shopping_list,
};
pub use sync::{sync_login, sync_logout, sync_status};
```

**Step 3: Add routes in `src/server/mod.rs`**

In the `api()` function, add sync routes after the existing routes (before `Ok(router)`):

```rust
        .route("/sync/status", get(handlers::sync_status))
        .route("/sync/login", post(handlers::sync_login))
        .route("/sync/logout", post(handlers::sync_logout))
```

**Step 4: Verify it compiles**

Run: `cargo check`
Expected: Successful compilation

**Step 5: Commit**

```bash
git add src/server/handlers/sync.rs src/server/handlers/mod.rs src/server/mod.rs
git commit -m "feat: add sync API endpoints (status, login, logout)"
```

---

### Task 5: Update Preferences page UI with CookCloud Sync section

**Files:**
- Modify: `templates/preferences.html`
- Modify: `src/server/templates.rs` (add sync fields to PreferencesTemplate)
- Modify: `src/server/ui.rs` (pass sync state to template)

**Step 1: Add sync fields to `PreferencesTemplate` in `src/server/templates.rs`**

Update the `PreferencesTemplate` struct (line 169):

```rust
#[derive(Template)]
#[template(path = "preferences.html")]
pub struct PreferencesTemplate {
    pub active: String,
    pub aisle_path: String,
    pub pantry_path: String,
    pub base_path: String,
    pub version: String,
    pub tr: Tr,
    pub sync_logged_in: bool,
    pub sync_email: Option<String>,
}
```

**Step 2: Update `preferences_page` handler in `src/server/ui.rs`**

Find the `preferences_page` handler (line 1327) and add sync state:

```rust
async fn preferences_page(
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
) -> impl IntoResponse {
    let session = state.sync_session.lock().unwrap();

    PreferencesTemplate {
        active: "preferences".to_string(),
        aisle_path: state
            .aisle_path
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_else(|| "Not configured".to_string()),
        pantry_path: state
            .pantry_path
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_else(|| "Not configured".to_string()),
        base_path: state.base_path.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        tr: Tr::new(lang),
        sync_logged_in: session.is_some(),
        sync_email: session.as_ref().and_then(|s| s.email.clone()),
    }
}
```

**Step 3: Update `templates/preferences.html`**

Add the CookCloud Sync section at the top of the `<div class="space-y-6">` block, right after the language selector section:

```html
        <!-- CookCloud Sync -->
        <div class="bg-gradient-to-r from-blue-50 to-cyan-50 p-6 rounded-2xl border-2 border-blue-200">
            <h2 class="text-lg font-semibold mb-4 text-blue-900">CookCloud Sync</h2>
            <div id="sync-section">
                {% if sync_logged_in %}
                <div class="flex items-center justify-between">
                    <div>
                        <p class="text-sm text-blue-800">
                            Signed in as <span class="font-medium">{{ sync_email.as_deref().unwrap_or("Unknown") }}</span>
                        </p>
                        <p class="text-xs text-blue-600 mt-1" id="sync-status-text">Syncing recipes...</p>
                    </div>
                    <button onclick="syncLogout()"
                            class="px-4 py-2 rounded-lg font-medium transition-all duration-200 border-2 bg-white text-red-600 border-red-300 hover:bg-red-50 hover:border-red-400">
                        Logout
                    </button>
                </div>
                {% else %}
                <div class="flex items-center justify-between">
                    <p class="text-sm text-blue-700">Sync your recipes across devices with CookCloud.</p>
                    <button onclick="syncLogin()"
                            class="px-4 py-2 rounded-lg font-medium transition-all duration-200 border-2 bg-gradient-to-r from-blue-500 to-blue-600 text-white border-blue-600 shadow-lg hover:scale-105">
                        Login to CookCloud
                    </button>
                </div>
                {% endif %}
            </div>
        </div>
```

**Step 4: Add JavaScript for login/logout in the `{% block scripts %}` section**

Add to the existing script block in `preferences.html`:

```javascript
    async function syncLogin() {
        try {
            const resp = await fetch('/api/sync/login', { method: 'POST' });
            if (!resp.ok) {
                const err = await resp.json();
                alert(err.error || 'Login failed');
                return;
            }
            // Poll for login completion
            const pollInterval = setInterval(async () => {
                const status = await fetch('/api/sync/status').then(r => r.json());
                if (status.logged_in) {
                    clearInterval(pollInterval);
                    window.location.reload();
                }
            }, 2000);
            // Stop polling after 5 minutes
            setTimeout(() => clearInterval(pollInterval), 300000);
        } catch (e) {
            alert('Failed to start login: ' + e.message);
        }
    }

    async function syncLogout() {
        try {
            await fetch('/api/sync/logout', { method: 'POST' });
            window.location.reload();
        } catch (e) {
            alert('Logout failed: ' + e.message);
        }
    }
```

**Step 5: Verify it compiles**

Run: `cargo check`
Expected: Successful compilation

**Step 6: Commit**

```bash
git add templates/preferences.html src/server/templates.rs src/server/ui.rs
git commit -m "feat: add CookCloud Sync section to Preferences page"
```

---

### Task 6: Manual integration testing

**Files:** None (testing only)

**Step 1: Build the project**

Run: `cargo build`
Expected: Successful build

**Step 2: Start the server with seed recipes**

Run: `cargo run -- server ./seed`
Expected: Server starts on http://localhost:9080

**Step 3: Test sync status endpoint**

Run: `curl http://localhost:9080/api/sync/status`
Expected: `{"logged_in":false,"email":null,"syncing":false}`

**Step 4: Visit Preferences page**

Open http://localhost:9080/preferences in browser.
Expected: See "CookCloud Sync" section with "Login to CookCloud" button.

**Step 5: Test login flow (if cook.md dev server available)**

Click "Login to CookCloud" button. Should open a new browser tab to cook.md auth page.
After authenticating, the preferences page should update to show the signed-in state.

**Step 6: Test logout**

Click "Logout" button. Should clear session and show login button again.

**Step 7: Commit (if any fixes needed)**

```bash
git add -A
git commit -m "fix: integration testing fixes for cookcloud sync"
```

---

### Task 7: Run lint and format checks

**Files:** Any files that need formatting fixes

**Step 1: Format code**

Run: `cargo fmt`

**Step 2: Run clippy**

Run: `cargo clippy`
Expected: No warnings or errors

**Step 3: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 4: Commit any fixes**

```bash
git add -A
git commit -m "style: rustfmt and clippy fixes for sync feature"
```
