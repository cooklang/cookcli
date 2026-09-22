//! WebSocket to LSP subprocess bridge
//!
//! This module provides a WebSocket endpoint that spawns a cooklang-language-server
//! subprocess and bridges messages between the WebSocket client and the LSP server.

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    sync::{mpsc, OwnedSemaphorePermit, Semaphore},
};
use tracing::{debug, error, info, warn};

use super::AppState;

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
pub async fn lsp_websocket(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
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

/// Handle a single LSP WebSocket connection
async fn handle_lsp_connection(
    socket: WebSocket,
    state: Arc<AppState>,
    // Frees its session slot when dropped, so it has to outlive everything
    // below — including the subprocess kill at the end of `run_bridge`.
    _permit: OwnedSemaphorePermit,
) {
    info!("LSP WebSocket connection established");

    // Spawn the LSP subprocess
    let lsp_process = match spawn_lsp_process(&state.base_path).await {
        Ok(process) => process,
        Err(e) => {
            error!("Failed to spawn LSP process: {}", e);
            return;
        }
    };

    // Run the bridge
    if let Err(e) = run_bridge(socket, lsp_process).await {
        error!("LSP bridge error: {}", e);
    }

    info!("LSP WebSocket connection closed");
}

/// Spawn the cooklang-language-server subprocess
async fn spawn_lsp_process(base_path: &camino::Utf8Path) -> Result<Child, std::io::Error> {
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
