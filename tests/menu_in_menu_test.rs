//! A menu used as a meal of another menu (#385): `- @./Brunches/Sunday.menu{}`.
//!
//! The reference names a `.menu` the way a meal names a recipe. The menu page
//! links it to that menu, the menu API reports it as one, and the shopping
//! list, `cook shopping-list` and `cook doctor` follow it as they follow a
//! recipe — its free-hand ingredients and its own recipes, at its factor.

#![cfg(feature = "server")]

#[path = "common/mod.rs"]
mod common;

use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

/// `Brunches/Sunday.menu` declares `servings: 2`, so `{4%servings}` on it is
/// ×2; `{2}` is ×2 too. It holds eggs, toast for 4 and a litre of juice.
/// `Loop A.menu` and `Loop B.menu` reference each other.
fn write_fixture(root: &Path) {
    std::fs::create_dir_all(root.join("Brunches")).unwrap();
    std::fs::write(
        root.join("Toast.cook"),
        "---\nservings: 2\n---\n\nToast @bread{2%slices}.\n",
    )
    .unwrap();
    std::fs::write(root.join("Eggs.cook"), "Fry @eggs{2}.\n").unwrap();
    std::fs::write(
        root.join("Brunches/Sunday.menu"),
        "---\nservings: 2\n---\n\n== Sunday ==\n\nBrunch: \\\n\
         - @./Eggs{} \\\n- @./Toast{4%servings} \\\n- @juice{1%l}\n",
    )
    .unwrap();
    // The same menu twice: once named with its extension, once without.
    std::fs::write(
        root.join("Week.menu"),
        "== Day 1 ==\n\nDinner: \\\n\
         - @./Brunches/Sunday.menu{4%servings} \\\n- @./Brunches/Sunday{2}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("Loop A.menu"),
        "- @./Loop B.menu{} \\\n- @./Eggs{}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("Loop B.menu"),
        "- @./Loop A.menu{} \\\n- @rice{1%cup}\n",
    )
    .unwrap();
}

/// Kills the spawned server when the test ends, pass or panic.
struct ServerGuard {
    child: Child,
    port: u16,
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

/// `free_port` only reserves a port long enough to learn its number, so retry
/// with a fresh one if another test claims it first.
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
    write_fixture(&recipes);

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
    for _ in 0..200 {
        if guard.child.try_wait().expect("poll server").is_some() {
            return None;
        }
        if let Ok(resp) = client.get(guard.url("/api/menus")).send().await {
            if resp.status().is_success() {
                return Some(guard);
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("cook server on port {port} never became ready");
}

async fn get_json(server: &ServerGuard, path: &str) -> Value {
    reqwest::get(server.url(path))
        .await
        .expect("request")
        .error_for_status()
        .expect("success")
        .json()
        .await
        .expect("json")
}

/// Every meal item of a menu, from the menu API.
async fn menu_items(server: &ServerGuard, menu: &str) -> Vec<Value> {
    let body = get_json(server, &format!("/api/menus/{menu}")).await;
    body["sections"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|section| section["meals"].as_array().unwrap().clone())
        .flat_map(|meal| meal["items"].as_array().unwrap().clone())
        .collect()
}

/// Puts `menu` on the shopping list, then asks for the list the way the
/// shopping list page does: the menu's own free-hand ingredients, then each
/// entry under it with its references. Name → (amount, unit).
async fn list_for_menu(server: &ServerGuard, menu: &str) -> BTreeMap<String, (f64, String)> {
    let client = reqwest::Client::new();
    let resp = client
        .post(server.url("/api/shopping_list/add_menu"))
        .json(&json!({ "path": menu, "scale": 1.0 }))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success(), "add_menu: {}", resp.status());

    let items = get_json(server, "/api/shopping_list/items").await;
    let mut requests = Vec::new();
    for item in items.as_array().unwrap() {
        requests.push(
            json!({ "recipe": item["path"], "scale": item["scale"], "included_references": [] }),
        );
        for recipe in item["recipes"].as_array().unwrap() {
            requests.push(json!({
                "recipe": recipe["path"],
                "scale": recipe["scale"],
                "included_references": recipe["included_references"],
            }));
        }
    }
    let list: Value = client
        .post(server.url("/api/shopping_list"))
        .json(&requests)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let mut totals = BTreeMap::new();
    for category in list["categories"].as_array().unwrap() {
        for item in category["items"].as_array().unwrap() {
            let quantity = &item["quantities"][0];
            totals.insert(
                item["name"].as_str().unwrap().to_string(),
                (
                    quantity["value"]["value"]["value"].as_f64().unwrap(),
                    quantity["unit"].as_str().unwrap_or("").to_string(),
                ),
            );
        }
    }
    totals
}

fn amount(value: f64, unit: &str) -> (f64, String) {
    (value, unit.to_string())
}

#[tokio::test]
async fn the_menu_api_reports_a_menu_reference_as_a_menu() {
    let server = start_server().await;

    let items = menu_items(&server, "Week.menu").await;

    assert_eq!(items.len(), 2);
    for item in &items {
        assert_eq!(item["kind"], "recipe_reference");
        assert_eq!(item["menu"], true, "{item}");
        // With or without `.menu` in the reference, the file is the menu.
        assert_eq!(item["path"], "./Brunches/Sunday.menu", "{item}");
        // `{4%servings}` against the menu's `servings: 2`, and `{2}`.
        assert_eq!(item["scale"], 2.0, "{item}");
    }

    // A recipe reference reads as it always has, with no `menu` key.
    let items = menu_items(&server, "Brunches/Sunday.menu").await;
    let eggs = &items[0];
    assert_eq!(eggs["path"], "./Eggs.cook");
    assert!(eggs.get("menu").is_none(), "{eggs}");
}

#[tokio::test]
async fn the_menu_page_links_a_menu_reference_to_the_menu() {
    let server = start_server().await;

    let html = reqwest::get(server.url("/recipe/Week.menu"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    // Both spellings link to the menu's own page, by its servings.
    assert_eq!(
        html.matches(r#"href="/recipe/Brunches/Sunday.menu?servings=4""#)
            .count(),
        2,
        "{html}"
    );
    assert_eq!(html.matches("Brunches › Sunday\n").count(), 2, "{html}");
    assert!(
        !html.contains("Sunday.menu\n"),
        "the label drops the extension"
    );
    assert!(html.contains(r#"<span class="tag">Menu</span>"#), "{html}");
}

#[tokio::test]
async fn the_shopping_list_counts_a_menu_used_as_a_meal() {
    let server = start_server().await;

    let totals = list_for_menu(&server, "Week.menu").await;

    // Sunday twice at ×2: its juice, its eggs, and its toast for 4 servings
    // (8 slices) at ×2.
    assert_eq!(totals["juice"], amount(4.0, "l"));
    assert_eq!(totals["eggs"], amount(8.0, ""));
    assert_eq!(totals["bread"], amount(16.0, "slices"));
}

#[tokio::test]
async fn menus_that_reference_each_other_are_counted_once() {
    let server = start_server().await;

    let totals = list_for_menu(&server, "Loop A.menu").await;

    // As `cook shopping-list` counts them: B's rice, A's eggs once.
    assert_eq!(totals["rice"], amount(1.0, "cup"));
    assert_eq!(totals["eggs"], amount(2.0, ""));

    // B is stored under A without its reference back to A.
    let items = get_json(&server, "/api/shopping_list/items").await;
    let loop_b = &items[0]["recipes"][0];
    assert_eq!(loop_b["path"], "Loop B.menu");
    assert_eq!(loop_b["included_references"], json!([]));
}

fn cook(dir: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin("cook"));
    cmd.args(args).arg("--base-path").arg(dir.join("recipes"));
    common::with_isolated_config(&mut cmd, dir)
        .output()
        .expect("run cook")
}

#[test]
fn cook_shopping_list_follows_a_menu_used_as_a_meal() {
    let dir = TempDir::new().unwrap();
    write_fixture(&dir.path().join("recipes"));

    let output = cook(dir.path(), &["shopping-list", "Week.menu"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in [r"juice\s+4 l", r"eggs\s+8", r"bread\s+16 slices"] {
        assert!(
            regex::Regex::new(line).unwrap().is_match(&stdout),
            "{line}: {stdout}"
        );
    }
}

#[test]
fn cook_doctor_accepts_a_reference_to_a_menu() {
    let dir = TempDir::new().unwrap();
    write_fixture(&dir.path().join("recipes"));

    let output = cook(dir.path(), &["doctor", "validate"]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("All recipe references are valid"),
        "{stdout}"
    );
}
