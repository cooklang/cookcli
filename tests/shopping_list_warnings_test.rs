//! What the shopping list page is told about recipes it could not count in
//! full (#682).
//!
//! A reference asking for an amount its recipe cannot be scaled to is listed as
//! written, and the warning saying so used to reach only the server's log. The
//! page could not tell the shopper why the sub-recipe's ingredients were
//! missing. `POST /api/shopping_list` now sends it in `warnings`.

#![cfg(feature = "server")]

#[path = "common/mod.rs"]
mod common;

use serde_json::{json, Value};
use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

/// The carbonara asks its fresh pasta for 600 g, and the pasta declares only
/// servings, so it cannot be scaled to a weight. The salad is fine.
fn write_fixture(root: &Path) {
    std::fs::create_dir_all(root.join("Plats")).unwrap();
    std::fs::write(
        root.join("Plats/Pâtes fraîches.cook"),
        "---\nservings: 4\n---\n\nKnead @flour{400%g} with @eggs{4}.\n",
    )
    .unwrap();
    std::fs::write(
        root.join("Plats/Carbonara.cook"),
        "---\nservings: 4\n---\n\nBoil @./Plats/Pâtes fraîches{600%g}, \
         add @guanciale{200%g}.\n",
    )
    .unwrap();
    std::fs::write(root.join("Salad.cook"), "Toss @lettuce{1}.\n").unwrap();
}

/// Kills the spawned server when the test ends, pass or panic.
struct ServerGuard {
    child: Child,
    port: u16,
    dir: TempDir,
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

/// `free_port` only reserves a port long enough to learn its number, so retry
/// with a fresh one if another test claims it first.
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
    let recipes = dir.path().join("recipes");
    write_fixture(&recipes);

    let port = free_port();
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin("cook"));
    cmd.arg("server")
        .arg(&recipes)
        .arg("--port")
        .arg(port.to_string());
    let child = common::with_isolated_config(&mut cmd, dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn cook server");

    let mut guard = ServerGuard { child, port, dir };
    let client = reqwest::Client::new();
    for _ in 0..200 {
        if guard.child.try_wait().expect("poll server").is_some() {
            return None;
        }
        if let Ok(resp) = client.get(guard.url("/api/recipes")).send().await {
            if resp.status().is_success() {
                return Some(guard);
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("cook server on port {port} never became ready");
}

/// The list for `requests`, as the shopping list page asks for it.
async fn list(server: &ServerGuard, requests: Value) -> Value {
    reqwest::Client::new()
        .post(server.url("/api/shopping_list"))
        .json(&requests)
        .send()
        .await
        .expect("request")
        .error_for_status()
        .expect("success")
        .json()
        .await
        .expect("json")
}

fn warnings(list: &Value) -> Vec<String> {
    list["warnings"]
        .as_array()
        .expect("`warnings` is always sent")
        .iter()
        .map(|w| w.as_str().unwrap().to_string())
        .collect()
}

fn item_names(list: &Value) -> Vec<String> {
    list["categories"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|c| c["items"].as_array().unwrap().clone())
        .map(|i| i["name"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn a_reference_listed_as_written_is_explained_to_the_page() {
    let server = start_server().await;

    let list = list(
        &server,
        json!([{ "recipe": "Plats/Carbonara.cook", "scale": 0.5 }]),
    )
    .await;

    let names = item_names(&list);
    assert!(names.contains(&"Pâtes fraîches".to_string()), "{names:?}");
    assert!(!names.contains(&"flour".to_string()), "{names:?}");

    let warnings = warnings(&list);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(
        warnings[0].starts_with("Plats/Carbonara.cook: Could not scale recipe reference"),
        "named after the recipe holding the reference, relative to the recipe \
         directory: {warnings:?}"
    );
    assert!(warnings[0].contains("`yield: 500%g`"), "{warnings:?}");

    let base = server.dir.path().to_string_lossy().to_string();
    assert!(
        !warnings[0].contains(&base),
        "the server's own folders stay out of what it sends: {warnings:?}"
    );
}

#[tokio::test]
async fn a_list_with_nothing_to_say_sends_no_warnings() {
    let server = start_server().await;

    let list = list(&server, json!([{ "recipe": "Salad.cook" }])).await;

    assert_eq!(item_names(&list), vec!["lettuce".to_string()]);
    assert!(warnings(&list).is_empty(), "{list}");
}

/// The same recipe twice on the list raises the same warning twice; the page
/// is told once.
#[tokio::test]
async fn the_same_warning_is_sent_once() {
    let server = start_server().await;

    let list = list(
        &server,
        json!([
            { "recipe": "Plats/Carbonara.cook", "scale": 0.5 },
            { "recipe": "Plats/Carbonara.cook", "scale": 1 },
            { "recipe": "Salad.cook" }
        ]),
    )
    .await;

    assert_eq!(warnings(&list).len(), 2, "one per amount asked for: {list}");

    let list = self::list(
        &server,
        json!([
            { "recipe": "Plats/Carbonara.cook", "scale": 0.5 },
            { "recipe": "Plats/Carbonara.cook", "scale": 0.5 }
        ]),
    )
    .await;
    assert_eq!(warnings(&list).len(), 1, "{list}");
}
