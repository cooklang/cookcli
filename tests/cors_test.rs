//! End-to-end tests for `cook server`'s CORS policy: the `tower_http`
//! `CorsLayer` built from `--cors-origin` / `--cors-allow-credentials`, and the
//! server-side write guard in `src/server/cors.rs` that sits inside it.
//!
//! Two mechanisms, two testing strategies:
//!
//! - The CORS **response headers** are browser-enforced, so a real `OPTIONS`
//!   preflight (with `Origin` and `Access-Control-Request-Method`) is the right
//!   way to observe them.
//! - The **write guard** exists precisely because headers cannot express its
//!   rule: `POST` is a CORS-safelisted method, so a browser never consults
//!   `Access-Control-Allow-Methods` before sending one — `allow_methods([GET])`
//!   does nothing to stop a cross-origin `POST`. Under the wildcard-origin
//!   default, `AllowOrigin::any()` also answers every preflight, including a
//!   `POST` preflight, with `Access-Control-Allow-Origin: *`. So a test that
//!   sends a `POST` preflight and asserts that header is absent would fail
//!   against *correct* code — it proves nothing about whether the write itself
//!   is refused. The only way to observe the guard is to send a real
//!   cross-origin `POST` and check the status code it comes back with. Do not
//!   "simplify" the write-guard tests below into preflight assertions.

#![cfg(feature = "server")]

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ORIGIN};
use reqwest::{Client, Method, Response, StatusCode};
use std::net::TcpListener;
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;
use tempfile::TempDir;

/// Kills the spawned server when the test ends, pass or panic.
struct ServerGuard {
    child: Child,
    port: u16,
    #[allow(dead_code)] // keeps the fixture directory alive for the server's lifetime
    dir: TempDir,
}

impl ServerGuard {
    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", self.port, path)
    }

    /// The `Origin` a same-origin request from this server's own web UI would
    /// carry.
    fn own_origin(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
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

/// One minimal recipe, just enough for the server to have something to scan.
fn write_fixture(dir: &TempDir) {
    std::fs::write(
        dir.path().join("Recipe.cook"),
        "Mix @flour{100%g} and @water{100%ml}.\n",
    )
    .unwrap();
}

/// `free_port` only reserves a port long enough to learn its number, so with
/// several tests booting servers at once another one can claim it first. The
/// server exits 1 on a bound port, so retry with a fresh one.
async fn start_server(extra_args: &[&str]) -> ServerGuard {
    for _ in 0..5 {
        if let Some(server) = try_start_server(extra_args).await {
            return server;
        }
    }
    panic!("could not start cook server on a free port after 5 attempts");
}

async fn try_start_server(extra_args: &[&str]) -> Option<ServerGuard> {
    let dir = TempDir::new().expect("temp dir");
    write_fixture(&dir);

    let port = free_port();
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin("cook"));
    cmd.arg("server")
        .arg(dir.path())
        .arg("--port")
        .arg(port.to_string());
    for arg in extra_args {
        cmd.arg(arg);
    }
    let child = cmd
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn cook server");

    let mut guard = ServerGuard { child, port, dir };

    // Plain GET with no Origin header: unaffected by any --cors-* flag, so
    // this is a valid readiness probe regardless of which policy a test asks
    // for.
    let client = Client::new();
    let url = guard.url("/api/menus");
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

/// Runs `cook server` against a fresh temp fixture and returns once the
/// process exits, without waiting for readiness. Used for the startup
/// validation tests, where the process is expected to fail before it ever
/// binds a listener.
fn run_server_startup(extra_args: &[&str]) -> Output {
    let dir = TempDir::new().expect("temp dir");
    write_fixture(&dir);

    // Reserved so a wrongly-successful startup can't collide with another
    // test's server rather than failing visibly.
    let port = free_port();
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin("cook"));
    cmd.arg("server")
        .arg(dir.path())
        .arg("--port")
        .arg(port.to_string());
    for arg in extra_args {
        cmd.arg(arg);
    }
    cmd.output().expect("run cook server")
}

/// Sends an `OPTIONS` preflight for `POST /api/pantry/add` from `origin`,
/// declaring an intent to send `request_method`, and returns the response
/// headers for the caller to assert on.
async fn preflight(server: &ServerGuard, origin: &str, request_method: &str) -> HeaderMap {
    Client::new()
        .request(Method::OPTIONS, server.url("/api/pantry/add"))
        .header(ORIGIN, origin)
        .header("access-control-request-method", request_method)
        .send()
        .await
        .expect("preflight request")
        .headers()
        .clone()
}

/// Reads a header as UTF-8 text, for readable assertion failures.
fn header(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(HeaderName::from_bytes(name.as_bytes()).expect("valid header name"))
        .map(|v: &HeaderValue| v.to_str().unwrap_or("<non-utf8>").to_string())
}

/// Sends a real (non-preflight) `POST /api/pantry/add`, optionally with an
/// `Origin` header, and returns the response. This is what actually exercises
/// the write guard — see the module doc comment for why a preflight cannot.
async fn post_pantry_add(server: &ServerGuard, origin: Option<&str>) -> Response {
    let mut req = Client::new()
        .post(server.url("/api/pantry/add"))
        .json(&serde_json::json!({ "section": "Test", "name": "Test Item" }));
    if let Some(origin) = origin {
        req = req.header(ORIGIN, origin);
    }
    req.send().await.expect("pantry add request")
}

async fn get_with_origin(server: &ServerGuard, path: &str, origin: &str) -> Response {
    Client::new()
        .get(server.url(path))
        .header(ORIGIN, origin)
        .send()
        .await
        .expect("GET request")
}

// ---------------------------------------------------------------------------
// Preflight / header tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn default_policy_preflight_get_is_wide_open() {
    let server = start_server(&[]).await;
    let headers = preflight(&server, "http://evil.test", "GET").await;

    assert_eq!(
        header(&headers, "access-control-allow-origin").as_deref(),
        Some("*"),
        "default policy must allow any origin for GET, got headers: {headers:?}"
    );
    let methods = header(&headers, "access-control-allow-methods");
    assert!(
        methods.as_deref().is_some_and(|m| m.contains("GET")),
        "GET must be an allowed method, got access-control-allow-methods: {methods:?}"
    );
}

#[tokio::test]
async fn default_policy_preflight_put_is_not_in_allowed_methods() {
    let server = start_server(&[]).await;
    let headers = preflight(&server, "http://evil.test", "PUT").await;

    // Deliberately not asserting anything about access-control-allow-origin
    // here: AllowOrigin::any() answers "*" on every preflight regardless of
    // the requested method, so its presence says nothing about PUT being
    // allowed.
    assert_eq!(
        header(&headers, "access-control-allow-methods").as_deref(),
        Some("GET"),
        "wildcard-origin policy must only ever advertise GET, got headers: {headers:?}"
    );
}

#[tokio::test]
async fn explicit_origin_preflight_post_from_listed_origin_is_allowed() {
    let server = start_server(&["--cors-origin", "http://app.test"]).await;
    let headers = preflight(&server, "http://app.test", "POST").await;

    assert_eq!(
        header(&headers, "access-control-allow-origin").as_deref(),
        Some("http://app.test"),
        "listed origin must be echoed back, got headers: {headers:?}"
    );
    let methods = header(&headers, "access-control-allow-methods");
    assert!(
        methods.as_deref().is_some_and(|m| m.contains("POST")),
        "POST must be allowed once an explicit origin is named, got access-control-allow-methods: {methods:?}"
    );
    let allow_headers = header(&headers, "access-control-allow-headers");
    assert!(
        allow_headers
            .as_deref()
            .is_some_and(|h| h.to_lowercase().contains("content-type")),
        "content-type must be an allowed request header, got access-control-allow-headers: {allow_headers:?}"
    );
}

#[tokio::test]
async fn explicit_origin_preflight_post_from_unlisted_origin_is_refused() {
    let server = start_server(&["--cors-origin", "http://app.test"]).await;
    let headers = preflight(&server, "http://evil.test", "POST").await;

    assert_eq!(
        header(&headers, "access-control-allow-origin"),
        None,
        "an origin outside the explicit list must get no access-control-allow-origin, got headers: {headers:?}"
    );
}

#[tokio::test]
async fn cors_allow_credentials_is_advertised_for_listed_origins() {
    let server = start_server(&[
        "--cors-origin",
        "http://app.test",
        "--cors-allow-credentials",
    ])
    .await;
    let headers = preflight(&server, "http://app.test", "POST").await;

    assert_eq!(
        header(&headers, "access-control-allow-credentials").as_deref(),
        Some("true"),
        "credentials must be advertised once opted in, got headers: {headers:?}"
    );
}

// ---------------------------------------------------------------------------
// Write-guard tests (real requests, not preflights — see module doc comment)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn default_policy_cross_origin_post_is_refused_with_403() {
    let server = start_server(&[]).await;
    let resp = post_pantry_add(&server, Some("http://evil.test")).await;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "cross-origin POST under the wildcard default must be refused, got {status}: {body}"
    );
    assert!(
        body.contains("--cors-origin"),
        "refusal body must tell the operator how to fix it, got: {body}"
    );
}

#[tokio::test]
async fn default_policy_same_origin_post_is_not_blocked() {
    let server = start_server(&[]).await;
    let own_origin = server.own_origin();
    let resp = post_pantry_add(&server, Some(&own_origin)).await;

    let status = resp.status();
    assert_ne!(
        status,
        StatusCode::FORBIDDEN,
        "a POST whose Origin matches the server's own Host (the web UI's own request) \
         must not be blocked by the write guard, got {status}"
    );
}

#[tokio::test]
async fn default_policy_post_without_origin_is_not_blocked() {
    let server = start_server(&[]).await;
    let resp = post_pantry_add(&server, None).await;

    let status = resp.status();
    assert_ne!(
        status,
        StatusCode::FORBIDDEN,
        "a POST with no Origin header (curl, scripts, other non-browser clients) \
         must not be blocked by the write guard, got {status}"
    );
}

#[tokio::test]
async fn default_policy_cross_origin_get_is_allowed() {
    let server = start_server(&[]).await;
    let resp = get_with_origin(&server, "/api/menus", "http://evil.test").await;

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "reads must stay open to any origin under the default policy, got {}",
        resp.status()
    );
}

#[tokio::test]
async fn explicit_origin_write_guard_matches_the_configured_list() {
    let server = start_server(&["--cors-origin", "http://app.test"]).await;

    let allowed = post_pantry_add(&server, Some("http://app.test")).await;
    assert_ne!(
        allowed.status(),
        StatusCode::FORBIDDEN,
        "a POST from a listed --cors-origin must not be blocked, got {}",
        allowed.status()
    );

    let refused = post_pantry_add(&server, Some("http://evil.test")).await;
    assert_eq!(
        refused.status(),
        StatusCode::FORBIDDEN,
        "a POST from an origin outside the --cors-origin list must be refused, got {}",
        refused.status()
    );
}

#[tokio::test]
async fn no_csrf_check_disables_the_write_guard() {
    let server = start_server(&["--no-csrf-check"]).await;
    let resp = post_pantry_add(&server, Some("http://evil.test")).await;

    let status = resp.status();
    assert_ne!(
        status,
        StatusCode::FORBIDDEN,
        "--no-csrf-check must disable the write guard entirely, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Startup validation tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn credentials_without_explicit_origin_fails_to_start() {
    let output = run_server_startup(&["--cors-allow-credentials"]);

    assert!(
        !output.status.success(),
        "server must refuse to start with --cors-allow-credentials and no --cors-origin, \
         exit status: {}",
        output.status
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--cors-origin"),
        "startup error must point at --cors-origin as the fix, got stderr: {stderr}"
    );
}

#[tokio::test]
async fn wildcard_mixed_with_explicit_origin_fails_to_start() {
    let output = run_server_startup(&["--cors-origin", "*", "--cors-origin", "http://app.test"]);

    assert!(
        !output.status.success(),
        "server must refuse to start when --cors-origin '*' is combined with an explicit origin, \
         exit status: {}",
        output.status
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot be combined"),
        "startup error must explain the conflict, got stderr: {stderr}"
    );
}
