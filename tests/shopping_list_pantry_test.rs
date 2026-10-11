//! The web shopping list says when it could not use the pantry file.
//!
//! A `pantry.conf` that does not parse — an aisle.conf-style `//` comment is
//! enough — used to give a list made without the pantry, with nothing on the
//! page or in the server log to say so (#690). `POST /api/shopping_list` now
//! carries the reason as `pantry_error`, worded as the pantry page words it.

#![cfg(feature = "server")]

use serde_json::{json, Value};
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

#[path = "common/mod.rs"]
mod common;

/// Kills the spawned server when the test ends, pass or panic.
struct ServerGuard {
    child: Child,
    port: u16,
    _dir: TempDir,
}

impl ServerGuard {
    /// The shopping list for the one recipe in the directory.
    async fn list(&self) -> Value {
        let response = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{}/api/shopping_list", self.port))
            .json(&json!([{ "recipe": "Pancakes.cook", "scale": 1 }]))
            .send()
            .await
            .expect("request");
        assert!(response.status().is_success(), "{}", response.status());
        response.json().await.expect("json")
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
    std::fs::write(
        recipes.join("Pancakes.cook"),
        "Mix @milk{1%l}, @flour{200%g} and @eggs{2}.\n",
    )
    .unwrap();
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

    let mut guard = ServerGuard {
        child,
        port,
        _dir: dir,
    };

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/api/recipes");
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

/// The names on the list, whatever their aisle.
fn names(list: &Value) -> Vec<String> {
    list["categories"]
        .as_array()
        .expect("categories")
        .iter()
        .flat_map(|category| category["items"].as_array().expect("items"))
        .map(|item| item["name"].as_str().expect("name").to_string())
        .collect()
}

#[tokio::test]
async fn a_pantry_that_reads_is_taken_off_and_no_error_is_sent() {
    let server = start_server("# my pantry\n[dairy]\nmilk = \"1%l\"\n").await;
    let list = server.list().await;

    assert_eq!(list["pantry_error"], Value::Null, "{list}");
    assert_eq!(list["pantry_items"], json!(["milk"]), "{list}");
    assert!(!names(&list).contains(&"milk".to_string()), "{list}");
}

#[tokio::test]
async fn a_pantry_that_does_not_parse_is_reported() {
    let server = start_server("// my pantry\n[dairy]\nmilk = \"1%l\"\n").await;
    let list = server.list().await;

    // The list is still made, without the pantry.
    assert_eq!(list["pantry_items"], json!([]), "{list}");
    assert!(names(&list).contains(&"milk".to_string()), "{list}");

    // And says why, as the pantry page does: the file and the parser's
    // complaint with its line, the file named relative to the recipes
    // (with the platform's separator, `config\pantry.conf` on Windows).
    let error = list["pantry_error"].as_str().expect("pantry_error");
    let file = std::path::Path::new("config").join("pantry.conf");
    assert!(error.contains(&*file.to_string_lossy()), "{error}");
    assert!(error.contains("line 1"), "{error}");
    assert!(error.contains("// my pantry"), "{error}");
    assert!(
        !error.contains(&*std::env::temp_dir().to_string_lossy()),
        "the server's folders stay private: {error}"
    );
}
