# Async and Security Fixes Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix blocking I/O in async handlers, address TOCTOU race conditions, add request size limits, and improve error handling.

**Architecture:** Replace blocking `std::fs` operations with `tokio::fs` in async handlers. Add body size limits via Axum layer. Improve LSP bridge task cancellation. Extract magic numbers to constants.

**Tech Stack:** Rust, Tokio, Axum, JavaScript

---

## Task 1: Fix Blocking I/O in `recipe_raw` (P0)

**Files:**
- Modify: `src/server/handlers/recipes.rs:156`

**Step 1: Replace `std::fs::read_to_string` with `tokio::fs::read_to_string`**

Change line 156 from:

```rust
    std::fs::read_to_string(&file_path).map_err(|e| {
        tracing::error!("Failed to read recipe file {}: {}", file_path, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
```

To:

```rust
    tokio::fs::read_to_string(&file_path).await.map_err(|e| {
        tracing::error!("Failed to read recipe file {}: {}", file_path, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
```

**Step 2: Verify the change compiles**

Run: `cargo build -p cookcli 2>&1 | head -30`
Expected: No errors related to `recipe_raw`

**Step 3: Commit**

```bash
git add src/server/handlers/recipes.rs
git commit -m "$(cat <<'EOF'
fix: use tokio::fs for async file reading in recipe_raw

Replace blocking std::fs::read_to_string with tokio::fs version
to prevent thread pool starvation under load.
EOF
)"
```

---

## Task 2: Fix Blocking I/O in `recipe_save` (P0)

**Files:**
- Modify: `src/server/handlers/recipes.rs:162-214`

**Step 1: Update imports at top of file**

Add `use tokio::io::AsyncWriteExt;` if not present.

**Step 2: Replace blocking file operations with async versions**

Replace lines 188-206 (the atomic write section) with:

```rust
    // Atomic write: write to temp file, then rename
    let temp_path = file_path.with_extension("tmp");

    let mut temp_file = tokio::fs::File::create(&temp_path).await.map_err(|e| {
        tracing::error!("Failed to create temp file {}: {}", temp_path, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    temp_file.write_all(body.as_bytes()).await.map_err(|e| {
        tracing::error!("Failed to write to temp file {}: {}", temp_path, e);
        let _ = tokio::fs::remove_file(&temp_path);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tokio::fs::rename(&temp_path, &file_path).await.map_err(|e| {
        tracing::error!("Failed to rename temp file to {}: {}", file_path, e);
        let _ = tokio::fs::remove_file(&temp_path);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
```

Note: The `let _ = tokio::fs::remove_file(...)` without `.await` is intentional - we fire-and-forget cleanup on error since we're already returning an error.

**Step 3: Fix the cleanup to properly await**

Actually, we should properly clean up. Update error handlers to:

```rust
    temp_file.write_all(body.as_bytes()).await.map_err(|e| {
        tracing::error!("Failed to write to temp file {}: {}", temp_path, e);
        // Fire-and-forget cleanup - spawn so we don't block the error path
        let temp_path_clone = temp_path.clone();
        tokio::spawn(async move { let _ = tokio::fs::remove_file(&temp_path_clone).await; });
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tokio::fs::rename(&temp_path, &file_path).await.map_err(|e| {
        tracing::error!("Failed to rename temp file to {}: {}", file_path, e);
        let temp_path_clone = temp_path.clone();
        tokio::spawn(async move { let _ = tokio::fs::remove_file(&temp_path_clone).await; });
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
```

**Step 4: Verify the change compiles**

Run: `cargo build -p cookcli 2>&1 | head -30`
Expected: No errors related to `recipe_save`

**Step 5: Commit**

```bash
git add src/server/handlers/recipes.rs
git commit -m "$(cat <<'EOF'
fix: use tokio::fs for async file operations in recipe_save

Replace blocking std::fs operations with tokio::fs versions
to prevent thread pool starvation under load.
EOF
)"
```

---

## Task 3: Fix Blocking I/O in `recipe_delete` (P0)

**Files:**
- Modify: `src/server/handlers/recipes.rs:277`

**Step 1: Replace `std::fs::remove_file` with `tokio::fs::remove_file`**

Change line 277 from:

```rust
    std::fs::remove_file(&file_path).map_err(|e| {
        tracing::error!("Failed to delete recipe file {}: {}", file_path, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
```

To:

```rust
    tokio::fs::remove_file(&file_path).await.map_err(|e| {
        tracing::error!("Failed to delete recipe file {}: {}", file_path, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
```

**Step 2: Verify the change compiles**

Run: `cargo build -p cookcli 2>&1 | head -30`
Expected: No errors related to `recipe_delete`

**Step 3: Commit**

```bash
git add src/server/handlers/recipes.rs
git commit -m "$(cat <<'EOF'
fix: use tokio::fs for async file deletion in recipe_delete

Replace blocking std::fs::remove_file with tokio::fs version
to prevent thread pool starvation under load.
EOF
)"
```

---

## Task 4: Fix Blocking I/O in `edit_page` (P1)

**Files:**
- Modify: `src/server/ui.rs:678`

**Step 1: Replace `std::fs::read_to_string` with `tokio::fs::read_to_string`**

Change line 678 from:

```rust
    let content = std::fs::read_to_string(file_path).map_err(|e| {
        tracing::error!("Failed to read recipe file: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
```

To:

```rust
    let content = tokio::fs::read_to_string(file_path).await.map_err(|e| {
        tracing::error!("Failed to read recipe file: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
```

**Step 2: Verify the change compiles**

Run: `cargo build -p cookcli 2>&1 | head -30`
Expected: No errors related to `edit_page`

**Step 3: Commit**

```bash
git add src/server/ui.rs
git commit -m "$(cat <<'EOF'
fix: use tokio::fs for async file reading in edit_page

Replace blocking std::fs::read_to_string with tokio::fs version.
EOF
)"
```

---

## Task 5: Fix TOCTOU Race Condition in `create_recipe` (P0)

**Files:**
- Modify: `src/server/ui.rs:816-849`

**Step 1: Reorder path validation to check BEFORE creating directories**

The current code creates directories first, then validates. We need to validate the path structure first.

Replace lines 814-849 with:

```rust
    let file_path = state.base_path.join(format!("{}.cook", recipe_path));

    // Security: Validate path structure before any filesystem operations
    // Check that the constructed path, when normalized, stays within base_path
    let base_canonical = match state.base_path.canonicalize_utf8() {
        Ok(p) => p,
        Err(_) => {
            return new_page_error("Internal error: invalid base path", &original_filename);
        }
    };

    // Validate parent path components don't escape base_path
    // We do this by checking the joined path doesn't contain .. after normalization
    let normalized_path = file_path.as_str().replace("\\", "/");
    if normalized_path.contains("/../") || normalized_path.ends_with("/..") {
        tracing::warn!("Path traversal attempt detected in: {}", recipe_path);
        return new_page_error("Invalid recipe path", &original_filename);
    }

    // For the file path, we check the parent directory
    if let Some(parent) = file_path.parent() {
        // Create parent directories if they don't exist
        if !parent.exists() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                tracing::error!("Failed to create directories: {}", e);
                return new_page_error("Failed to create directory", &original_filename);
            }
        }

        // Now verify the created parent is under base_path
        match parent.canonicalize_utf8() {
            Ok(parent_canonical) => {
                if !parent_canonical.starts_with(&base_canonical) {
                    tracing::warn!(
                        "Path traversal attempt: {} not under {}",
                        parent_canonical,
                        base_canonical
                    );
                    // Clean up the created directory if it's outside base_path
                    let _ = tokio::fs::remove_dir_all(parent).await;
                    return new_page_error("Invalid recipe path", &original_filename);
                }
            }
            Err(_) => {
                return new_page_error("Invalid recipe path", &original_filename);
            }
        }
    }
```

**Step 2: Verify the change compiles**

Run: `cargo build -p cookcli 2>&1 | head -30`
Expected: No errors related to `create_recipe`

**Step 3: Commit**

```bash
git add src/server/ui.rs
git commit -m "$(cat <<'EOF'
fix: address TOCTOU race condition in create_recipe

Validate path structure before filesystem operations and
add cleanup if directory creation results in path outside base_path.
EOF
)"
```

---

## Task 6: Fix Blocking Canonicalize in `create_recipe` (P1)

**Files:**
- Modify: `src/server/ui.rs:816, 834`

**Step 1: Use `spawn_blocking` for synchronous canonicalize calls**

The `canonicalize_utf8()` method is synchronous. Wrap it in `spawn_blocking`:

Replace the base_canonical assignment:

```rust
    let base_path_clone = state.base_path.clone();
    let base_canonical = match tokio::task::spawn_blocking(move || base_path_clone.canonicalize_utf8()).await {
        Ok(Ok(p)) => p,
        _ => {
            return new_page_error("Internal error: invalid base path", &original_filename);
        }
    };
```

And wrap the parent canonicalize similarly:

```rust
        match tokio::task::spawn_blocking({
            let parent = parent.to_owned();
            move || parent.canonicalize_utf8()
        }).await {
            Ok(Ok(parent_canonical)) => {
                if !parent_canonical.starts_with(&base_canonical) {
                    // ... existing code
                }
            }
            _ => {
                return new_page_error("Invalid recipe path", &original_filename);
            }
        }
```

**Step 2: Verify the change compiles**

Run: `cargo build -p cookcli 2>&1 | head -30`
Expected: No errors

**Step 3: Commit**

```bash
git add src/server/ui.rs
git commit -m "$(cat <<'EOF'
fix: use spawn_blocking for synchronous canonicalize calls

Wrap blocking canonicalize_utf8() calls in tokio::task::spawn_blocking
to avoid stalling the async runtime.
EOF
)"
```

---

## Task 7: Add Request Body Size Limit (P1)

**Files:**
- Modify: `src/server/mod.rs:128-141`

**Step 1: Add DefaultBodyLimit import**

Add to imports at top of file:

```rust
use axum::extract::DefaultBodyLimit;
```

**Step 2: Add body size limit layer to the router**

Change lines 128-141 from:

```rust
    let app = Router::new()
        .nest("/api", api(&state)?)
        .merge(ui::ui())
        .route("/static/*file", get(serve_static))
        .nest_service("/api/static", ServeDir::new(&state.base_path));

    let app = app
        .with_state(state)
        .layer(axum::middleware::from_fn(language::language_middleware))
        .layer(
            CorsLayer::new()
                .allow_origin("*".parse::<HeaderValue>().unwrap())
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE]),
        );
```

To:

```rust
    // Maximum request body size: 1MB (reasonable for recipe files)
    const MAX_BODY_SIZE: usize = 1024 * 1024;

    let app = Router::new()
        .nest("/api", api(&state)?)
        .merge(ui::ui())
        .route("/static/*file", get(serve_static))
        .nest_service("/api/static", ServeDir::new(&state.base_path));

    let app = app
        .with_state(state)
        .layer(DefaultBodyLimit::max(MAX_BODY_SIZE))
        .layer(axum::middleware::from_fn(language::language_middleware))
        .layer(
            CorsLayer::new()
                .allow_origin("*".parse::<HeaderValue>().unwrap())
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE]),
        );
```

**Step 3: Verify the change compiles**

Run: `cargo build -p cookcli 2>&1 | head -30`
Expected: No errors

**Step 4: Commit**

```bash
git add src/server/mod.rs
git commit -m "$(cat <<'EOF'
fix: add 1MB request body size limit

Add DefaultBodyLimit layer to prevent potential DoS via
huge recipe uploads.
EOF
)"
```

---

## Task 8: Fix LSP Bridge Task Cleanup (P1)

**Files:**
- Modify: `src/server/lsp_bridge.rs:82, 189-199`

**Step 1: Document the channel buffer size**

Add a constant at the top of the file (after the imports):

```rust
/// Buffer size for LSP message channel.
/// 32 messages provides adequate buffering for typical LSP traffic
/// while preventing unbounded memory growth.
const LSP_MESSAGE_BUFFER_SIZE: usize = 32;
```

**Step 2: Use the constant**

Change line 82 from:

```rust
    let (tx, mut rx) = mpsc::channel::<String>(32);
```

To:

```rust
    let (tx, mut rx) = mpsc::channel::<String>(LSP_MESSAGE_BUFFER_SIZE);
```

**Step 3: Improve task cleanup with abort handles**

Replace lines 84-199 with proper task cancellation:

```rust
    // Task: Read from LSP stdout and send to channel
    let stdout_handle = tokio::spawn(async move {
        loop {
            // ... existing stdout_task code unchanged ...
        }
    });

    // Task: Read from WebSocket and write to LSP stdin
    let stdin_handle = tokio::spawn(async move {
        // ... existing stdin_task code unchanged ...
    });

    // Task: Send messages from channel to WebSocket
    let ws_send_handle = tokio::spawn(async move {
        // ... existing ws_send_task code unchanged ...
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

    // Abort remaining tasks
    stdout_handle.abort();
    stdin_handle.abort();
    ws_send_handle.abort();
```

Note: The existing code already kills the LSP process, which will cause stdout_task to exit. The abort calls ensure clean shutdown of any lingering tasks.

**Step 4: Verify the change compiles**

Run: `cargo build -p cookcli 2>&1 | head -30`
Expected: No errors

**Step 5: Commit**

```bash
git add src/server/lsp_bridge.rs
git commit -m "$(cat <<'EOF'
fix: improve LSP bridge task cleanup and document buffer size

- Add constant for LSP message buffer size with documentation
- Abort remaining tasks when any task completes to prevent leaks
EOF
)"
```

---

## Task 9: Add Input Validation Before Sanitization (P2)

**Files:**
- Modify: `src/server/ui.rs:791-810`

**Step 1: Validate filename before sanitization**

Add validation before the sanitization step. Change lines 791-810:

From:

```rust
    let original_filename = form.filename.clone();

    // Sanitize path - allow alphanumeric, space, dash, underscore, and forward slash
    let recipe_path: String = form
        .filename
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_' || *c == '/')
        .collect();
```

To:

```rust
    let original_filename = form.filename.clone();

    // Validate input before sanitization
    if form.filename.trim().is_empty() {
        return new_page_error("Recipe name cannot be empty", &original_filename);
    }

    // Sanitize path - allow alphanumeric, space, dash, underscore, and forward slash
    let recipe_path: String = form
        .filename
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_' || *c == '/')
        .collect();
```

**Step 2: Verify the change compiles**

Run: `cargo build -p cookcli 2>&1 | head -30`
Expected: No errors

**Step 3: Commit**

```bash
git add src/server/ui.rs
git commit -m "$(cat <<'EOF'
fix: validate filename before sanitization in create_recipe

Check for empty input before sanitization to provide clear error
messages for empty filenames.
EOF
)"
```

---

## Task 10: Extract Magic Numbers to Constants in JavaScript (P2)

**Files:**
- Modify: `templates/edit.html:88, 103, 274, 435`

**Step 1: Add constants at the top of the script block**

After line 77 (`let isSaving = false;`), add:

```javascript
// Timing constants
const AUTOSAVE_DELAY_MS = 1000;
const LSP_INITIAL_CONNECT_DELAY_MS = 500;
const LSP_RECONNECT_DELAY_MS = 3000;
const COMPLETION_TIMEOUT_MS = 2000;
```

**Step 2: Replace magic numbers with constants**

Change line 88 from:
```javascript
            autosaveTimer = setTimeout(() => saveRecipe(), 1000);
```
To:
```javascript
            autosaveTimer = setTimeout(() => saveRecipe(), AUTOSAVE_DELAY_MS);
```

The `LSP_DEBOUNCE_MS` constant already exists at line 239.

Change line 274 from:
```javascript
        setTimeout(connectLsp, 3000);
```
To:
```javascript
        setTimeout(connectLsp, LSP_RECONNECT_DELAY_MS);
```

Change line 417 from:
```javascript
setTimeout(connectLsp, 500);
```
To:
```javascript
setTimeout(connectLsp, LSP_INITIAL_CONNECT_DELAY_MS);
```

Change line 435 from:
```javascript
            setTimeout(() => {
```
To (the timeout value):
```javascript
            setTimeout(() => {
                if (pendingCompletions.has(id)) {
                    pendingCompletions.delete(id);
                    resolve([]);
                }
            }, COMPLETION_TIMEOUT_MS);
```

**Step 3: Verify the template renders**

Run: `cargo build -p cookcli 2>&1 | head -30`
Expected: No errors (template is just HTML/JS)

**Step 4: Commit**

```bash
git add templates/edit.html
git commit -m "$(cat <<'EOF'
refactor: extract magic numbers to named constants in editor

Replace hardcoded timing values with named constants for
better maintainability: AUTOSAVE_DELAY_MS, LSP_*_DELAY_MS,
COMPLETION_TIMEOUT_MS.
EOF
)"
```

---

## Task 11: Improve Error Display in JavaScript (P2)

**Files:**
- Modify: `templates/edit.html:161, 210`

**Step 1: Add a toast notification function**

After the `updateSaveStatus` function (around line 136), add:

```javascript
function showToast(message, type = 'error') {
    // Create toast element
    const toast = document.createElement('div');
    toast.className = `fixed bottom-4 right-4 px-4 py-2 rounded-lg shadow-lg z-50 ${
        type === 'error' ? 'bg-red-500 text-white' : 'bg-green-500 text-white'
    }`;
    toast.textContent = message;
    document.body.appendChild(toast);

    // Remove after 5 seconds
    setTimeout(() => {
        toast.remove();
    }, 5000);
}
```

**Step 2: Update saveRecipe error handling**

Change lines 159-165 from:
```javascript
        } else {
            updateSaveStatus('error');
            console.error('Save failed:', await response.text());
        }
    } catch (error) {
        updateSaveStatus('error');
        console.error('Save failed:', error.message);
```

To:
```javascript
        } else {
            updateSaveStatus('error');
            const errorText = await response.text();
            console.error('Save failed:', errorText);
            showToast(`Save failed: ${errorText || 'Unknown error'}`);
        }
    } catch (error) {
        updateSaveStatus('error');
        console.error('Save failed:', error.message);
        showToast(`Save failed: ${error.message}`);
```

**Step 3: Update deleteRecipe error handling**

Change lines 205-214 from:
```javascript
        } else {
            console.error('Delete failed:', await response.text());
            hideDeleteModal();
            alert('Failed to delete recipe');
        }
    } catch (error) {
        console.error('Delete failed:', error.message);
        hideDeleteModal();
        alert('Failed to delete recipe');
    }
```

To:
```javascript
        } else {
            const errorText = await response.text();
            console.error('Delete failed:', errorText);
            hideDeleteModal();
            showToast(`Failed to delete recipe: ${errorText || 'Unknown error'}`);
        }
    } catch (error) {
        console.error('Delete failed:', error.message);
        hideDeleteModal();
        showToast(`Failed to delete recipe: ${error.message}`);
    }
```

**Step 4: Verify the template renders**

Run: `cargo build -p cookcli 2>&1 | head -30`
Expected: No errors

**Step 5: Commit**

```bash
git add templates/edit.html
git commit -m "$(cat <<'EOF'
fix: display errors to user via toast notifications

Replace console.error-only error handling with user-visible
toast notifications for save and delete failures.
EOF
)"
```

---

## Task 12: Fix Blocking I/O in `pantry_page` (P1)

**Files:**
- Modify: `src/server/ui.rs:1153`

**Step 1: Replace `std::fs::read_to_string` with `tokio::fs::read_to_string`**

Change line 1153 from:
```rust
        if let Ok(content) = std::fs::read_to_string(path) {
```

To:
```rust
        if let Ok(content) = tokio::fs::read_to_string(path).await {
```

**Step 2: Verify the change compiles**

Run: `cargo build -p cookcli 2>&1 | head -30`
Expected: No errors

**Step 3: Commit**

```bash
git add src/server/ui.rs
git commit -m "$(cat <<'EOF'
fix: use tokio::fs for async file reading in pantry_page

Replace blocking std::fs::read_to_string with tokio::fs version.
EOF
)"
```

---

## Task 13: Run Full Test Suite

**Step 1: Build release**

Run: `make release`
Expected: Build succeeds

**Step 2: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 3: Manual smoke test**

Run: `cargo run -- server ./seed`

1. Open http://localhost:9080
2. Navigate to a recipe
3. Click Edit
4. Make a change and verify autosave works
5. Try creating a new recipe

**Step 4: Commit any final fixes if needed**

---

## Summary of Changes

| Priority | Issue | File | Fix |
|----------|-------|------|-----|
| P0 | Blocking I/O in recipe_raw | recipes.rs:156 | tokio::fs::read_to_string |
| P0 | Blocking I/O in recipe_save | recipes.rs:191-206 | tokio::fs::File, write_all, rename |
| P0 | Blocking I/O in recipe_delete | recipes.rs:277 | tokio::fs::remove_file |
| P0 | TOCTOU race condition | ui.rs:826-849 | Validate before create, cleanup on escape |
| P1 | Blocking canonicalize | ui.rs:816,834 | spawn_blocking |
| P1 | Missing body size limit | mod.rs | DefaultBodyLimit layer |
| P1 | LSP task cleanup | lsp_bridge.rs | abort handles |
| P1 | Blocking I/O in edit_page | ui.rs:678 | tokio::fs::read_to_string |
| P1 | Blocking I/O in pantry_page | ui.rs:1153 | tokio::fs::read_to_string |
| P2 | Magic numbers | edit.html | Named constants |
| P2 | Input validation | ui.rs:791 | Validate before sanitize |
| P2 | Error swallowing | edit.html | Toast notifications |
