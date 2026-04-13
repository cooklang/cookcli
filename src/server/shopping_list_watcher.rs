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

use anyhow::{Context, Result};
use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use std::time::Duration;
use tokio::sync::broadcast;

/// Broadcast channel used to fan out change events to every open SSE stream.
pub type ChangeSender = broadcast::Sender<ShoppingListChangeEvent>;

/// Channel capacity: small buffer of most-recent events. Slow subscribers
/// get `Lagged` rather than stalling the watcher.
const CHANNEL_CAPACITY: usize = 16;

/// Bridge channel capacity (notify thread → tokio task). With 200 ms
/// debouncing this should never fill in normal use; bounding it caps
/// memory if a runaway producer touches files in a tight loop.
const BRIDGE_CAPACITY: usize = 32;

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
    // to an async task which fans them out to `tx`. Bounded so a runaway
    // producer can't grow memory; on overflow we drop the batch (the next
    // change will trigger a re-fetch anyway).
    let (evt_tx, mut evt_rx) = tokio::sync::mpsc::channel::<DebounceEventResult>(BRIDGE_CAPACITY);

    let mut debouncer = new_debouncer(DEBOUNCE, None, move |res: DebounceEventResult| {
        // If the async side is shut down or backlogged, drop the batch.
        if let Err(e) = evt_tx.try_send(res) {
            tracing::debug!("shopping list watcher: dropping batch ({e})");
        }
    })
    .context("initializing filesystem debouncer")?;

    debouncer
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
