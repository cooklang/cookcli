# CookCloud Sync Integration Design

## Overview

Add CookCloud sync to CookCLI so recipes sync automatically when the server is running. Users log in via the Preferences page in the web UI. The sync engine (`cooklang-sync-client`) runs continuously, watching the local directory and polling the remote server.

## Architecture

Sync is embedded directly into the existing server process. No separate daemon.

```
Server startup
  ├── Build AppState (existing)
  ├── Load session from ~/.config/cook/session.json
  ├── If authenticated → spawn sync task (runs continuously)
  └── Start Axum server (existing)

Server shutdown
  └── Cancel sync task via CancellationToken
```

## Auth Flow

Browser-based OAuth, adapted from sync-agent:

1. User clicks "Login to CookCloud" on Preferences page
2. Frontend POSTs to `/api/sync/login`
3. Server binds a temporary TCP listener on random localhost port
4. Server returns redirect URL: `cook.md/auth/desktops?callback=http://localhost:{port}/auth/callback&state={uuid}`
5. Frontend opens that URL in a new browser tab
6. User authenticates on cook.md
7. cook.md redirects to the localhost callback with `?token={jwt}&state={uuid}`
8. Server validates state, extracts JWT, stores in session file
9. Server starts sync task
10. Frontend polls `/api/sync/status` to detect login completion

For logout: POST `/api/sync/logout` → clear session file, cancel sync task.

## Session Storage

JSON file at `~/.config/cook/session.json`:

```json
{
  "jwt": "eyJ...",
  "user_id": "123",
  "email": "user@example.com"
}
```

JWT claims extracted via base64 decode (no cryptographic verification, same as sync-agent).

## Sync Engine

Single call to `cooklang_sync_client::run_async()` which runs continuously:
- Watches local recipe directory for changes
- Polls remote server for updates
- Handles conflict resolution internally

Parameters:
- `CancellationToken` — for shutdown
- `recipes_dir` — base_path from AppState
- `db_path` — `~/.config/cook/sync.db`
- `sync_endpoint` — `https://cook.md/api` (or debug endpoint)
- `jwt` — from session
- `namespace_id` — extracted from JWT via `extract_uid_from_jwt()`
- `download_only: false` — bidirectional sync

Token refresh: background task checks hourly, refreshes via `POST /api/sessions/renew` if < 1 hour remaining.

## AppState Changes

```rust
pub struct AppState {
    // Existing
    pub base_path: Utf8PathBuf,
    pub aisle_path: Option<Utf8PathBuf>,
    pub pantry_path: Option<Utf8PathBuf>,
    // New
    pub sync_session: Arc<Mutex<Option<SyncSession>>>,
    pub sync_handle: Arc<Mutex<Option<SyncHandle>>>,
}
```

`SyncSession` holds JWT + user info. `SyncHandle` holds the JoinHandle + CancellationToken for the running sync task.

## New API Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/sync/status` | GET | Returns `{ logged_in, email, syncing }` |
| `/api/sync/login` | POST | Returns `{ login_url }` for browser redirect, starts callback listener |
| `/api/sync/logout` | POST | Clears session, stops sync |

## New Files

- `src/server/sync.rs` — sync module (session management, sync task lifecycle, auth flow)
- `src/server/handlers/sync.rs` — API handlers for sync endpoints

## Preferences Page Changes

Add "CookCloud Sync" section:
- **Logged out**: "Login to CookCloud" button
- **Logged in**: user email, sync status indicator, "Logout" button
- Status fetched via JS polling `/api/sync/status` every 5s (or on page load)

## New Dependencies

```toml
cooklang-sync-client = "0.3.0"
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
base64 = "0.22"
uuid = { version = "1", features = ["v4"] }
tokio-util = "0.7"
```

## Endpoint Constants

Same pattern as sync-agent:
- Debug: `http://localhost:3000/api` (API), `http://localhost:8000` (sync)
- Release: `https://cook.md/api` (both)

## Error Handling

- Auth failures from sync → clear session, stop sync, show error in UI
- Network errors → sync client handles retries internally
- Invalid/expired JWT → token refresh task handles, or clear session on failure
