//! Integration tests for `GET /api/shopping_list/events`: the server announces
//! changes to `.shopping-list` and `.shopping-checked`, and only changes.
//!
//! The watcher behind the stream lives in the private
//! `server::shopping_list_watcher` module, so these boot `cook server` against
//! a temporary recipe directory on its own port and read the real stream.
//!
//! The first test pins a regression that only Linux could show: inotify
//! reports every `open()`, and the watcher used to announce the shopping list
//! page's own reads as changes, so an open page re-fetched the list in a loop.
//! On other platforms reads raise no filesystem events, and it passes either
//! way.

#![cfg(feature = "server")]

use serde_json::{json, Value};
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

/// Kills the spawned server when the test ends, pass or panic.
struct ServerGuard {
    child: Child,
    port: u16,
    // Held so the recipe directory outlives the server.
    _dir: TempDir,
}

impl ServerGuard {
    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", self.port, path)
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

/// One recipe, already on the shopping list, with one of its ingredients
/// ticked off — so every read the shopping list page makes has a file to read.
fn write_fixture(dir: &TempDir) {
    let root = dir.path();
    std::fs::write(
        root.join("Soup.cook"),
        "Simmer @water{1%l} with @salt{1%tsp}.\n",
    )
    .unwrap();
    std::fs::write(root.join(".shopping-list"), "./Soup\n").unwrap();
    std::fs::write(root.join(".shopping-checked"), "+ salt\n").unwrap();
}

/// `free_port` only reserves a port long enough to learn its number, so with
/// several tests booting servers at once another one can claim it first. The
/// server exits 1 on a bound port, so retry with a fresh one.
async fn start_server() -> ServerGuard {
    for _ in 0..5 {
        if let Some(server) = try_start_server().await {
            return server;
        }
    }
    panic!("could not start cook server on a free port after 5 attempts");
}

async fn try_start_server() -> Option<ServerGuard> {
    let dir = TempDir::new().expect("temp dir");
    write_fixture(&dir);

    let port = free_port();
    let child = Command::new(assert_cmd::cargo::cargo_bin("cook"))
        .arg("server")
        .arg(dir.path())
        .arg("--port")
        .arg(port.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn cook server");

    let mut guard = ServerGuard {
        child,
        port,
        _dir: dir,
    };

    // Probed on a route that reads neither shopping list file, so waiting for
    // the server can't itself be mistaken for activity on them.
    let client = reqwest::Client::new();
    let url = guard.url("/api/recipes");
    for _ in 0..200 {
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

/// An open event stream, and whatever of it has arrived but not been read.
struct Events {
    response: reqwest::Response,
    unread: String,
}

impl Events {
    /// Subscribe. The server subscribes a connection before answering it, so
    /// every change made after this returns is announced on this stream.
    async fn open(server: &ServerGuard) -> Events {
        let response = reqwest::get(server.url("/api/shopping_list/events"))
            .await
            .expect("open the event stream");
        assert!(
            response.status().is_success(),
            "the event stream answered {}",
            response.status()
        );
        Events {
            response,
            unread: String::new(),
        }
    }

    /// The data of the next `change` event, or `None` if none arrives within
    /// `wait`. Anything else on the stream — the keep-alive comments — is
    /// skipped.
    async fn next_change(&mut self, wait: Duration) -> Option<Value> {
        tokio::time::timeout(wait, async {
            loop {
                // Events are separated by a blank line.
                if let Some(end) = self.unread.find("\n\n") {
                    let event: String = self.unread.drain(..end + 2).collect();
                    if let Some(data) = change_data(&event) {
                        return data;
                    }
                    continue;
                }
                let chunk = self
                    .response
                    .chunk()
                    .await
                    .expect("read the event stream")
                    .expect("the event stream ended");
                self.unread.push_str(&String::from_utf8_lossy(&chunk));
            }
        })
        .await
        .ok()
    }
}

/// The parsed `data` of a `change` event, or `None` for any other event.
fn change_data(event: &str) -> Option<Value> {
    let mut name = None;
    let mut data = None;
    for line in event.lines() {
        if let Some(value) = line.strip_prefix("event:") {
            name = Some(value.trim());
        } else if let Some(value) = line.strip_prefix("data:") {
            data = Some(value.trim());
        }
    }
    if name != Some("change") {
        return None;
    }
    let data = data.expect("a change event carries data");
    Some(serde_json::from_str(data).expect("change data is JSON"))
}

async fn post(server: &ServerGuard, path: &str, body: Value) {
    let resp = reqwest::Client::new()
        .post(server.url(path))
        .json(&body)
        .send()
        .await
        .unwrap_or_else(|e| panic!("POST {path}: {e}"));
    assert!(
        resp.status().is_success(),
        "POST {path} answered {}",
        resp.status()
    );
}

/// Everything the shopping list page reads to render itself: the stored list,
/// the list generated from it, and the ticks. None of it is a change.
#[tokio::test]
async fn reading_the_list_does_not_announce_a_change() {
    let server = start_server().await;
    let mut events = Events::open(&server).await;
    let client = reqwest::Client::new();

    let items: Vec<Value> = client
        .get(server.url("/api/shopping_list/items"))
        .send()
        .await
        .expect("GET items")
        .json()
        .await
        .expect("items are JSON");
    assert_eq!(items.len(), 1, "the fixture's list holds one recipe");

    // The same request the page builds from those items.
    let recipes: Vec<Value> = items
        .iter()
        .map(|item| json!({ "recipe": item["path"], "scale": item["scale"] }))
        .collect();
    post(&server, "/api/shopping_list", json!(recipes)).await;

    let checked = client
        .get(server.url("/api/shopping_list/checked"))
        .send()
        .await
        .expect("GET checked");
    assert!(checked.status().is_success());

    // The watcher waits 200 ms for a burst of events to settle before it
    // reports one, so a read mistaken for a change would arrive well inside
    // this.
    assert_eq!(events.next_change(Duration::from_secs(1)).await, None);
}

/// A tick is a change to the checked log and an added recipe a change to the
/// list, and each event names the file that changed — which is how the page
/// knows whether a tick means re-reading only the ticks.
#[tokio::test]
async fn writing_announces_which_file_changed() {
    let server = start_server().await;
    let mut events = Events::open(&server).await;

    post(
        &server,
        "/api/shopping_list/check",
        json!({ "name": "water" }),
    )
    .await;
    assert_eq!(
        events.next_change(Duration::from_secs(10)).await,
        Some(json!({ "file": "checked" }))
    );

    post(
        &server,
        "/api/shopping_list/add",
        json!({ "path": "Soup", "scale": 2.0 }),
    )
    .await;
    assert_eq!(
        events.next_change(Duration::from_secs(10)).await,
        Some(json!({ "file": "list" }))
    );
}
