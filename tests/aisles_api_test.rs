//! End-to-end tests for managing `aisle.conf` from the web UI.
//!
//! Changes edit only the lines concerned, so the rest of the file — its
//! comments and the blank lines that group an aisle — comes back as it was.
//! A change made against an older revision of the file is refused, and so is
//! text the aisle parser would not read. A recipe directory without an aisle
//! file can get one, and the shopping list uses it straight away.

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

const AISLES: &str = "\
// Front of the store first

[fruit and veg]
apples
onion | onions // red or white

leek

[dairy]
milk
";

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

    fn aisle_file(&self) -> PathBuf {
        self.dir.path().join("recipes/config/aisle.conf")
    }

    fn aisle_text(&self) -> String {
        std::fs::read_to_string(self.aisle_file()).expect("read aisle.conf")
    }

    /// The answer as sent, for checking what it does not say.
    async fn text(&self, method: reqwest::Method, path: &str, body: Value) -> (StatusCode, String) {
        let resp = reqwest::Client::new()
            .request(method, self.url(path))
            .json(&body)
            .send()
            .await
            .expect("request");
        let status = resp.status();
        (status, resp.text().await.expect("text body"))
    }

    async fn get(&self, path: &str) -> (StatusCode, Value) {
        let resp = reqwest::get(self.url(path)).await.expect("GET");
        let status = resp.status();
        (status, resp.json().await.expect("JSON body"))
    }

    async fn send(&self, method: reqwest::Method, path: &str, body: Value) -> (StatusCode, Value) {
        let resp = reqwest::Client::new()
            .request(method, self.url(path))
            .json(&body)
            .send()
            .await
            .expect("request");
        let status = resp.status();
        (status, resp.json().await.unwrap_or(Value::Null))
    }

    async fn change(&self, body: Value) -> (StatusCode, Value) {
        self.send(reqwest::Method::POST, "/api/aisles/changes", body)
            .await
    }

    /// The aisle the shopping list puts each of the Omelette's ingredients
    /// in.
    async fn list_categories(&self) -> Vec<(String, String)> {
        let (status, list) = self
            .send(
                reqwest::Method::POST,
                "/api/shopping_list",
                json!([{ "recipe": "Omelette.cook", "scale": 1.0 }]),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{list}");
        let mut found = Vec::new();
        for category in list["categories"].as_array().unwrap() {
            for item in category["items"].as_array().unwrap() {
                found.push((
                    item["name"].as_str().unwrap().to_string(),
                    category["category"].as_str().unwrap().to_string(),
                ));
            }
        }
        found.sort();
        found
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

/// `free_port` only reserves a port long enough to learn its number, so with
/// several tests booting servers at once another one can claim it first. The
/// server exits 1 on a bound port, so retry with a fresh one.
async fn start_server(aisles: Option<&str>) -> ServerGuard {
    start_server_with(aisles, None).await
}

/// As [`start_server`], with `global` as the aisle file of the isolated
/// global config directory, which the server falls back on when the recipe
/// directory has none.
async fn start_server_with(aisles: Option<&str>, global: Option<&str>) -> ServerGuard {
    for _ in 0..5 {
        if let Some(server) = try_start_server(aisles, global).await {
            return server;
        }
    }
    panic!("could not start cook server on a free port after 5 attempts");
}

async fn try_start_server(aisles: Option<&str>, global: Option<&str>) -> Option<ServerGuard> {
    let dir = TempDir::new().expect("temp dir");
    let recipes = dir.path().join("recipes");
    std::fs::create_dir_all(&recipes).unwrap();
    std::fs::write(
        recipes.join("Omelette.cook"),
        "Beat @eggs{3} with @milk{50%ml}, then add @chives{1%tbsp}.\n",
    )
    .unwrap();
    if let Some(aisles) = aisles {
        std::fs::create_dir_all(recipes.join("config")).unwrap();
        std::fs::write(recipes.join("config/aisle.conf"), aisles).unwrap();
    }
    if let Some(global) = global {
        // Where `common::with_isolated_config` points `COOK_CONFIG_DIR`.
        let config = dir.path().join(".cook-config");
        std::fs::create_dir_all(&config).unwrap();
        std::fs::write(config.join("aisle.conf"), global).unwrap();
    }

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
async fn the_aisles_are_read_in_file_order() {
    let server = start_server(Some(AISLES)).await;

    let (status, aisles) = server.get("/api/aisles").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(aisles["configured"], true);
    assert_eq!(
        aisles["aisles"],
        json!([
            {
                "name": "fruit and veg",
                "ingredients": [
                    { "names": ["apples"] },
                    { "names": ["onion", "onions"] },
                    { "names": ["leek"] },
                ],
            },
            { "name": "dairy", "ingredients": [{ "names": ["milk"] }] },
        ])
    );
    assert_eq!(aisles["warnings"], json!([]));

    let page = reqwest::get(server.url("/aisles")).await.unwrap();
    assert_eq!(page.status(), StatusCode::OK);
    assert!(page.text().await.unwrap().contains("fruit and veg"));
}

#[tokio::test]
async fn changes_touch_only_their_own_lines() {
    let server = start_server(Some(AISLES)).await;
    let (_, aisles) = server.get("/api/aisles").await;
    let revision = aisles["revision"].as_str().unwrap().to_string();

    let (status, after) = server
        .change(json!({
            "revision": revision,
            "action": "add_ingredient",
            "aisle": "dairy",
            "names": ["eggs", "egg"],
        }))
        .await;
    assert_eq!(status, StatusCode::OK, "{after}");
    assert_ne!(after["revision"], json!(revision));
    assert_eq!(
        server.aisle_text(),
        AISLES.replace("milk\n", "milk\neggs | egg\n")
    );

    // Renaming and moving keep the comment on the line.
    let (status, _) = server
        .change(json!({
            "action": "update_ingredient",
            "name": "ONIONS",
            "names": ["onion", "onions", "shallot"],
            "aisle": "dairy",
        }))
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = server
        .change(json!({ "action": "move_aisle", "name": "dairy", "position": 0 }))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        server.aisle_text(),
        "// Front of the store first\n\n\
         [dairy]\nmilk\neggs | egg\nonion | onions | shallot // red or white\n\n\
         [fruit and veg]\napples\n\nleek\n"
    );

    // The shopping list follows at once, with no restart.
    assert_eq!(
        server.list_categories().await,
        [
            ("chives".to_string(), "other".to_string()),
            ("eggs".to_string(), "dairy".to_string()),
            ("milk".to_string(), "dairy".to_string()),
        ]
    );
}

#[tokio::test]
async fn a_change_against_an_old_revision_is_refused() {
    let server = start_server(Some(AISLES)).await;
    let (_, aisles) = server.get("/api/aisles").await;
    let stale = aisles["revision"].clone();

    let (status, _) = server
        .change(json!({ "action": "add_aisle", "name": "frozen" }))
        .await;
    assert_eq!(status, StatusCode::OK);
    let current = server.aisle_text();

    let (status, body) = server
        .change(json!({ "revision": stale, "action": "remove_aisle", "name": "dairy" }))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_ne!(body["revision"], stale);
    assert_eq!(server.aisle_text(), current);

    let (status, _) = server
        .send(
            reqwest::Method::PUT,
            "/api/aisles/raw",
            json!({ "revision": stale, "content": "[x]\n" }),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(server.aisle_text(), current);
}

#[tokio::test]
async fn names_already_used_or_unreadable_are_refused() {
    let server = start_server(Some(AISLES)).await;

    for (body, expected) in [
        // Ingredient names are matched ignoring case, as the list does.
        (
            json!({ "action": "add_ingredient", "aisle": "dairy", "names": ["Apples"] }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "action": "add_aisle", "name": "Dairy" }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "action": "add_ingredient", "aisle": "dairy", "names": ["cream // fresh"] }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "action": "add_ingredient", "aisle": "frozen", "names": ["peas"] }),
            StatusCode::NOT_FOUND,
        ),
        (
            json!({ "action": "remove_ingredient", "name": "caviar" }),
            StatusCode::NOT_FOUND,
        ),
    ] {
        let (status, answer) = server.change(body.clone()).await;
        assert_eq!(status, expected, "{body} → {answer}");
        assert!(answer["error"].is_string(), "{answer}");
    }
    assert_eq!(server.aisle_text(), AISLES);
}

#[tokio::test]
async fn the_text_is_saved_only_when_the_parser_reads_it() {
    let server = start_server(Some(AISLES)).await;

    let (status, raw) = server.get("/api/aisles/raw").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(raw["content"], AISLES);

    let (status, body) = server
        .send(
            reqwest::Method::PUT,
            "/api/aisles/raw",
            json!({ "content": "[a]\nx\n\n[b]\nx\n" }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["error"].as_str().unwrap().starts_with("Line 5:"),
        "{body}"
    );
    assert_eq!(server.aisle_text(), AISLES);

    let text = "[pantry] // dry goods\nflour\n";
    let (status, aisles) = server
        .send(
            reqwest::Method::PUT,
            "/api/aisles/raw",
            json!({ "content": text, "revision": raw["revision"] }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(aisles["aisles"][0]["name"], "pantry");
    assert_eq!(server.aisle_text(), text);
}

#[tokio::test]
async fn an_aisle_file_can_be_started_from_nothing() {
    let server = start_server(None).await;

    let (status, aisles) = server.get("/api/aisles").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(aisles["configured"], false);
    let (status, _) = server
        .change(json!({ "action": "add_aisle", "name": "dairy" }))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(!server.aisle_file().exists());

    let (status, created) = server
        .send(reqwest::Method::POST, "/api/aisles", json!(null))
        .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["configured"], true);
    assert_eq!(server.aisle_text(), "");
    let (status, _) = server
        .send(reqwest::Method::POST, "/api/aisles", json!(null))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);

    for change in [
        json!({ "action": "add_aisle", "name": "dairy" }),
        json!({ "action": "add_ingredient", "aisle": "dairy", "names": ["eggs"] }),
    ] {
        let (status, body) = server.change(change).await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }
    assert_eq!(server.aisle_text(), "[dairy]\neggs\n");

    // The server started without an aisle file, and uses this one anyway.
    assert_eq!(
        server.list_categories().await,
        [
            ("chives".to_string(), "other".to_string()),
            ("eggs".to_string(), "dairy".to_string()),
            ("milk".to_string(), "other".to_string()),
        ]
    );
}

#[tokio::test]
async fn several_changes_are_made_together_or_not_at_all() {
    let server = start_server(Some(AISLES)).await;

    let (status, body) = server
        .change(json!({ "changes": [
            { "action": "add_aisle", "name": "frozen" },
            { "action": "add_ingredient", "aisle": "frozen", "names": ["peas"] },
            { "action": "add_ingredient", "aisle": "dairy", "names": ["Apples"] },
        ] }))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["error"],
        "Change 3: \"Apples\" is already in the \"fruit and veg\" aisle"
    );
    assert_eq!(server.aisle_text(), AISLES);

    // A new aisle and what goes in it, as the pages send them.
    let (status, body) = server
        .change(json!({ "changes": [
            { "action": "add_aisle", "name": "herbs" },
            { "action": "add_ingredient", "aisle": "herbs", "names": ["chives"] },
        ] }))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(server.aisle_text(), format!("{AISLES}\n[herbs]\nchives\n"));
    assert!(server
        .list_categories()
        .await
        .contains(&("chives".to_string(), "herbs".to_string())));

    let (status, _) = server.change(json!({ "changes": [] })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn grouped_names_show_on_the_list_as_the_main_one() {
    let server = start_server(Some(AISLES)).await;

    let (status, body) = server
        .change(json!({ "action": "add_names", "name": "leek", "names": ["chives"] }))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        server.aisle_text(),
        AISLES.replace("\nleek\n", "\nleek | chives\n")
    );
    assert!(server
        .list_categories()
        .await
        .contains(&("leek".to_string(), "fruit and veg".to_string())));

    let (status, body) = server
        .change(json!({
            "action": "merge_ingredients",
            "names": ["chives", "onions"],
            "main": "onion",
        }))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        server.aisle_text(),
        AISLES.replace("\nleek\n", "\n").replace(
            "onion | onions // red or white",
            "onion | onions | leek | chives // red or white"
        )
    );
    assert_eq!(
        body["aisles"][0]["ingredients"][1]["names"],
        json!(["onion", "onions", "leek", "chives"])
    );
    let list = server.list_categories().await;
    assert!(
        list.contains(&("onion".to_string(), "fruit and veg".to_string())),
        "{list:?}"
    );
    assert!(!list.iter().any(|(name, _)| name == "chives"), "{list:?}");
}

#[tokio::test]
async fn uncategorized_lists_what_no_aisle_names() {
    let server = start_server(Some(AISLES)).await;

    let names = |body: &Value| -> Vec<String> {
        body["ingredients"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["name"].as_str().unwrap().to_string())
            .collect()
    };

    let (status, body) = server.get("/api/aisles/uncategorized").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(names(&body), ["chives", "eggs"]);
    assert_eq!(
        body["ingredients"][0]["recipes"],
        json!([{ "path": "Omelette.cook", "link": "Omelette" }])
    );

    server
        .change(json!({ "action": "add_ingredient", "aisle": "dairy", "names": ["eggs"] }))
        .await;
    let (_, body) = server.get("/api/aisles/uncategorized").await;
    assert_eq!(names(&body), ["chives"]);
}

#[tokio::test]
async fn answers_never_name_the_server_directories() {
    let server = start_server(Some(AISLES)).await;
    let root = server.dir.path().to_str().unwrap().to_string();

    let (_, aisles) = server.get("/api/aisles").await;
    assert_eq!(aisles["file"], "config/aisle.conf");
    assert_eq!(aisles["shared"], false);
    assert!(aisles.get("path").is_none(), "{aisles}");

    for (method, path, body, status) in [
        (
            reqwest::Method::GET,
            "/api/aisles",
            json!(null),
            StatusCode::OK,
        ),
        (
            reqwest::Method::GET,
            "/api/aisles/raw",
            json!(null),
            StatusCode::OK,
        ),
        (reqwest::Method::GET, "/aisles", json!(null), StatusCode::OK),
        (
            reqwest::Method::POST,
            "/api/aisles",
            json!(null),
            StatusCode::CONFLICT,
        ),
        (
            reqwest::Method::POST,
            "/api/aisles/changes",
            json!({ "action": "add_ingredient", "aisle": "dairy", "names": ["eggs"] }),
            StatusCode::OK,
        ),
        (
            reqwest::Method::POST,
            "/api/aisles/changes",
            json!({ "action": "remove_ingredient", "name": "caviar" }),
            StatusCode::NOT_FOUND,
        ),
    ] {
        let (got, text) = server.text(method.clone(), path, body).await;
        assert_eq!(got, status, "{method} {path}: {text}");
        assert!(
            !text.contains(&root),
            "{method} {path} names {root}: {text}"
        );
    }
}

#[tokio::test]
async fn the_global_aisle_file_is_shared_but_not_located() {
    let server = start_server_with(None, Some("[pantry]\nrice\n")).await;
    let root = server.dir.path().to_str().unwrap().to_string();

    let (status, text) = server
        .text(reqwest::Method::GET, "/api/aisles", json!(null))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!text.contains(&root), "{text}");
    let aisles: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(aisles["configured"], true);
    assert_eq!(aisles["file"], Value::Null);
    assert_eq!(aisles["shared"], true);
    assert_eq!(aisles["aisles"][0]["name"], "pantry");
}
