# Recipe Editor Phase 3: LSP WebSocket Bridge

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create a WebSocket endpoint that bridges browser connections to the cooklang-language-server subprocess, enabling LSP features in the editor.

**Architecture:** Browser ↔ WebSocket ↔ Bridge ↔ stdio ↔ LSP subprocess

**Tech Stack:** Axum WebSocket, tokio process, JSON-RPC message framing

---

## Task 1: Add WebSocket feature to Axum

**Files:**
- Modify: `Cargo.toml`

**Step 1: Update axum dependency**

Change:
```toml
axum = { version = "0.7" }
```

To:
```toml
axum = { version = "0.7", features = ["ws"] }
```

**Step 2: Verify build**

Run: `cargo build`

Expected: Build succeeds with WebSocket support available.

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "build: add WebSocket feature to axum"
```

---

## Task 2: Create LSP bridge module

**Files:**
- Create: `src/server/lsp_bridge.rs`

**Step 1: Create the LSP bridge module**

```rust
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
async fn run_bridge(socket: WebSocket, mut lsp_process: Child) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let stdin = lsp_process.stdin.take().ok_or("Failed to get stdin")?;
    let stdout = lsp_process.stdout.take().ok_or("Failed to get stdout")?;

    let (mut ws_sender, mut ws_receiver) = socket.split();
    let mut stdin_writer = stdin;
    let mut stdout_reader = BufReader::new(stdout);

    // Channel for sending messages from LSP to WebSocket
    let (tx, mut rx) = mpsc::channel::<String>(32);

    // Task: Read from LSP stdout and send to channel
    let stdout_task = tokio::spawn(async move {
        let mut headers = String::new();
        loop {
            headers.clear();

            // Read headers until empty line
            loop {
                let mut line = String::new();
                match stdout_reader.read_line(&mut line).await {
                    Ok(0) => {
                        debug!("LSP stdout closed");
                        return;
                    }
                    Ok(_) => {
                        if line == "\r\n" || line == "\n" {
                            break;
                        }
                        headers.push_str(&line);
                    }
                    Err(e) => {
                        error!("Error reading LSP stdout: {}", e);
                        return;
                    }
                }
            }

            // Parse Content-Length header
            let content_length: usize = headers
                .lines()
                .find_map(|line| {
                    line.strip_prefix("Content-Length: ")
                        .and_then(|v| v.trim().parse().ok())
                })
                .unwrap_or(0);

            if content_length == 0 {
                warn!("No Content-Length header found");
                continue;
            }

            // Read the JSON content
            let mut content = vec![0u8; content_length];
            if let Err(e) = stdout_reader.read_exact(&mut content).await {
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
        use futures_util::StreamExt;

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
        use futures_util::SinkExt;

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
```

**Step 2: Add futures-util dependency for stream operations**

Add to Cargo.toml dependencies:
```toml
futures-util = "0.3"
```

**Step 3: Commit**

```bash
git add src/server/lsp_bridge.rs Cargo.toml Cargo.lock
git commit -m "feat(lsp): add WebSocket to LSP subprocess bridge

Bridges browser WebSocket connections to cooklang-language-server:
- Spawns 'cook lsp' subprocess per connection
- Forwards JSON-RPC messages bidirectionally
- Handles LSP Content-Length framing
- Cleans up subprocess on disconnect"
```

---

## Task 3: Register LSP bridge in server module

**Files:**
- Modify: `src/server/mod.rs`

**Step 1: Add module declaration**

Add after other module declarations (around line 53):
```rust
mod lsp_bridge;
```

**Step 2: Add WebSocket route**

In the `api()` function, add the LSP route. Change the function to:
```rust
fn api(_state: &AppState) -> Result<Router<Arc<AppState>>> {
    let router = Router::new()
        // ... existing routes ...
        .route("/ws/lsp", get(lsp_bridge::lsp_websocket));

    Ok(router)
}
```

**Step 3: Verify build**

Run: `cargo build`

**Step 4: Commit**

```bash
git add src/server/mod.rs
git commit -m "feat(lsp): register WebSocket LSP endpoint at /api/ws/lsp"
```

---

## Task 4: Test WebSocket endpoint manually

**Step 1: Start the server**

Run: `cargo run -- server ./seed`

**Step 2: Test WebSocket connection**

Use websocat or browser console to test:
```bash
# Install websocat if needed: cargo install websocat
websocat ws://localhost:9080/api/ws/lsp
```

Then send an LSP initialize message:
```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":"file:///tmp","capabilities":{}}}
```

Expected: Receive an initialize response from the LSP server.

**Step 3: Verify server logs**

Check that logs show:
- "LSP WebSocket connection established"
- "Spawning LSP process"
- Message forwarding debug logs

---

## Task 5: Add connection status indicator to editor

**Files:**
- Modify: `templates/edit.html`

**Step 1: Add status bar to template**

Add a status bar at the bottom of the editor area showing LSP connection status:

```html
<!-- Status bar (add before closing </div> of flex container) -->
<div id="status-bar" class="mt-2 px-4 py-2 bg-gray-100 rounded-lg flex items-center justify-between text-sm">
    <div class="flex items-center gap-2">
        <span id="lsp-status" class="flex items-center gap-1">
            <span id="lsp-indicator" class="w-2 h-2 rounded-full bg-gray-400"></span>
            <span id="lsp-text">Disconnected</span>
        </span>
    </div>
    <div id="cursor-position" class="text-gray-500">
        Line 1, Col 1
    </div>
</div>
```

**Step 2: Add JavaScript for LSP connection**

Add to the script section:
```javascript
// LSP WebSocket connection
let lspSocket = null;
let lspMessageId = 1;

function connectLsp() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/api/ws/lsp`;

    lspSocket = new WebSocket(wsUrl);

    lspSocket.onopen = function() {
        updateLspStatus('connected');
        // Send initialize request
        sendLspRequest('initialize', {
            processId: null,
            rootUri: 'file://' + window.location.pathname,
            capabilities: {
                textDocument: {
                    completion: {
                        completionItem: {
                            snippetSupport: false
                        }
                    },
                    publishDiagnostics: {
                        relatedInformation: true
                    }
                }
            }
        });
    };

    lspSocket.onclose = function() {
        updateLspStatus('disconnected');
        // Reconnect after delay
        setTimeout(connectLsp, 3000);
    };

    lspSocket.onerror = function(error) {
        console.error('LSP WebSocket error:', error);
        updateLspStatus('error');
    };

    lspSocket.onmessage = function(event) {
        try {
            const message = JSON.parse(event.data);
            handleLspMessage(message);
        } catch (e) {
            console.error('Failed to parse LSP message:', e);
        }
    };
}

function sendLspRequest(method, params) {
    if (!lspSocket || lspSocket.readyState !== WebSocket.OPEN) {
        return null;
    }
    const id = lspMessageId++;
    const message = {
        jsonrpc: '2.0',
        id: id,
        method: method,
        params: params
    };
    lspSocket.send(JSON.stringify(message));
    return id;
}

function sendLspNotification(method, params) {
    if (!lspSocket || lspSocket.readyState !== WebSocket.OPEN) {
        return;
    }
    const message = {
        jsonrpc: '2.0',
        method: method,
        params: params
    };
    lspSocket.send(JSON.stringify(message));
}

function handleLspMessage(message) {
    if (message.id !== undefined) {
        // Response to a request
        if (message.method === 'initialize') {
            // Send initialized notification
            sendLspNotification('initialized', {});
        }
        console.log('LSP response:', message);
    } else if (message.method) {
        // Server notification
        console.log('LSP notification:', message.method, message.params);
    }
}

function updateLspStatus(status) {
    const indicator = document.getElementById('lsp-indicator');
    const text = document.getElementById('lsp-text');

    switch (status) {
        case 'connected':
            indicator.className = 'w-2 h-2 rounded-full bg-green-500';
            text.textContent = 'LSP Connected';
            break;
        case 'disconnected':
            indicator.className = 'w-2 h-2 rounded-full bg-gray-400';
            text.textContent = 'Disconnected';
            break;
        case 'error':
            indicator.className = 'w-2 h-2 rounded-full bg-red-500';
            text.textContent = 'LSP Error';
            break;
    }
}

// Connect LSP on page load
document.addEventListener('DOMContentLoaded', function() {
    // ... existing editor init ...
    connectLsp();
});
```

**Step 3: Commit**

```bash
git add templates/edit.html
git commit -m "feat(editor): add LSP connection status indicator

Shows connection status in editor status bar:
- Green: Connected
- Gray: Disconnected
- Red: Error
Includes auto-reconnect on disconnect."
```

---

## Task 6: Add i18n keys for LSP status

**Files:**
- Modify: `locales/en/common.ftl`
- Modify: `locales/uk/common.ftl`
- Modify: `locales/de/common.ftl`
- Modify: `locales/es/common.ftl`
- Modify: `locales/fr/common.ftl`

**Step 1: Add keys to all locale files**

English (en):
```ftl
lsp-connected = LSP Connected
lsp-disconnected = Disconnected
lsp-error = LSP Error
```

Add appropriate translations for other locales.

**Step 2: Commit**

```bash
git add locales/
git commit -m "feat(i18n): add LSP status translation keys"
```

---

## Summary

Phase 3 creates the WebSocket bridge infrastructure:
- **Task 1**: Add WebSocket feature to Axum
- **Task 2**: Create LSP bridge module
- **Task 3**: Register endpoint in server
- **Task 4**: Manual testing
- **Task 5**: Add connection status UI
- **Task 6**: Add i18n keys

After Phase 3, the editor connects to the LSP server. Phase 4 will integrate completions and diagnostics.
