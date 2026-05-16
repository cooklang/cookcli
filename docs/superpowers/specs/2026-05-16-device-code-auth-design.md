# Device-Code Authentication for CookCloud Sync — Design

## Summary

Replace CookCLI's current browser-popup login flow with the OAuth 2.0 Device Authorization Grant (RFC 8628). The new flow works in any environment — native desktop, Docker, SSH, behind a reverse proxy — because CookCLI never needs to open a browser on the host where it runs. The existing "Login to CookCloud" button in the web UI is fixed, and a new `cook login` / `cook logout` CLI command is added. CookCLI and `cook.md` share one device-code flow; the CookCLI web handler and CLI command share one Rust implementation.

## Motivation

[Issue #290](https://github.com/cooklang/cookcli/issues/290): users running CookCLI in Docker cannot sign in. The current flow fails twice over:

1. The server calls `open::that(login_url)` — there is no browser inside the container, so the call errors with `No such file or directory`.
2. Even with a browser, the OAuth callback URL is `http://localhost:RANDOM_PORT` bound to the *container's* loopback — unreachable from the host browser.

`cook.md` further restricts callback URLs to `http://localhost` / `http://127.0.0.1` (`DesktopAuthCallback#safe_callback_url?`), so loosening port/host alone is not enough. A different flow is required.

## Goals

- "Login to CookCloud" works in Docker, SSH, native, and reverse-proxy deployments.
- Headless CLI login via `cook login` for environments with no web UI at all.
- Single shared mechanism between web UI and CLI; single shared Rust module.
- Standards-based: OAuth 2.0 Device Authorization Grant (RFC 8628).
- JWT issuance, session storage, refresh, and sync remain unchanged downstream.

## Non-goals (v1)

- Revoking active JWTs. They remain 100-day, stateless, as today.
- Scoped tokens / per-app permissions. CookCLI continues to get full account access.
- QR code rendering in the web UI. `verification_uri_complete` is returned, so a QR can be bolted on later trivially.
- Registered-clients allow-list on `cook.md`. `client_name` is free-text, HTML-escaped, with a length cap.
- File-watcher in `cook server` to pick up newly-written `session.json`. CLI login still requires a server restart, as today.

## End-to-end Flow

```
┌──────────────┐                       ┌──────────────┐                 ┌────────────┐
│ User browser │                       │   CookCLI    │                 │  cook.md   │
│  (any device)│                       │  (server/CLI)│                 │            │
└──────┬───────┘                       └──────┬───────┘                 └─────┬──────┘
       │  click Login / run `cook login`      │                               │
       │ ────────────────────────────────────▶│                               │
       │                                      │  POST /oauth/device/code      │
       │                                      │   {client_name}               │
       │                                      │ ─────────────────────────────▶│
       │                                      │  {device_code, user_code,     │
       │                                      │   verification_uri,           │
       │                                      │   verification_uri_complete,  │
       │                                      │   expires_in:900, interval:5} │
       │                                      │ ◀─────────────────────────────│
       │  show user_code + verification_uri   │                               │
       │ ◀────────────────────────────────────│                               │
       │                                      │  POST /oauth/device/token     │
       │                                      │   (loop every `interval`s)    │
       │                                      │ ─────────────────────────────▶│
       │                                      │  400 authorization_pending    │
       │                                      │ ◀─────────────────────────────│
       │  user opens verification_uri in any browser, signs in if needed,    │
       │  enters user_code, sees "CookCLI on linux/docker — Approve?"        │
       │  ──────────────────────────────────────────────────────────────────▶│
       │                                      │  POST /oauth/device/token     │
       │                                      │ ─────────────────────────────▶│
       │                                      │  200 {token: <JWT>}           │
       │                                      │ ◀─────────────────────────────│
       │                                      │  save session.json,           │
       │                                      │  start sync                   │
```

---

## cook.md (Rails)

### Storage

Redis, matching the existing `SessionCode` pattern. Two keys per flow, both TTL 15 min:

- `device_code:{sha256(device_code)}` → JSON `{user_code, client_name, status, user_id?, last_polled_at}`
- `user_code:{user_code}` → `{device_code_hash}`  *(reverse lookup; deleted on approve/deny so user codes are single-use)*

State values: `pending`, `approved`, `denied`. Expiry is implicit via Redis TTL (no `expired` state needed).

### Codes

- **`user_code`**: 8 characters, format `XXXX-XXXX`, alphabet `23456789ABCDEFGHJKLMNPQRSTUVWXYZ` (excludes `0/O/1/I/L`). ≈10¹² combinations.
- **`device_code`**: 32 random bytes, base64url-encoded. Hashed with SHA-256 before storage; plain returned to client once.

### Endpoints

| Route | Auth | Purpose |
|---|---|---|
| `POST /oauth/device/code` | none | Issue a `device_code` + `user_code`. Body: `{client_name}` (optional). Rate-limit 10/min/IP. |
| `POST /oauth/device/token` | none | Poll. Body: `{device_code, grant_type:"urn:ietf:params:oauth:grant-type:device_code"}`. Returns `200 {token}` or `400 {error}` per RFC 8628. |
| `GET /device` | required (redirects to existing `/auth/desktop/code` sign-in, return URL preserved) | Form to enter `user_code`. Pre-fills from `?user_code=` query param. |
| `POST /device/approve` | required | Marks the matched device code approved; sets `user_id` to current user. |
| `POST /device/deny` | required | Marks denied. |

### Token endpoint responses (RFC 8628 contract)

- `200 {"token": "<JWT>", "token_type": "Bearer"}` — approved. Redis entry deleted after this response, so subsequent polls fail with `expired_token`.
- `400 {"error": "authorization_pending"}` — still waiting.
- `400 {"error": "slow_down"}` — polled faster than `interval`; client must bump interval by 5s.
- `400 {"error": "access_denied"}` — user denied.
- `400 {"error": "expired_token"}` — Redis key absent (TTL elapsed or unknown device_code).
- `400 {"error": "invalid_grant"}` — wrong `grant_type`.

### `/device` page UX

1. Open `https://cook.md/device`. If not signed in → existing `/auth/desktop/code` email-code flow runs, then returns to `/device` with the original `?user_code=` preserved.
2. Form: single input. Auto-uppercases, accepts both `WDJBMJHT` and `WDJB-MJHT`. Hyphen stripped before lookup.
3. After submit: page renders `"CookCLI 0.30 (linux/docker) wants to access your CookCloud account as alice@example.com"` with **Approve** / **Deny** buttons (CSRF-protected `POST`).
4. After Approve: "All done — return to CookCLI" page. After Deny: "Authorization denied" page.

`client_name` is free-text from the client, HTML-escaped, max 80 chars. We accept the small phishing risk this entails (an attacker-supplied `"GitHub login"` string) because the surrounding page makes it clear *this is CookCloud authorizing CookCLI*. Mitigation via a registered-clients allow-list is deferred.

### JWT minting

Unchanged. On approve, the controller calls the existing `Token.encode(uid: user.id, email: user.email)`. 100-day expiry, same JWT secret, same shape. Existing token-refresh code (`/api/sessions/renew`) keeps working as-is.

### Rate limiting

Using whatever rack-attack-style middleware the repo conventionally uses:

- `POST /oauth/device/code`: 10/min/IP.
- `POST /oauth/device/token`: per-`device_code` `slow_down` enforcement (compares now vs `last_polled_at`), plus a global 60/min/IP hard cap.
- `POST /device` (user code submission): 10 per 15 min per session.

Failed user-code lookups return the same response as expired codes ("Code expired or invalid") so the form does not leak enumeration signal.

### Files added (cook.md)

- `app/controllers/oauth/device_controller.rb` — `/oauth/device/code`, `/oauth/device/token`
- `app/controllers/device_controller.rb` — `GET /device`, `POST /device/approve`, `POST /device/deny`
- `app/views/devices/show.html.erb`, `approve.html.erb`, `success.html.erb`, `denied.html.erb`, `expired.html.erb`
- `lib/device_flow.rb` — `DeviceFlow.create!`, `.find_by_user_code`, `.poll`, `.approve!`, `.deny!` (Redis I/O, encoding/decoding, state transitions)
- `spec/requests/oauth/device_spec.rb`, `spec/requests/device_spec.rb`
- `config/routes.rb` — five new routes

No new DB migration. No new Warden strategy (existing email-code sign-in is reused for the `/device` page).

---

## CookCLI (Rust)

### New shared module: `src/server/sync/device_flow.rs`

Used by both the HTTP handler (`POST /api/sync/login`) and the new `cook login` CLI command.

```rust
pub struct DeviceCodeResponse {
    pub device_code: String,               // server-side only
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}

pub async fn request_device_code(client_name: &str) -> Result<DeviceCodeResponse>;

/// Polls /oauth/device/token at `interval`, respects `slow_down`, surfaces
/// denial/expiry. Returns the JWT on success.
pub async fn poll_for_token(
    device_code: &str,
    interval: Duration,
    expires_at: Instant,
    cancel: CancellationToken,
) -> Result<String, DeviceFlowError>;

pub enum DeviceFlowError {
    AccessDenied,
    Expired,
    Cancelled,
    Network(reqwest::Error),
    BadResponse(String),
}
```

`client_name` is built as `format!("CookCLI {} ({}/{})", VERSION, OS, host_label)` where `host_label` is `"docker"` if `/.dockerenv` exists, `"cli"` for the CLI command, else the machine hostname. Best-effort; harmless if wrong.

### HTTP handler changes (`src/server/handlers/sync.rs`)

```
POST /api/sync/login
   1. Reject if already logged in or if a flow is already in progress.
   2. Call request_device_code(client_name).
   3. Store {device_code, expires_at, user_code, verification_uri,
            verification_uri_complete} in AppState::pending_device_flow
      (Mutex<Option<DeviceFlowState>>).
   4. Spawn a background tokio task that calls poll_for_token. On success →
      save session.json, start sync.
   5. Return {user_code, verification_uri, verification_uri_complete,
            expires_in} to the browser.

GET /api/sync/status
   Existing body PLUS, when a flow is pending:
     {pending_login: {user_code, verification_uri,
                      verification_uri_complete, expires_at}}.
   This lets the preferences page survive a reload mid-flow.

POST /api/sync/cancel_login   (new)
   Aborts the in-flight device flow via CancellationToken, clears state.
```

### Web UI (`templates/preferences.html`)

The Login button now opens an in-page card. No browser is spawned by the server.

```
┌──────────────────────────────────────────────┐
│  Sign in to CookCloud                        │
│                                              │
│  1. Open  https://cook.md/device  ↗          │
│  2. Enter this code:                         │
│                                              │
│         ┌──────────────────────┐             │
│         │     WDJB - MJHT      │  [Copy]     │
│         └──────────────────────┘             │
│                                              │
│  Expires in 14:58                            │
│                                              │
│  [Open cook.md/device]   [Cancel]            │
└──────────────────────────────────────────────┘
```

- "Open cook.md/device" is a plain `<a target="_blank" href="{verification_uri_complete}">` — the user's own browser handles it.
- "Copy" button: `navigator.clipboard.writeText(user_code)`.
- JS polls `/api/sync/status` every 2s (same cadence as today). On `logged_in: true` → page reload. On code expiry → "Code expired — try again" + re-enable button.

`verification_uri_complete` includes `user_code` in the URL for click-through convenience. RFC 8628 explicitly supports this. The URL is only ever shown to the signed-in user inside their own browser, so the phishing-resistance trade-off is acceptable.

### CLI: `cook login`

New `src/login.rs`. Reuses the same `device_flow` module.

```
$ cook login
First open https://cook.md/device in any browser and enter this code:

    WDJB-MJHT

(Press Enter to open it automatically in your default browser, or Ctrl-C to abort.)

Waiting for authorization... ⠋
✓ Logged in as alice@example.com
```

Behaviour:

1. `client_name = "CookCLI <ver> (<os>/cli)"`.
2. On Enter, calls `open::that(verification_uri_complete)`. If that fails (Docker exec, SSH), prints "Couldn't open browser automatically — please visit the URL above manually" and keeps waiting. **Browser-open failure is no longer fatal**, unlike today.
3. Spinner while polling; respects `interval` and `slow_down`.
4. On success: writes `~/.config/cook/session.json` exactly like the web flow.
5. On expiry / denial / Ctrl-C: clear error message, non-zero exit code, no half-written session file.

### CLI: `cook logout`

New `src/logout.rs`.

- Deletes `~/.config/cook/session.json`.
- No network call. JWT revocation is out of scope (cook.md JWTs are stateless; adding revocation would require new infrastructure).
- If no session exists: prints `"Not logged in."`, exits 0.

### Interaction with a running `cook server`

If `cook serve` is running while `cook login` writes a new `session.json`, the server does not pick it up until restart — same as today's behaviour. `cook login --help` notes this. A file-watcher in the server is a future improvement.

### Files changed (CookCLI)

**Added:**

- `src/server/sync/device_flow.rs`
- `src/login.rs`
- `src/logout.rs`

**Modified:**

- `src/server/handlers/sync.rs` — replace `browser_login_flow` & helpers; extend `LoginResponse`; add `cancel_login`.
- `src/server/mod.rs` (or wherever `AppState` lives) — add `pending_device_flow: Mutex<Option<DeviceFlowState>>`. Remove `login_in_progress: AtomicBool`.
- `src/args.rs`, `src/main.rs` — wire `Login(LoginArgs)` and `Logout(LogoutArgs)` Command variants.
- `templates/preferences.html` — replace login-button JS with the device-code card.

**Deleted:**

- In `src/server/handlers/sync.rs`: `browser_login_flow`, `wait_for_callback`, `handle_get_callback`, `extract_token`, `cors_origin`.

**Cargo.toml:**

- `open = "5.3"` stays (CLI `cook login` uses it; `cook server --open` already does too).
- `uuid` usage in `sync.rs` goes away (was only for CSRF state), but the crate likely stays for other reasons — verify before removing.

---

## Security

| Risk | Mitigation |
|---|---|
| **User-code brute force** | 8 chars × 32-char alphabet ≈ 10¹² combos; rate limit `POST /device` to 10/15 min/session. Failed lookups indistinguishable from expired codes. |
| **Device-code leakage** | Hashed (SHA-256) in Redis. Plain value only on the wire (TLS) and in CookCLI process memory. Never logged. |
| **Replay** | Device code deleted from Redis after first successful `/token` returns the JWT; reverse user_code index deleted on approve/deny. |
| **Phishing via `client_name`** | HTML-escaped, capped at 80 chars. Surrounding page makes ownership clear ("CookCloud → CookCLI"). 15-min TTL bounds the window. |
| **CSRF on approve/deny** | Standard Rails authenticity tokens on `POST /device/approve` and `/device/deny`. |
| **Polling abuse** | `slow_down` per `device_code` (bumps interval by 5s); 60/min/IP hard cap on `/oauth/device/token`. |
| **Session file on disk** | Existing `0o600` permissions in `SyncSession::save()` — unchanged. |
| **Token in logs** | No `tracing::*` call takes `device_code` or JWT as a field. Code review item. |
| **TLS** | Production uses `https://cook.md`; `COOK_ENDPOINT` env override allows http for dev. Document the risk of plaintext in non-dev. |
| **Cross-process collision** | CookCLI server uses `Mutex<Option<DeviceFlowState>>` to allow one in-flight flow; CLI is a separate process with its own state — each gets an independent `device_code`. |

---

## Testing

**cook.md (RSpec, real Redis in CI):**

- `Oauth::DeviceController#code`: issues unique codes; IP rate limit enforced.
- `Oauth::DeviceController#token`: full state machine — pending → approved → token issued and Redis key deleted; pending → denied → `access_denied`; expired/unknown → `expired_token`; rapid poll → `slow_down`; wrong `grant_type` → `invalid_grant`.
- `DeviceController`: unauthenticated GET redirects to sign-in with `user_code` preserved; case-insensitive and hyphen-tolerant user_code lookup; approve/deny update Redis correctly; one-shot enforcement of user_code.

**CookCLI:** the repo has no Rust test harness today (per `CONTRIBUTING.md`). This design does not introduce one. Instead, the implementation plan will include a manual test matrix:

- Web UI on native macOS / Linux / Docker / behind a reverse proxy.
- `cook login` on macOS / Linux / SSH session / `docker exec`.
- Cancellation paths: Ctrl-C, "Cancel" button, browser tab closed mid-flow.
- Expired-code path (let the 15 min run out).
- Denial path.
- Already-logged-in idempotency.

---

## Rollout

1. Ship cook.md changes. Endpoints go live as soon as the deploy lands.
2. Verify with a local CookCLI build pointed at staging via `COOK_ENDPOINT`.
3. Ship CookCLI release. Older CookCLI binaries continue working against the legacy `/auth/desktops?callback=...` flow.
4. After a deprecation window long enough for the fleet to update (target: ~6 months, tied to release cadence), remove the legacy `/auth/desktops` callback redirect on cook.md. The `Auth::DesktopsController` file goes away; `DesktopAuthCallback` concern can be deleted.

## Open questions

None at design time. All items raised during brainstorming were resolved:

- `client_name` is free text (no registered-clients list in v1).
- Device codes are single-use (consumed by the first successful `/token` poll).
- `verification_uri_complete` is included and the web UI links to it.
- The `open` crate stays in CookCLI for `cook login` (graceful-failure) and `cook server --open`.
- No feature flag on the cook.md side — the new endpoints ship live.
