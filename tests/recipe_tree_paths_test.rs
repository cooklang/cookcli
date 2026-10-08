//! `GET /api/recipes` names files relative to the recipe directory.
//!
//! The tree comes from `cooklang_find` with absolute paths, which told any
//! client — a visitor who is not signed in included — where the collection
//! sits on the server and under which account. Every `path`, and every
//! recipe's `source.path`, is now relative to the collection, the root's
//! being `""`; each still opens its recipe through `GET /api/recipes/{path}`.

#![cfg(feature = "server")]

#[path = "common/mod.rs"]
mod common;

use serde_json::Value;
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

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

/// A recipe at the root, one in a folder, and a menu.
fn write_fixture(dir: &TempDir) {
    let root = dir.path();
    std::fs::write(root.join("Soup.cook"), "Simmer @water{1%l}.\n").unwrap();
    std::fs::create_dir_all(root.join("Breakfast")).unwrap();
    std::fs::write(
        root.join("Breakfast/Pancakes.cook"),
        "Whisk @flour{200%g} and @milk{300%ml}.\n",
    )
    .unwrap();
    std::fs::write(
        root.join("Week.menu"),
        "== Day 1 ==\n\nLunch:\n- @./Soup{}\n",
    )
    .unwrap();
}

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
        .arg(dir.path())
        .arg("--port")
        .arg(port.to_string());
    let child = common::with_isolated_config(&mut cmd, dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn cook server");

    let mut guard = ServerGuard { child, port, dir };

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

/// Every recipe node's `path` and `recipe.source.path`, depth first.
fn recipe_paths(node: &Value, found: &mut Vec<(String, String)>) {
    if !node["recipe"].is_null() {
        found.push((
            node["path"].as_str().unwrap().to_string(),
            node["recipe"]["source"]["path"]
                .as_str()
                .unwrap()
                .to_string(),
        ));
    }
    for child in node["children"].as_object().unwrap().values() {
        recipe_paths(child, found);
    }
}

#[tokio::test]
async fn the_tree_never_names_the_server_directory() {
    let server = start_server().await;
    let root = server.dir.path().to_str().unwrap().to_string();

    let text = reqwest::get(server.url("/api/recipes"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(!text.contains(&root), "the tree names {root}: {text}");

    let tree: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(tree["path"], "");
    assert_eq!(tree["children"]["Breakfast"]["path"], "Breakfast");

    let mut found = Vec::new();
    recipe_paths(&tree, &mut found);
    found.sort();
    assert_eq!(
        found,
        [
            (
                "Breakfast/Pancakes.cook".to_string(),
                "Breakfast/Pancakes.cook".to_string()
            ),
            ("Soup.cook".to_string(), "Soup.cook".to_string()),
            ("Week.menu".to_string(), "Week.menu".to_string()),
        ]
    );
}

#[tokio::test]
async fn each_path_opens_its_recipe() {
    let server = start_server().await;
    let tree: Value = reqwest::get(server.url("/api/recipes"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let mut found = Vec::new();
    recipe_paths(&tree, &mut found);
    for (path, _) in found.iter().filter(|(path, _)| path.ends_with(".cook")) {
        let resp = reqwest::get(server.url(&format!("/api/recipes/{path}")))
            .await
            .unwrap();
        assert!(resp.status().is_success(), "{path}: {}", resp.status());
    }
}
