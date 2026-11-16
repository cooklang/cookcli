//! End-to-end tests that `POST /api/shopping_list/remove` takes the entry the
//! page names, not the first one with the same path.
//!
//! The same recipe can be on the list twice — once with a sub-recipe and once
//! without, or at two scales — and Remove on the second copy used to take the
//! first. The page now sends the entry's position along with its path; the
//! server removes that entry only while its path still matches, and answers
//! `409` rather than guess when the list changed underneath.
//!
//! A remove also drops the ticks of ingredients no longer on the list. It
//! must keep the ones the page shows under an aisle file's common name.

#![cfg(feature = "server")]

use serde_json::{json, Value};
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

#[path = "common/mod.rs"]
mod common;

struct ServerGuard {
    child: Child,
    port: u16,
    /// Holds the recipe directory for the server's lifetime.
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

/// `<tmp>/recipes/` is what the server serves: a soup that may pull in a stock,
/// and two recipes using `eggs`, which the aisle file lists as `egg`.
fn write_fixture(dir: &TempDir) {
    let recipes = dir.path().join("recipes");
    std::fs::create_dir_all(recipes.join("config")).unwrap();
    std::fs::write(
        recipes.join("soup.cook"),
        "Simmer @leeks{2} in @./stock{1%l}.\n",
    )
    .unwrap();
    std::fs::write(recipes.join("stock.cook"), "Boil @bones{1%kg}.\n").unwrap();
    std::fs::write(
        recipes.join("pancakes.cook"),
        "Whisk @eggs{2} into @milk{300%ml}.\n",
    )
    .unwrap();
    std::fs::write(recipes.join("omelette.cook"), "Beat @eggs{3}.\n").unwrap();
    std::fs::write(
        recipes.join("config/aisle.conf"),
        "[dairy]\negg | eggs\nmilk\n",
    )
    .unwrap();
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
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin("cook"));
    cmd.arg("server")
        .arg(dir.path().join("recipes"))
        .arg("--port")
        .arg(port.to_string());
    let child = common::with_isolated_config(&mut cmd, dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn cook server");

    let mut guard = ServerGuard {
        child,
        port,
        _dir: dir,
    };

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

/// POST `body` to `path` and return the status it answered with.
async fn post(server: &ServerGuard, path: &str, body: Value) -> reqwest::StatusCode {
    reqwest::Client::new()
        .post(server.url(path))
        .json(&body)
        .send()
        .await
        .unwrap_or_else(|e| panic!("post {path}: {e}"))
        .status()
}

async fn items(server: &ServerGuard) -> Vec<Value> {
    let text = reqwest::get(server.url("/api/shopping_list/items"))
        .await
        .expect("get items")
        .text()
        .await
        .expect("items body");
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("items is not JSON ({e}): {text}"))
}

/// The soup twice: first with its stock at ×1, then without it at ×2 — what
/// adding it, unticking the stock and adding it again stores.
async fn soup_twice(server: &ServerGuard) {
    for (scale, refs) in [(1.0, json!(["./stock.cook"])), (2.0, json!([]))] {
        let status = post(
            server,
            "/api/shopping_list/add",
            json!({ "path": "soup.cook", "scale": scale, "included_references": refs }),
        )
        .await;
        assert!(status.is_success(), "add soup ×{scale} → {status}");
    }
}

#[tokio::test]
async fn remove_with_an_index_takes_that_entry() {
    let server = start_server().await;
    soup_twice(&server).await;

    let status = post(
        &server,
        "/api/shopping_list/remove",
        json!({ "path": "soup.cook", "index": 1 }),
    )
    .await;
    assert!(status.is_success(), "remove → {status}");

    let items = items(&server).await;
    assert_eq!(items.len(), 1, "one soup left: {items:#?}");
    assert_eq!(items[0]["scale"], 1.0, "the first soup stays: {items:#?}");
    assert_eq!(
        items[0]["included_references"],
        json!(["stock.cook"]),
        "with its stock: {items:#?}"
    );
}

#[tokio::test]
async fn remove_answers_409_and_keeps_the_list_when_the_entry_moved() {
    let server = start_server().await;
    soup_twice(&server).await;
    let before = items(&server).await;

    for index in [2, 7] {
        let status = post(
            &server,
            "/api/shopping_list/remove",
            json!({ "path": "soup.cook", "index": index }),
        )
        .await;
        assert_eq!(status, reqwest::StatusCode::CONFLICT, "index {index}");
    }
    let status = post(
        &server,
        "/api/shopping_list/remove",
        json!({ "path": "stock.cook", "index": 0 }),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::CONFLICT, "another path");

    assert_eq!(items(&server).await, before);
}

#[tokio::test]
async fn remove_without_an_index_still_takes_the_first_entry() {
    let server = start_server().await;
    soup_twice(&server).await;

    let status = post(
        &server,
        "/api/shopping_list/remove",
        json!({ "path": "soup.cook" }),
    )
    .await;
    assert!(status.is_success(), "remove → {status}");

    let items = items(&server).await;
    assert_eq!(items.len(), 1, "one soup left: {items:#?}");
    assert_eq!(items[0]["scale"], 2.0, "the second soup stays: {items:#?}");
}

async fn checked(server: &ServerGuard) -> Vec<String> {
    reqwest::get(server.url("/api/shopping_list/checked"))
        .await
        .expect("get checked")
        .json()
        .await
        .expect("checked is a list of names")
}

/// Pancakes then an omelette on the list, `ticks` ticked, then the pancakes
/// removed: what is still ticked.
async fn ticks_after_removing_the_pancakes(ticks: &[&str]) -> Vec<String> {
    let server = start_server().await;
    for path in ["pancakes.cook", "omelette.cook"] {
        let status = post(
            &server,
            "/api/shopping_list/add",
            json!({ "path": path, "scale": 1.0 }),
        )
        .await;
        assert!(status.is_success(), "add {path} → {status}");
    }
    for name in ticks {
        let status = post(&server, "/api/shopping_list/check", json!({ "name": name })).await;
        assert!(status.is_success(), "check {name} → {status}");
    }

    let status = post(
        &server,
        "/api/shopping_list/remove",
        json!({ "path": "pancakes.cook", "index": 0 }),
    )
    .await;
    assert!(status.is_success(), "remove → {status}");
    checked(&server).await
}

#[tokio::test]
async fn remove_keeps_a_tick_on_the_aisle_name_of_an_ingredient_still_listed() {
    // The page shows the omelette's `eggs` as `egg`, and that is what it ticks.
    assert_eq!(ticks_after_removing_the_pancakes(&["egg"]).await, ["egg"]);
}

#[tokio::test]
async fn remove_keeps_a_tick_on_the_recipe_name_of_an_ingredient_still_listed() {
    assert_eq!(ticks_after_removing_the_pancakes(&["eggs"]).await, ["eggs"]);
}

#[tokio::test]
async fn remove_drops_the_tick_of_an_ingredient_no_longer_listed() {
    assert_eq!(
        ticks_after_removing_the_pancakes(&["egg", "milk"]).await,
        ["egg"]
    );
}
