//! WebSocket to LSP subprocess bridge
//!
//! This module provides a WebSocket endpoint that spawns a cooklang-language-server
//! subprocess and bridges messages between the WebSocket client and the LSP server.

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    sync::mpsc,
};
use tracing::{debug, error, info, warn};

use super::AppState;

/// WebSocket upgrade handler for LSP connections
pub async fn lsp_websocket(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_lsp_connection(socket, state))
}

/// Handle a single LSP WebSocket connection
async fn handle_lsp_connection(socket: WebSocket, state: Arc<AppState>) {
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
    let (tx, mut rx) = mpsc::channel::<String>(32);

    // Task: Read from LSP stdout and send to channel
    let stdout_task = tokio::spawn(async move {
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
            if let Err(e) = tokio::io::AsyncReadExt::read_exact(&mut stdout_reader, &mut content).await {
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
    let stdin_task = tokio::spawn(async move {
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
    let ws_send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = ws_sender.send(Message::Text(msg)).await {
                error!("Error sending to WebSocket: {}", e);
                return;
            }
        }
    });

    // Wait for any task to complete
    tokio::select! {
        _ = stdout_task => {
            debug!("LSP stdout task completed");
        }
        _ = stdin_task => {
            debug!("WebSocket stdin task completed");
        }
        _ = ws_send_task => {
            debug!("WebSocket send task completed");
        }
    }

    // Kill the LSP process
    let _ = lsp_process.kill().await;

    Ok(())
}
