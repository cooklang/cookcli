//! End-to-end tests for changing `pantry.conf` from the web UI.
//!
//! Every change edits only the entry concerned, so the comments and layout
//! someone wrote by hand — or on the page's Text tab — survive it. A section
//! can be renamed in place, and the whole file can be read and replaced as
//! text, once it reads as a pantry and was not changed in the meantime.

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

const PANTRY: &str = "\
# What is at home
water = \"unlim\"

# cold things
[fridge]
milk = \"1%l\" # semi-skimmed
eggs = { quantity = \"6\", expire = \"2027-01-01\" }

[shelf]
rice = \"1%kg\"
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

    fn pantry_file(&self) -> PathBuf {
        self.dir.path().join("recipes/config/pantry.conf")
    }

    fn pantry_text(&self) -> String {
        std::fs::read_to_string(self.pantry_file()).expect("read pantry.conf")
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

    async fn rename(&self, section: &str, new_name: &str) -> (StatusCode, Value) {
        self.send(
            reqwest::Method::POST,
            "/api/pantry/rename",
            json!({ "section": section, "new_name": new_name }),
        )
        .await
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
async fn start_server(pantry: &str) -> ServerGuard {
    for _ in 0..5 {
        if let Some(server) = try_start_server(pantry).await {
            return server;
        }
    }
    panic!("could not start cook server on a free port after 5 attempts");
}

async fn try_start_server(pantry: &str) -> Option<ServerGuard> {
    let dir = TempDir::new().expect("temp dir");
    let recipes = dir.path().join("recipes");
    std::fs::create_dir_all(recipes.join("config")).unwrap();
    std::fs::write(recipes.join("config/pantry.conf"), pantry).unwrap();

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
async fn a_renamed_section_keeps_its_place_items_and_comments() {
    let server = start_server(PANTRY).await;

    let (status, body) = server.rename("fridge", " Fridge door ").await;
    assert_eq!(status, StatusCode::OK, "{body}");

    assert_eq!(
        server.pantry_text(),
        PANTRY.replace("[fridge]", "[\"Fridge door\"]")
    );
    let (_, pantry) = server.get("/api/pantry").await;
    assert_eq!(pantry["Fridge door"][0]["name"], "milk", "{pantry}");
}

#[tokio::test]
async fn a_rename_that_cannot_be_made_is_refused_and_writes_nothing() {
    let server = start_server(PANTRY).await;

    for (section, new_name, expected) in [
        ("fridge", "Shelf", StatusCode::BAD_REQUEST),
        ("fridge", "WATER", StatusCode::BAD_REQUEST),
        ("fridge", "  ", StatusCode::BAD_REQUEST),
        ("general", "top", StatusCode::BAD_REQUEST),
        ("fridge", "general", StatusCode::BAD_REQUEST),
        ("freezer", "cold", StatusCode::NOT_FOUND),
    ] {
        let (status, body) = server.rename(section, new_name).await;
        assert_eq!(status, expected, "{section} → {new_name}: {body}");
        assert!(body["error"].is_string(), "{body}");
    }
    assert_eq!(server.pantry_text(), PANTRY, "nothing may be written");
}

/// What the page changes item by item leaves the rest of the file as it was:
/// the comments, the top-level item, the short and long forms.
#[tokio::test]
async fn item_changes_keep_the_rest_of_the_file() {
    let server = start_server(PANTRY).await;

    let (status, body) = server
        .send(
            reqwest::Method::POST,
            "/api/pantry/add",
            json!({ "section": "shelf", "name": "pasta", "quantity": "500%g" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = server
        .send(
            reqwest::Method::PUT,
            "/api/pantry/fridge/milk",
            json!({ "quantity": "2%l" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = server
        .send(reqwest::Method::DELETE, "/api/pantry/shelf/rice", json!({}))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    assert_eq!(
        server.pantry_text(),
        PANTRY
            .replace("milk = \"1%l\"", "milk = \"2%l\"")
            .replace("rice = \"1%kg\"\n", "pasta = \"500%g\"\n")
    );
}

#[tokio::test]
async fn an_item_change_that_cannot_be_made_is_refused() {
    let server = start_server(PANTRY).await;

    let (status, body) = server
        .send(
            reqwest::Method::POST,
            "/api/pantry/add",
            json!({ "section": "fridge", "name": "milk" }),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "an item already there: {body}"
    );
    let (status, body) = server
        .send(
            reqwest::Method::PUT,
            "/api/pantry/fridge/butter",
            json!({ "quantity": "1" }),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    let (status, body) = server
        .send(
            reqwest::Method::DELETE,
            "/api/pantry/fridge/butter",
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");

    // Saving the edit dialog untouched sends nothing to change.
    let (status, body) = server
        .send(reqwest::Method::PUT, "/api/pantry/fridge/milk", json!({}))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    assert_eq!(server.pantry_text(), PANTRY, "nothing may be written");
}

/// A pantry the parser cannot read is reported, not replaced by a pantry
/// holding only the new item.
#[tokio::test]
async fn an_unreadable_pantry_is_not_replaced_by_an_add() {
    let broken = "[fridge\nmilk = \"1%l\"\n";
    let server = start_server(broken).await;

    let (status, body) = server
        .send(
            reqwest::Method::POST,
            "/api/pantry/add",
            json!({ "section": "fridge", "name": "eggs" }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(server.pantry_text(), broken);
}

#[tokio::test]
async fn the_text_round_trips_and_names_no_path() {
    let server = start_server(PANTRY).await;

    let (status, raw) = server.get("/api/pantry/raw").await;
    assert_eq!(status, StatusCode::OK, "{raw}");
    assert_eq!(raw["content"], PANTRY);
    assert!(raw.get("path").is_none(), "no absolute path: {raw}");

    let text = format!("{PANTRY}\n# added by hand\n[freezer]\npeas = \"1%kg\"\n");
    let (status, saved) = server
        .send(
            reqwest::Method::PUT,
            "/api/pantry/raw",
            json!({ "content": text, "revision": raw["revision"] }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(server.pantry_text(), text);
    assert_eq!(saved["content"], text.as_str());
    assert_ne!(saved["revision"], raw["revision"]);

    let (_, pantry) = server.get("/api/pantry").await;
    assert_eq!(pantry["freezer"][0]["name"], "peas", "{pantry}");
}

#[tokio::test]
async fn the_text_is_saved_only_when_it_reads_as_a_pantry() {
    let server = start_server(PANTRY).await;

    let (status, body) = server
        .send(
            reqwest::Method::PUT,
            "/api/pantry/raw",
            json!({ "content": "[fridge]\nmilk = \"1%l\"\nmilk = \"2%l\"\n" }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    let error = body["error"].as_str().unwrap();
    assert!(error.contains("line 3"), "{error}");
    assert!(
        !error.contains(&*server.dir.path().to_string_lossy()),
        "no absolute path: {error}"
    );
    assert_eq!(server.pantry_text(), PANTRY);
}

#[tokio::test]
async fn text_edited_from_an_old_revision_is_refused() {
    let server = start_server(PANTRY).await;
    let (_, raw) = server.get("/api/pantry/raw").await;

    // Someone else changes the file in the meantime.
    let (status, _) = server.rename("shelf", "cupboard").await;
    assert_eq!(status, StatusCode::OK);
    let changed = server.pantry_text();

    let (status, body) = server
        .send(
            reqwest::Method::PUT,
            "/api/pantry/raw",
            json!({ "content": "[x]\ny = \"1\"\n", "revision": raw["revision"] }),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    let (_, now) = server.get("/api/pantry/raw").await;
    assert_eq!(body["revision"], now["revision"], "the current revision");
    assert_eq!(server.pantry_text(), changed, "nothing may be written");
}
