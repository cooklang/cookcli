//! Reloads the users file when it changes, so `cook server user add` and
//! friends take effect on a running server without a restart.
//!
//! The file's directory is watched rather than the file itself: the `user`
//! commands, and most editors, replace the file by renaming a new one over
//! it, which a watch on the old file would not survive.

use super::users::Users;
use super::Auth;
use anyhow::{Context as _, Result};
use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult, DebouncedEvent};
use std::sync::Arc;
use std::time::Duration;

/// Collapses the create + rename burst of one atomic rewrite.
const DEBOUNCE: Duration = Duration::from_millis(200);

/// Starts watching `auth`'s users file for as long as the process runs.
pub fn spawn(auth: Arc<Auth>) -> Result<()> {
    let dir = auth
        .users_path()
        .parent()
        .context("the users file has no parent directory")?
        .to_path_buf();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<DebounceEventResult>(8);

    let mut debouncer = new_debouncer(DEBOUNCE, None, move |result| {
        // A full channel means a reload is already queued; that one will read
        // the latest contents.
        let _ = tx.try_send(result);
    })
    .context("initializing filesystem debouncer")?;
    debouncer
        .watch(dir.as_std_path(), RecursiveMode::NonRecursive)
        .with_context(|| format!("watching {dir}"))?;

    tokio::spawn(async move {
        // Owned by the task so the watcher lives as long as it does.
        let _debouncer = debouncer;
        while let Some(result) = rx.recv().await {
            match result {
                Ok(events) if touches_users_file(&auth, &events) => reload(&auth),
                Ok(_) => {}
                Err(errors) => {
                    for err in errors {
                        tracing::warn!("users file watcher error: {err}");
                    }
                }
            }
        }
    });
    Ok(())
}

/// Whether a batch of events may have changed the users file. Reads are
/// ignored: on Linux every read arrives as an event, and reloading on it
/// would reload again on the reload's own read.
fn touches_users_file(auth: &Auth, events: &[DebouncedEvent]) -> bool {
    let name = auth.users_path().file_name();
    events.iter().any(|event| event.need_rescan())
        || events
            .iter()
            .filter(|event| !crate::server::shopping_list_watcher::is_read(&event.kind))
            .flat_map(|event| &event.paths)
            .any(|path| path.file_name().and_then(|n| n.to_str()) == name)
}

/// Re-reads the users file. A broken or missing file keeps the current list:
/// a half-finished edit must not lock everyone out, or let everyone in.
fn reload(auth: &Auth) {
    let path = auth.users_path();
    if !path.exists() {
        tracing::warn!(
            "the users file {path} was removed; keeping the current users until the server \
             restarts"
        );
        return;
    }
    match Users::load(path) {
        Ok(users) if users == *auth.users() => {}
        Ok(users) => {
            tracing::info!("reloaded {path}: {} user(s)", users.len());
            auth.replace_users(users);
        }
        Err(err) => tracing::error!("{err:#}; keeping the previous users"),
    }
}
