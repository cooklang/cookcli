# Plan — Read-only `cook server` with persistent authenticated write access

* Upstream reference: [cooklang/cookcli#312](https://github.com/cooklang/cookcli/issues/312).
* Date: 2026-04-29.

## 1. Context

Today, `cook server` exposes every operation without access control: any
client that can reach the HTTP port can create, edit, delete recipes, and
mutate the shopping list and pantry. This is a problem for self-hosters who
want to publish their cookbook in read-only mode while keeping mutations
private.

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
- TOML configuration, password stored with a **mandatory** prefix tag
  (`plain:` / `bcrypt:`). The prefix scheme leaves room for additional
  algorithms (e.g. argon2) without a config-format break.
- `--auth` / `--no-auth` flags to force / disable explicitly.

**Product decision (deviation from the issue):** to preserve backward
compatibility, the default behavior with no configuration is **not**
`ReadOnly` but `Disabled` (legacy behavior + console warning). Users
explicitly opt into security by dropping a `server.toml` file or passing
`--auth`. See §4 for the resolution table.

### 1.2 Non-goals

- Multi-user support: YAGNI, a single `username`/`password` is enough. The
  TOML schema stays a flat object (no `[[auth.users]]`).
- Roles, fine-grained ACLs, per-recipe permissions.
- OAuth / OIDC / SSO (the existing CookCloud auth stays separate — see §3.5).
- Reverse-proxy auth integration (X-Remote-User…). Possible later.
- Rate-limiting / lockout. Documented as a known limitation.

## 2. Threat model

- **Public network exposure**: an operator runs `cook server --host` on
  their LAN or behind a public tunnel. We want to prevent silent
  modification or deletion of recipes.
- **CSRF**: a third-party site loaded in the same browser as an
  authenticated session. The codebase already has `validate_same_origin`
  on recipe creation ([src/server/ui.rs:884](../../src/server/ui.rs)).
  We extend the same protection to all write routes (cookie
  `SameSite=Lax` + Origin/Referer check on non-GET methods).
- **Cookie theft**: we accept the local risk — no managed TLS in scope.
  Recommend HTTPS via reverse proxy in the docs.
- **Password brute-force**: no lockout in this first iteration; mention in
  the docs and apply a constant delay on the login handler (~250 ms sleep
  regardless of outcome) to slow online attacks.

## 3. Architecture

### 3.1 Overview

Three new pieces under `src/server/`:

```
src/server/
├── auth/
│   ├── mod.rs          # AuthConfig, AuthState, helpers, AuthMode enum
│   ├── config.rs       # TOML loading/parsing, password verifier factory
│   ├── session.rs      # SessionStore (memory + JSON file), SessionId
│   ├── middleware.rs   # require_auth + extract_auth (axum middleware)
│   └── handlers.rs     # POST /login, POST /logout, GET /login (page)
└── mod.rs              # build_state wires auth in; we add two
                        # sub-routers (write_api / write_ui) + middleware
```

### 3.2 Configuration

`server.toml` is loaded from (in order):
1. `--auth-config <PATH>` (new optional CLI flag)
2. `./config/server.toml` (next to recipes — existing convention via
   `Context::aisle()` / `pantry()` at [src/main.rs:92](../../src/main.rs))
3. `~/.config/cook/server.toml` (or platform equivalent via
   `crate::global_file_path`)

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
# The prefix scheme is the extension point for future algorithms (argon2,
# scrypt, …) without breaking existing configs.
password = "plain:changeme"

# Optional: session cookie lifetime in days.
# session_ttl_days = 7 (default)
```

Rust types and evolution-friendly architecture:

```rust
// src/server/auth/config.rs
#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub username: String,
    pub password: String,                  // raw prefixed value
    #[serde(default = "default_session_ttl_days")]
    pub session_ttl_days: u32,
}

fn default_session_ttl_days() -> u32 { 7 }

// Trait + factory so adding a new algorithm doesn't require touching
// middleware or handlers.
pub trait PasswordVerifier: Send + Sync {
    fn verify(&self, candidate: &str) -> bool;
    fn algorithm(&self) -> &'static str;   // "plain" | "bcrypt"
}

pub fn parse_password(value: &str) -> Result<Box<dyn PasswordVerifier>> {
    // Strict prefix dispatch — no auto-detection.
    match split_prefix(value) {
        Some(("plain", rest))  => Ok(Box::new(PlainPassword(rest.into()))),
        Some(("bcrypt", rest)) => Ok(Box::new(BcryptPassword::new(rest)?)),
        Some((other, _))       => bail!("unknown password algorithm: {other}"),
        None                   => bail!("password must start with an algorithm prefix (plain: or bcrypt:)"),
    }
}
```

**Initial iteration**: ship `plain` + `bcrypt` only. The prefix scheme
makes it straightforward to add another algorithm later (argon2, scrypt,
…) by registering a new variant — but no placeholder code is added now.

**Mandatory startup warning** when the resolved algorithm is `plain`:

```
WARN: Authentication is using plaintext password storage. Use only for
      local development. For production, hash your password with
      `cook server hash-password` and store the bcrypt result.
```

### 3.3 Server state (`AuthState`)

Extend `AppState` ([src/server/mod.rs:354](../../src/server/mod.rs)):

```rust
pub struct AppState {
    // ... existing fields
    pub auth: Arc<AuthState>,
}

pub struct AuthState {
    pub mode: AuthMode,
    pub config: Option<AuthConfig>,    // None if --no-auth or no creds
    pub sessions: Arc<RwLock<SessionStore>>,
}

pub enum AuthMode {
    /// `--auth` + creds OR creds present without flag → write routes
    /// require a valid session. Anonymous users see read routes. The nav
    /// shows a "Sign in" link.
    Authenticated,

    /// `--auth` without creds → strict read-only mode with no login path.
    /// All writes return 401. The nav shows a banner:
    /// "🔒 Read-only — auth required but not configured".
    ReadOnly,

    /// No creds + no `--auth` → legacy behavior (default). All routes
    /// open. Console warning at startup. No UI banner (would be too
    /// intrusive for existing local users). This mode preserves
    /// backward compatibility.
    Disabled,
}
```

### 3.4 Sessions

- **Format**: opaque token, 32 random bytes encoded as base64url (use
  `base64`, already in `Cargo.toml` for the `sync` feature; otherwise add
  `rand`).
- **In-memory store**: `HashMap<SessionId, SessionRecord>` behind an
  `RwLock`.
- **Persistence**: JSON file at `~/.config/cook/server-sessions.json` —
  reuse `crate::global_file_path` ([src/main.rs:161](../../src/main.rs)).
  Read at startup, written after every mutation. File permissions: `0600`
  on Unix (skipped on Windows). Contents:

  ```json
  {
    "sessions": [
      {
        "id": "<base64url>",
        "username": "admin",
        "created_at": "2026-04-29T10:15:00Z",
        "expires_at": "2026-05-29T10:15:00Z",
        "last_seen_at": "2026-04-29T10:30:00Z"
      }
    ]
  }
  ```

- **Cookie**: `cook_session=<id>; Path=<prefix>/; HttpOnly; SameSite=Lax;
  Max-Age=<ttl>` (default TTL: 7 days). **No** `Secure` flag by default
  (users often run plain HTTP locally). Document that public exposure
  needs a TLS reverse proxy and consider a future `--secure-cookie` flag.
  The `Path` must follow `state.url_prefix`: when the server runs behind
  a reverse proxy at `/cook`, `Path=/cook/` prevents the cookie from
  leaking to other apps on the same origin.
- **Invalidation**: expired sessions are purged on load and on each check
  (lazy). Logout removes the session server-side AND sends a cookie with
  `Max-Age=0`.

### 3.5 Relationship to existing CookCloud sync

The `sync` feature
(see [src/server/handlers/sync.rs](../../src/server/handlers/sync.rs))
implements an OAuth-like login flow against CookCloud and is fully
independent. The two coexist:
- Local auth protects access **to the HTTP server** (who can write to
  this server?).
- CookCloud sync authenticates **against the external service** (who
  syncs to the cloud?).

Naming clash to keep in mind:

| Route | Purpose | Local-auth requirement |
|---|---|---|
| `/login`, `/logout` (new) | Local server auth (this plan) | `/login` is public (entry point); `/logout` requires a session |
| `/api/sync/login`, `/api/sync/logout` (existing) | Bind the server to a CookCloud account | **Both protected by `require_auth`** — they mutate server state, and we don't want an anonymous LAN visitor to be able to bind/unbind the server's CookCloud association |

### 3.6 Middleware

Two axum functions:

```rust
// src/server/auth/middleware.rs

/// Attaches an `AuthIdentity` (Anonymous | User { username }) extension to
/// the request. Used on read routes to adapt the UI.
pub async fn extract_auth(
    State(state): State<Arc<AppState>>,
    cookies: CookieJar,
    mut req: Request,
    next: Next,
) -> Response { ... }

/// Returns 401 when the request is not authenticated OR when the mode is
/// AuthMode::ReadOnly. Also enforces the Origin check for non-GET
/// methods (CSRF).
pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    cookies: CookieJar,
    req: Request,
    next: Next,
) -> Response { ... }
```

Behavior:
- `require_auth` returns JSON `{"error": "unauthorized"}` 401 on API
  routes and redirects to `/login?next=<path>` on UI routes.
- Bring in `axum_extra::extract::CookieJar` (the `axum-extra` crate isn't
  in `Cargo.toml` yet — add it, or parse `header::COOKIE` by hand to
  avoid a dependency).

## 4. CLI surface

Extend `ServerArgs` ([src/server/mod.rs:69](../../src/server/mod.rs)):

```rust
pub struct ServerArgs {
    // ... existing

    /// Force authentication on. Server fails to start if no credentials are
    /// configured.
    #[arg(long, conflicts_with = "no_auth")]
    auth: bool,

    /// Disable authentication entirely. Restores the legacy open-write
    /// behavior. Use only on trusted networks.
    #[arg(long, conflicts_with = "auth")]
    no_auth: bool,

    /// Path to the server config TOML (auth credentials, session TTL).
    /// Default search: ./config/server.toml then ~/.config/cook/server.toml.
    #[arg(long, value_hint = clap::ValueHint::FilePath)]
    auth_config: Option<Utf8PathBuf>,
}
```

Mode resolution:

| `--auth` | `--no-auth` | creds present | Resolved mode |
|---|---|---|---|
| ✓ | – | ✓ | `Authenticated` |
| ✓ | – | ✗ | `ReadOnly` (read-only, login impossible) |
| – | ✓ | * | `Disabled` (console warning — explicit legacy opt-in) |
| – | – | ✓ | `Authenticated` (inferred from creds presence) |
| – | – | ✗ | `Disabled` (**default**, console warning) |

Note: the bottom-row `Disabled` default preserves current behavior for
users who upgrade without changing their configuration — the non-breaking
trade-off. To switch to read-only without creds, the operator must pass
`--auth` explicitly.

Console output at startup (in addition to existing messages):

```
Authentication: Authenticated (admin, bcrypt)   # algorithm in parentheses
Session store: ~/.config/cook/server-sessions.json (3 active)
```

Other modes:

```
Authentication: Disabled (no credentials configured — anyone can write)
Authentication: Disabled (--no-auth)
Authentication: ReadOnly (--auth set but no credentials configured)
WARN: Authentication is using plaintext password storage. ...
```

### 4.1 `hash-password` helper subcommand

So users can generate a hash without an external dependency:

```bash
cook server hash-password
# Interactive prompt (no echo) → prints "bcrypt:$2b$12$..."

cook server hash-password --algorithm plain
# Prints "plain:<password>" — useful for test scripts
```

The prefix is part of the output so it can be pasted directly into
`password = "..."` in `server.toml`.

Implementation: turn `Server(server::ServerArgs)` into a subcommand with
two variants — `Server(ServerRunArgs)` and `ServerHashPassword(HashPwArgs)`
— or simply add a `--hash-password` flag that short-circuits the server
startup. The first option is cleaner; recommended.

## 5. Route classification

### 5.1 API (`/api/...`) — defined at [src/server/mod.rs:408](../../src/server/mod.rs)

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

### 5.2 UI (`/...`) — defined at [src/server/ui.rs:29](../../src/server/ui.rs)

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

Extend each `*Template` in
[src/server/templates.rs](../../src/server/templates.rs) with a shared
field:

```rust
pub struct AuthContext {
    pub mode: AuthModeView,        // ReadOnly | Authenticated | Disabled
    pub signed_in: bool,
    pub username: Option<String>,
}
```

The cleanest approach is a `BaseTemplate` trait or passing `AuthContext`
into every struct (the repo doesn't have a shared pattern today). Decide
at implementation time, ideally as a direct field on each template to
minimize the risk of forgetting one in Askama.

### 6.2 Template changes

Concretely hide / adapt:

- `templates/base.html` (nav): add a right-side
  `🔒 Sign in` link (anonymous + auth mode) or `👤 username | Logout`
  (signed in). Yellow banner on top when `mode == Disabled` (auth
  explicitly disabled).
- [templates/recipes.html:25](../../templates/recipes.html): the `+ New`
  button → `{% if signed_in or mode == Disabled %}`.
- [templates/recipe.html:53](../../templates/recipe.html): `Edit` link.
- [templates/menu.html:53](../../templates/menu.html): `Edit` link.
- [templates/shopping_list.html:14](../../templates/shopping_list.html):
  `Clear list` button. Check/uncheck checkboxes are hidden or disabled.
- [templates/pantry.html](../../templates/pantry.html): add modal,
  edit/delete buttons.
- [templates/edit.html](../../templates/edit.html) and
  [templates/new.html](../../templates/new.html): protected at the server
  level (middleware redirects), so no template-level hiding needed —
  optionally add a defensive JS guard.
- New minimal `login.html` template (username + password form, hidden
  `next` field, generic "Invalid credentials" error).

### 6.3 Strategy for shopping list / pantry pages

The pages themselves stay visible. The JS that calls write endpoints must
handle 401:
- When `auth.mode == ReadOnly`: pass a flag through the HTML
  (`<body data-readonly="true">`) and disable controls (checkboxes,
  "Add to shopping list" buttons, etc.) client-side.
- When `auth.mode == Authenticated` and anonymous: show a toast "Sign in
  to modify" linking to `/login`.

### 6.4 i18n

Add to all locales (`locales/*/common.ftl`):

```
auth-sign-in = Sign in
auth-sign-out = Sign out
auth-readonly-banner = Read-only mode — write operations require credentials.
auth-disabled-warning = Authentication is disabled. Anyone can modify recipes.
auth-login-title = Sign in
auth-login-username = Username
auth-login-password = Password
auth-login-submit = Sign in
auth-login-error = Invalid credentials
auth-write-blocked-toast = Sign in to modify recipes
```

At minimum in `en-US`; other languages can fall back to the English
strings until translated (the project has done this before).

## 7. Implementation phases

Split for clean intermediate commits.

### Phase 1 — Skeleton: config + mode

1. Add the `bcrypt` crate to `Cargo.toml`.
2. Create `src/server/auth/{mod.rs, config.rs}`.
3. Load `server.toml` from `Context` (mirroring `aisle()`/`pantry()`).
4. Extend `ServerArgs` with `--auth`, `--no-auth`, `--auth-config`.
5. Resolve `AuthMode` in `build_state`; log it at startup.
6. No HTTP-visible behavior change yet — just plumbing.

### Phase 2 — Sessions and middleware

1. Create `src/server/auth/session.rs` with `SessionStore`
   (memory + JSON file).
2. Load sessions at startup, purge expired ones.
3. Create `src/server/auth/middleware.rs`:
   - `extract_auth` (read cookie, attach `AuthIdentity`)
   - `require_auth` (401 / login redirect)
4. Not yet wired into the router — just compilable and unit-testable.

### Phase 3 — Login / logout routes

1. `src/server/auth/handlers.rs`: `GET /login`, `POST /login`,
   `POST /logout`.
2. `templates/login.html` (very simple, mirror `templates/new.html`).
3. bcrypt verification with constant delay to mitigate timing leaks.
4. `cook server hash-password` subcommand.
5. Integration tests: login OK, login KO, expiration, persistence.

### Phase 4 — Wire middleware into the router

In `src/server/mod.rs::run`:
1. Split the API into `read_api` (no `require_auth`) and `write_api`
   (with `require_auth`).
2. Same for UI: `read_ui` vs `write_ui`.
3. Apply `extract_auth` globally (every route).
4. Final router shape:

```rust
let api = Router::new()
    .merge(read_api(&state)?)
    .merge(write_api(&state)?.layer(from_fn_with_state(state.clone(), require_auth)));

let ui = Router::new()
    .merge(read_ui())
    .merge(write_ui().layer(from_fn_with_state(state.clone(), require_auth)));

let inner = Router::new()
    .nest("/api", api)
    .merge(ui)
    .merge(auth_routes())            // /login, /logout
    .route("/static/*file", ...)
    .nest_service("/api/static", ...);
```

### Phase 5 — Templates and JS

1. Add `AuthContext` to the Askama structs.
2. Update the templates listed in §6.2.
3. Add the banner and nav links in `base.html`.
4. Update the JS in `shopping_list.html` and `pantry.html` to handle 401
   and the read-only state.

### Phase 6 — i18n + docs

1. Add the keys in `locales/en-US/common.ftl` (at least).
2. Update [docs/server.md](../../docs/server.md):
   - "Authentication" section
   - Examples with `--auth`, `--no-auth`, `hash-password`
   - HTTPS / reverse-proxy warning
3. Note in `README.md` (changelog or security section).

### Phase 7 — Tests

- Unit: config parsing, hash/verify, session store roundtrip, middleware
  (mock state).
- Integration (via `reqwest`, like the existing `tests/`):
  - GET `/api/recipes` anonymous → 200
  - PUT `/api/recipes/foo.cook` anonymous → 401
  - PUT after login → 200
  - Logout → cookie cleared → PUT → 401
  - `--no-auth` mode: PUT anonymous → 200 (legacy)
  - `ReadOnly` mode (no creds): PUT → 401, GET → 200
  - Session persistence: restart the server, the cookie is still valid
  - Invalid Origin on PUT/POST → 403 (CSRF)
- Playwright E2E (`tests/e2e/`): new `auth.spec.ts`
  - login flow happy path
  - write buttons hidden when anonymous
  - read-only banner visible

## 8. Backward compatibility

- Existing users with no config and no flag stay in `Disabled`
  → **legacy behavior preserved**, no action required. A console warning
  at startup invites them to enable auth. No UI change.
- CI / scripts that POST anonymously to the API keep working unchanged.
- Users who want to enable security drop a `server.toml` with creds
  → automatic switch to `Authenticated`. Or pass `--auth` to force it
  (no creds → `ReadOnly`).
- No data migration needed (no DB schema).
- The changelog should clearly promote enabling auth for network-exposed
  setups, without making it mandatory.

## 9. Security implementation checklist

- [ ] bcrypt cost ≥ 12 in `hash-password` (configurable later).
- [ ] Constant-time comparison for `plain` (`subtle::ConstantTimeEq` or
  similar) — even with a trivial algorithm, prevent a timing leak from
  revealing the password length.
- [ ] Mandatory console warning at startup when the resolved algorithm is
  `plain`.
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
  `successful login` / `failed login` + source IP.
- [ ] Generic login response ("Invalid credentials"); no distinction
  between wrong user and wrong password.

## 10. Documentation to produce

- `docs/server.md`: Auth section (TOML config, flags, hash-password).
- `docs/auth.md` (new)? Optional — can fit in `server.md`.
- README: mention in the Server section.
- Changelog (release notes): security highlight.
- Update [CLAUDE.md](../../CLAUDE.md) if the project structure shifts.

## 11. Open questions

Locked decisions (recap):

- ✅ **Password format**: mandatory prefix tag (`plain:` / `bcrypt:`); no
  auto-detection. Plain is accepted in dev with a warning. The prefix
  scheme is the extension point for future algorithms; no other algorithm
  is reserved or stubbed in the initial code.
- ✅ **Sessions**: server-side (HashMap + JSON file), not JWT.
- ✅ **CookCloud sync**: stays separate from local auth.
- ✅ **Multi-user**: YAGNI, flat schema.
- ✅ **Default mode without creds**: `Disabled` (non-breaking).
- ✅ **Session TTL**: 7 days.

Still to decide at implementation time:

1. **Cookie `Path` with `url_prefix`**: trivial to implement
   (`Path={state.url_prefix}/` or `/` if empty), but **needs explicit
   testing** in a reverse-proxy setup with `--url-prefix`.
2. **bcrypt crate choice**: `bcrypt` (simple) vs `password-hash` /
   RustCrypto's `bcrypt` (more modular). Pick when first added to
   `Cargo.toml`.
3. **Constant-time comparison for `plain`**: use `subtle` or reuse an
   existing primitive. Decide while coding.

## 12. Definition of done

The feature is shippable when:

- [ ] `cook server` with no config starts in `Disabled` (legacy preserved)
      with a console warning.
- [ ] `cook server --auth` without creds starts in `ReadOnly` and blocks
      all writes.
- [ ] `cook server --no-auth` explicitly reproduces the current behavior.
- [ ] `server.toml` with `password = "plain:..."` works and emits the
      plaintext-storage warning.
- [ ] `server.toml` with `password = "bcrypt:$2b..."` works without any
      warning.
- [ ] `server.toml` with a password missing the prefix (e.g. bare
      `$2b...` or bare `mypassword`) fails fast at startup with a clear
      error.
- [ ] With creds in `server.toml`, web login works, and the cookie
      survives both server AND browser restarts (TTL 7d).
- [ ] Every route in the §5.1 / §5.2 tables returns 401 (API) / login
      redirect (UI) under `Authenticated` + anonymous.
- [ ] CSRF tested on at least one POST route.
- [ ] Cookie `Path` is correct when `--url-prefix` is used.
- [ ] `cook server hash-password` produces `bcrypt:...`; with
      `--algorithm plain` produces `plain:...`.
- [ ] Integration tests are green.
- [ ] `cargo fmt`, `cargo clippy`, `cargo test` pass cleanly (see
      [CLAUDE.md](../../CLAUDE.md) — Before Creating a PR).
- [ ] [docs/server.md](../../docs/server.md) is up to date.

## 13. Appendix — code touchpoints

Quick reference for a future session:

- [src/server/mod.rs:69](../../src/server/mod.rs) — `ServerArgs`
- [src/server/mod.rs:247](../../src/server/mod.rs) — `build_state`
- [src/server/mod.rs:354](../../src/server/mod.rs) — `AppState`
- [src/server/mod.rs:408](../../src/server/mod.rs) — `api()` (split read/write)
- [src/server/ui.rs:29](../../src/server/ui.rs) — `ui()` (split read/write +
  auth routes)
- [src/server/templates.rs](../../src/server/templates.rs) — add
  `AuthContext` to template structs
- [src/main.rs:122](../../src/main.rs) — `configure_context` /
  `Context` (`server.toml` loading)
- [src/args.rs:86](../../src/args.rs) — possible `Server::HashPassword`
  subcommand
- [Cargo.toml](../../Cargo.toml) — `bcrypt`, possibly `axum-extra`
- [templates/base.html](../../templates/base.html) — nav + banner
- [templates/recipe.html](../../templates/recipe.html),
  [templates/menu.html](../../templates/menu.html),
  [templates/recipes.html](../../templates/recipes.html),
  [templates/shopping_list.html](../../templates/shopping_list.html),
  [templates/pantry.html](../../templates/pantry.html) — hide write actions
- [docs/server.md](../../docs/server.md) — user-facing documentation
- [locales/en-US/common.ftl](../../locales/en-US/common.ftl) — i18n keys
- [tests/](../../tests) — new integration tests
