//! End-to-end tests that `GET /api/shopping_list/items` names each entry by
//! the `title` its recipe declares, not by its file name.
//!
//! The shopping list page shows these names in its "Selected recipes" panel:
//! a recipe at `Breakfast/easy-pancakes.cook` that declares
//! `title: Easy Pancakes` must read as the latter, as it does on its own page.
//! The same goes for a menu, the recipes nested in it, and the sub-recipes an
//! entry pulls in (`included_reference_names`). Whatever declares no title, or
//! cannot be read, keeps its file name.

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

/// `<tmp>/recipes/` is what the server serves.
fn write_fixture(dir: &TempDir) {
    let recipes = dir.path().join("recipes");
    for sub in ["Breakfast", "Shared", "Menus"] {
        std::fs::create_dir_all(recipes.join(sub)).unwrap();
    }

    std::fs::write(
        recipes.join("Breakfast").join("easy-pancakes.cook"),
        "---\ntitle: Easy Pancakes\n---\nMix @flour{100%g} and pour @./Shared/syrup{} over.\n",
    )
    .unwrap();
    std::fs::write(
        recipes.join("Shared").join("syrup.cook"),
        "---\ntitle: Maple Syrup\n---\nWarm @maple sap{1%cup}.\n",
    )
    .unwrap();
    // No title: keeps its file name.
    std::fs::write(recipes.join("plain.cook"), "Boil @water{1%l}.\n").unwrap();
    std::fs::write(
        recipes.join("Menus").join("week.menu"),
        "---\ntitle: Week One\n---\nMonday: \\\n- @../Breakfast/easy-pancakes{}\n- @../plain{}\n",
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

async fn post(server: &ServerGuard, path: &str, body: Value) {
    let resp = reqwest::Client::new()
        .post(server.url(path))
        .json(&body)
        .send()
        .await
        .unwrap_or_else(|e| panic!("post {path}: {e}"));
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    assert!(status.is_success(), "{path} {body} → {status}: {text}");
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

fn entry<'a>(items: &'a [Value], path: &str) -> &'a Value {
    items
        .iter()
        .find(|item| item["path"] == path)
        .unwrap_or_else(|| panic!("no entry for {path} in {items:#?}"))
}

#[tokio::test]
async fn a_recipe_is_named_by_its_title_and_so_are_its_sub_recipes() {
    let server = start_server().await;
    post(
        &server,
        "/api/shopping_list/add",
        json!({
            "path": "Breakfast/easy-pancakes.cook",
            "scale": 1.0,
            "included_references": ["Shared/syrup"],
        }),
    )
    .await;

    let items = items(&server).await;
    let pancakes = entry(&items, "Breakfast/easy-pancakes.cook");
    assert_eq!(pancakes["name"], "Easy Pancakes");
    assert_eq!(pancakes["included_reference_names"], json!(["Maple Syrup"]));
}

#[tokio::test]
async fn a_reference_that_steps_up_is_named_from_the_recipe_writing_it() {
    let server = start_server().await;
    post(
        &server,
        "/api/shopping_list/add",
        json!({
            "path": "Breakfast/easy-pancakes.cook",
            "scale": 1.0,
            "included_references": ["../Shared/syrup"],
        }),
    )
    .await;

    let items = items(&server).await;
    let pancakes = entry(&items, "Breakfast/easy-pancakes.cook");
    assert_eq!(pancakes["included_reference_names"], json!(["Maple Syrup"]));
}

#[tokio::test]
async fn without_a_title_or_a_file_the_file_name_stays() {
    let server = start_server().await;
    post(
        &server,
        "/api/shopping_list/add",
        json!({ "path": "plain.cook", "scale": 1.0 }),
    )
    .await;
    post(
        &server,
        "/api/shopping_list/add",
        json!({
            "path": "Breakfast/gone.cook",
            "scale": 1.0,
            "included_references": ["Shared/also-gone"],
        }),
    )
    .await;

    let items = items(&server).await;
    assert_eq!(entry(&items, "plain.cook")["name"], "plain");
    let gone = entry(&items, "Breakfast/gone.cook");
    assert_eq!(gone["name"], "gone");
    assert_eq!(gone["included_reference_names"], json!(["also-gone"]));
}

#[tokio::test]
async fn a_menu_and_the_recipes_in_it_are_named_by_their_titles() {
    let server = start_server().await;
    post(
        &server,
        "/api/shopping_list/add_menu",
        json!({ "path": "Menus/week.menu", "scale": 1.0 }),
    )
    .await;

    let items = items(&server).await;
    let menu = entry(&items, "Menus/week.menu");
    assert_eq!(menu["name"], "Week One");

    let recipes = menu["recipes"].as_array().expect("menu has recipes");
    assert_eq!(
        entry(recipes, "Breakfast/easy-pancakes")["name"],
        "Easy Pancakes"
    );
    assert_eq!(entry(recipes, "plain")["name"], "plain");
    // The pancakes' own sub-recipe, written `@./Shared/syrup` from `Breakfast/`.
    assert_eq!(
        entry(recipes, "Breakfast/easy-pancakes")["included_reference_names"],
        json!(["Maple Syrup"])
    );
}
