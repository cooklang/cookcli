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

#[path = "common/mod.rs"]
mod common;

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

/// One minimal recipe, just enough for the server to have something to scan,
/// plus a pantry config so `POST /api/pantry/add` has somewhere to write and
/// a permitted write returns a real success instead of `404` (no pantry
/// configured) either way.
fn write_fixture(dir: &TempDir) {
    std::fs::write(
        dir.path().join("Recipe.cook"),
        "Mix @flour{100%g} and @water{100%ml}.\n",
    )
    .unwrap();

    let config_dir = dir.path().join("config");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(
        config_dir.join("pantry.conf"),
        "[pantry]\nflour = \"1%kg\"\n",
    )
    .unwrap();
}

/// `free_port` only reserves a port long enough to learn its number, so with
/// several tests booting servers at once another one can claim it first. The
/// server exits 1 on a bound port, so retry with a fresh one.
async fn start_server(extra_args: &[&str]) -> ServerGuard {
    start_server_with_env(extra_args, &[]).await
}

/// [`start_server`] with extra environment variables, for the settings a
/// container names that way rather than on the command line.
async fn start_server_with_env(extra_args: &[&str], env: &[(&str, &str)]) -> ServerGuard {
    for _ in 0..5 {
        if let Some(server) = try_start_server(extra_args, env).await {
            return server;
        }
    }
    panic!("could not start cook server on a free port after 5 attempts");
}

async fn try_start_server(extra_args: &[&str], env: &[(&str, &str)]) -> Option<ServerGuard> {
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
    for (name, value) in env {
        cmd.env(name, value);
    }
    let child = common::with_isolated_config(&mut cmd, dir.path())
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
    // 30s. Every test here boots its own server, so a full parallel run spawns
    // more of them at once than the suites this pattern came from — a loaded
    // machine can take well past the 10s that was enough for nine.
    for _ in 0..600 {
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
    common::with_isolated_config(&mut cmd, dir.path())
        .output()
        .expect("run cook server")
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
async fn a_forwarded_host_header_cannot_fake_same_origin() {
    // The guard reads the real `Host` header, never `Forwarded` /
    // `X-Forwarded-Host`. Those are set by whoever is on the other end of a
    // direct connection, so trusting them would let `Origin: http://evil.test`
    // plus `X-Forwarded-Host: evil.test` pass as same-origin.
    let server = start_server(&[]).await;

    for spoof in [
        ("x-forwarded-host", "evil.test"),
        ("forwarded", "host=evil.test"),
    ] {
        let resp = Client::new()
            .post(server.url("/api/pantry/add"))
            .json(&serde_json::json!({ "section": "Test", "name": "Test Item" }))
            .header(ORIGIN, "http://evil.test")
            .header(spoof.0, spoof.1)
            .send()
            .await
            .expect("pantry add request");

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "{}: {} must not make a cross-origin write look same-origin, got {status}: {body}",
            spoof.0,
            spoof.1
        );
    }
}

#[tokio::test]
async fn default_policy_same_origin_post_is_not_blocked() {
    let server = start_server(&[]).await;
    let own_origin = server.own_origin();
    let resp = post_pantry_add(&server, Some(&own_origin)).await;

    let status = resp.status();
    assert_eq!(
        status,
        StatusCode::OK,
        "a POST whose Origin matches the server's own Host (the web UI's own request) \
         must not be blocked by the write guard and must actually succeed, got {status}"
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
    assert_eq!(
        allowed.status(),
        StatusCode::OK,
        "a POST from a listed --cors-origin must not be blocked and must actually succeed, got {}",
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
// LSP websocket tests (real handshakes: browsers apply no CORS to WebSockets,
// and the handshake is a GET the write guard lets through, so refusing the
// upgrade is the only defence, and only a handshake can observe it)
// ---------------------------------------------------------------------------

mod lsp_socket {
    use super::*;
    use tokio_tungstenite::tungstenite::{self, client::IntoClientRequest};

    type Socket = tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >;

    /// Performs a WebSocket handshake with `/api/ws/lsp`, adding `headers` to
    /// the ones the protocol requires (a `host` given here replaces the
    /// generated one; the connection still goes to 127.0.0.1). Returns the
    /// status the server answered with, and the socket if it upgraded.
    async fn handshake(
        server: &ServerGuard,
        headers: &[(&str, &str)],
    ) -> (StatusCode, Option<Socket>) {
        let mut request = format!("ws://127.0.0.1:{}/api/ws/lsp", server.port)
            .into_client_request()
            .expect("valid websocket request");
        for (name, value) in headers {
            request.headers_mut().insert(
                HeaderName::from_bytes(name.as_bytes()).expect("valid header name"),
                HeaderValue::from_str(value).expect("valid header value"),
            );
        }
        match tokio_tungstenite::connect_async(request).await {
            Ok((socket, response)) => (response.status(), Some(socket)),
            Err(tungstenite::Error::Http(response)) => (response.status(), None),
            Err(e) => panic!("websocket handshake failed: {e}"),
        }
    }

    #[tokio::test]
    async fn a_cross_origin_page_is_refused_with_403() {
        let server = start_server(&[]).await;
        let (status, socket) = handshake(&server, &[("origin", "http://evil.test")]).await;

        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "a page on another origin must not get a language server"
        );
        assert!(socket.is_none());
    }

    #[tokio::test]
    async fn the_servers_own_page_is_upgraded() {
        // The editor's own connection, under both of the names a local server
        // is normally reached by.
        let server = start_server(&[]).await;
        for host in [
            format!("127.0.0.1:{}", server.port),
            format!("localhost:{}", server.port),
        ] {
            let origin = format!("http://{host}");
            let (status, _socket) =
                handshake(&server, &[("host", &host), ("origin", &origin)]).await;
            assert_eq!(
                status,
                StatusCode::SWITCHING_PROTOCOLS,
                "Origin {origin} on Host {host} is the server's own editor and must connect"
            );
        }
    }

    #[tokio::test]
    async fn a_rebound_dns_name_is_refused_even_though_it_matches() {
        // DNS rebinding: a hostile page at rebind.test re-resolves its own name
        // to 127.0.0.1, so its Origin and the Host it sends agree exactly.
        let server = start_server(&[]).await;
        let host = format!("rebind.test:{}", server.port);
        let origin = format!("http://{host}");
        let (status, _socket) = handshake(&server, &[("host", &host), ("origin", &origin)]).await;

        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "a same-origin pair under a DNS name must not be trusted without --cors-origin"
        );
    }

    #[tokio::test]
    async fn a_client_without_origin_is_upgraded() {
        // No browser, so nothing to protect it from: editors and scripts.
        let server = start_server(&[]).await;
        let (status, _socket) = handshake(&server, &[]).await;

        assert_eq!(status, StatusCode::SWITCHING_PROTOCOLS);
    }

    #[tokio::test]
    async fn a_listed_origin_is_upgraded_and_others_are_not() {
        let server = start_server(&["--cors-origin", "http://app.test"]).await;

        let (status, _socket) = handshake(&server, &[("origin", "http://app.test")]).await;
        assert_eq!(
            status,
            StatusCode::SWITCHING_PROTOCOLS,
            "a listed --cors-origin must be able to connect"
        );

        let (status, _socket) = handshake(&server, &[("origin", "http://evil.test")]).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn a_full_server_still_refuses_a_cross_origin_page_with_403() {
        // The origin is checked before a session slot is claimed, so a page
        // that may not connect is told that, rather than being told how many
        // sessions are in use — and it cannot take slots from the editor by
        // being refused over and over.
        let server = start_server(&["--max-lsp-sessions", "1"]).await;

        let own_origin = server.own_origin();
        let (status, socket) = handshake(&server, &[("origin", &own_origin)]).await;
        assert_eq!(status, StatusCode::SWITCHING_PROTOCOLS);
        let _held = socket.expect("the only session slot is now taken");

        let (status, _socket) = handshake(&server, &[("origin", "http://evil.test")]).await;
        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "a full server must refuse a cross-origin page for being cross-origin, not for \
             being full"
        );
    }

    #[tokio::test]
    async fn an_origin_named_by_the_environment_is_upgraded() {
        // A container names its origin with COOK_CORS_ORIGIN rather than a
        // flag, and the editor there would lose its language server if the
        // socket read a different policy than writes do.
        let server = start_server_with_env(&[], &[("COOK_CORS_ORIGIN", "http://app.test")]).await;

        let (status, _socket) = handshake(&server, &[("origin", "http://app.test")]).await;
        assert_eq!(
            status,
            StatusCode::SWITCHING_PROTOCOLS,
            "an origin from COOK_CORS_ORIGIN must be able to connect"
        );

        let (status, _socket) = handshake(&server, &[("origin", "http://evil.test")]).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn no_csrf_check_disables_the_origin_check() {
        let server = start_server(&["--no-csrf-check"]).await;
        let (status, _socket) = handshake(&server, &[("origin", "http://evil.test")]).await;

        assert_eq!(status, StatusCode::SWITCHING_PROTOCOLS);
    }

    /// Whole conversations with the language server, which exists only in a
    /// build with the `lsp` feature.
    #[cfg(feature = "lsp")]
    mod session {
        use super::*;
        use futures_util::{SinkExt, StreamExt};
        use serde_json::{json, Value};
        use tokio_tungstenite::tungstenite::Message;

        async fn send(socket: &mut Socket, message: Value) {
            socket
                .send(Message::text(message.to_string()))
                .await
                .expect("send to the language server");
        }

        /// Reads messages until one satisfies `wanted`, skipping log messages
        /// and anything else the language server sends in between.
        async fn receive(socket: &mut Socket, wanted: impl Fn(&Value) -> bool) -> Value {
            let read = async {
                while let Some(frame) = socket.next().await {
                    if let Message::Text(text) = frame.expect("read from the language server") {
                        let message: Value =
                            serde_json::from_str(text.as_str()).expect("messages are bare JSON");
                        if wanted(&message) {
                            return message;
                        }
                    }
                }
                panic!("the socket closed before the expected message arrived");
            };
            tokio::time::timeout(Duration::from_secs(30), read)
                .await
                .expect("the language server did not answer within 30s")
        }

        /// The response to request `id`, as opposed to a request from the
        /// server.
        fn is_response(message: &Value, id: u64) -> bool {
            message["id"] == id && message.get("method").is_none()
        }

        #[tokio::test]
        async fn the_workspace_root_is_the_base_path_whatever_the_client_names() {
            // A client that names another directory as its root, then tries to
            // move there, must still only be offered the server's own recipes.
            // The probe document lives in that directory too, so the fallback
            // to the document's parent (used when there is no root) is covered.
            let server = start_server(&[]).await;
            let elsewhere = TempDir::new().expect("temp dir");
            std::fs::write(
                elsewhere.path().join("Elsewhere.cook"),
                "Boil @water{1%l}.\n",
            )
            .unwrap();
            let elsewhere_uri = url::Url::from_file_path(elsewhere.path())
                .expect("file URI")
                .to_string();
            let folder = json!([{ "uri": elsewhere_uri, "name": "elsewhere" }]);
            let probe_uri = format!("{elsewhere_uri}/Probe.cook");

            let (status, socket) = handshake(&server, &[]).await;
            assert_eq!(status, StatusCode::SWITCHING_PROTOCOLS);
            let mut socket = socket.expect("upgraded");

            send(
                &mut socket,
                json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                    "processId": null,
                    "rootUri": elsewhere_uri,
                    "rootPath": elsewhere.path().to_str(),
                    "workspaceFolders": folder,
                    "capabilities": {}
                }}),
            )
            .await;
            receive(&mut socket, |m| is_response(m, 1)).await;
            send(
                &mut socket,
                json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }),
            )
            .await;
            send(
                &mut socket,
                json!({ "jsonrpc": "2.0", "method": "workspace/didChangeWorkspaceFolders", "params": {
                    "event": { "added": folder, "removed": [] }
                }}),
            )
            .await;
            send(
                &mut socket,
                json!({ "jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
                    "textDocument": {
                        "uri": probe_uri,
                        "languageId": "cooklang",
                        "version": 1,
                        "text": "@./"
                    }
                }}),
            )
            .await;
            // Diagnostics are published once the document is open, so
            // completion cannot overtake it.
            receive(&mut socket, |m| {
                m["method"] == "textDocument/publishDiagnostics"
            })
            .await;
            send(
                &mut socket,
                json!({ "jsonrpc": "2.0", "id": 2, "method": "textDocument/completion", "params": {
                    "textDocument": { "uri": probe_uri },
                    "position": { "line": 0, "character": 3 }
                }}),
            )
            .await;
            let response = receive(&mut socket, |m| is_response(m, 2)).await;

            let labels: Vec<&str> = response["result"]["items"]
                .as_array()
                .unwrap_or_else(|| panic!("a completion list, got {response}"))
                .iter()
                .filter_map(|item| item["label"].as_str())
                .collect();
            assert!(
                labels.contains(&"./Recipe"),
                "the base path's recipes must be offered, got {labels:?}"
            );
            assert!(
                !labels.iter().any(|label| label.contains("Elsewhere")),
                "nothing outside the base path may be listed, got {labels:?}"
            );
        }
    }
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

// ---------------------------------------------------------------------------
// DNS rebinding
//
// A page served from `http://evil.test:{port}` can re-point `evil.test` at
// this machine once it has loaded. From then on its requests carry
// `Origin: http://evil.test:{port}` *and* `Host: evil.test:{port}`, so an
// Origin that matches the Host proves nothing on its own. Only a Host that no
// DNS answer can redirect — `localhost` or an IP address — counts as the
// server's own address; any other name has to be listed with `--cors-origin`.
//
// These tests set `Host` by hand. reqwest keeps a caller's `Host` (hyper only
// fills it in when it is missing), and `a_host_header_override_reaches_the_server`
// pins that down: without it, the refusals below could come from an ordinary
// Origin/Host mismatch and prove nothing about rebinding.
// ---------------------------------------------------------------------------

impl ServerGuard {
    /// Whether the fixture directory holds a recipe named `name`.
    fn recipe_exists(&self, name: &str) -> bool {
        self.dir.path().join(format!("{name}.cook")).exists()
    }
}

/// `POST /api/pantry/add` as a page on `origin` sends it to a server it
/// reached as `host`.
async fn post_pantry_add_as(server: &ServerGuard, host: &str, origin: &str) -> Response {
    Client::new()
        .post(server.url("/api/pantry/add"))
        .json(&serde_json::json!({ "section": "Test", "name": "Test Item" }))
        .header(reqwest::header::HOST, host)
        .header(ORIGIN, origin)
        .send()
        .await
        .expect("pantry add request")
}

/// Submits the web UI's new-recipe form, `POST /new`, with extra `headers`.
///
/// Redirects are not followed: a created recipe (303 to its editor) and a
/// validation error (303 back to the form) would otherwise both end on a
/// `200` page, while a refusal is a `403` either way.
async fn post_new_recipe(server: &ServerGuard, name: &str, headers: &[(&str, &str)]) -> Response {
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("http client");
    let mut req = client.post(server.url("/new")).form(&[("filename", name)]);
    for (header, value) in headers {
        req = req.header(*header, *value);
    }
    req.send().await.expect("new recipe request")
}

/// Asserts that `POST /new` created recipe `name` and sent the browser to its
/// editor.
fn assert_recipe_created(server: &ServerGuard, name: &str, resp: &Response) {
    let status = resp.status();
    assert_eq!(
        status,
        StatusCode::SEE_OTHER,
        "{name}: the form post must be accepted and redirect, got {status}"
    );
    assert_eq!(
        header(resp.headers(), "location").as_deref(),
        Some(format!("/edit/{name}.cook").as_str()),
        "{name}: the redirect must lead to the new recipe's editor, not back to the form"
    );
    assert!(server.recipe_exists(name), "{name}: {name}.cook must exist");
}

#[tokio::test]
async fn a_host_header_override_reaches_the_server() {
    // The server's own origin, sent to another Host. It is refused only if the
    // server sees the Host set here: were it replaced with 127.0.0.1:{port},
    // Origin and Host would match and the write would go through.
    let server = start_server(&[]).await;
    let resp = post_pantry_add_as(&server, "cook.test", &server.own_origin()).await;

    let status = resp.status();
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "the Host header a test sets must reach the server unchanged, got {status}"
    );
}

#[tokio::test]
async fn a_rebound_host_name_cannot_write() {
    let server = start_server(&[]).await;
    let host = format!("evil.test:{}", server.port);
    let origin = format!("http://{host}");
    let resp = post_pantry_add_as(&server, &host, &origin).await;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "an Origin that matches a host *name* may come from a page that rebound the name to \
         this server, so it must not pass as same-origin, got {status}: {body}"
    );
    // Someone who really opens the web UI at a host name gets the same
    // refusal, so it has to tell them the exact flag.
    assert!(
        body.contains(&format!("--cors-origin {origin}")),
        "refusal must name the flag that allows this origin, got: {body}"
    );
    assert!(
        body.contains("--no-csrf-check"),
        "refusal must still mention --no-csrf-check, got: {body}"
    );
}

#[tokio::test]
async fn localhost_is_trusted_as_the_servers_own_address() {
    // Browsers resolve `localhost` themselves, so no DNS answer can point it
    // anywhere else. 127.0.0.1 is `default_policy_same_origin_post_is_not_blocked`.
    let server = start_server(&[]).await;
    let host = format!("localhost:{}", server.port);
    let resp = post_pantry_add_as(&server, &host, &format!("http://{host}")).await;

    let status = resp.status();
    assert_eq!(
        status,
        StatusCode::OK,
        "the web UI opened at http://localhost must be able to write, got {status}"
    );
}

#[tokio::test]
async fn a_host_name_named_by_cors_origin_can_write() {
    // How someone who opens the web UI at a host name — a NAS on the local
    // network, a reverse proxy that passes Host through — keeps it working.
    let server = start_server(&["--cors-origin", "http://cook.test"]).await;
    let resp = post_pantry_add_as(&server, "cook.test", "http://cook.test").await;

    let status = resp.status();
    assert_eq!(
        status,
        StatusCode::OK,
        "a host name listed with --cors-origin must be able to write, got {status}"
    );
}

#[tokio::test]
async fn new_recipe_form_refuses_a_rebound_host_name() {
    let server = start_server(&[]).await;
    let host = format!("evil.test:{}", server.port);
    let origin = format!("http://{host}");
    let referer = format!("{origin}/new");

    // With an Origin, the write guard refuses. With only a Referer — a browser
    // that leaves Origin off a same-origin form post — the guard sees no
    // browser and lets it through, so the form's own check has to refuse.
    for (name, headers) in [
        (
            "ReboundOrigin",
            [("host", host.as_str()), ("origin", origin.as_str())],
        ),
        (
            "ReboundReferer",
            [("host", host.as_str()), ("referer", referer.as_str())],
        ),
    ] {
        let resp = post_new_recipe(&server, name, &headers).await;

        let status = resp.status();
        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "{name}: a form post to a rebound host name must be refused, got {status}"
        );
        assert!(
            !server.recipe_exists(name),
            "{name}: a refused form post must not create the recipe"
        );
    }
}

#[tokio::test]
async fn new_recipe_form_accepts_its_own_address() {
    let server = start_server(&[]).await;
    let origin = server.own_origin();
    let referer = format!("{origin}/new");

    for (name, headers) in [
        ("OwnOrigin", [("origin", origin.as_str())]),
        ("OwnReferer", [("referer", referer.as_str())]),
    ] {
        let resp = post_new_recipe(&server, name, &headers).await;
        assert_recipe_created(&server, name, &resp);
    }
}

#[tokio::test]
async fn new_recipe_form_honours_cors_origin() {
    // Behind a reverse proxy that rewrites Host, the browser's Origin never
    // matches it. Naming the public origin already lets the API write; the
    // form has to accept it too.
    let server = start_server(&["--cors-origin", "http://app.test"]).await;

    for (name, headers) in [
        ("ListedOrigin", [("origin", "http://app.test")]),
        ("ListedReferer", [("referer", "http://app.test/new")]),
    ] {
        let resp = post_new_recipe(&server, name, &headers).await;
        assert_recipe_created(&server, name, &resp);
    }
}

// ---------------------------------------------------------------------------
// Origins from the environment
//
// `COOK_CORS_ORIGIN` exists for containers: the published image's command
// already names the recipe directory and `--host`, so adding one flag means
// restating all of it, while a compose file adds a variable in one line.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn cors_origin_can_come_from_the_environment() {
    let server = start_server_with_env(&[], &[("COOK_CORS_ORIGIN", "http://app.test")]).await;

    let allowed = post_pantry_add(&server, Some("http://app.test")).await;
    assert_eq!(
        allowed.status(),
        StatusCode::OK,
        "an origin named by COOK_CORS_ORIGIN must write as if it had been passed as a flag, got {}",
        allowed.status()
    );

    let refused = post_pantry_add(&server, Some("http://evil.test")).await;
    assert_eq!(
        refused.status(),
        StatusCode::FORBIDDEN,
        "naming one origin in the environment must not let every origin write, got {}",
        refused.status()
    );
}

#[tokio::test]
async fn several_origins_fit_in_the_environment_variable() {
    let server = start_server_with_env(
        &[],
        &[("COOK_CORS_ORIGIN", "http://app.test, http://other.test")],
    )
    .await;

    for origin in ["http://app.test", "http://other.test"] {
        let resp = post_pantry_add(&server, Some(origin)).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "{origin} was listed in COOK_CORS_ORIGIN, got {}",
            resp.status()
        );
    }
}

#[tokio::test]
async fn the_cors_origin_flag_overrides_the_environment() {
    let server = start_server_with_env(
        &["--cors-origin", "http://flag.test"],
        &[("COOK_CORS_ORIGIN", "http://env.test")],
    )
    .await;

    let allowed = post_pantry_add(&server, Some("http://flag.test")).await;
    assert_eq!(
        allowed.status(),
        StatusCode::OK,
        "the flag's origin must be the one in force, got {}",
        allowed.status()
    );

    let refused = post_pantry_add(&server, Some("http://env.test")).await;
    assert_eq!(
        refused.status(),
        StatusCode::FORBIDDEN,
        "a command line must replace the environment's origins, not add to them, got {}",
        refused.status()
    );
}

#[tokio::test]
async fn an_empty_cors_origin_environment_variable_is_not_an_origin() {
    // `COOK_CORS_ORIGIN=${UNDEFINED}` in a compose file. The server must start
    // as if nothing were set: `start_server_with_env` only returns once it
    // answers, so getting a guard back is half the assertion.
    let server = start_server_with_env(&[], &[("COOK_CORS_ORIGIN", "")]).await;

    let own_origin = server.own_origin();
    let resp = post_pantry_add(&server, Some(&own_origin)).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "an empty variable must leave the default policy in place, got {}",
        resp.status()
    );
}
