# Plan — `cook server` authentication

* Upstream reference: [cooklang/cookcli#312](https://github.com/cooklang/cookcli/issues/312).
* Date: 2026-04-29 (revised 2026-04-30).

This plan is the single source of truth for implementing the
authentication feature. Each phase ships code AND the tests that lock
its behavior, so a session re-running this plan ends up with the same
shipped surface and a green test suite at every checkpoint.

## 1. Context

Today, `cook server` exposes every operation without access control: any
client that can reach the HTTP port can create, edit, delete recipes,
and mutate the shopping list and pantry. This is a problem for
self-hosters who want to publish their cookbook with read access while
keeping mutations private.

### 1.1 Requirement (issue summary)

| Operation | Anonymous | Authenticated |
|---|---|---|
| Browse / view recipes | ✅ | ✅ |
| Search | ✅ | ✅ |
| View shopping list / cart | ✅ | ✅ |
| Create / edit / delete recipe | ❌ 401 | ✅ |
| Add / modify / clear cart | ❌ 401 | ✅ |
| Any other write | ❌ 401 | ✅ |

Additional constraints:
- Persistent session (no re-login on refresh or browser restart).
- TOML configuration with a **mandatory** prefix tag on the password
  (`plain:` / `bcrypt:`), leaving room for additional algorithms.
- A single `--enable-auth` flag to opt into authentication. It is the
  source of truth; presence of `server.toml` alone never activates auth.

### 1.2 Non-goals

- Multi-user support: YAGNI, a single `username`/`password` is enough.
- Roles, fine-grained ACLs, per-recipe permissions.
- OAuth / OIDC / SSO (the existing CookCloud auth stays separate — see §3.5).
- Reverse-proxy auth integration (X-Remote-User…). Possible later.
- Rate-limiting / lockout. Documented as a known limitation.

### 1.3 Product decisions

1. **Default with no configuration is `Disabled`.** Backward compatible
   for upgrades. A console warning at startup invites operators to
   enable auth.
2. **Authentication activates only when `--enable-auth` is passed.** The
   presence of `server.toml` alone is intentionally not enough — keeps
   the resolution table trivial (one flag, one rule), avoids surprise
   activation when a stale config file is left behind, and removes the
   need for an opposite `--no-auth` override.
3. **`--enable-auth` without credentials is a startup error**, not a
   third "ReadOnly" mode. The two production states are the only ones
   needed: protected (with creds) or open (legacy). Anonymous users in
   `Authenticated` mode already get the read-only experience.

## 2. Threat model

- **Public network exposure**: an operator runs `cook server --host` on
  their LAN or behind a public tunnel. We want to prevent silent
  modification or deletion of recipes.
- **CSRF**: a third-party site loaded in the same browser as an
  authenticated session. The codebase already has `validate_same_origin`
  on recipe creation. We extend the same protection to all write routes
  (cookie `SameSite=Lax` + Origin/Referer check on non-GET methods).
- **Cookie theft**: we accept the local risk — no managed TLS in scope.
  Recommend HTTPS via reverse proxy in the docs.
- **Password brute-force**: no lockout in this first iteration; mention
  in the docs and apply a constant ~250 ms delay on the login handler
  regardless of outcome.

## 3. Architecture

### 3.1 Overview

Three new pieces under `src/server/`:

```
src/server/
├── auth/
│   ├── mod.rs          # AuthMode, AuthState, resolve_mode
│   ├── config.rs       # ServerConfig, AuthConfig, Password enum
│   ├── session.rs      # SessionStore (memory + JSON file), SessionId
│   ├── middleware.rs   # require_auth + extract_auth
│   └── handlers.rs     # POST /login, POST /logout, GET /login (page)
└── mod.rs              # build_state wires auth in; we add two
                        # sub-routers (write_api / write_ui) + middleware
```

### 3.2 Configuration

`server.toml` is loaded from (in order):
1. `--server-config <PATH>` (new optional CLI flag)
2. `./config/server.toml` (next to recipes — existing convention via
   `Context::aisle()` / `pantry()` at [src/main.rs:92](../../src/main.rs))
3. `~/.config/cook/server.toml` (or platform equivalent via
   `crate::global_file_path`)

The file name is `server.toml` (not `auth.toml`) on the assumption that
future server-level settings unrelated to auth will live in the same
file under their own sections. The CLI flag `--server-config` matches
that intent.

Schema:

```toml
[auth]
username = "admin"

# Password value with a MANDATORY prefix tag. The prefix selects the
# algorithm:
#   plain:<password>      → direct comparison
#   bcrypt:$2b$12$...     → bcrypt verification
#
# A missing or unknown prefix is a fatal error at startup.
#
# The prefix scheme is the extension point for future algorithms
# (argon2, scrypt, …) without breaking existing configs.
password = "plain:changeme"
```

Session TTL is hardcoded to 7 days. If a real need for configurability
appears, add a field then — no point in shipping a knob no one asked
for.

Rust types:

```rust
// src/server/auth/config.rs
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub auth: Option<AuthConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub username: String,
    pub password: String, // raw prefixed value, parsed at startup
}

pub enum Password {
    Plain(String),
    Bcrypt(String),
}

impl Password {
    pub fn parse(value: &str) -> Result<Self> {
        // Strict prefix dispatch — no auto-detection.
        match value.split_once(':') {
            Some(("plain", rest))  => Ok(Self::Plain(rest.into())),
            Some(("bcrypt", rest)) => {
                bcrypt::HashParts::from_str(rest)?; // validate shape
                Ok(Self::Bcrypt(rest.into()))
            }
            Some((other, _)) => bail!("unknown password algorithm: {other}"),
            None => bail!("password must start with an algorithm prefix (plain: or bcrypt:)"),
        }
    }

    pub fn verify(&self, candidate: &str) -> bool { ... }
    pub fn algorithm(&self) -> &'static str { ... }
}
```

A simple enum is enough for the two algorithms shipped today; adding
`Argon2` later is a new variant + matching arms — no trait/factory
indirection needed.

**Initial iteration**: ship `plain` + `bcrypt` only.

**Mandatory startup warning** when the resolved algorithm is `plain`:

```
WARN: Authentication is using plaintext password storage. Use only for
      local development. For production, hash your password with
      `cook server hash-password` and store the bcrypt result.
```

### 3.3 Server state (`AuthState`)

Extend `AppState`:

```rust
pub struct AppState {
    // ... existing fields
    pub auth: Arc<AuthState>,
}

pub struct AuthState {
    pub mode: AuthMode,
    pub config: Option<AuthConfig>,    // None if no creds loaded
    pub sessions: Arc<RwLock<SessionStore>>,
}

pub enum AuthMode {
    /// `--enable-auth` + creds → write routes require a valid session.
    /// Anonymous users see read routes. The nav shows a "Sign in" link.
    Authenticated,

    /// `--enable-auth` absent → legacy behavior (default). All routes
    /// open, regardless of whether `server.toml` exists. Console
    /// warning at startup. This mode preserves backward compatibility.
    Disabled,
}
```

`--enable-auth` without credentials aborts at startup with a clear
error message ("--enable-auth requires credentials in server.toml"),
so a third `ReadOnly` variant is unnecessary.

### 3.4 Sessions

- **Format**: opaque token, 32 random bytes encoded as base64url. Use
  `rand::rngs::OsRng` (add `rand` to deps); `base64` is currently
  gated behind the `sync` feature, ungate it or use `data-encoding`.
- **In-memory store**: `HashMap<SessionId, SessionRecord>` behind an
  `RwLock`.
- **Persistence**: JSON file at `~/.config/cook/server-sessions.json`
  via `crate::global_file_path`. Read at startup, written after every
  mutation. File permissions: `0600` on Unix (skipped on Windows).
  Contents:

  ```json
  {
    "sessions": [
      {
        "id": "<base64url>",
        "username": "admin",
        "created_at": "2026-04-29T10:15:00Z",
        "expires_at": "2026-05-29T10:15:00Z"
      }
    ]
  }
  ```

  Tests override the path via the `COOK_SESSION_FILE` env var (no
  hidden CLI flag).

- **Cookie**: `cook_session=<id>; Path=<prefix>/; HttpOnly;
  SameSite=Lax; Max-Age=604800` (7 days). **No** `Secure` flag by
  default (users often run plain HTTP locally). Document that public
  exposure needs a TLS reverse proxy. The `Path` must follow
  `state.url_prefix`: when the server runs behind a reverse proxy at
  `/cook`, `Path=/cook/` prevents the cookie from leaking to other
  apps on the same origin.
- **Invalidation**: expired sessions are purged on load and on each
  check (lazy). Logout removes the session server-side AND sends a
  cookie with `Max-Age=0`.

### 3.5 Relationship to existing CookCloud sync

The `sync` feature implements an OAuth-like login flow against
CookCloud and is fully independent. The two coexist:
- Local auth protects access **to the HTTP server** (who can write to
  this server?).
- CookCloud sync authenticates **against the external service** (who
  syncs to the cloud?).

`/api/sync/login` and `/api/sync/logout` mutate server state and are
both protected by `require_auth` so an anonymous LAN visitor cannot
bind/unbind the server's CookCloud association. `/login` and `/logout`
(new) are the local-server routes from this plan.

### 3.6 Middleware

Two axum functions:

```rust
// src/server/auth/middleware.rs

/// Attaches an `AuthIdentity` (Anonymous | User { username }) extension
/// to the request. Used on read routes to adapt the UI.
pub async fn extract_auth(...) -> Response { ... }

/// Returns 401 (API) or redirects to /login?next=… (UI) when the
/// request is not authenticated. Enforces Origin/Referer check on
/// non-GET methods.
pub async fn require_auth(...) -> Response { ... }
```

**Mounting order matters.** Apply `require_auth` to the inner write
sub-routers (`write_api`, `write_ui`) **before** they get merged into
`Router::nest("/api", …)`. This way `req.uri().path()` inside
`require_auth` is the full request path (`/api/recipes/foo.cook`, not
`/recipes/foo.cook`) and the API-vs-UI branch
(`starts_with("/api/")`) works without `OriginalUri`. Mounting in the
right order eliminates a class of bugs entirely; no regression test is
needed for it.

Cookie parsing: parse `header::COOKIE` directly with a small helper —
avoids adding `axum-extra` for one struct.

The existing `validate_same_origin` ([src/server/ui.rs:884](../../src/server/ui.rs))
moves into the auth module and is reused by the middleware.

## 4. CLI surface

### 4.1 `ServerArgs` flags

```rust
pub struct ServerArgs {
    // ... existing

    /// Enable authentication. Without this flag the server runs open
    /// (legacy behavior), even if `server.toml` is present.
    #[arg(long)]
    enable_auth: bool,

    /// Path to the server config TOML (auth credentials).
    /// Default search: ./config/server.toml then ~/.config/cook/server.toml.
    #[arg(long, value_hint = clap::ValueHint::FilePath)]
    server_config: Option<Utf8PathBuf>,
}
```

The session-file path is overridden via the `COOK_SESSION_FILE` env
var (used by the integration tests for hermeticity), not via a hidden
CLI flag.

### 4.2 Mode resolution

Single source of truth: the flag.

| `--enable-auth` | creds present | Resolved mode |
|---|---|---|
| ✓ | ✓ | `Authenticated` |
| ✓ | ✗ | **startup error** ("--enable-auth requires credentials in server.toml") |
| – | ✓ | `Disabled` (server.toml present but ignored — operator must opt in via the flag) |
| – | ✗ | `Disabled` (**default**, console warning) |

### 4.3 Console output at startup

```
Authentication: Authenticated (admin, bcrypt)
Session store: ~/.config/cook/server-sessions.json (3 active)
```

```
Authentication: Disabled (anyone can write). Drop a `server.toml` with [auth] credentials and pass --enable-auth to enable access control.
```

```
Authentication: Disabled (server.toml present but --enable-auth was not passed — anyone can write)
WARN: Authentication is using plaintext password storage. ...
```

### 4.4 `hash-password` helper subcommand

So users can generate a hash without an external dependency:

```bash
cook server hash-password
# Interactive prompt (no echo) → prints "bcrypt:$2b$12$..."
```

The prefix is part of the output so it can be pasted directly into
`password = "..."` in `server.toml`.

When stdin is **not** a TTY (`std::io::IsTerminal::is_terminal(&stdin)`
returns false), the command reads a single line from stdin and skips
the confirmation prompt. This is what the integration tests use; no
hidden CLI flag is needed.

Implementation: turn `Server(server::ServerArgs)` into a subcommand
with two variants — `Server(ServerRunArgs)` and
`ServerHashPassword(HashPwArgs)`.

Initial scope is bcrypt only. `--algorithm plain` would be a follow-up
if a real script use case appears.

## 5. Route classification

### 5.1 API (`/api/...`)

| Route | Method | Category | Notes |
|---|---|---|---|
| `/api/recipes` | GET | read | |
| `/api/recipes/raw/*path` | GET | read | |
| `/api/recipes/*path` | GET | read | |
| `/api/recipes/*path` | PUT | **write** | recipe_save |
| `/api/recipes/*path` | DELETE | **write** | recipe_delete |
| `/api/menus` | GET | read | |
| `/api/menus/*path` | GET | read | |
| `/api/search` | GET | read | |
| `/api/stats` | GET | read | |
| `/api/reload` | GET / POST | read | refreshes the cache, no FS write — classified **read** |
| `/api/shopping_list` | POST | read | computes from request body, no mutation |
| `/api/shopping_list/items` | GET | read | |
| `/api/shopping_list/checked` | GET | read | |
| `/api/shopping_list/events` | GET | read | SSE |
| `/api/shopping_list/add` | POST | **write** | |
| `/api/shopping_list/add_menu` | POST | **write** | |
| `/api/shopping_list/remove` | POST | **write** | |
| `/api/shopping_list/clear` | POST | **write** | |
| `/api/shopping_list/check` | POST | **write** | |
| `/api/shopping_list/uncheck` | POST | **write** | |
| `/api/shopping_list/compact` | POST | **write** | |
| `/api/pantry` | GET | read | |
| `/api/pantry/expiring` | GET | read | |
| `/api/pantry/depleted` | GET | read | |
| `/api/pantry/add` | POST | **write** | |
| `/api/pantry/:section/:name` | PUT | **write** | |
| `/api/pantry/:section/:name` | DELETE | **write** | |
| `/api/sync/status` | GET | read | `sync` feature |
| `/api/sync/login` | POST | **write** | `sync` feature |
| `/api/sync/logout` | POST | **write** | `sync` feature |
| `/ws/lsp` | GET (upgrade) | **write** | LSP editor — only meaningful when editing |

### 5.2 UI (`/...`)

| Route | Method | Category | Notes |
|---|---|---|---|
| `/` | GET | read | |
| `/directory/*path` | GET | read | |
| `/recipe/*path` | GET | read | |
| `/shopping-list` | GET | read | the page itself is read |
| `/pantry` | GET | read | same |
| `/preferences` | GET | read | |
| `/edit/*path` | GET | **write** | redirects to `/login?next=...` if anon |
| `/new` | GET | **write** | same |
| `/new` | POST | **write** | `create_recipe` |
| `/login` | GET / POST | (public) | new auth route |
| `/logout` | POST | (auth) | new auth route |

### 5.3 Static assets

`/static/*` and `/api/static/*` stay public (assets, recipe images).

## 6. Frontend

### 6.1 Template context

Extend each `*Template` in `src/server/templates.rs` with a shared
field:

```rust
pub struct AuthContext {
    pub auth_enabled: bool,        // true when AuthMode::Authenticated
    pub signed_in: bool,
    pub username: Option<String>,
}
```

Pass `AuthContext` as a direct field on each template struct (no
shared trait — the repo doesn't have one and Askama makes that
ergonomic). The risk of forgetting one struct is caught by the
rendered-HTML tests in Phase 5.

### 6.2 Template changes

Concretely hide / adapt:

- `templates/base.html` (nav): right-side `🔒 Sign in` link
  (anonymous + auth_enabled) or `👤 username | Logout` (signed in).
- `templates/recipes.html`: the `+ New` button → only when signed_in
  or auth disabled.
- `templates/recipe.html`: `Edit` link.
- `templates/menu.html`: `Edit` link.
- `templates/shopping_list.html`: `Clear list` button hidden;
  check/uncheck checkboxes disabled.
- `templates/pantry.html`: add modal, edit/delete buttons.
- `templates/edit.html` and `templates/new.html`: protected at the
  server level (middleware redirects), so no template-level hiding
  needed.
- New minimal `templates/login.html` (username + password form, hidden
  `next` field, generic "Invalid credentials" error).

### 6.3 Strategy for shopping list / pantry pages

The pages themselves stay visible. The JS that calls write endpoints
must handle 401 with a toast "Sign in to modify" linking to `/login`.

### 6.4 i18n

Use English strings inline for the auth-specific keys. Add to
`locales/*/common.ftl` only when a translation contribution lands —
keeps the diff small and avoids untranslated keys polluting all
locales.

## 7. Test stack

### 7.1 Tier conventions

* **Unit tests** — `#[cfg(test)] mod tests` co-located with code, runs
  under `--no-default-features` (no sync/self-update gating). Cover
  pure helpers with no axum involved.
* **Integration tests** — `tests/server_auth_test.rs` driving the
  binary via `assert_cmd::Command::cargo_bin("cook")`. Shared fixtures
  in [tests/common/mod.rs](../../tests/common/mod.rs).
* **End-to-end (Playwright)** — **not added in this scope.** The HTTP
  integration tests cover behavior; UI rendering is verified via
  rendered-HTML substring assertions in the integration suite. A
  Playwright `auth` project is a follow-up if regressions appear.

### 7.2 Test infrastructure

In [tests/common/mod.rs](../../tests/common/mod.rs):

```rust
pub fn pick_free_port() -> u16 {
    // Bind to :0, read the port, drop the listener, hand the port to
    // the child process. Avoids flaky CI from hard-coded ports.
}

pub struct ServerHandle {
    child: std::process::Child,
    pub base_url: String,
    pub url_prefix: String,
    _temp_dirs: Vec<TempDir>,
}

pub struct ServerSpawn<'a> {
    extra_args: Vec<&'a str>,
    recipes: TempDir,
    auth: Option<(String, String)>,        // (username, password)
    enable_auth: bool,
    session_path: Option<Utf8PathBuf>,
}

impl<'a> ServerSpawn<'a> {
    pub fn new(recipes: TempDir) -> Self;
    /// Sets creds AND --enable-auth (Authenticated mode).
    pub fn with_auth(self, user: &str, pw: &str) -> Self;
    /// Sets creds WITHOUT --enable-auth (asserts the flag-is-truth rule).
    pub fn with_creds_only(self, user: &str, pw: &str) -> Self;
    /// Reuse a session file across spawns (persistence test).
    pub fn with_session_path(self, p: Utf8PathBuf) -> Self;
    pub fn arg(self, s: &'a str) -> Self;
    pub fn spawn(self) -> Result<ServerHandle>;
}
```

A single builder replaces the four `spawn_*` functions of earlier
drafts. `COOK_SESSION_FILE` env var is set on the child to override
the session-file path when tests need to share it across restarts.

Implementation notes:
- Wait loop: poll `GET /api/recipes` with a 5-second timeout. Bail
  with the captured stderr if the server fails to start.
- `Drop` kills the child (`child.kill()` + `child.wait()`); never leak
  processes between tests.
- Default `--server-config` to a guaranteed-nonexistent path unless
  the caller already supplied one. Without this, the test would pick
  up the developer's `~/.config/cook/server.toml` (if any), making
  behavior non-hermetic.
- HTTP client: `reqwest::blocking::Client` with `.cookie_store(true)`
  for session-aware tests. `reqwest` is already a dev-dependency.

### 7.3 Test conventions

- **bcrypt cost = 4 in tests.** `bcrypt::hash(..., 12)` takes ~250 ms
  on dev hardware; cost=4 is ~5 ms.
- **Stable substrings, not snapshots, for HTML assertions.** Snapshots
  break on every i18n / template tweak.

### 7.4 Optional: in-process router builder (deferred)

Today `run_server` couples router assembly to `tokio::main` and
`TcpListener::bind`. Extracting a `pub(crate) fn build_router(state:
Arc<AppState>) -> Router<()>` would let some tests run in-process with
`tower::ServiceExt::oneshot` and skip the subprocess overhead.
**Decision deferred to implementation.**

## 8. Implementation phases

Each phase ships code AND the tests that lock its behavior. At the end
of each phase, `cargo fmt && cargo clippy && cargo test` should pass
cleanly, and the next phase starts from a green tree.

### Phase 1 — Skeleton: config, mode, CLI flags

**Code:**

1. Add `bcrypt` and `rand` crates to `Cargo.toml`.
2. Create `src/server/auth/{mod.rs, config.rs}` with:
   - `ServerConfig`, `AuthConfig`, `Password::parse`, `load_server_config`
   - `AuthMode` enum (`Authenticated` / `Disabled`)
   - `resolve_mode(flag: bool, creds: Option<&AuthConfig>) -> Result<AuthMode>`
     — returns an error when `flag && creds.is_none()`.
3. Add `Context::server_config()` in `src/main.rs` (mirrors `aisle()`
   and `pantry()`).
4. Extend `ServerArgs` with `--enable-auth` and `--server-config`
   (see §4.1).
5. Resolve `AuthMode` in `build_state`; log the result at startup
   (see §4.3).
6. **No HTTP-visible behavior change yet** — just plumbing.

**Tests (unit, in `src/server/auth/`):**

| File | Test | Verifies |
|---|---|---|
| `config.rs` | `parse_password_round_trip` | `plain:foo` accepts `foo`; `bcrypt:<hash>` accepts the original |
| `config.rs` | `parse_password_rejects_bad_prefix` | bare value, unknown prefix, malformed bcrypt hash → error |
| `config.rs` | `load_server_config_minimal` | absent → `Ok(None)`; valid `[auth]` → `Ok(Some(_))`; malformed TOML → error; `[other]`-only → `Ok(Some(ServerConfig { auth: None }))` |
| `mod.rs` | `resolve_mode_table` | the four lines of §4.2 (param-table style) |

**Checkpoint:** `cargo test --no-default-features` passes; `cook server`
with no flag prints the new mode line at startup but otherwise behaves
identically to before.

### Phase 2 — Sessions and middleware (no router wiring yet)

**Code:**

1. Create `src/server/auth/session.rs`:
   - `SessionId` (256-bit token via `rand::rngs::OsRng`, base64url)
   - `SessionStore` with memory + JSON file persistence, lazy
     expiration purge on load and on each check
   - `0600` permissions on Unix
   - Session-file path resolution: `COOK_SESSION_FILE` env var if set,
     otherwise `crate::global_file_path("server-sessions.json")`.
2. Create `src/server/auth/middleware.rs`:
   - `extract_auth` (read cookie, attach `AuthIdentity` extension)
   - `require_auth` (returns 401 / login redirect, enforces
     Origin/Referer CSRF check on non-GET methods)
   - Pure helper: `parse_session_cookie`. Move
     `validate_same_origin` from [src/server/ui.rs:884](../../src/server/ui.rs)
     into this module so it's reused.
3. **Not yet wired into the router.**

**Tests (unit, in `src/server/auth/`):**

| File | Test | Verifies |
|---|---|---|
| `session.rs` | `roundtrip_through_disk` | create → save → load → existing session recovered; expired entry purged |
| `middleware.rs` | `parse_session_cookie_basic` | matching cookie returned among others; absent header → None; empty value → None |
| `middleware.rs` | `validate_same_origin_basic` | matching Origin → true; cross-origin → false; missing both → false; matching Referer fallback → true |

**Checkpoint:** middleware compiles in isolation; session store
roundtrips through disk. Still no HTTP behavior change.

### Phase 3 — Login, logout, hash-password subcommand

**Code:**

1. `src/server/auth/handlers.rs`:
   - `GET /login` (renders the form; redirects home when mode is
     `Disabled`)
   - `POST /login` (verifies creds, creates session, sets cookie,
     redirects to `next`; 250 ms constant delay regardless of outcome)
   - `POST /logout` (clears cookie + server-side session)
   - Pure helpers: `sanitize_next` (open-redirect guard),
     `build_session_cookie`, `clear_session_cookie`
2. `templates/login.html` (minimal; mirrors `templates/new.html`).
3. `cook server hash-password` subcommand
   (`Server::HashPassword(HashPwArgs)` variant). Auto-detects non-TTY
   stdin via `IsTerminal` and skips the confirmation prompt then.

Add the `tests/common/mod.rs` infrastructure described in §7.2
(`pick_free_port`, `ServerHandle`, `ServerSpawn` builder).

**Tests:**

*Unit (in `src/server/auth/handlers.rs`):*

| Test | Verifies |
|---|---|
| `sanitize_next_basic` | local path kept; absolute URL / protocol-relative / backslash-escape rejected; URL-encoded local path decoded; `None` → fallback `<prefix>/` |
| `sanitize_next_under_prefix` | local path under url_prefix kept unchanged |
| `build_session_cookie_attributes` | output contains `HttpOnly`, `SameSite=Lax`, the right `Path` (with and without prefix), `Max-Age=604800` |
| `clear_session_cookie_max_age_zero` | sanity |

*Integration (in `tests/server_auth_test.rs`, hash-password block):*

| Test | Verifies |
|---|---|
| `hash_password_outputs_valid_bcrypt` | piped stdin `"foo"` → stdout starts with `bcrypt:` and verifies against `"foo"` |
| `hash_password_rejects_empty_input` | empty stdin → error exit code |

**Checkpoint:** `cook server hash-password` works end-to-end.
`ServerHandle` infrastructure is in place, ready for Phase 4.
Login/logout HTTP routes exist but writes are still open (no
middleware wired yet).

### Phase 4 — Wire middleware into the router

This is the phase where authentication actually gates writes. Most of
the HTTP integration tests live here.

**Code:**

In `src/server/mod.rs::run`:

1. Split the API into `read_api` (no `require_auth`) and `write_api`
   (with `require_auth` applied **inside the sub-router**, before the
   `nest`).
2. Same for UI: `read_ui` vs `write_ui`.
3. Apply `extract_auth` globally (every route).
4. Final router shape:

```rust
let write_api = write_api()
    .layer(from_fn_with_state(state.clone(), require_auth));
let write_ui = write_ui()
    .layer(from_fn_with_state(state.clone(), require_auth));

let api = Router::new().merge(read_api(&state)?).merge(write_api);
let ui  = Router::new().merge(read_ui()).merge(write_ui);

let inner = Router::new()
    .nest("/api", api)
    .merge(ui)
    .merge(auth_routes())            // /login, /logout
    .route("/static/*file", ...)
    .nest_service("/api/static", ...);
```

Mounting `require_auth` *before* the `nest("/api", …)` means
`req.uri().path()` inside the middleware is the full request path, so
the API-vs-UI branch works without `OriginalUri`.

5. Resolve the session-file path: `COOK_SESSION_FILE` env var if set,
   otherwise `global_file_path("server-sessions.json")`.

**Tests (integration, in `tests/server_auth_test.rs`):**

*Mode `Disabled`:*

| Test | Verifies |
|---|---|
| `disabled_anonymous_writes_succeed` | spawn without flags AND spawn with creds-but-no-flag → PUT `/api/recipes/x.cook` 200, POST `/api/shopping_list/clear` 200 (covers both lines of "Disabled" in §4.2) |
| `disabled_login_page_redirects_home` | GET `/login` → 303 to `/` |

*Mode `Authenticated`, anonymous:*

| Test | Verifies |
|---|---|
| `anonymous_reads_pass` | GET `/api/recipes`, GET `/recipe/x.cook` → 200 |
| `anonymous_api_writes_return_401_json` | PUT `/api/recipes/x.cook`, PUT `/api/pantry/dairy/milk`, POST `/api/shopping_list/clear`, DELETE `/api/recipes/x.cook` → 401 + `{"error":"unauthorized"}` |
| `anonymous_ui_writes_redirect_to_login` | GET `/edit/x.cook` → 303 with `Location: /login?next=%2Fedit%2Fx.cook`; GET `/new` → 303 with `next=%2Fnew` |

*Login flow:*

| Test | Verifies |
|---|---|
| `login_good_creds_sets_cookie_and_redirects` | POST `/login` (Origin OK) → 303 + `Set-Cookie: cook_session=…; HttpOnly; SameSite=Lax; Path=/; Max-Age=604800`; `next=/edit/foo` honored |
| `login_bad_creds_redirect_with_error` | bad password → 303 to `/login?error=…&next=…`, no `Set-Cookie` |
| `login_constant_delay` | bad creds AND unknown user both ≥250 ms (single test, both paths) |
| `login_csrf_blocks` | POST `/login` without Origin OR cross-origin → 403 |
| `unknown_session_cookie_is_anonymous` | injected `cook_session=garbage` → PUT 401 |
| `next_open_redirect_blocked` | `next=//evil.com` → `Location: /` |
| `authenticated_writes_succeed` | login → PUT 200 + file written under temp dir; POST `/api/shopping_list/clear` → 200 |
| `logout_clears_cookie_and_session` | login → POST `/logout` (Origin OK) → `Max-Age=0`; subsequent PUT → 401; logout without Origin → 403 |

*CSRF on writes:*

| Test | Verifies |
|---|---|
| `authenticated_writes_csrf_check` | session OK without Origin → 403; cross-origin → 403; matching Referer-only → 200; GETs unchecked → 200 |

*Session persistence:*

| Test | Verifies |
|---|---|
| `session_survives_server_restart` | login → kill → respawn pointing at the same `COOK_SESSION_FILE` → PUT 200 with the original cookie |
| `expired_session_purged_on_load` | seed file with `expires_at` in the past → 0 active sessions |

*`--url-prefix`:*

| Test | Verifies |
|---|---|
| `prefixed_anonymous_put_returns_401` | spawn with `--url-prefix /cook` → PUT `/cook/api/recipes/x.cook` → 401 |
| `prefixed_login_works_end_to_end` | UI redirect path includes `/cook`; cookie `Path=/cook/`; login under `/cook/` → 303 with cookie |

**Checkpoint:** All HTTP-level behavior is locked. Manual smoke
testing through curl is no longer needed.

### Phase 5 — Templates and JS

**Code:**

1. `AuthContext` on each Askama struct. See §6.1.
2. `templates/base.html`: nav adapts (Sign in / username · Sign out).
3. `templates/recipes.html`, `recipe.html`, `menu.html`: hide write
   actions when anonymous.
4. `templates/shopping_list.html`, `pantry.html`: JS handles 401 by
   showing a "Sign in to modify" toast linking to `/login`.
5. `templates/login.html` is wired into the GET handler from Phase 3.

**Tests (integration, rendered-HTML substrings):**

| Test | Verifies |
|---|---|
| `anonymous_home_shows_signin_link` | GET `/` body contains `Sign in`; no `Logout` |
| `anonymous_recipes_hides_new_button` | GET `/` body does not contain the `+ New` button |
| `signed_in_home_shows_user_chip` | GET `/` after login contains `admin` and `Logout` |

**Checkpoint:** UI matches the resolved mode.

### Phase 6 — Documentation

Update [docs/server.md](../../docs/server.md) with an "Authentication"
section: modes table, `server.toml` example, `--enable-auth` /
`--server-config` examples, `cook server hash-password` reference,
HTTPS / reverse-proxy recommendation.

Add a one-line pointer in the README server section.

`CLAUDE.md` updates are a follow-up if the implementation diverges
from this plan.

**Checkpoint:** docs published; feature shippable.

## 9. Backward compatibility

- Existing users with no config and no flag stay in `Disabled`
  → **legacy behavior preserved**, no action required. A console
  warning at startup invites them to enable auth.
- CI / scripts that POST anonymously to the API keep working unchanged.
- Users who want to enable security drop a `server.toml` with creds
  AND pass `--enable-auth` → switch to `Authenticated`. Passing
  `--enable-auth` without any creds is a startup error pointing them
  to the docs.
- No data migration needed (no DB schema).
- The changelog should clearly promote enabling auth for
  network-exposed setups, without making it mandatory.

## 10. Security implementation checklist

- [ ] bcrypt cost ≥ 12 in `hash-password` (configurable later).
- [ ] Mandatory console warning at startup when the resolved algorithm
      is `plain`.
- [ ] Constant delay on `POST /login` (~250 ms) regardless of whether
      the user exists.
- [ ] Cookie `HttpOnly`, `SameSite=Lax`, no `Secure` by default
      (documented).
- [ ] Cookie `Path` aligned with `state.url_prefix` (see §3.4).
- [ ] 256-bit tokens generated via `rand::rngs::OsRng`.
- [ ] CSRF check (Origin/Referer) on every protected non-GET method —
      promote the existing `validate_same_origin` into middleware.
- [ ] `0600` permissions on `server-sessions.json` (Unix).
- [ ] Logging: NEVER log username/password/hash. Log
      `successful login` / `failed login`.
- [ ] Generic login response ("Invalid credentials"); no distinction
      between wrong user and wrong password.

## 11. Definition of done

The feature is shippable when:

- [ ] `cook server` with no config starts in `Disabled` (legacy
      preserved) with a console warning.
- [ ] `cook server --enable-auth` without creds **fails fast** at
      startup with a clear error.
- [ ] `cook server` with a valid `server.toml` but **without**
      `--enable-auth` stays in `Disabled` (config alone is ignored —
      the flag is the source of truth).
- [ ] `server.toml` with `password = "plain:..."` works and emits the
      plaintext-storage warning.
- [ ] `server.toml` with `password = "bcrypt:$2b..."` works without
      any warning.
- [ ] `server.toml` with a password missing the prefix (e.g. bare
      `$2b...` or bare `mypassword`) fails fast at startup with a
      clear error.
- [ ] With creds in `server.toml` + `--enable-auth`, web login works,
      and the cookie survives both server AND browser restarts (TTL 7d).
- [ ] Every route in the §5.1 / §5.2 tables returns 401 (API) / login
      redirect (UI) under `Authenticated` + anonymous.
- [ ] CSRF is tested on at least one POST route (Phase 4 covers four).
- [ ] Cookie `Path` is correct when `--url-prefix` is used.
- [ ] `cook server hash-password` produces `bcrypt:...`; non-TTY stdin
      works for scripts.
- [ ] `cargo test` passes locally and on CI; no port conflicts after
      5 sequential runs of the integration suite.
- [ ] `cargo fmt` and `cargo clippy --all-targets -- -D warnings` are
      clean (see [CLAUDE.md](../../CLAUDE.md) — Before Creating a PR).
- [ ] [docs/server.md](../../docs/server.md) is up to date.
