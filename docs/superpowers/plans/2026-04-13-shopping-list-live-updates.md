# Shopping List Live Updates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Push live updates from `.shopping-list` and `.shopping-checked` file changes to the open `/shopping_list` page so sync pulls, other-tab writes, and direct file edits are reflected without a manual reload.

**Architecture:** A single `notify` + `notify-debouncer-full` filesystem watcher over `base_path` feeds a `tokio::sync::broadcast` channel on `AppState`. A new SSE endpoint (`GET /api/shopping_list/events`) subscribes each connected browser; the shopping list page opens an `EventSource` and re-fetches through the existing `loadShoppingList()` path on every message.

**Tech Stack:** Rust + axum (existing), `notify` 6, `notify-debouncer-full` 0.3, `tokio::sync::broadcast` (already in tokio), browser `EventSource`.

**Spec:** `docs/superpowers/specs/2026-04-13-shopping-list-live-updates-design.md`

## File Structure

- **Create:** `src/server/shopping_list_watcher.rs` — owns the watcher lifecycle, the debounce filter, and the broadcast sender. Tested in isolation via the pure `is_relevant_event` filter.
- **Modify:** `Cargo.toml` — add `notify` and `notify-debouncer-full`.
- **Modify:** `src/server/mod.rs` — declare the new module, hold the `broadcast::Sender` on `AppState`, spawn the watcher at startup, mount the SSE route.
- **Create:** `src/server/handlers/shopping_list_events.rs` — SSE handler. Kept separate from `handlers/shopping_list.rs` because it's long-lived streaming, not a request/response handler; different concerns, different lifetime.
- **Modify:** `src/server/handlers/mod.rs` — re-export the new handler.
- **Modify:** `templates/shopping_list.html` — open an `EventSource`, re-run `loadShoppingList()` on each message, close on `beforeunload`.
- **Create:** `tests/e2e/shopping-list-live.spec.ts` — Playwright test that mutates `.shopping-list` out-of-band and asserts the page updates.

---

### Task 1: Add filesystem-watcher dependencies

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add `notify` and `notify-debouncer-full` to dependencies**

In `Cargo.toml`, add two lines inside the `[dependencies]` table (keep alphabetical ordering near the `mime_guess = "2.0"` line — `notify` sorts just after):

```toml
notify = "6"
notify-debouncer-full = "0.3"
```

- [ ] **Step 2: Verify the crates resolve**

Run: `cargo check -p cookcli --no-default-features --features sync`
Expected: success, new crates downloaded. No compile errors in existing code.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add notify + notify-debouncer-full for file watching"
```

---

### Task 2: Introduce the watcher module with a unit-tested event filter

The watcher's filter — "does this debounced event touch `.shopping-list` or `.shopping-checked` in `base_path`?" — is pure and easy to test. Write it test-first.

**Files:**
- Create: `src/server/shopping_list_watcher.rs`
- Modify: `src/server/mod.rs` (declare the module)

- [ ] **Step 1: Declare the new module**

In `src/server/mod.rs`, next to the existing `mod shopping_list_store;` line (around line 56), add:

```rust
mod shopping_list_watcher;
```

- [ ] **Step 2: Write the failing unit test for `classify_path`**

Create `src/server/shopping_list_watcher.rs` with:

```rust
//! Filesystem watcher that broadcasts `.shopping-list` / `.shopping-checked`
//! changes so open browsers can refresh without reload.
//!
//! Startup is best-effort: if `notify` fails to initialize (permission
//! issues, unsupported platform), the server logs a warning and continues
//! without live updates. The SSE endpoint still serves — it just never emits.

use camino::Utf8Path;
use serde::Serialize;
use std::path::Path;

/// Which of the two watched files changed. The client uses this only as a
/// hint for logging; it re-fetches regardless.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WatchedFile {
    List,
    Checked,
}

/// Event broadcast to every subscribed SSE connection.
#[derive(Debug, Clone, Serialize)]
pub struct ShoppingListChangeEvent {
    pub file: WatchedFile,
}

/// Classify a filesystem path as one of the two watched files, or None if
/// it's something we don't care about (recipe files, temp files, backups,
/// directories, etc.).
///
/// `base_path` is the server's recipe directory; paths outside of it (or
/// nested deeper than immediate children) are ignored.
pub fn classify_path(base_path: &Utf8Path, path: &Path) -> Option<WatchedFile> {
    let parent = path.parent()?;
    // Compare as Utf8Path to avoid surprises with non-UTF8 OsStr on the LHS;
    // our base_path is already Utf8Path-typed.
    let parent = camino::Utf8Path::from_path(parent)?;
    if parent != base_path {
        return None;
    }
    let name = path.file_name()?.to_str()?;
    match name {
        ".shopping-list" => Some(WatchedFile::List),
        ".shopping-checked" => Some(WatchedFile::Checked),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;
    use std::path::PathBuf;

    fn base() -> Utf8PathBuf {
        Utf8PathBuf::from("/tmp/recipes")
    }

    #[test]
    fn classifies_shopping_list() {
        let p = PathBuf::from("/tmp/recipes/.shopping-list");
        assert_eq!(classify_path(&base(), &p), Some(WatchedFile::List));
    }

    #[test]
    fn classifies_shopping_checked() {
        let p = PathBuf::from("/tmp/recipes/.shopping-checked");
        assert_eq!(classify_path(&base(), &p), Some(WatchedFile::Checked));
    }

    #[test]
    fn ignores_temp_file_from_atomic_rename() {
        let p = PathBuf::from("/tmp/recipes/.shopping-checked.tmp");
        assert_eq!(classify_path(&base(), &p), None);
    }

    #[test]
    fn ignores_recipe_files() {
        let p = PathBuf::from("/tmp/recipes/Breakfast/Pancakes.cook");
        assert_eq!(classify_path(&base(), &p), None);
    }

    #[test]
    fn ignores_nested_shopping_list() {
        // A `.shopping-list` inside a subdirectory is not our file.
        let p = PathBuf::from("/tmp/recipes/subdir/.shopping-list");
        assert_eq!(classify_path(&base(), &p), None);
    }

    #[test]
    fn ignores_path_outside_base() {
        let p = PathBuf::from("/etc/.shopping-list");
        assert_eq!(classify_path(&base(), &p), None);
    }
}
```

- [ ] **Step 3: Run tests to verify they fail to compile, then pass**

Run: `cargo test -p cookcli --lib server::shopping_list_watcher`
Expected: the tests compile and all six pass.

If the module wasn't declared in step 1, the test binary won't find it — re-check `src/server/mod.rs`.

- [ ] **Step 4: Commit**

```bash
git add src/server/mod.rs src/server/shopping_list_watcher.rs
git commit -m "feat(server): add shopping list watcher module with path classifier"
```

---

### Task 3: Implement the watcher task with debounce + broadcast

Add the live pieces on top of the filter: a broadcast sender type alias, a `spawn(...)` function that initializes the debouncer and forwards classified events, and the log-and-continue behavior for init errors.

**Files:**
- Modify: `src/server/shopping_list_watcher.rs`

- [ ] **Step 1: Extend the module with the spawn function**

Append this below the existing contents (before `#[cfg(test)] mod tests`):

```rust
use anyhow::{Context, Result};
use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use std::time::Duration;
use tokio::sync::broadcast;

/// Broadcast channel used to fan out change events to every open SSE stream.
pub type ChangeSender = broadcast::Sender<ShoppingListChangeEvent>;

/// Channel capacity: small buffer of most-recent events. Slow subscribers
/// get `Lagged` rather than stalling the watcher.
const CHANNEL_CAPACITY: usize = 16;

/// Debounce window. Collapses the create+modify burst produced by
/// `shopping_list_store::compact()`'s atomic rename.
const DEBOUNCE: Duration = Duration::from_millis(200);

/// Construct a broadcast sender and spawn a background task that watches
/// `base_path` for `.shopping-list` / `.shopping-checked` changes.
///
/// Returns only the sender — the task is detached. The watcher lives as
/// long as the process. On init failure, returns `Err`; the caller should
/// log and continue without live updates.
pub fn spawn(base_path: camino::Utf8PathBuf) -> Result<ChangeSender> {
    let (tx, _rx) = broadcast::channel::<ShoppingListChangeEvent>(CHANNEL_CAPACITY);
    let tx_for_task = tx.clone();

    // Channel for decoupling the notify thread (sync) from tokio. The
    // debouncer callback runs on notify's own thread; we forward batches
    // to an async task which fans them out to `tx`.
    let (evt_tx, mut evt_rx) = tokio::sync::mpsc::unbounded_channel::<DebounceEventResult>();

    let mut debouncer = new_debouncer(DEBOUNCE, None, move |res: DebounceEventResult| {
        // If the async side has shut down, silently drop.
        let _ = evt_tx.send(res);
    })
    .context("initializing filesystem debouncer")?;

    debouncer
        .watcher()
        .watch(base_path.as_std_path(), RecursiveMode::NonRecursive)
        .with_context(|| format!("watching {base_path}"))?;

    let base_for_task = base_path.clone();
    tokio::spawn(async move {
        // Hold the debouncer for the lifetime of the task so the watcher
        // thread isn't dropped.
        let _debouncer = debouncer;

        while let Some(result) = evt_rx.recv().await {
            match result {
                Ok(events) => {
                    // Collapse the batch: at most one event per file.
                    let mut fired_list = false;
                    let mut fired_checked = false;
                    for event in events {
                        for path in &event.paths {
                            match classify_path(&base_for_task, path) {
                                Some(WatchedFile::List) if !fired_list => {
                                    fired_list = true;
                                }
                                Some(WatchedFile::Checked) if !fired_checked => {
                                    fired_checked = true;
                                }
                                _ => {}
                            }
                        }
                    }
                    if fired_list {
                        // Send returns Err when there are no receivers — fine.
                        let _ = tx_for_task.send(ShoppingListChangeEvent {
                            file: WatchedFile::List,
                        });
                    }
                    if fired_checked {
                        let _ = tx_for_task.send(ShoppingListChangeEvent {
                            file: WatchedFile::Checked,
                        });
                    }
                }
                Err(errors) => {
                    for err in errors {
                        tracing::warn!("shopping list watcher error: {err}");
                    }
                }
            }
        }
    });

    Ok(tx)
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p cookcli`
Expected: clean compile. If the `notify_debouncer_full` API types differ on your installed version, cross-reference the crate's docs — `DebounceEventResult = Result<Vec<DebouncedEvent>, Vec<notify::Error>>` and `DebouncedEvent` has a `.paths: Vec<PathBuf>` field.

- [ ] **Step 3: Verify existing unit tests still pass**

Run: `cargo test -p cookcli --lib server::shopping_list_watcher`
Expected: six tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/server/shopping_list_watcher.rs
git commit -m "feat(server): spawn debounced shopping-list file watcher"
```

---

### Task 4: Hold the broadcast sender on `AppState` and start the watcher

**Files:**
- Modify: `src/server/mod.rs`

- [ ] **Step 1: Add the field to `AppState`**

In `src/server/mod.rs`, find the `AppState` struct (around line 344). Add this field above the `#[cfg(feature = "sync")]`-gated fields so it's always present:

```rust
    /// Broadcasts filesystem changes to `.shopping-list` / `.shopping-checked`
    /// to every open SSE subscriber. `None` means watcher init failed; SSE
    /// clients can still connect but will never receive events.
    pub shopping_list_events: Option<shopping_list_watcher::ChangeSender>,
```

- [ ] **Step 2: Initialize the watcher in `build_state`**

In `build_state` (around line 246), after `tracing::info!("Pantry configuration: …");` and before the `#[cfg(feature = "sync")] let (session_path, session) = { … };` block, add:

```rust
    let shopping_list_events = match shopping_list_watcher::spawn(absolute_path.clone()) {
        Ok(tx) => Some(tx),
        Err(e) => {
            tracing::warn!(
                "Failed to start shopping list watcher; live updates disabled: {e:#}"
            );
            None
        }
    };
```

- [ ] **Step 3: Populate the field in the `AppState` literal**

In the same function, inside the `Ok(Arc::new(AppState { … }))` literal (around line 299), add `shopping_list_events,` alongside the other fields. Place it just before the `#[cfg(feature = "sync")]` fields to match the struct order.

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p cookcli`
Expected: clean compile with no warnings about unused fields.

- [ ] **Step 5: Smoke-test the server starts and logs**

Run: `cargo run -p cookcli -- server seed/`
Expected: startup logs include "Using absolute base path…" as today, and no warning about watcher init failure. `Ctrl+C` to stop.

- [ ] **Step 6: Commit**

```bash
git add src/server/mod.rs
git commit -m "feat(server): wire shopping list watcher into AppState"
```

---

### Task 5: Add the SSE handler

**Files:**
- Create: `src/server/handlers/shopping_list_events.rs`
- Modify: `src/server/handlers/mod.rs`

- [ ] **Step 1: Create the handler**

Create `src/server/handlers/shopping_list_events.rs`:

```rust
//! SSE endpoint that streams `.shopping-list` / `.shopping-checked` change
//! pings to the browser. Each connected client re-fetches the shopping
//! list data via the existing JSON endpoints on every event.

use crate::server::shopping_list_watcher::ShoppingListChangeEvent;
use crate::server::AppState;
use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use futures_util::stream::{self, Stream};
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

pub async fn shopping_list_events(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream: Box<dyn Stream<Item = Result<Event, Infallible>> + Send + Unpin> =
        match &state.shopping_list_events {
            Some(tx) => {
                let rx = tx.subscribe();
                // Lagged receivers become empty events (client will re-fetch
                // anyway). Channel close would end the stream, but the
                // sender is held by AppState for the life of the process.
                let s = BroadcastStream::new(rx).filter_map(
                    |res: Result<ShoppingListChangeEvent, _>| match res {
                        Ok(evt) => Some(Ok(Event::default()
                            .event("change")
                            .json_data(evt)
                            .unwrap_or_else(|_| Event::default().event("change")))),
                        Err(_lagged) => {
                            tracing::debug!("SSE subscriber lagged — client will catch up on next fetch");
                            None
                        }
                    },
                );
                Box::new(s)
            }
            None => {
                // Watcher failed to init; serve a well-formed but silent stream.
                Box::new(stream::pending::<Result<Event, Infallible>>())
            }
        };

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("ping"),
    )
}
```

- [ ] **Step 2: Add the required deps**

Two new dependencies are needed. In `Cargo.toml`, add:

```toml
tokio-stream = { version = "0.1", features = ["sync"] }
```

`futures-util` is already in deps. Run:

Run: `cargo check -p cookcli`
Expected: clean compile. If the `BroadcastStream` import fails, confirm the `sync` feature on `tokio-stream`.

- [ ] **Step 3: Re-export the handler**

In `src/server/handlers/mod.rs`:

1. Add `pub mod shopping_list_events;` alphabetically among the `pub mod …;` declarations (between `shopping_list` and `stats`).
2. Add `pub use shopping_list_events::shopping_list_events;` alphabetically among the `pub use …;` lines (just after the `shopping_list::{…}` re-export).

The final top of the file should read (showing only the new lines in context):

```rust
pub mod shopping_list;
pub mod shopping_list_events;
pub mod stats;
...
pub use shopping_list::{
    add_menu_to_shopping_list, add_to_shopping_list, check_shopping_item, clear_shopping_list,
    compact_checked, get_checked_items, get_shopping_list_items, remove_from_shopping_list,
    shopping_list, uncheck_shopping_item,
};
pub use shopping_list_events::shopping_list_events;
pub use stats::stats;
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p cookcli`
Expected: clean compile.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/server/handlers/shopping_list_events.rs src/server/handlers/mod.rs
git commit -m "feat(server): add SSE endpoint for shopping list change events"
```

---

### Task 6: Mount the SSE route

**Files:**
- Modify: `src/server/mod.rs`

- [ ] **Step 1: Add the route**

In `src/server/mod.rs`, inside the `fn api(...) -> Result<Router<Arc<AppState>>>` function (around line 394), add the new route next to the other `shopping_list/*` routes:

```rust
        .route(
            "/shopping_list/events",
            get(handlers::shopping_list_events),
        )
```

- [ ] **Step 2: Verify compilation and smoke-test**

Run: `cargo run -p cookcli -- server seed/` then, from another terminal:

```bash
curl -N http://127.0.0.1:9080/api/shopping_list/events
```

Expected: connection stays open; every 30s a `:ping` keepalive arrives. Now in a third terminal:

```bash
touch seed/.shopping-list
```

Expected: the `curl` terminal prints one `event: change\ndata: {"file":"list"}\n\n` block within ~250 ms. Touching again rapidly produces ~one event per debounce window. `Ctrl+C` to stop everything.

- [ ] **Step 3: Commit**

```bash
git add src/server/mod.rs
git commit -m "feat(server): mount /api/shopping_list/events SSE route"
```

---

### Task 7: Subscribe from the shopping list page

**Files:**
- Modify: `templates/shopping_list.html`

- [ ] **Step 1: Open the EventSource after initial load**

In `templates/shopping_list.html`, the script currently ends (around line 553) with a bare call to `loadShoppingList();`. Replace that tail with the following, which keeps initial load unchanged and adds the subscription after it:

```javascript
loadShoppingList();

// Subscribe to server-pushed change events so sync pulls, other-tab edits,
// or direct `.shopping-list` edits refresh the UI without reload.
// EventSource auto-reconnects on disconnect; we just close it on unload.
let shoppingListEvents = null;
function subscribeToShoppingListEvents() {
    try {
        shoppingListEvents = new EventSource('{{ prefix }}/api/shopping_list/events');
        shoppingListEvents.addEventListener('change', () => {
            // Re-run the same path as initial page load. `loadShoppingList`
            // triggers `generateList` when the list is non-empty, so both
            // the sidebar and the categorized view refresh.
            loadShoppingList();
        });
        shoppingListEvents.addEventListener('error', () => {
            // Browser will auto-reconnect. Nothing to do.
        });
    } catch (e) {
        console.error('Failed to subscribe to shopping list events:', e);
    }
}
subscribeToShoppingListEvents();

window.addEventListener('beforeunload', () => {
    if (shoppingListEvents) {
        shoppingListEvents.close();
        shoppingListEvents = null;
    }
});
```

- [ ] **Step 2: Manual end-to-end sanity check**

In one terminal: `cargo run -p cookcli -- server seed/`
Open http://127.0.0.1:9080/shopping_list in a browser.
In another terminal, write a valid list entry out-of-band and expect the sidebar's "Selected recipes" to update without reload:

```bash
printf 'Breakfast/Easy Pancakes\n' > seed/.shopping-list
```

Expected: within ~300 ms, the page shows `Easy Pancakes (×1)` in the selected-recipes sidebar, and the categorized list appears below it.

Now clear and expect an empty state:

```bash
: > seed/.shopping-list
```

Expected: the sidebar reverts to "No recipes" and the main area shows the empty message.

Finally, open a second browser tab on the same URL; add a recipe via the recipe page from tab 1; confirm tab 2 updates without reload.

- [ ] **Step 3: Commit**

```bash
git add templates/shopping_list.html
git commit -m "feat(ui): subscribe shopping list page to live change events"
```

---

### Task 8: Playwright e2e test

**Files:**
- Create: `tests/e2e/shopping-list-live.spec.ts`

- [ ] **Step 1: Write the test**

Create `tests/e2e/shopping-list-live.spec.ts`:

```typescript
import { test, expect } from '@playwright/test';
import { TestHelpers } from '../fixtures/test-helpers';
import * as fs from 'node:fs';
import * as path from 'node:path';

// Seed directory used by the dev server started by Playwright's `webServer`.
// Kept in sync with `playwright.config.ts`'s `cwd`/command.
const SEED_DIR = path.resolve(__dirname, '../../seed');
const LIST_FILE = path.join(SEED_DIR, '.shopping-list');

test.describe('Shopping list live updates', () => {
  let helpers: TestHelpers;
  let originalContent: string | null;

  test.beforeEach(async ({ page }) => {
    helpers = new TestHelpers(page);
    originalContent = fs.existsSync(LIST_FILE)
      ? fs.readFileSync(LIST_FILE, 'utf8')
      : null;
    // Start from an empty list so assertions are deterministic.
    fs.writeFileSync(LIST_FILE, '');
    await helpers.navigateTo('/shopping_list');
  });

  test.afterEach(async () => {
    if (originalContent === null) {
      if (fs.existsSync(LIST_FILE)) fs.unlinkSync(LIST_FILE);
    } else {
      fs.writeFileSync(LIST_FILE, originalContent);
    }
  });

  test('updates the sidebar when .shopping-list changes on disk', async ({ page }) => {
    // Baseline: empty state visible.
    await expect(page.getByText(/no recipes/i)).toBeVisible();

    // Out-of-band write: add a seed recipe.
    fs.writeFileSync(LIST_FILE, 'Breakfast/Easy Pancakes\n');

    // The selected-recipes sidebar should pick it up via SSE + re-fetch.
    await expect(
      page.locator('#selected-recipes').getByText(/Easy Pancakes/i),
    ).toBeVisible({ timeout: 5_000 });

    // Remove it out-of-band → back to empty.
    fs.writeFileSync(LIST_FILE, '');
    await expect(page.getByText(/no recipes/i)).toBeVisible({ timeout: 5_000 });
  });
});
```

- [ ] **Step 2: Run the test**

Run: `npx playwright test shopping-list-live.spec.ts --project=chromium`
Expected: the test passes. If it fails because the seed directory contains a path other than `Breakfast/Easy Pancakes.cook`, adjust the recipe path in the test to any existing file under `seed/`.

- [ ] **Step 3: Commit**

```bash
git add tests/e2e/shopping-list-live.spec.ts
git commit -m "test(e2e): verify shopping list updates live on file change"
```

---

### Task 9: Final verification

- [ ] **Step 1: Lint, format, and test the Rust side**

Run these in sequence:

```bash
cargo fmt
cargo clippy -p cookcli -- -D warnings
cargo test -p cookcli
```

Expected: formatting unchanged (or only whitespace), zero clippy warnings, all tests pass.

- [ ] **Step 2: Run the full e2e suite to confirm no regressions**

Run: `npx playwright test --project=chromium`
Expected: all specs pass. If any pre-existing test fails that was already flaky before this branch, note it but do not fix in this PR.

- [ ] **Step 3: Final commit if `cargo fmt` made any changes**

```bash
git add -A
git diff --cached --quiet || git commit -m "style: cargo fmt"
```
