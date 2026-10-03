//! Optional ingredients on the web shopping list (spec proposal 0018): choosing
//! them when a recipe is added, the `? name{quantity}` selection lines that
//! stores in `.shopping-list`, and how the generated list marks them.
//!
//! Each test boots `cook server` against a temporary recipe directory on its
//! own port, as `menu_api_test` does.

#![cfg(feature = "server")]

#[path = "common/mod.rs"]
mod common;

use serde_json::{json, Value};
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

    fn list_file(&self) -> String {
        std::fs::read_to_string(self.dir.path().join(".shopping-list")).unwrap_or_default()
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

fn write_fixture(dir: &TempDir) {
    let root = dir.path();
    std::fs::write(
        root.join("Eggs on toast.cook"),
        "Fry @eggs{2} in @butter{10%g}.\n\n\
         Serve on @toast{2%slices}, garnished with @?chives and @?chilli flakes{1%pinch}.\n",
    )
    .unwrap();
    std::fs::write(root.join("Cake.cook"), "Bake @flour{200%g}.\n").unwrap();
    std::fs::write(
        root.join("Plan.menu"),
        "==Day 1==\n\nBreakfast: @./Eggs on toast{}\n\nDessert: @?./Cake{}\n",
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
    let url = guard.url("/api/shopping_list/items");
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

async fn post(server: &ServerGuard, path: &str, body: Value) -> Value {
    let resp = reqwest::Client::new()
        .post(server.url(path))
        .json(&body)
        .send()
        .await
        .unwrap_or_else(|e| panic!("POST {path}: {e}"));
    let status = resp.status();
    let text = resp.text().await.expect("response body");
    assert!(status.is_success(), "POST {path} answered {status}: {text}");
    serde_json::from_str(&text).unwrap_or(Value::Null)
}

/// `(name, optional, quantities)` for every item of a generated list.
fn items(list: &Value) -> Vec<(String, bool, Vec<Value>)> {
    list["categories"]
        .as_array()
        .expect("categories")
        .iter()
        .flat_map(|c| c["items"].as_array().expect("items").clone())
        .map(|i| {
            (
                i["name"].as_str().expect("name").to_string(),
                i["optional"].as_bool().expect("optional is a bool"),
                i["quantities"].as_array().expect("quantities").clone(),
            )
        })
        .collect()
}

/// A chosen optional ingredient is stored with its amount at the recipe's
/// scale, and comes back on the list marked optional; one not chosen stays
/// off it.
#[tokio::test]
async fn a_chosen_optional_ingredient_is_stored_and_listed_as_optional() {
    let server = start_server().await;

    post(
        &server,
        "/api/shopping_list/add",
        json!({
            "path": "Eggs on toast.cook",
            "scale": 2.0,
            "optional_ingredients": ["chilli flakes"],
        }),
    )
    .await;

    assert_eq!(
        server.list_file(),
        "./Eggs on toast.cook{2}\n  ? chilli flakes{2%pinch}\n"
    );

    let stored: Value = reqwest::get(server.url("/api/shopping_list/items"))
        .await
        .expect("GET items")
        .json()
        .await
        .expect("items are JSON");
    let entry = &stored[0];
    assert_eq!(
        entry["optional_ingredients"],
        json!([{ "name": "chilli flakes", "quantity": "2%pinch" }])
    );

    // What the list page posts back: the stored entry as it came.
    let list = post(
        &server,
        "/api/shopping_list",
        json!([{
            "recipe": entry["path"],
            "scale": entry["scale"],
            "optional_ingredients": entry["optional_ingredients"],
        }]),
    )
    .await;

    let got: Vec<(String, bool)> = items(&list)
        .into_iter()
        .map(|(name, optional, _)| (name, optional))
        .collect();
    assert!(got.contains(&("eggs".to_string(), false)), "{got:?}");
    assert!(
        got.contains(&("chilli flakes".to_string(), true)),
        "{got:?}"
    );
    assert!(
        !got.iter().any(|(name, _)| name == "chives"),
        "an optional ingredient nobody chose stays off the list: {got:?}"
    );

    // The amount is the stored one, not scaled by the entry's ×2 again.
    let chilli = items(&list)
        .into_iter()
        .find(|(name, _, _)| name == "chilli flakes")
        .expect("chilli flakes listed");
    assert_eq!(chilli.2.len(), 1);
    assert_eq!(chilli.2[0]["unit"], "pinch");
    assert_eq!(
        chilli.2[0]["value"]["value"]["value"],
        json!(2.0),
        "{:?}",
        chilli.2
    );
}

/// Adding a recipe without choosing anything leaves its optional ingredients
/// off entirely.
#[tokio::test]
async fn optional_ingredients_are_left_off_unless_chosen() {
    let server = start_server().await;

    let list = post(
        &server,
        "/api/shopping_list",
        json!([{ "recipe": "Eggs on toast.cook", "scale": 1.0 }]),
    )
    .await;

    let names: Vec<String> = items(&list).into_iter().map(|(name, ..)| name).collect();
    assert_eq!(names.len(), 3, "eggs, butter and toast only: {names:?}");
    assert!(items(&list).iter().all(|(_, optional, _)| !optional));
}

/// A menu is added whole, and an optional recipe in it (`@?./Cake{}`) is not
/// part of that.
#[tokio::test]
async fn a_menu_leaves_out_its_optional_recipes() {
    let server = start_server().await;

    post(
        &server,
        "/api/shopping_list/add_menu",
        json!({ "path": "Plan.menu", "scale": 1.0 }),
    )
    .await;

    let file = server.list_file();
    assert!(file.contains("./Eggs on toast"), "{file}");
    assert!(!file.contains("Cake"), "{file}");
}
