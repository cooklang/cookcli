//! Typing items into the web shopping list: `recognize`, `add_items` and
//! `edit_extra_item` under `/api/shopping_list`.

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

    async fn get_json(&self, path: &str) -> Value {
        reqwest::Client::new()
            .get(self.url(path))
            .send()
            .await
            .expect("request")
            .json()
            .await
            .expect("json")
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
    std::fs::create_dir_all(recipes.join("config")).unwrap();
    std::fs::write(
        recipes.join("Cake.cook"),
        "Mix @sugar{200%g} and @flour{300%g}.\n",
    )
    .unwrap();
    std::fs::write(
        recipes.join("config/aisle.conf"),
        "[baking]\nsugar | caster sugar\nflour\n\n[veg]\ntomatoes\n",
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
async fn recognize_reads_lines_without_writing() {
    let server = start_server().await;
    let resp = server
        .post(
            "/api/shopping_list/recognize",
            json!({ "lines": ["Sugar 1.5kg", "Tomato 3", "  "] }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(
        body,
        json!([
            { "name": "sugar", "quantity": "1.5%kg", "chip": "1.5 kg", "category": "baking", "edit_text": "sugar 1.5 kg" },
            { "name": "tomatoes", "quantity": "3", "chip": "x3", "category": "veg", "edit_text": "tomatoes 3" },
            { "name": "", "quantity": null, "chip": null, "category": null, "edit_text": "" }
        ])
    );
    assert_eq!(server.list_file(), "");
}

#[tokio::test]
async fn recognize_splits_pasted_text() {
    let server = start_server().await;
    let body: Value = server
        .post(
            "/api/shopping_list/recognize",
            json!({ "text": "## Shop\n- Milk 2 l\n\n* [ ] bread\n" }),
        )
        .await
        .json()
        .await
        .unwrap();
    let names: Vec<&str> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Milk", "bread"]);
}

#[tokio::test]
async fn add_items_writes_merges_and_answers_rows() {
    let server = start_server().await;
    let resp = server
        .post(
            "/api/shopping_list/add_items",
            json!({ "lines": ["Sugar 1.5kg", "Caster sugar 1 kg", "Tomato 2", "Paper towels"] }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    let rows: Vec<&str> = body["added"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["row"].as_str().unwrap())
        .collect();
    assert_eq!(rows, ["sugar", "sugar", "tomatoes", "Paper towels"]);
    assert_eq!(
        server.list_file(),
        "sugar{1.5%kg}\ncaster sugar{1%kg}\ntomatoes{2}\nPaper towels\n"
    );

    server
        .post(
            "/api/shopping_list/add_items",
            json!({ "lines": ["sugar 1 kg"] }),
        )
        .await;
    assert!(
        server.list_file().starts_with("sugar{2.5%kg}\n"),
        "{}",
        server.list_file()
    );
}

#[tokio::test]
async fn add_items_refuses_nothing_to_add_and_too_much() {
    let server = start_server().await;
    let resp = server
        .post(
            "/api/shopping_list/add_items",
            json!({ "lines": ["  ", "{}"] }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let many: Vec<String> = (0..201).map(|i| format!("item {i}")).collect();
    let resp = server
        .post("/api/shopping_list/add_items", json!({ "lines": many }))
        .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let long = "x".repeat(501);
    let resp = server
        .post("/api/shopping_list/recognize", json!({ "lines": [long] }))
        .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(server.list_file(), "");
}

#[tokio::test]
async fn adding_unticks_the_row() {
    let server = start_server().await;
    server
        .post(
            "/api/shopping_list/add_items",
            json!({ "lines": ["sugar 1 kg"] }),
        )
        .await;
    server
        .post("/api/shopping_list/check", json!({ "name": "sugar" }))
        .await;
    let checked = server.get_json("/api/shopping_list/checked").await;
    assert!(
        checked.as_array().unwrap().iter().any(|n| n == "sugar"),
        "{checked}"
    );

    server
        .post(
            "/api/shopping_list/add_items",
            json!({ "lines": ["Sugar 500 g"] }),
        )
        .await;
    let checked = server.get_json("/api/shopping_list/checked").await;
    assert!(
        !checked.as_array().unwrap().iter().any(|n| n == "sugar"),
        "{checked}"
    );
}

#[tokio::test]
async fn extra_items_carry_their_edit_text() {
    let server = start_server().await;
    std::fs::write(
        server.recipes().join(".shopping-list"),
        "milk{2%l}\nparsley{2%bunches}\n",
    )
    .unwrap();
    let body = server.get_json("/api/shopping_list/extra_items").await;
    assert_eq!(
        body,
        json!([
            { "name": "milk", "quantity": "2%l", "edit_text": "milk 2 l" },
            { "name": "parsley", "quantity": "2%bunches", "edit_text": "parsley" }
        ])
    );
}

#[tokio::test]
async fn edit_replaces_keeps_an_unshown_quantity_and_reports_a_gone_item() {
    let server = start_server().await;
    std::fs::write(
        server.recipes().join(".shopping-list"),
        "milk{2%l}\nparsley{2%bunches}\n",
    )
    .unwrap();

    let resp = server
        .post(
            "/api/shopping_list/edit_extra_item",
            json!({ "name": "milk", "quantity": "2%l", "line": "oat milk 1 l" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(
        body,
        json!({ "name": "oat milk", "quantity": "1%l", "row": "oat milk" })
    );

    server
        .post(
            "/api/shopping_list/edit_extra_item",
            json!({ "name": "parsley", "quantity": "2%bunches", "line": "flat parsley" }),
        )
        .await;
    assert_eq!(
        server.list_file(),
        "oat milk{1%l}\nflat parsley{2%bunches}\n"
    );

    let resp = server
        .post(
            "/api/shopping_list/edit_extra_item",
            json!({ "name": "milk", "quantity": "2%l", "line": "milk 3 l" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let resp = server
        .post(
            "/api/shopping_list/edit_extra_item",
            json!({ "name": "oat milk", "quantity": "1%l", "line": "  " }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn an_edit_folded_into_another_line_answers_that_line() {
    let server = start_server().await;
    std::fs::write(
        server.recipes().join(".shopping-list"),
        "sugar{1%kg}\nflour{2%kg}\n",
    )
    .unwrap();
    let resp = server
        .post(
            "/api/shopping_list/edit_extra_item",
            json!({ "name": "flour", "quantity": "2%kg", "line": "Sugar 2 kg" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(
        body,
        json!({ "name": "sugar", "quantity": "3%kg", "row": "sugar" })
    );
    assert_eq!(server.list_file(), "sugar{3%kg}\n");
}

/// The names of every row of a generated list.
fn row_names(list: &Value) -> Vec<String> {
    list["categories"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|category| category["items"].as_array().unwrap().clone())
        .map(|item| item["name"].as_str().unwrap().to_string())
        .collect()
}

/// A free-hand name is listed as written: `#`, `~` and `@` are no Cooklang
/// markup on the shopping list.
#[tokio::test]
async fn free_hand_names_that_look_like_markup_are_listed_as_written() {
    let server = start_server().await;
    let resp = server
        .post(
            "/api/shopping_list",
            json!([
                { "name": "#1 flour", "quantity": "2" },
                { "name": "~ramen" },
                { "name": "@milk", "quantity": "1%l" }
            ]),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let list: Value = resp.json().await.unwrap();
    let mut names = row_names(&list);
    names.sort();
    assert_eq!(names, ["#1 flour", "@milk", "~ramen"], "{list}");
}

/// Their ticks survive a compaction, as the names are on the list, and
/// unticking takes them off again.
#[tokio::test]
async fn free_hand_names_that_look_like_markup_tick_and_untick() {
    let server = start_server().await;
    std::fs::write(
        server.recipes().join(".shopping-list"),
        "#1 flour{2}\n~ramen\n@milk{1%l}\n",
    )
    .unwrap();
    let names = ["#1 flour", "~ramen", "@milk"];
    for name in names {
        let resp = server
            .post("/api/shopping_list/check", json!({ "name": name }))
            .await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
    let resp = server.post("/api/shopping_list/compact", json!({})).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let checked = server.get_json("/api/shopping_list/checked").await;
    for name in names {
        assert!(
            checked.as_array().unwrap().iter().any(|n| n == name),
            "{name} still ticked: {checked}"
        );
    }

    for name in names {
        server
            .post("/api/shopping_list/uncheck", json!({ "name": name }))
            .await;
    }
    let checked = server.get_json("/api/shopping_list/checked").await;
    assert_eq!(checked, json!([]));
}

/// A free-hand item without a name is refused, as no row could show it.
#[tokio::test]
async fn a_free_hand_item_without_a_name_is_refused() {
    let server = start_server().await;
    for name in ["", "  "] {
        let resp = server
            .post(
                "/api/shopping_list",
                json!([{ "name": name, "quantity": "2" }]),
            )
            .await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "{name:?}");
    }
}

/// Typed in, they land on rows of their own names, and adding one again
/// unticks its row.
#[tokio::test]
async fn adding_unticks_a_row_whose_name_looks_like_markup() {
    let server = start_server().await;
    let lines = json!({ "lines": ["#1 flour 2", "~ramen", "@milk 1 l"] });
    let resp = server
        .post("/api/shopping_list/add_items", lines.clone())
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(server.list_file(), "#1 flour{2}\n~ramen\n@milk{1%l}\n");

    let names = ["#1 flour", "~ramen", "@milk"];
    for name in names {
        server
            .post("/api/shopping_list/check", json!({ "name": name }))
            .await;
    }
    let checked = server.get_json("/api/shopping_list/checked").await;
    assert_eq!(checked.as_array().unwrap().len(), 3, "{checked}");
    server.post("/api/shopping_list/add_items", lines).await;
    let checked = server.get_json("/api/shopping_list/checked").await;
    assert_eq!(checked, json!([]));
    assert_eq!(server.list_file(), "#1 flour{4}\n~ramen\n@milk{2%l}\n");
}
