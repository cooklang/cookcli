//! Some days of a meal plan to the shopping list (#385).
//!
//! `POST /api/shopping_list/add_menu` with `dates` adds only the recipes and
//! free-hand ingredients written in those days' sections: the recipes as
//! recipe entries, the ingredients as free-hand lines, both native to the
//! `.shopping-list` format.

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

    fn list_file(&self) -> String {
        std::fs::read_to_string(self.recipes().join(".shopping-list")).unwrap_or_default()
    }

    async fn post(&self, path: &str, body: Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(self.url(path))
            .json(&body)
            .send()
            .await
            .expect("request")
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

const PLAN: &str = "---\nservings: 2\nplan:\n  start: 2026-10-05\n  days: 3\n---\n\n\
== Monday (2026-10-05) ==\n\n\
Breakfast: \\\n\
- @./Pancakes{4%servings} \\\n\
- @almonds{50%g}\n\n\
== Tuesday (2026-10-06) ==\n\n\
Dinner: \\\n\
- @./Soup{} \\\n\
- @bread{1%loaf}\n\n\
== Wednesday (2026-10-07) ==\n\n\
Snacks: \\\n\
- @almonds{50%g} \\\n\
- @dark chocolate{30%g} \\\n\
- @salt{}\n\n\
== Leftovers ==\n\n\
- @./Soup{2}\n";

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
    std::fs::create_dir_all(&recipes).unwrap();
    std::fs::write(
        recipes.join("Pancakes.cook"),
        "---\nservings: 2\n---\n\nMix @flour{200%g} and @milk{300%ml}.\n",
    )
    .unwrap();
    std::fs::write(recipes.join("Soup.cook"), "Simmer @leeks{2}.\n").unwrap();
    std::fs::write(recipes.join("Plan.menu"), PLAN).unwrap();

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

/// The names on the list the page would show for what is stored now.
async fn generated_names(server: &ServerGuard) -> Vec<String> {
    let client = reqwest::Client::new();
    let items: Vec<Value> = client
        .get(server.url("/api/shopping_list/items"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let extra: Vec<Value> = client
        .get(server.url("/api/shopping_list/extra_items"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // What the shopping list page posts: each recipe, then the extra items.
    let mut request: Vec<Value> = items
        .iter()
        .map(|item| json!({ "recipe": item["path"], "scale": item["scale"] }))
        .collect();
    request.extend(extra);

    let list: Value = server
        .post("/api/shopping_list", Value::Array(request))
        .await
        .json()
        .await
        .unwrap();
    let mut names: Vec<String> = list["categories"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|category| category["items"].as_array().unwrap().clone())
        .map(|item| item["name"].as_str().unwrap().to_string())
        .collect();
    names.sort();
    names
}

#[tokio::test]
async fn only_the_chosen_days_go_on_the_list() {
    let server = start_server().await;

    let resp = server
        .post(
            "/api/shopping_list/add_menu",
            json!({ "path": "Plan.menu", "scale": 2.0, "dates": ["2026-10-05", "2026-10-07"] }),
        )
        .await;

    assert_eq!(resp.status(), StatusCode::OK);
    // Pancakes at 4 of its 2 servings, ×2 for the page's servings; almonds
    // from both days merged; nothing from Tuesday or the undated section.
    assert_eq!(
        server.list_file(),
        "./Pancakes{4}\nalmonds{200%g}\ndark chocolate{60%g}\nsalt\n"
    );
    assert_eq!(
        generated_names(&server).await,
        ["almonds", "dark chocolate", "flour", "milk", "salt"]
    );
}

#[tokio::test]
async fn without_dates_the_whole_menu_is_one_entry_as_before() {
    let server = start_server().await;

    let resp = server
        .post(
            "/api/shopping_list/add_menu",
            json!({ "path": "Plan.menu", "scale": 1.0 }),
        )
        .await;

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        server.list_file(),
        "./Plan.menu\n  ./Pancakes{2}\n  ./Soup\n  ./Soup{2}\n"
    );
}

#[tokio::test]
async fn an_extra_item_can_be_removed_on_its_own() {
    let server = start_server().await;
    server
        .post(
            "/api/shopping_list/add_menu",
            json!({ "path": "Plan.menu", "scale": 1.0, "dates": ["2026-10-07"] }),
        )
        .await;
    assert_eq!(
        server.list_file(),
        "almonds{50%g}\ndark chocolate{30%g}\nsalt\n"
    );

    let resp = server
        .post(
            "/api/shopping_list/remove_extra_item",
            json!({ "name": "almonds", "quantity": "50%g" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let resp = server
        .post(
            "/api/shopping_list/remove_extra_item",
            json!({ "name": "salt" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    assert_eq!(server.list_file(), "dark chocolate{30%g}\n");
}

#[tokio::test]
async fn days_must_be_dates() {
    let server = start_server().await;

    for dates in [json!([]), json!(["next monday"]), json!(["2026-02-30"])] {
        let resp = server
            .post(
                "/api/shopping_list/add_menu",
                json!({ "path": "Plan.menu", "scale": 1.0, "dates": dates }),
            )
            .await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "{dates}");
    }
    assert_eq!(server.list_file(), "");
}

#[tokio::test]
async fn a_day_with_nothing_on_it_adds_nothing() {
    let server = start_server().await;

    let resp = server
        .post(
            "/api/shopping_list/add_menu",
            json!({ "path": "Plan.menu", "scale": 1.0, "dates": ["2027-01-01"] }),
        )
        .await;

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(server.list_file(), "");
}
