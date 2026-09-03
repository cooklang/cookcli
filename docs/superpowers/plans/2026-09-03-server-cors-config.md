# Configurable Server CORS Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `cook server`'s CORS policy configurable via `--cors-origin` and `--cors-allow-credentials`, with a safer default (wildcard origin, `GET` only), and rename the misleading `--no-cors` flag to `--no-csrf-check`.

**Architecture:** A new `src/server/cors.rs` turns the two flags into a validated `CorsConfig` value, then into a `tower_http::cors::CorsLayer`. Validation happens in `run()` before the socket binds, so bad flag combinations fail fast. The unrelated `--no-cors` flag (a CSRF check on `POST /new`, not CORS) is renamed with a hidden clap alias for backwards compatibility.

**Tech Stack:** Rust, clap 4 (derive), axum, tower-http 0.7 (`cors` feature), anyhow, reqwest (dev), tempfile + assert_cmd (dev).

**Spec:** `docs/superpowers/specs/2026-09-03-server-cors-config-design.md`

---

## Background the engineer needs

**What CORS is here.** The server has no authentication. CORS is enforced by
browsers only: it decides which *web pages from other origins* may read
responses from this server. `curl`, the server's own web UI, and every
non-browser client ignore it entirely. So tightening it cannot break the UI or
scripts — only cross-origin browser JavaScript.

**Preflight.** Before a "non-simple" cross-origin request (anything that isn't
a plain `GET`/`POST` with a safelisted content type), the browser sends
`OPTIONS` with `Origin` and `Access-Control-Request-Method` headers. The server
answers with `Access-Control-Allow-Origin` / `-Methods` / `-Headers`. tower-http's
`CorsLayer` handles this automatically — you never write an `OPTIONS` route.
Tests below drive preflight by sending `OPTIONS` with those headers by hand.

**Where things live.**
- `src/server/mod.rs` — `ServerArgs` (clap), `run()`, `build_state()`, `AppState`.
  The hardcoded `CorsLayer` is at lines 206-210.
- `src/server/ui.rs:304` — the only read of `AppState.cors`, the CSRF check.
- `src/web/api_docs.rs:56` — the CORS note shown at `/api-docs`.
- `docs/api.md` — **generated** from `api_docs.rs`. Never hand-edit; regenerate
  with `UPDATE_API_DOCS=1 cargo test --test api_docs_md_test`.
- `docs/server.md` — hand-written `cook server` reference.

**Everything under `src/server/` is behind `#[cfg(feature = "server")]`** (see
`src/lib.rs:20`). The `server` feature is on by default, so plain `cargo test`
covers it.

## File Structure

- **Create** `src/server/cors.rs` — `CorsOrigins`, `CorsConfig`, validation,
  layer construction, and the unit tests for validation. One responsibility:
  turn flags into a CORS policy. Nothing else in the codebase needs to know how
  tower-http is configured.
- **Create** `tests/cors_test.rs` — end-to-end header assertions against a real
  booted server, mirroring `tests/menu_api_test.rs`.
- **Modify** `src/server/mod.rs` — declare the module, add the two flags, rename
  `cors` → `csrf_check` on `ServerArgs` and `AppState`, call the validator in
  `run()`, replace the hardcoded layer.
- **Modify** `src/server/ui.rs` — one field rename at line 304.
- **Modify** `src/web/api_docs.rs` — the CORS note text.
- **Modify** `docs/server.md`, `docs/api.md` (regenerated).

---

### Task 1: `CorsConfig` validation

**Files:**
- Create: `src/server/cors.rs`
- Modify: `src/server/mod.rs` (add `mod cors;`)
- Test: `src/server/cors.rs` (inline `#[cfg(test)] mod tests`)

- [ ] **Step 1: Create the module file with types and a stub, plus the failing tests**

Create `src/server/cors.rs` with exactly this content:

```rust
//! CORS policy for `cook server`, built from the `--cors-origin` and
//! `--cors-allow-credentials` flags.
//!
//! CORS is enforced by browsers only, so this governs what cross-origin web
//! pages may do with the API. The web UI itself, `curl`, and every non-browser
//! client are unaffected by anything here.

use anyhow::{bail, Result};
use axum::http::{header, HeaderValue, Method};
use tower_http::cors::{AllowOrigin, CorsLayer};

/// Which origins may make cross-origin requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorsOrigins {
    /// `--cors-origin '*'`, or no `--cors-origin` at all. Read-only: see
    /// [`CorsConfig::methods`].
    Any,
    /// One or more explicit origins, in the order given on the command line.
    List(Vec<HeaderValue>),
}

/// A validated CORS policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorsConfig {
    origins: CorsOrigins,
    allow_credentials: bool,
}

impl CorsConfig {
    /// Validates a `--cors-origin` / `--cors-allow-credentials` combination.
    ///
    /// `origins` is the raw repeated flag; empty means the flag was not given.
    pub fn from_args(origins: &[String], allow_credentials: bool) -> Result<Self> {
        todo!("Task 1 Step 3")
    }

    /// The methods this policy allows cross-origin.
    ///
    /// A wildcard origin means *any* page in the user's browser can reach the
    /// server, so it gets read-only access. Naming an origin is an explicit
    /// statement of trust, and unlocks the mutating routes.
    pub fn methods(&self) -> Vec<Method> {
        match &self.origins {
            CorsOrigins::Any => vec![Method::GET],
            CorsOrigins::List(_) => {
                vec![Method::GET, Method::POST, Method::PUT, Method::DELETE]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origins(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_flag_defaults_to_any() {
        let config = CorsConfig::from_args(&[], false).expect("valid");
        assert_eq!(config.origins, CorsOrigins::Any);
        assert!(!config.allow_credentials);
    }

    #[test]
    fn explicit_wildcard_is_any() {
        let config = CorsConfig::from_args(&origins(&["*"]), false).expect("valid");
        assert_eq!(config.origins, CorsOrigins::Any);
    }

    #[test]
    fn any_allows_only_get() {
        let config = CorsConfig::from_args(&[], false).expect("valid");
        assert_eq!(config.methods(), vec![Method::GET]);
    }

    #[test]
    fn explicit_origins_keep_order_and_allow_mutation() {
        let config =
            CorsConfig::from_args(&origins(&["http://a.test", "https://b.test:8443"]), false)
                .expect("valid");
        assert_eq!(
            config.origins,
            CorsOrigins::List(vec![
                HeaderValue::from_static("http://a.test"),
                HeaderValue::from_static("https://b.test:8443"),
            ])
        );
        assert_eq!(
            config.methods(),
            vec![Method::GET, Method::POST, Method::PUT, Method::DELETE]
        );
    }

    #[test]
    fn wildcard_mixed_with_explicit_origin_is_rejected() {
        let err = CorsConfig::from_args(&origins(&["*", "http://a.test"]), false)
            .expect_err("must reject");
        assert!(
            err.to_string().contains("cannot be combined"),
            "unhelpful error: {err}"
        );
    }

    #[test]
    fn credentials_with_wildcard_is_rejected() {
        let err = CorsConfig::from_args(&[], true).expect_err("must reject");
        assert!(
            err.to_string().contains("--cors-origin"),
            "error must point at the fix: {err}"
        );
    }

    #[test]
    fn credentials_with_explicit_origins_is_allowed() {
        let config =
            CorsConfig::from_args(&origins(&["http://a.test"]), true).expect("valid");
        assert!(config.allow_credentials);
    }

    #[test]
    fn origin_with_trailing_slash_is_rejected() {
        let err =
            CorsConfig::from_args(&origins(&["http://a.test/"]), false).expect_err("must reject");
        assert!(
            err.to_string().contains("http://a.test/"),
            "error must name the bad origin: {err}"
        );
    }

    #[test]
    fn origin_without_scheme_is_rejected() {
        CorsConfig::from_args(&origins(&["a.test"]), false).expect_err("must reject");
    }

    #[test]
    fn empty_origin_is_rejected() {
        CorsConfig::from_args(&origins(&[""]), false).expect_err("must reject");
    }
}
```

Then add the module declaration in `src/server/mod.rs`. The existing block at
lines 49-53 reads:

```rust
mod fs_atomic;
mod handlers;
mod lsp_bridge;
mod shopping_list_watcher;
mod ui;
```

Change it to (alphabetical, matching the existing order):

```rust
mod cors;
mod fs_atomic;
mod handlers;
mod lsp_bridge;
mod shopping_list_watcher;
mod ui;
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib server::cors`

Expected: the tests compile and every one of them fails, panicking at the
`todo!("Task 1 Step 3")` with `not yet implemented`.

Unused-import warnings for `bail`, `header`, `AllowOrigin` and `CorsLayer` are
expected at this point — Step 3 uses `bail` and Task 2 uses the other three.
Leave them; do not add an `allow(unused_imports)`.

- [ ] **Step 3: Implement `from_args`**

Replace the `todo!("Task 1 Step 3")` body with:

```rust
    pub fn from_args(origins: &[String], allow_credentials: bool) -> Result<Self> {
        let wildcard = origins.iter().any(|o| o == "*");
        if wildcard && origins.len() > 1 {
            bail!(
                "--cors-origin '*' cannot be combined with explicit origins; \
                 pass either '*' or a list of origins, not both"
            );
        }

        let origins = if origins.is_empty() || wildcard {
            CorsOrigins::Any
        } else {
            let parsed = origins
                .iter()
                .map(|origin| parse_origin(origin))
                .collect::<Result<Vec<_>>>()?;
            CorsOrigins::List(parsed)
        };

        if allow_credentials && origins == CorsOrigins::Any {
            bail!(
                "--cors-allow-credentials requires explicit --cors-origin values; \
                 browsers reject credentialed requests against a wildcard origin"
            );
        }

        Ok(Self {
            origins,
            allow_credentials,
        })
    }
```

And add this free function to the module, after the `impl CorsConfig` block:

```rust
/// Parses one `--cors-origin` value.
///
/// A browser sends a bare origin — scheme, host, optional port — in the
/// `Origin` header. Anything richer (a path, a trailing slash) would compare
/// unequal and silently never match, so it is rejected up front instead.
fn parse_origin(origin: &str) -> Result<HeaderValue> {
    let Some((scheme, rest)) = origin.split_once("://") else {
        bail!("invalid --cors-origin {origin:?}: expected a scheme, e.g. http://localhost:3000");
    };
    if scheme.is_empty() || rest.is_empty() {
        bail!("invalid --cors-origin {origin:?}: expected scheme://host[:port]");
    }
    if rest.contains('/') {
        bail!(
            "invalid --cors-origin {origin:?}: an origin has no path or trailing slash, \
             e.g. http://localhost:3000"
        );
    }
    HeaderValue::from_str(origin)
        .map_err(|e| anyhow::anyhow!("invalid --cors-origin {origin:?}: {e}"))
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib server::cors`

Expected: `test result: ok. 10 passed`. Warnings about unused `header`,
`AllowOrigin`, `CorsLayer` imports are expected and go away in Task 2.

- [ ] **Step 5: Commit**

```bash
git add src/server/cors.rs src/server/mod.rs
git commit -m "feat(server): validate CORS origin and credential flags

Claude-Session: https://claude.ai/code/session_013cPWotrjEY4Jac87e6vHsh"
```

---

### Task 2: Build the `CorsLayer`

**Files:**
- Modify: `src/server/cors.rs`
- Test: `src/server/cors.rs` (inline tests)

There is no way to inspect a built `CorsLayer` from a unit test — tower-http
exposes no getters. The behaviour is covered end-to-end in Task 5. This task
only adds the constructor and a smoke test that it does not panic (tower-http
*does* panic on some invalid combinations, e.g. `AllowOrigin::list` containing
a wildcard, so a construction test has real value).

- [ ] **Step 1: Write the failing test**

Add these two tests inside the existing `mod tests` block in
`src/server/cors.rs`:

```rust
    #[test]
    fn layer_builds_for_wildcard() {
        let config = CorsConfig::from_args(&[], false).expect("valid");
        let _layer = config.layer();
    }

    #[test]
    fn layer_builds_for_explicit_origins_with_credentials() {
        let config =
            CorsConfig::from_args(&origins(&["http://a.test"]), true).expect("valid");
        let _layer = config.layer();
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib server::cors`

Expected: compile error, `no method named 'layer' found for struct 'CorsConfig'`.

- [ ] **Step 3: Implement `layer`**

Add this method to the `impl CorsConfig` block in `src/server/cors.rs`, after
`methods`:

```rust
    /// Builds the tower-http layer for this policy.
    ///
    /// `content-type` is always allowed: without it a cross-origin JSON `POST`
    /// fails preflight no matter what the origin setting is, so there is
    /// nothing here worth making configurable. The CORS-safelisted request
    /// headers (`Accept`, `Accept-Language`, `Content-Language`) need no entry
    /// — browsers permit them regardless.
    pub fn layer(&self) -> CorsLayer {
        let allow_origin = match &self.origins {
            CorsOrigins::Any => AllowOrigin::any(),
            CorsOrigins::List(list) => AllowOrigin::list(list.iter().cloned()),
        };

        CorsLayer::new()
            .allow_origin(allow_origin)
            .allow_methods(self.methods())
            .allow_headers([header::CONTENT_TYPE])
            .allow_credentials(self.allow_credentials)
    }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib server::cors`

Expected: `test result: ok. 12 passed`, with no unused-import warnings left.

- [ ] **Step 5: Commit**

```bash
git add src/server/cors.rs
git commit -m "feat(server): build the CORS layer from the validated config

Claude-Session: https://claude.ai/code/session_013cPWotrjEY4Jac87e6vHsh"
```

---

### Task 3: Wire the flags into `cook server`

**Files:**
- Modify: `src/server/mod.rs`

No test in this task — it is pure wiring, and Task 5 tests the result through
the real binary. Verification here is `cargo build` plus `--help` output.

- [ ] **Step 1: Add the two flags to `ServerArgs`**

In `src/server/mod.rs`, the `ServerArgs` struct currently ends with the `cors`
field (lines 99-104):

```rust
    /// Enable cors verification
    ///
    /// When enabled, the POST /new path require a seemless
    /// Origin and Host header.
    #[arg(long = "no-cors", action = clap::ArgAction::SetFalse)]
    cors: bool,
```

Leave that field alone for now (Task 4 renames it) and insert the two new
fields **immediately before** it:

```rust
    /// Origin allowed to make cross-origin browser requests (repeatable)
    ///
    /// Pass once per origin, e.g. --cors-origin http://localhost:3000. Use "*"
    /// for any origin, which is the default. A wildcard origin allows GET
    /// only; naming explicit origins also allows POST, PUT and DELETE.
    /// "*" cannot be combined with explicit origins.
    #[arg(long = "cors-origin", value_name = "ORIGIN")]
    cors_origin: Vec<String>,

    /// Allow cross-origin requests to carry cookies and credentials
    ///
    /// Requires at least one explicit --cors-origin; browsers reject
    /// credentialed requests against a wildcard origin.
    #[arg(long = "cors-allow-credentials", default_value_t = false)]
    cors_allow_credentials: bool,
```

- [ ] **Step 2: Validate the flags early in `run()`**

In `run()`, the function currently opens with:

```rust
    let addr = match args.host {
        Some(Some(addr)) => addr,
        Some(None) => "::".parse()?,
        None => [127, 0, 0, 1].into(),
    };
    let addr = SocketAddr::from((addr, args.port));
    let open = args.open;

    let state = build_state(ctx, args)?;
```

Insert the CORS validation **before** `let state = build_state(...)`, so a bad
flag combination fails before anything is printed or bound:

```rust
    let addr = match args.host {
        Some(Some(addr)) => addr,
        Some(None) => "::".parse()?,
        None => [127, 0, 0, 1].into(),
    };
    let addr = SocketAddr::from((addr, args.port));
    let open = args.open;

    // Validate before binding or printing anything, so a bad flag combination
    // fails immediately rather than after the "Listening on ..." banner.
    let cors = cors::CorsConfig::from_args(&args.cors_origin, args.cors_allow_credentials)?;

    let state = build_state(ctx, args)?;
```

- [ ] **Step 3: Replace the hardcoded layer**

Still in `run()`, this block (lines 206-210 before your edits):

```rust
        .layer(
            CorsLayer::new()
                .allow_origin("*".parse::<HeaderValue>().unwrap())
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE]),
        );
```

becomes:

```rust
        .layer(cors.layer());
```

- [ ] **Step 4: Drop the now-unused imports**

Line 46 currently reads:

```rust
use tower_http::{cors::CorsLayer, services::ServeDir};
```

Change it to:

```rust
use tower_http::services::ServeDir;
```

Line 37 currently reads:

```rust
    http::{header, HeaderValue, Method, Response, StatusCode},
```

`HeaderValue` and `Method` were only used by the block you just deleted;
`header`, `Response` and `StatusCode` are still used by `serve_static` at
`src/server/mod.rs:500-516`. So change it to:

```rust
    http::{header, Response, StatusCode},
```

- [ ] **Step 5: Build and check the help text**

Run: `cargo build`
Expected: compiles with no warnings.

Run: `cargo run -- server --help`
Expected: the output lists `--cors-origin <ORIGIN>` and
`--cors-allow-credentials`.

Run: `cargo run -- server --cors-allow-credentials`
Expected: exits non-zero, prints
`Error: --cors-allow-credentials requires explicit --cors-origin values; browsers reject credentialed requests against a wildcard origin`,
and does **not** print "Listening on".

Run: `cargo run -- server --cors-origin '*' --cors-origin http://a.test`
Expected: exits non-zero with the "cannot be combined with explicit origins"
message.

- [ ] **Step 6: Commit**

```bash
git add src/server/mod.rs
git commit -m "feat(server): add --cors-origin and --cors-allow-credentials

The default is now a wildcard origin restricted to GET. Naming explicit
origins unlocks POST/PUT/DELETE, and content-type is always allowed so
cross-origin JSON requests can pass preflight.

Closes #465

Claude-Session: https://claude.ai/code/session_013cPWotrjEY4Jac87e6vHsh"
```

---

## REVISION (after Task 2-3 code review)

Tasks 1-3 are complete and committed. A code review of Tasks 2-3 found that the
plan's central premise was wrong: **`allow_methods([GET])` does not block a
cross-origin `POST`**, because `POST` is a CORS-safelisted method and the
`Access-Control-Allow-Methods` list is only consulted for non-safelisted ones.
Verified in a real browser — a cross-origin page wrote to
`seed/config/pantry.conf` via `POST /api/pantry/add`.

Adding `content-type` to `allow_headers` also removes the accidental protection
that today's *unset* `allow_headers` provides, so as it stands the new default
is more permissive than the code it replaces.

The design doc has been corrected (see its "Correction: `allow_methods` cannot
enforce read-only" and "The cross-origin write guard" sections, commit
`54f187c`). The remaining tasks are renumbered and rewritten below. Read-only
is now enforced **server-side** by a write guard; CORS headers describe the
policy rather than enforce it.

Remaining tasks:

- **Task 4** — review fixes to Tasks 2-3 (help text, smoke tests, polish).
- **Task 5** — the `--no-cors` → `--no-csrf-check` rename (unchanged from the
  original Task 4).
- **Task 6** — the cross-origin write guard (new).
- **Task 7** — end-to-end tests (the original Task 5, with the `POST`
  assertions rewritten: a preflight assertion cannot detect the hole, since
  `AllowOrigin::any()` always answers `*`).
- **Task 8** — documentation (the original Task 6, with corrected text).
- **Task 9** — full verification and PR (the original Task 7).

Full task text is supplied to each implementer at dispatch time. The original
Tasks 4-7 are preserved below for reference; where they conflict with this
revision, the revision wins.

---

## ORIGINAL TASKS 4-7 (superseded — see REVISION above)

### Task 4: Rename `--no-cors` to `--no-csrf-check`

**Files:**
- Modify: `src/server/mod.rs`
- Modify: `src/server/ui.rs:304`

This flag never touched the CORS layer. It disables a same-origin check on the
HTML form `POST /new`, which is CSRF protection. Standing next to
`--cors-origin`, the old name is actively misleading. `alias` (not
`visible_alias`) keeps the old spelling working without showing it in `--help`.

- [ ] **Step 1: Rename the `ServerArgs` field**

In `src/server/mod.rs`, replace:

```rust
    /// Enable cors verification
    ///
    /// When enabled, the POST /new path require a seemless
    /// Origin and Host header.
    #[arg(long = "no-cors", action = clap::ArgAction::SetFalse)]
    cors: bool,
```

with:

```rust
    /// Disable the same-origin (CSRF) check on the new-recipe form
    ///
    /// By default, POST /new is rejected unless the request's Origin or
    /// Referer matches the Host it was sent to. This has nothing to do with
    /// the --cors-* flags, which govern the API's cross-origin policy. The
    /// former spelling --no-cors still works.
    #[arg(long = "no-csrf-check", alias = "no-cors", action = clap::ArgAction::SetFalse)]
    csrf_check: bool,
```

- [ ] **Step 2: Rename the `AppState` field and its initialiser**

In the `AppState` struct definition, replace:

```rust
    pub cors: bool,
```

with:

```rust
    /// When true, `POST /new` requires a same-origin `Origin`/`Referer`.
    /// Cleared by `--no-csrf-check`. Unrelated to the CORS layer.
    pub csrf_check: bool,
```

In `build_state`, in the `Ok(Arc::new(AppState { .. }))` literal, replace:

```rust
        cors: args.cors,
```

with:

```rust
        csrf_check: args.csrf_check,
```

- [ ] **Step 3: Update the single read site**

In `src/server/ui.rs`, line 304 reads:

```rust
    if state.cors && !validate_same_origin(&headers, &host) {
```

Change it to:

```rust
    if state.csrf_check && !validate_same_origin(&headers, &host) {
```

- [ ] **Step 4: Verify no other references remain**

Run: `grep -rn "state.cors\|args.cors\b\|\"no-cors\"" src/`
Expected: only the `alias = "no-cors"` line in `src/server/mod.rs`.

Run: `cargo build`
Expected: compiles with no warnings.

Run: `cargo run -- server --no-cors --help`
Expected: prints the help text (the alias is accepted, not an unknown-argument
error). The `--no-cors` spelling itself does not appear in the options list.

- [ ] **Step 5: Commit**

```bash
git add src/server/mod.rs src/server/ui.rs
git commit -m "refactor(server): rename --no-cors to --no-csrf-check

The flag gates the same-origin check on POST /new, not the CORS layer.
--no-cors is kept as a hidden alias so existing invocations keep working.

Claude-Session: https://claude.ai/code/session_013cPWotrjEY4Jac87e6vHsh"
```

---

### Task 5: End-to-end CORS header tests

**Files:**
- Create: `tests/cors_test.rs`

This mirrors `tests/menu_api_test.rs`: boot the real `cook` binary on an
ephemeral port against a temp recipe directory, then assert response headers.
`assert_cmd::cargo::cargo_bin("cook")` resolves the just-built binary, so
`cargo test` builds it for you.

- [ ] **Step 1: Write the failing test file**

Create `tests/cors_test.rs`:

```rust
//! End-to-end checks on the CORS headers `cook server` returns, for each
//! `--cors-origin` / `--cors-allow-credentials` combination.
//!
//! `CorsConfig` is unit-tested in `src/server/cors.rs`, but nothing there can
//! observe what tower-http actually puts on the wire. These tests drive real
//! preflight requests against a booted server to close that gap.

#![cfg(feature = "server")]

use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

/// Kills the spawned server when the test ends, pass or panic.
struct ServerGuard {
    child: Child,
    port: u16,
    #[allow(dead_code)]
    dir: TempDir,
}

impl ServerGuard {
    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", self.port, path)
    }
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    listener.local_addr().expect("local addr").port()
}

fn write_fixture(dir: &TempDir) {
    std::fs::write(
        dir.path().join("Toast.cook"),
        "---\ntitle: Toast\n---\n\nToast the @bread{2%slice}.\n",
    )
    .expect("write fixture");
}

/// `free_port` only reserves a port long enough to learn its number, so with
/// several tests booting servers at once another one can claim it first. The
/// server exits 1 on a bound port, so retry with a fresh one.
async fn start_server(cors_args: &[&str]) -> ServerGuard {
    for _ in 0..5 {
        if let Some(server) = try_start_server(cors_args).await {
            return server;
        }
    }
    panic!("could not start cook server on a free port after 5 attempts");
}

async fn try_start_server(cors_args: &[&str]) -> Option<ServerGuard> {
    let dir = TempDir::new().expect("temp dir");
    write_fixture(&dir);

    let port = free_port();
    let child = Command::new(assert_cmd::cargo::cargo_bin("cook"))
        .arg("server")
        .arg(dir.path())
        .arg("--port")
        .arg(port.to_string())
        .args(cors_args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn cook server");

    let mut guard = ServerGuard { child, port, dir };

    let client = reqwest::Client::new();
    let url = guard.url("/api/recipes");
    for _ in 0..200 {
        if guard.child.try_wait().expect("poll server").is_some() {
            // Port was taken between reserving and binding it.
            return None;
        }
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                return Some(guard);
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("cook server on port {port} never became ready");
}

/// Sends a CORS preflight and returns the response headers as
/// `(name, value)` pairs, lowercased names.
async fn preflight(server: &ServerGuard, origin: &str, method: &str) -> Vec<(String, String)> {
    let resp = reqwest::Client::new()
        .request(reqwest::Method::OPTIONS, server.url("/api/recipes"))
        .header("Origin", origin)
        .header("Access-Control-Request-Method", method)
        .header("Access-Control-Request-Headers", "content-type")
        .send()
        .await
        .expect("send preflight");

    resp.headers()
        .iter()
        .map(|(k, v)| {
            (
                k.as_str().to_lowercase(),
                v.to_str().unwrap_or_default().to_string(),
            )
        })
        .collect()
}

fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

#[tokio::test]
async fn default_policy_allows_get_from_any_origin() {
    let server = start_server(&[]).await;
    let headers = preflight(&server, "http://evil.test", "GET").await;

    assert_eq!(
        header(&headers, "access-control-allow-origin"),
        Some("*"),
        "the default policy must stay open for reads"
    );
    assert!(
        header(&headers, "access-control-allow-methods")
            .expect("allow-methods")
            .to_uppercase()
            .contains("GET"),
        "GET must be allowed: {headers:?}"
    );
}

#[tokio::test]
async fn default_policy_refuses_cross_origin_post() {
    let server = start_server(&[]).await;
    let headers = preflight(&server, "http://evil.test", "POST").await;

    assert_eq!(
        header(&headers, "access-control-allow-origin"),
        None,
        "a wildcard origin must not unlock mutating methods: {headers:?}"
    );
}

#[tokio::test]
async fn explicit_origin_allows_post_and_content_type() {
    let server = start_server(&["--cors-origin", "http://app.test"]).await;
    let headers = preflight(&server, "http://app.test", "POST").await;

    assert_eq!(
        header(&headers, "access-control-allow-origin"),
        Some("http://app.test")
    );
    assert!(
        header(&headers, "access-control-allow-methods")
            .expect("allow-methods")
            .to_uppercase()
            .contains("POST"),
        "POST must be allowed for an explicit origin: {headers:?}"
    );
    assert!(
        header(&headers, "access-control-allow-headers")
            .expect("allow-headers")
            .to_lowercase()
            .contains("content-type"),
        "content-type must always be allowed, or JSON POSTs fail preflight: {headers:?}"
    );
}

#[tokio::test]
async fn explicit_origin_refuses_other_origins() {
    let server = start_server(&["--cors-origin", "http://app.test"]).await;
    let headers = preflight(&server, "http://evil.test", "POST").await;

    assert_eq!(
        header(&headers, "access-control-allow-origin"),
        None,
        "an unlisted origin must not be allowed: {headers:?}"
    );
}

#[tokio::test]
async fn credentials_are_allowed_with_an_explicit_origin() {
    let server = start_server(&[
        "--cors-origin",
        "http://app.test",
        "--cors-allow-credentials",
    ])
    .await;
    let headers = preflight(&server, "http://app.test", "POST").await;

    assert_eq!(
        header(&headers, "access-control-allow-credentials"),
        Some("true")
    );
}

#[tokio::test]
async fn credentials_without_an_origin_is_a_startup_error() {
    let dir = TempDir::new().expect("temp dir");
    write_fixture(&dir);

    let output = Command::new(assert_cmd::cargo::cargo_bin("cook"))
        .arg("server")
        .arg(dir.path())
        .arg("--port")
        .arg(free_port().to_string())
        .arg("--cors-allow-credentials")
        .output()
        .expect("run cook server");

    assert!(!output.status.success(), "must exit non-zero");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--cors-origin"),
        "the error must point at the fix, got: {stderr}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("Listening on"),
        "must fail before binding, got: {stdout}"
    );
}

#[tokio::test]
async fn wildcard_mixed_with_explicit_origin_is_a_startup_error() {
    let dir = TempDir::new().expect("temp dir");
    write_fixture(&dir);

    let output = Command::new(assert_cmd::cargo::cargo_bin("cook"))
        .arg("server")
        .arg(dir.path())
        .arg("--port")
        .arg(free_port().to_string())
        .arg("--cors-origin")
        .arg("*")
        .arg("--cors-origin")
        .arg("http://app.test")
        .output()
        .expect("run cook server");

    assert!(!output.status.success(), "must exit non-zero");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot be combined"),
        "unhelpful error: {stderr}"
    );
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test --test cors_test`

Expected: `test result: ok. 7 passed`. Tasks 1-4 already implemented the
behaviour, so these should pass on the first run — that is fine; the unit tests
in Task 1 were the failing-first step for the logic, and this task's job is to
prove the wire format.

If `default_policy_refuses_cross_origin_post` or
`explicit_origin_refuses_other_origins` fails because
`access-control-allow-origin` is present, the layer is more permissive than
intended — re-check `CorsConfig::methods` and `layer` from Tasks 1-2 before
touching the test.

- [ ] **Step 3: Commit**

```bash
git add tests/cors_test.rs
git commit -m "test(server): cover CORS headers end to end

Claude-Session: https://claude.ai/code/session_013cPWotrjEY4Jac87e6vHsh"
```

---

### Task 6: Documentation

**Files:**
- Modify: `src/web/api_docs.rs:53-57`
- Modify: `docs/api.md` (generated — do not hand-edit)
- Modify: `docs/server.md`

- [ ] **Step 1: Update the API docs note**

In `src/web/api_docs.rs`, replace:

```rust
            note(
                "CORS",
                "All origins are allowed, for the methods GET, POST, PUT and DELETE.",
            ),
```

with:

```rust
            note(
                "CORS",
                "`GET` is allowed from any origin. Cross-origin `POST`, `PUT` and \
                 `DELETE` require the server to be started with one or more \
                 `--cors-origin <ORIGIN>` flags; `content-type` is always an allowed \
                 request header.",
            ),
```

- [ ] **Step 2: Confirm the docs test now fails**

Run: `cargo test --test api_docs_md_test`

Expected: FAIL with "docs/api.md is stale — it no longer matches
src/web/api_docs.rs."

- [ ] **Step 3: Regenerate `docs/api.md`**

Run: `UPDATE_API_DOCS=1 cargo test --test api_docs_md_test`

Expected: prints `regenerated .../docs/api.md` and passes.

Run: `git diff docs/api.md`

Expected: only the `- **CORS:**` bullet changed.

- [ ] **Step 4: Verify it now passes without the env var**

Run: `cargo test --test api_docs_md_test`
Expected: `test result: ok. 1 passed`.

- [ ] **Step 5: Update `docs/server.md`**

The Options table currently reads:

```markdown
| Option | Description |
|--------|-------------|
| `--host [<ADDRESS>]` | Allow connections from external hosts (default: localhost only). Optionally bind to a specific address. |
| `-p, --port <PORT>` | Port number (default: 9080) |
| `--open` | Automatically open the web interface in your default browser |
```

Replace it with:

```markdown
| Option | Description |
|--------|-------------|
| `--host [<ADDRESS>]` | Allow connections from external hosts (default: localhost only). Optionally bind to a specific address. |
| `-p, --port <PORT>` | Port number (default: 9080) |
| `--open` | Automatically open the web interface in your default browser |
| `--cors-origin <ORIGIN>` | Origin allowed to make cross-origin browser requests. Repeatable. `*` for any origin (default). |
| `--cors-allow-credentials` | Allow cross-origin requests to carry cookies and credentials. Requires an explicit `--cors-origin`. |
| `--no-csrf-check` | Disable the same-origin check on the new-recipe form (`POST /new`). |
```

In the Examples block, after the `cook server --host` example, append:

```bash
# Let a frontend at localhost:3000 use the full API, including writes
cook server --cors-origin http://localhost:3000
```

In the Notes list, insert these two bullets directly after the
`Use --host on trusted networks only` bullet:

```markdown
- Cross-origin browser requests default to `GET` from any origin. Naming origins with `--cors-origin` also permits `POST`, `PUT` and `DELETE` from them — so a page you have not listed cannot modify your recipes. CORS is enforced by browsers only; `curl` and other non-browser clients are unaffected either way. See [the API reference](api.md).
- `--no-csrf-check` is unrelated to the `--cors-*` flags: it turns off the same-origin check on the web UI's new-recipe form. Its former spelling, `--no-cors`, still works.
```

- [ ] **Step 6: Commit**

```bash
git add src/web/api_docs.rs docs/api.md docs/server.md
git commit -m "docs: document the configurable server CORS flags

Claude-Session: https://claude.ai/code/session_013cPWotrjEY4Jac87e6vHsh"
```

- [ ] **Step 7: Note the website follow-up**

`docs/api.md` and `docs/server.md` are the source of truth; cooklang.org syncs
from them one-directionally via its own `scripts/sync-cli-docs.sh`. Do **not**
edit the website repo here. Mention in the PR description that the sync script
should be re-run after merge.

---

### Task 7: Full verification

**Files:** none

- [ ] **Step 1: Format**

Run: `cargo fmt`
Then: `git diff --stat`
Expected: either no changes, or formatting-only changes to the files you
touched. If there are changes, commit them:

```bash
git add -A
git commit -m "style: cargo fmt

Claude-Session: https://claude.ai/code/session_013cPWotrjEY4Jac87e6vHsh"
```

- [ ] **Step 2: Lint**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: no warnings, no errors. Fix anything reported and commit the fix.

- [ ] **Step 3: Full test suite**

Run: `cargo test`
Expected: everything passes. Pay particular attention to `cors_test`,
`api_docs_md_test`, and any snapshot test that captures `cook server --help`
output — if a snapshot covers the help text, review the diff and accept it with
`cargo insta accept` only after confirming the new flags are what changed.

- [ ] **Step 4: Manual smoke test**

```bash
cargo run -- server ./seed --port 9099 &
sleep 2
# Default: GET allowed from anywhere
curl -si -X OPTIONS http://127.0.0.1:9099/api/recipes \
  -H 'Origin: http://evil.test' -H 'Access-Control-Request-Method: GET' \
  | grep -i access-control
# Default: POST not allowed
curl -si -X OPTIONS http://127.0.0.1:9099/api/recipes \
  -H 'Origin: http://evil.test' -H 'Access-Control-Request-Method: POST' \
  | grep -i access-control
kill %1
```

Expected: the first `curl` prints `access-control-allow-origin: *` and an
allow-methods line containing `GET`; the second prints nothing.

Then confirm the web UI still works normally:

```bash
cargo run -- server ./seed --port 9099 --open
```

Expected: recipes list, a recipe page, and the shopping list all work — CORS
does not apply to same-origin requests, so nothing in the UI should change.
Stop the server with Ctrl-C.

- [ ] **Step 5: Open the PR**

```bash
git push -u origin HEAD
gh pr create --title "feat(server): make CORS configuration configurable" --body "$(cat <<'EOF'
Closes #465.

`cook server` applied one hardcoded CORS policy that was simultaneously too
permissive (any page could reach the mutating routes of a `--host`-exposed
server) and too restrictive (`allow_headers` was never set, so cross-origin
JSON `POST` failed preflight regardless).

- `--cors-origin <ORIGIN>` — repeatable; `*` for any origin, the default.
- `--cors-allow-credentials` — requires explicit origins.
- A wildcard origin now allows `GET` only. Naming explicit origins also allows
  `POST`, `PUT` and `DELETE`.
- `content-type` is always an allowed request header.
- `--no-cors` is renamed `--no-csrf-check`, since it gates the same-origin
  check on `POST /new` and never touched the CORS layer. The old spelling
  still works as a hidden alias.

**Behaviour change:** cross-origin `POST`/`PUT`/`DELETE` now requires
`--cors-origin`. Same-origin traffic — the web UI, `curl`, every non-browser
client — is unaffected, since CORS is browser-enforced only.

Design: `docs/superpowers/specs/2026-09-03-server-cors-config-design.md`

**Follow-up:** re-run cooklang.org's `scripts/sync-cli-docs.sh` after merge to
pick up the `docs/api.md` and `docs/server.md` changes.

https://claude.ai/code/session_013cPWotrjEY4Jac87e6vHsh
EOF
)"
```
