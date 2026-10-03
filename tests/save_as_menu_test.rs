//! `POST /api/shopping_list/save_as_menu`: the shopping list written to a new
//! menu, which `add_menu` turns back into the same list (#385, the request
//! from #380).

#![cfg(feature = "server")]

use reqwest::StatusCode;
use serde_json::{json, Value};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

#[path = "common/mod.rs"]
mod common;

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

    fn recipes(&self) -> PathBuf {
        self.dir.path().join("recipes")
    }

    async fn post(&self, path: &str, body: Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(self.url(path))
            .json(&body)
            .send()
            .await
            .expect("request")
    }

    async fn save(&self, name: &str) -> reqwest::Response {
        self.post("/api/shopping_list/save_as_menu", json!({ "name": name }))
            .await
    }

    /// The recipes on the list as `(path, scale)`, a menu's in its place.
    async fn listed(&self) -> Vec<(String, f64)> {
        let items: Vec<Value> = reqwest::get(self.url("/api/shopping_list/items"))
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let mut out = Vec::new();
        for item in &items {
            match item["recipes"].as_array() {
                Some(recipes) => out.extend(recipes.iter().map(pair)),
                None => out.push(pair(item)),
            }
        }
        out
    }
}

fn pair(item: &Value) -> (String, f64) {
    (
        item["path"].as_str().unwrap().to_string(),
        item["scale"].as_f64().unwrap(),
    )
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
    let recipes = dir.path().join("recipes");
    std::fs::create_dir_all(recipes.join("Breakfast")).unwrap();
    std::fs::create_dir_all(recipes.join("Plans")).unwrap();
    std::fs::write(
        recipes.join("Soup.cook"),
        "---\nservings: 2\n---\n\nSimmer @water{1%l}.\n",
    )
    .unwrap();
    std::fs::write(
        recipes.join("Breakfast/Pancakes.cook"),
        "Whisk @flour{200%g} and @milk{300%ml}.\n",
    )
    .unwrap();
    std::fs::write(
        recipes.join("Plans/Week.menu"),
        "== Day 1 ==\n\nDinner: \\\n- @./Soup{4%servings} \\\n- @./Breakfast/Pancakes{}\n",
    )
    .unwrap();

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

#[tokio::test]
async fn a_saved_list_comes_back_as_the_same_list() {
    let server = start_server().await;
    let add = server
        .post(
            "/api/shopping_list/add",
            json!({ "path": "Breakfast/Pancakes", "scale": 1.5 }),
        )
        .await;
    assert!(add.status().is_success());
    let add = server
        .post(
            "/api/shopping_list/add_menu",
            json!({ "path": "Plans/Week.menu", "scale": 2.0 }),
        )
        .await;
    assert!(add.status().is_success());
    let before = server.listed().await;
    assert_eq!(
        before,
        [
            ("Breakfast/Pancakes".to_string(), 1.5),
            ("Soup".to_string(), 4.0),
            ("Breakfast/Pancakes".to_string(), 2.0),
        ]
    );

    let resp = server.save("Plans/Saved").await;

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body, json!({ "path": "Plans/Saved.menu", "skipped": [] }));
    assert_eq!(
        std::fs::read_to_string(server.recipes().join("Plans/Saved.menu")).unwrap(),
        "---\ntitle: Saved\n---\n\n\
         - @./Breakfast/Pancakes{1.5} \\\n\
         - @./Soup{4} \\\n\
         - @./Breakfast/Pancakes{2}\n"
    );
    // Saving leaves the list alone.
    assert_eq!(server.listed().await, before);

    // Adding the menu back gives the same recipes at the same factors.
    assert!(server
        .post("/api/shopping_list/clear", json!({}))
        .await
        .status()
        .is_success());
    let add = server
        .post(
            "/api/shopping_list/add_menu",
            json!({ "path": "Plans/Saved.menu", "scale": 1.0 }),
        )
        .await;
    assert!(add.status().is_success());
    assert_eq!(server.listed().await, before);

    // And the menu page links each recipe at that factor.
    let page = reqwest::get(server.url("/recipe/Plans/Saved.menu"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(
        page.contains("/recipe/Breakfast/Pancakes?scale=1.5"),
        "{page}"
    );
    assert!(page.contains("/recipe/Soup?servings=8"), "{page}");
}

#[tokio::test]
async fn an_empty_list_saves_nothing() {
    let server = start_server().await;

    let resp = server.save("Empty").await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "The shopping list has no recipes to save");
    assert!(!server.recipes().join("Empty.menu").exists());
}

#[tokio::test]
async fn an_existing_menu_is_never_overwritten() {
    let server = start_server().await;
    server
        .post(
            "/api/shopping_list/add",
            json!({ "path": "Soup", "scale": 1.0 }),
        )
        .await;
    let before = std::fs::read_to_string(server.recipes().join("Plans/Week.menu")).unwrap();

    let resp = server.save("Plans/Week").await;

    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "A menu with this name already exists");
    assert_eq!(
        std::fs::read_to_string(server.recipes().join("Plans/Week.menu")).unwrap(),
        before
    );

    let resp = server.save("  /// ").await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_name_cannot_leave_the_collection() {
    let server = start_server().await;
    server
        .post(
            "/api/shopping_list/add",
            json!({ "path": "Soup", "scale": 1.0 }),
        )
        .await;

    // Dots are not kept, so `..` has nothing to climb with.
    let resp = server.save("../../Escaped").await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["path"], "Escaped.menu");
    assert!(server.recipes().join("Escaped.menu").exists());
    assert!(!server.dir.path().join("Escaped.menu").exists());
}

#[cfg(unix)]
#[tokio::test]
async fn a_menu_cannot_be_written_through_a_symlink_out_of_the_collection() {
    let server = start_server().await;
    server
        .post(
            "/api/shopping_list/add",
            json!({ "path": "Soup", "scale": 1.0 }),
        )
        .await;
    let outside = server.dir.path().join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, server.recipes().join("Linked")).unwrap();

    let resp = server.save("Linked/Week").await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "Invalid menu path");
    assert!(!outside.join("Week.menu").exists());
    assert!(server.recipes().join("Linked").exists(), "the link is kept");
}
