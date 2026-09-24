//! End-to-end tests for the `--max-lsp-sessions` cap on `GET /api/ws/lsp`.
//!
//! Every accepted socket on that endpoint spawns a `cook lsp` subprocess and
//! keeps it alive for as long as the socket is open, and the endpoint has no
//! authentication in front of it. Without a cap, anything that can reach the
//! port — a script, or another machine when the server runs with `--host` —
//! exhausts the host's processes and memory just by reconnecting in a loop.
//!
//! What these tests pin:
//!
//! - sockets up to the cap are accepted;
//! - the one past it is **refused at the handshake** with `503`, not accepted
//!   and then dropped. That distinction is the point of claiming the slot
//!   before the upgrade: `templates/edit.html` reconnects three seconds after
//!   any close, so the editor behaves the same either way, but only a refused
//!   handshake tells an operator (or this test) what actually happened;
//! - closing a socket frees its slot, so the cap is a cap on *concurrent*
//!   sessions rather than a budget that runs out.
//!
//! The server runs as a real spawned process rather than as an in-process
//! router: the subprocess-per-socket behaviour under test only exists in a
//! real `cook server`, and `std::env::current_exe()` — which the bridge uses
//! to find the `lsp` subcommand — would otherwise point at this test binary.
//!
//! Every test asks for a cap far below the default, so the suite costs a
//! handful of `cook lsp` processes rather than the default eight per server.

#![cfg(all(feature = "server", feature = "lsp"))]

#[path = "common/mod.rs"]
mod common;

use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite, MaybeTlsStream, WebSocketStream};

/// How long to wait for a closed socket's slot to come free. The server
/// releases it only after the bridge task notices the close, kills the
/// subprocess and reaps it, which is asynchronous with respect to the client.
const SLOT_RELEASE_TIMEOUT: Duration = Duration::from_secs(30);

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

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

    fn ws_url(&self) -> String {
        format!("ws://127.0.0.1:{}/api/ws/lsp", self.port)
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

/// One recipe, so the server has something to scan and the language server
/// something to open.
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
async fn start_server(max_sessions: u16) -> ServerGuard {
    for _ in 0..5 {
        if let Some(server) = try_start_server(max_sessions).await {
            return server;
        }
    }
    panic!("could not start cook server on a free port after 5 attempts");
}

async fn try_start_server(max_sessions: u16) -> Option<ServerGuard> {
    let dir = TempDir::new().expect("temp dir");
    write_fixture(&dir);

    let port = free_port();
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin("cook"));
    cmd.arg("server")
        .arg(dir.path())
        .arg("--port")
        .arg(port.to_string())
        .arg("--max-lsp-sessions")
        .arg(max_sessions.to_string());
    let child = common::with_isolated_config(&mut cmd, dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn cook server");

    let mut guard = ServerGuard { child, port, dir };

    // A plain GET on an ordinary route: readiness only, and deliberately not
    // the websocket route, which would burn a session slot to answer.
    let client = reqwest::Client::new();
    let url = guard.url("/api/menus");
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

/// Opens one LSP websocket. The handshake completes only after the server has
/// claimed a session slot, so a returned socket is proof the slot is held.
async fn connect(server: &ServerGuard) -> Result<Socket, tungstenite::Error> {
    tokio_tungstenite::connect_async(server.ws_url())
        .await
        .map(|(socket, _response)| socket)
}

async fn connect_expecting_success(server: &ServerGuard) -> Socket {
    connect(server).await.expect("websocket handshake")
}

/// Opens one LSP websocket expecting to be turned away, and returns the status
/// the refusal came back with.
///
/// An accepted handshake fails the test here, and so does anything that is not
/// an HTTP response — a transport error, or a socket that opened and was then
/// closed. Being upgraded and then dropped is precisely what claiming the slot
/// before the upgrade exists to avoid, so it must not read as a refusal.
async fn connect_expecting_refusal(
    server: &ServerGuard,
    context: &str,
) -> tungstenite::http::StatusCode {
    let Err(error) = connect(server).await else {
        panic!("{context}");
    };
    match error {
        tungstenite::Error::Http(response) => response.status(),
        other => panic!("expected an HTTP refusal, got {other}"),
    }
}

/// Sends a websocket handshake by hand and reads the whole HTTP response.
///
/// `tokio-tungstenite` reports the refusal's status faithfully but fills the
/// response body from whatever bytes happened to be buffered behind the
/// headers, so asserting on the JSON through it would be asserting on read
/// timing. `reqwest` never upgrades — it just returns the `503` as an ordinary
/// response — which is exactly what is wanted here.
///
/// The headers are the ones `axum`'s `WebSocketUpgrade` extractor checks; the
/// key is a fixed nonce because nothing verifies it on a request that is
/// refused before the upgrade.
async fn handshake_response(server: &ServerGuard) -> reqwest::Response {
    reqwest::Client::new()
        .get(server.url("/api/ws/lsp"))
        .header("connection", "upgrade")
        .header("upgrade", "websocket")
        .header("sec-websocket-version", "13")
        .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
        .send()
        .await
        .expect("handshake request")
}

/// Retries until a slot frees up, or gives up with the last refusal.
async fn connect_once_a_slot_frees(server: &ServerGuard) -> Socket {
    let deadline = Instant::now() + SLOT_RELEASE_TIMEOUT;
    loop {
        match connect(server).await {
            Ok(socket) => return socket,
            Err(error) => {
                assert!(
                    Instant::now() < deadline,
                    "no session slot freed within {SLOT_RELEASE_TIMEOUT:?}; \
                     last handshake failed with {error}"
                );
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }
}

#[tokio::test]
async fn sessions_up_to_the_cap_are_accepted() {
    let server = start_server(3).await;

    // Held in a `Vec` for the rest of the test: dropping a socket would free
    // its slot and make the next handshake prove nothing.
    let sockets: Vec<Socket> = {
        let mut sockets = Vec::new();
        for _ in 0..3 {
            sockets.push(connect_expecting_success(&server).await);
        }
        sockets
    };

    assert_eq!(sockets.len(), 3);
}

/// An accepted socket is a *working* session, not merely an open one.
///
/// Worth its own test because the cap is carried by a permit threaded through
/// the bridge, and a mistake there — dropping it too early, or holding it past
/// a subprocess that never started — would still leave the handshake
/// succeeding. A round-trip through `cook lsp` is what rules that out.
#[tokio::test]
async fn a_session_within_the_cap_talks_to_the_language_server() {
    use futures_util::{SinkExt, StreamExt};

    let server = start_server(1).await;
    let mut socket = connect_expecting_success(&server).await;

    // The bridge re-frames this with the `Content-Length` header the
    // subprocess expects, and strips the header off the reply.
    socket
        .send(tungstenite::Message::text(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}"#,
        ))
        .await
        .expect("send initialize");

    let message = tokio::time::timeout(Duration::from_secs(30), socket.next())
        .await
        .expect("the language server should answer `initialize`")
        .expect("the socket should stay open")
        .expect("a websocket message");

    let reply: serde_json::Value =
        serde_json::from_str(message.to_text().expect("a text frame")).expect("JSON-RPC reply");
    assert_eq!(reply["id"], 1, "reply to the request we sent, got {reply}");
    assert!(
        reply["result"]["capabilities"].is_object(),
        "`initialize` should come back with server capabilities, got {reply}"
    );
}

#[tokio::test]
async fn a_session_past_the_cap_is_refused_with_503() {
    let server = start_server(2).await;

    let _held = [
        connect_expecting_success(&server).await,
        connect_expecting_success(&server).await,
    ];

    let status = connect_expecting_refusal(
        &server,
        "the third handshake should be refused while both slots are held",
    )
    .await;
    assert_eq!(status, tungstenite::http::StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn the_refusal_body_is_the_usual_json_error_shape() {
    let server = start_server(1).await;
    let _held = connect_expecting_success(&server).await;

    let response = handshake_response(&server).await;
    assert_eq!(response.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);

    let body: serde_json::Value = response.json().await.expect("JSON body");
    let message = body["error"].as_str().expect("an `error` string");
    assert!(
        message.contains("--max-lsp-sessions"),
        "the refusal should name the flag that raises the cap, got {message:?}"
    );
}

#[tokio::test]
async fn closing_a_session_frees_its_slot() {
    let server = start_server(1).await;

    let socket = connect_expecting_success(&server).await;
    let status = connect_expecting_refusal(
        &server,
        "the only slot should be taken while the first socket is open",
    )
    .await;
    assert_eq!(status, tungstenite::http::StatusCode::SERVICE_UNAVAILABLE);

    // Dropped rather than closed politely: a browser tab that crashes or a
    // laptop that sleeps is the case the cap has to survive, and it is the
    // one where the server, not the client, has to notice.
    drop(socket);

    let _reconnected = connect_once_a_slot_frees(&server).await;
}

#[tokio::test]
async fn zero_sessions_disables_the_bridge() {
    let server = start_server(0).await;

    let response = handshake_response(&server).await;
    assert_eq!(response.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);

    let body: serde_json::Value = response.json().await.expect("JSON body");
    let message = body["error"].as_str().expect("an `error` string");
    assert!(
        message.contains("disabled"),
        "a cap of 0 should say the bridge is off rather than that it is busy, got {message:?}"
    );
}
