//! WebSocket to LSP subprocess bridge
//!
//! This module provides a WebSocket endpoint that spawns a cooklang-language-server
//! subprocess and bridges messages between the WebSocket client and the LSP server.

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    http::{header, HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    Json,
};
use camino::Utf8Path;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    sync::{mpsc, OwnedSemaphorePermit, Semaphore},
};
use tracing::{debug, error, info, warn};

use super::{cors, AppState};

/// Buffer size for LSP message channel.
/// 32 messages provides adequate buffering for typical LSP traffic
/// while preventing unbounded memory growth.
const LSP_MESSAGE_BUFFER_SIZE: usize = 32;

/// How many `cook lsp` subprocesses the bridge runs at once unless
/// `--max-lsp-sessions` says otherwise.
///
/// The built-in editor opens one socket per edit tab, so eight is roomy for
/// the single-user case this server is built for, while keeping a client that
/// is *not* that editor — a script, or anything reaching a `--host` server —
/// from spawning subprocesses without bound.
pub const DEFAULT_MAX_SESSIONS: u16 = 8;

/// Caps how many LSP bridges — and so how many `cook lsp` subprocesses — run
/// at once.
///
/// The endpoint is unauthenticated and every accepted socket costs a process,
/// so without a cap one client can exhaust the machine's processes and memory.
pub struct SessionLimit {
    /// Separately `Arc`-wrapped so a permit can be owned by the bridge task,
    /// which outlives any borrow of the [`AppState`] holding this.
    permits: Arc<Semaphore>,
    max: u16,
}

impl SessionLimit {
    pub fn new(max: u16) -> Self {
        Self {
            permits: Arc::new(Semaphore::new(usize::from(max))),
            max,
        }
    }

    /// Claims a session slot, or returns `None` when every slot is taken.
    ///
    /// Never waits: a client queued behind a slot would hold an idle socket
    /// open for as long as someone else keeps an editor tab open, so a refusal
    /// the client can retry is the better answer.
    fn try_claim(&self) -> Option<OwnedSemaphorePermit> {
        Arc::clone(&self.permits).try_acquire_owned().ok()
    }

    /// The configured ceiling, for the message sent to a refused client.
    fn max(&self) -> u16 {
        self.max
    }
}

/// WebSocket upgrade handler for LSP connections
pub async fn lsp_websocket(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    // Before the session slot is claimed, so that a page which may not connect
    // cannot take slots from the editor even briefly by being refused over and
    // over, and so that a full server answers it `403` rather than telling it
    // how many sessions are in use.
    if state.csrf_check && !origin_may_connect(&state.cors, &headers, &uri) {
        warn!(
            origin = ?headers.get(header::ORIGIN),
            host = ?cors::request_authority(&headers, &uri),
            "refused a cross-origin WebSocket connection to the language server; \
             start the server with --cors-origin <ORIGIN> to allow that origin"
        );
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "Cross-origin pages may not connect to the language server. Start \
                          the server with --cors-origin <ORIGIN> to allow this origin, or \
                          --no-csrf-check to disable this check."
            })),
        )
            .into_response();
    }

    // Claimed before the upgrade, so a client that cannot be served is told so
    // by the handshake rather than by a socket that opens and immediately
    // closes — the editor treats those alike (it reconnects either way), but
    // only one of them is diagnosable from outside.
    let Some(permit) = state.lsp_sessions.try_claim() else {
        let max = state.lsp_sessions.max();
        warn!("refused an LSP WebSocket: {max} concurrent sessions allowed");
        let error = if max == 0 {
            "The language server bridge is disabled (--max-lsp-sessions 0).".to_string()
        } else {
            format!(
                "All {max} language server sessions are in use. Close an editor tab, \
                 or restart the server with a larger --max-lsp-sessions."
            )
        };
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": error })),
        )
            .into_response();
    };

    ws.on_upgrade(move |socket| handle_lsp_connection(socket, state, permit))
}

/// Whether this upgrade request may open a language server.
///
/// The handshake is a `GET`, so `cors::write_guard` lets it through, and
/// browsers apply no CORS to WebSockets at all: without this check, any page
/// the user visits could open `ws://127.0.0.1:9080/api/ws/lsp`. What a browser
/// does attach to every handshake is an `Origin` the page cannot choose, and
/// that is checked with [`cors::CorsConfig::trusts`].
///
/// A request with no `Origin` comes from no browser. The API has no
/// authentication for such a client to get around, and with the root pinned
/// (see [`Workspace`]) the language server shows it nothing the API does not.
fn origin_may_connect(cors: &cors::CorsConfig, headers: &HeaderMap, uri: &Uri) -> bool {
    let Some(origin) = headers.get(header::ORIGIN) else {
        return true;
    };
    let host = cors::request_authority(headers, uri).unwrap_or_default();
    origin
        .to_str()
        .is_ok_and(|origin| cors.trusts(origin, host))
}

/// Handle a single LSP WebSocket connection
async fn handle_lsp_connection(
    socket: WebSocket,
    state: Arc<AppState>,
    // Frees its session slot when dropped, so it has to outlive everything
    // below — including the subprocess kill at the end of `run_bridge`.
    _permit: OwnedSemaphorePermit,
) {
    info!("LSP WebSocket connection established");

    // Settled before anything is spawned: a language server whose root could
    // not be pinned would take whichever root the client names.
    let Some(workspace) = Workspace::new(&state.base_path) else {
        error!(
            "Cannot express {} as a file URI; refusing to start the LSP",
            state.base_path
        );
        return;
    };

    // Spawn the LSP subprocess
    let lsp_process = match spawn_lsp_process(&state.base_path).await {
        Ok(process) => process,
        Err(e) => {
            error!("Failed to spawn LSP process: {}", e);
            return;
        }
    };

    // Run the bridge
    if let Err(e) = run_bridge(socket, lsp_process, workspace).await {
        error!("LSP bridge error: {}", e);
    }

    info!("LSP WebSocket connection closed");
}

/// The workspace root the language server is told about: always the directory
/// this server serves, whatever the client asks for.
///
/// cooklang-language-server takes its root from the client, from `initialize`
/// (`workspaceFolders`, then `rootUri`, then `rootPath`) and later from
/// `workspace/didChangeWorkspaceFolders`. Completion then lists every `.cook`
/// and `.menu` file under that root, and it reads `config/aisle.conf` from it.
/// With no root at all, completion falls back to the parent directory of the
/// document's URI, which the client names as well; on Windows a
/// `file://host/...` URI makes that a `\\host\` network share. So a client able
/// to choose the root could list recipe and menu files anywhere the server's
/// user can read, and one able to drop it could steer the fallback.
///
/// The editor already names the base path, so it loses nothing. On Windows
/// its `'file://' + basePath` never parsed (the canonical `\\?\` prefix turns
/// into a query string), and it got the fallback instead.
struct Workspace {
    uri: String,
    path: String,
    name: String,
}

impl Workspace {
    fn new(base_path: &Utf8Path) -> Option<Self> {
        let uri = url::Url::from_file_path(base_path).ok()?;
        Some(Self {
            uri: uri.to_string(),
            path: base_path.to_string(),
            name: base_path.file_name().unwrap_or_default().to_string(),
        })
    }

    /// The message to forward in place of `text`, or `None` to drop it.
    ///
    /// Every forwarded message is re-serialized from what was parsed here, so
    /// the language server reads exactly what this function inspected. A
    /// duplicate `"method"` key, say, cannot mean one thing here and another
    /// there. JSON-RPC batches and anything else that is not a single object
    /// are dropped: the editor never sends them, and they would be one more
    /// shape to inspect.
    fn pin(&self, text: &str) -> Option<String> {
        let mut message: Value = serde_json::from_str(text).ok()?;
        let object = message.as_object_mut()?;
        match object.get("method").and_then(Value::as_str) {
            Some("initialize") => {
                // Positional (array) params cannot be rewritten by name, so
                // those are dropped along with a missing `params`.
                let params = object.get_mut("params")?.as_object_mut()?;
                params.insert("rootUri".into(), json!(self.uri));
                params.insert("rootPath".into(), json!(self.path));
                params.insert(
                    "workspaceFolders".into(),
                    json!([{ "uri": self.uri, "name": self.name }]),
                );
            }
            Some("workspace/didChangeWorkspaceFolders") => return None,
            _ => {}
        }
        Some(message.to_string())
    }
}

/// Spawn the cooklang-language-server subprocess
async fn spawn_lsp_process(base_path: &Utf8Path) -> Result<Child, std::io::Error> {
    // Get the path to the current executable
    let exe_path = std::env::current_exe()?;

    debug!("Spawning LSP process: {} lsp", exe_path.display());

    Command::new(exe_path)
        .arg("lsp")
        .current_dir(base_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit()) // Pass stderr through for debugging
        // `run_bridge` kills the child on every path it returns from, but it
        // can also be dropped mid-await — its task is aborted at shutdown, and
        // an early `?` returns before the kill. A child that outlives its
        // permit would make the session cap a cap on nothing, so tie the
        // process to the handle that holds the permit.
        .kill_on_drop(true)
        .spawn()
}

/// Bridge messages between WebSocket and LSP subprocess
async fn run_bridge(
    socket: WebSocket,
    mut lsp_process: Child,
    workspace: Workspace,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let stdin = lsp_process.stdin.take().ok_or("Failed to get stdin")?;
    let stdout = lsp_process.stdout.take().ok_or("Failed to get stdout")?;

    let (mut ws_sender, mut ws_receiver) = socket.split();
    let mut stdin_writer = stdin;
    let mut stdout_reader = BufReader::new(stdout);

    // Channel for sending messages from LSP to WebSocket
    let (tx, mut rx) = mpsc::channel::<String>(LSP_MESSAGE_BUFFER_SIZE);

    // Task: Read from LSP stdout and send to channel
    let mut stdout_handle = tokio::spawn(async move {
        loop {
            // Read headers until empty line
            let mut content_length: usize = 0;
            loop {
                let mut line = String::new();
                match stdout_reader.read_line(&mut line).await {
                    Ok(0) => {
                        debug!("LSP stdout closed");
                        return;
                    }
                    Ok(_) => {
                        let line = line.trim();
                        if line.is_empty() {
                            break;
                        }
                        if let Some(len_str) = line.strip_prefix("Content-Length: ") {
                            if let Ok(len) = len_str.parse() {
                                content_length = len;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error reading LSP stdout: {}", e);
                        return;
                    }
                }
            }

            if content_length == 0 {
                warn!("No Content-Length header found");
                continue;
            }

            // Read the JSON content
            let mut content = vec![0u8; content_length];
            if let Err(e) =
                tokio::io::AsyncReadExt::read_exact(&mut stdout_reader, &mut content).await
            {
                error!("Error reading LSP content: {}", e);
                return;
            }

            let json = match String::from_utf8(content) {
                Ok(s) => s,
                Err(e) => {
                    error!("Invalid UTF-8 from LSP: {}", e);
                    continue;
                }
            };

            debug!("LSP -> WS: {}", json);

            if tx.send(json).await.is_err() {
                debug!("WebSocket channel closed");
                return;
            }
        }
    });

    // Task: Read from WebSocket and write to LSP stdin
    let mut stdin_handle = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let Some(text) = workspace.pin(text.as_str()) else {
                        warn!(
                            "Dropped an LSP client message: not a single JSON object, \
                             or an attempt to move the workspace root"
                        );
                        continue;
                    };
                    debug!("WS -> LSP: {}", text);

                    // Write LSP message with Content-Length header
                    let message = format!("Content-Length: {}\r\n\r\n{}", text.len(), text);
                    if let Err(e) = stdin_writer.write_all(message.as_bytes()).await {
                        error!("Error writing to LSP stdin: {}", e);
                        return;
                    }
                    if let Err(e) = stdin_writer.flush().await {
                        error!("Error flushing LSP stdin: {}", e);
                        return;
                    }
                }
                Ok(Message::Close(_)) => {
                    debug!("WebSocket closed by client");
                    return;
                }
                Ok(_) => {
                    // Ignore binary, ping, pong messages
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    return;
                }
            }
        }
    });

    // Task: Send messages from channel to WebSocket
    let mut ws_send_handle = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = ws_sender.send(Message::Text(msg.into())).await {
                error!("Error sending to WebSocket: {}", e);
                return;
            }
        }
    });

    // Wait for any task to complete, then abort others
    tokio::select! {
        result = &mut stdout_handle => {
            debug!("LSP stdout task completed: {:?}", result);
        }
        result = &mut stdin_handle => {
            debug!("WebSocket stdin task completed: {:?}", result);
        }
        result = &mut ws_send_handle => {
            debug!("WebSocket send task completed: {:?}", result);
        }
    }

    // Abort remaining tasks to ensure clean shutdown
    stdout_handle.abort();
    stdin_handle.abort();
    ws_send_handle.abort();

    // Kill the LSP process
    let _ = lsp_process.kill().await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace() -> Workspace {
        Workspace {
            uri: "file:///srv/recipes".to_string(),
            path: "/srv/recipes".to_string(),
            name: "recipes".to_string(),
        }
    }

    fn pin(message: Value) -> Option<Value> {
        workspace()
            .pin(&message.to_string())
            .map(|text| serde_json::from_str(&text).expect("pin emits JSON"))
    }

    fn initialize(params: Value) -> Value {
        json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": params })
    }

    fn assert_pinned(params: &Value) {
        assert_eq!(params["rootUri"], "file:///srv/recipes");
        assert_eq!(params["rootPath"], "/srv/recipes");
        assert_eq!(
            params["workspaceFolders"],
            json!([{ "uri": "file:///srv/recipes", "name": "recipes" }])
        );
    }

    #[test]
    fn initialize_gets_the_base_path_whatever_root_the_client_names() {
        let pinned = pin(initialize(json!({
            "processId": null,
            "rootUri": "file:///",
            "rootPath": "/",
            "workspaceFolders": [{ "uri": "file:///home", "name": "home" }],
            "capabilities": { "textDocument": {} }
        })))
        .expect("initialize is forwarded");

        assert_pinned(&pinned["params"]);
        assert_eq!(pinned["id"], 1);
        assert_eq!(
            pinned["params"]["capabilities"],
            json!({ "textDocument": {} })
        );
    }

    #[test]
    fn initialize_without_a_root_still_gets_one() {
        // With no root, completion would fall back to the directory of a
        // document URI the client picks.
        let pinned = pin(initialize(json!({ "capabilities": {} }))).expect("forwarded");
        assert_pinned(&pinned["params"]);
    }

    #[test]
    fn initialize_that_cannot_be_rewritten_by_name_is_dropped() {
        assert_eq!(pin(initialize(json!(["file:///", {}]))), None);
        assert_eq!(pin(initialize(Value::Null)), None);
        assert_eq!(
            pin(json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" })),
            None
        );
    }

    #[test]
    fn workspace_folder_changes_are_dropped() {
        let change = json!({
            "jsonrpc": "2.0",
            "method": "workspace/didChangeWorkspaceFolders",
            "params": { "event": { "added": [{ "uri": "file:///", "name": "root" }], "removed": [] } }
        });
        assert_eq!(pin(change), None);
    }

    #[test]
    fn anything_but_a_single_object_is_dropped() {
        let batch = json!([initialize(
            json!({ "rootUri": "file:///", "capabilities": {} })
        )]);
        assert_eq!(pin(batch), None);
        assert_eq!(workspace().pin("not json"), None);
        assert_eq!(workspace().pin("42"), None);
    }

    #[test]
    fn a_duplicate_method_key_means_the_same_thing_to_the_server() {
        // serde_json keeps the last of two keys; the language server reads the
        // re-serialized message, which only has that one.
        let text = r#"{"jsonrpc":"2.0","id":1,"method":"textDocument/hover","method":"initialize","params":{"rootUri":"file:///","capabilities":{}}}"#;
        let pinned: Value =
            serde_json::from_str(&workspace().pin(text).expect("forwarded")).expect("JSON");
        assert_eq!(pinned["method"], "initialize");
        assert_pinned(&pinned["params"]);
    }

    #[test]
    fn other_messages_pass_through() {
        let completion = json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": "file:///elsewhere/Probe.cook" },
                "position": { "line": 0, "character": 3 }
            }
        });
        assert_eq!(pin(completion.clone()), Some(completion));

        let response = json!({ "jsonrpc": "2.0", "id": 3, "result": null });
        assert_eq!(pin(response.clone()), Some(response));
    }

    #[test]
    fn the_workspace_is_the_base_path_as_a_file_uri() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let path = Utf8Path::from_path(dir.path()).expect("UTF-8 temp dir");
        let workspace = Workspace::new(path).expect("an absolute path has a file URI");

        let uri = url::Url::parse(&workspace.uri).expect("valid URI");
        assert_eq!(uri.scheme(), "file");
        assert_eq!(uri.to_file_path().expect("file path"), dir.path());
        assert_eq!(workspace.path, path.as_str());
        assert_eq!(Some(workspace.name.as_str()), path.file_name());
    }

    #[cfg(windows)]
    #[test]
    fn a_canonicalized_windows_base_path_becomes_a_plain_drive_uri() {
        // `resolve_to_absolute_path` canonicalizes, which on Windows yields a
        // `\\?\` verbatim path.
        let workspace = Workspace::new(Utf8Path::new(r"\\?\C:\Users\cook\recipes"))
            .expect("verbatim disk paths have a file URI");
        assert_eq!(workspace.uri, "file:///C:/Users/cook/recipes");
    }
}
