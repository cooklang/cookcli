//! Filesystem watcher that broadcasts `.shopping-list` / `.shopping-checked`
//! changes so open browsers can refresh without reload.
//!
//! Only events that can mean a file changed are announced; a file merely
//! being read is not (see `is_read`).
//!
//! Startup is best-effort: if `notify` fails to initialize (permission
//! issues, unsupported platform), the server logs a warning and continues
//! without live updates. The SSE endpoint still serves — it just never emits.

use camino::Utf8Path;
use serde::Serialize;
use std::path::Path;

/// Which of the two watched files changed. The shopping list page re-reads
/// only what this names: the ticks for `Checked`, the whole list for `List`.
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

use notify::event::{AccessKind, AccessMode};
use notify::EventKind;
use notify_debouncer_full::DebouncedEvent;

/// Whether an event only reports a file being opened or read, which says
/// nothing about its contents.
///
/// On Linux, notify subscribes to inotify's `IN_OPEN`, so every `open()` of a
/// watched file arrives as `Access(Open)` — including the reads the shopping
/// list page makes when it re-fetches. Passing those on made each re-fetch
/// announce another change, and an open page reloaded the list every second
/// or so for as long as it stayed open. `Access(Close(Write))` is the
/// exception: it marks the end of a write.
fn is_read(kind: &EventKind) -> bool {
    match kind {
        EventKind::Access(AccessKind::Close(AccessMode::Write)) => false,
        EventKind::Access(_) => true,
        _ => false,
    }
}

/// The watched files a debounced batch of events changed, each at most once,
/// `.shopping-list` before `.shopping-checked`.
///
/// Every path of an event is classified, not only the first: an atomic
/// rewrite can arrive as a rename whose paths are the staging file and then
/// the file it replaced. A rescan notice means the platform dropped events,
/// so either file may have changed.
fn changed_files(base_path: &Utf8Path, events: &[DebouncedEvent]) -> Vec<WatchedFile> {
    if events.iter().any(|event| event.need_rescan()) {
        return vec![WatchedFile::List, WatchedFile::Checked];
    }

    let mut list = false;
    let mut checked = false;
    for event in events.iter().filter(|event| !is_read(&event.kind)) {
        for path in &event.paths {
            match classify_path(base_path, path) {
                Some(WatchedFile::List) => list = true,
                Some(WatchedFile::Checked) => checked = true,
                None => {}
            }
        }
    }

    let mut files = Vec::new();
    if list {
        files.push(WatchedFile::List);
    }
    if checked {
        files.push(WatchedFile::Checked);
    }
    files
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

/// Debounce window. Collapses the create+modify burst produced by the atomic
/// rename `cookcli_core::shopping_list::ShoppingListStore` writes with.
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
                    for file in changed_files(&base_for_task, &events) {
                        // Send returns Err when there are no receivers — fine.
                        let _ = tx_for_task.send(ShoppingListChangeEvent { file });
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

    /// The store stages every rewrite in a `.<name>.<pid>.tmp` sibling before
    /// renaming it into place. Reacting to the staging file would fire an
    /// event for a file no client can read.
    #[test]
    fn ignores_temp_file_from_atomic_rename() {
        let p = PathBuf::from("/tmp/recipes/..shopping-checked.4321.tmp");
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

    use notify::event::{CreateKind, DataChange, Flag, ModifyKind, RemoveKind, RenameMode};
    use notify::Event;
    use std::time::Instant;

    fn event(kind: EventKind, name: &str) -> Event {
        Event::new(kind).add_path(PathBuf::from(format!("/tmp/recipes/{name}")))
    }

    fn changed(events: Vec<Event>) -> Vec<WatchedFile> {
        let batch: Vec<DebouncedEvent> = events
            .into_iter()
            .map(|event| DebouncedEvent::new(event, Instant::now()))
            .collect();
        changed_files(&base(), &batch)
    }

    /// Linux reports every `open()` of a watched file, and the shopping list
    /// page reads both files whenever it re-fetches — so counting a read as a
    /// change kept the page re-fetching in a loop.
    #[test]
    fn reading_a_file_is_not_a_change() {
        for kind in [
            EventKind::Access(AccessKind::Open(AccessMode::Any)),
            EventKind::Access(AccessKind::Read),
            EventKind::Access(AccessKind::Close(AccessMode::Read)),
        ] {
            assert_eq!(
                changed(vec![
                    event(kind, ".shopping-list"),
                    event(kind, ".shopping-checked"),
                ]),
                vec![],
                "{kind:?}"
            );
        }
    }

    #[test]
    fn closing_a_file_after_writing_it_is_a_change() {
        let kind = EventKind::Access(AccessKind::Close(AccessMode::Write));
        assert_eq!(
            changed(vec![event(kind, ".shopping-checked")]),
            vec![WatchedFile::Checked]
        );
    }

    #[test]
    fn writing_creating_and_removing_are_changes() {
        for kind in [
            EventKind::Modify(ModifyKind::Data(DataChange::Any)),
            // Windows reports a write without saying what changed.
            EventKind::Modify(ModifyKind::Any),
            EventKind::Create(CreateKind::File),
            EventKind::Remove(RemoveKind::File),
        ] {
            assert_eq!(
                changed(vec![event(kind, ".shopping-checked")]),
                vec![WatchedFile::Checked],
                "{kind:?}"
            );
        }
    }

    /// The store rewrites a file by renaming a staging copy over it, and the
    /// file that was replaced is the rename's second path.
    #[test]
    fn a_rename_counts_for_the_file_it_replaces() {
        let rename = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
            .add_path(PathBuf::from("/tmp/recipes/..shopping-list.4321.tmp"))
            .add_path(PathBuf::from("/tmp/recipes/.shopping-list"));
        assert_eq!(changed(vec![rename]), vec![WatchedFile::List]);
    }

    #[test]
    fn a_batch_names_each_changed_file_once_list_first() {
        let write = EventKind::Modify(ModifyKind::Data(DataChange::Any));
        let open = EventKind::Access(AccessKind::Open(AccessMode::Any));
        assert_eq!(
            changed(vec![
                event(write, ".shopping-checked"),
                event(open, ".shopping-list"),
                event(write, ".shopping-list"),
                event(write, ".shopping-list"),
                event(write, "Pancakes.cook"),
            ]),
            vec![WatchedFile::List, WatchedFile::Checked]
        );
    }

    #[test]
    fn a_batch_of_unrelated_changes_names_nothing() {
        let write = EventKind::Modify(ModifyKind::Data(DataChange::Any));
        assert_eq!(changed(vec![event(write, "Pancakes.cook")]), vec![]);
    }

    /// A rescan means the platform dropped events, so there is no telling
    /// which file they were about.
    #[test]
    fn a_rescan_counts_for_both_files() {
        let rescan = Event::new(EventKind::Any).set_flag(Flag::Rescan);
        assert_eq!(
            changed(vec![rescan]),
            vec![WatchedFile::List, WatchedFile::Checked]
        );
    }
}
