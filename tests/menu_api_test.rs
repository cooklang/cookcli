//! Integration tests for how `GET /api/menus/*path` resolves the `scale` of a
//! `recipe_reference`, plus a guard on the `.shopping-list` that
//! `POST /api/shopping_list/add_menu` writes.
//!
//! Both endpoints share `reference_scale_factor`, which lives in the private
//! `server::handlers::common` module and cannot be reached from an integration
//! test, so these drive the real HTTP endpoints instead. Each test boots `cook
//! server` against a temporary recipe directory on its own port.

#![cfg(feature = "server")]

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

/// Recipes and menus shared by every test below.
///
/// `Servings Recipe` declares `servings: 2`, so `{10%servings}` must resolve to
/// a multiplier of 5. `Yield Recipe` declares `yield: 750%ml`, so `{750%ml}`
/// must resolve to 1.
fn write_fixture(dir: &TempDir) {
    let root = dir.path();

    std::fs::write(
        root.join("Servings Recipe.cook"),
        "---\nservings: 2\n---\n\nMix @flour{100%g} and @water{100%ml}.\n",
    )
    .unwrap();

    std::fs::write(
        root.join("Yield Recipe.cook"),
        "---\nyield: 750%ml\n---\n\nSimmer @stock{750%ml}.\n",
    )
    .unwrap();

    std::fs::write(
        root.join("Plan.menu"),
        "---\ntitle: Plan\n---\n\n\
         ==Day 1==\n\n\
         Breakfast: \\\n\
         - @./Servings Recipe{10%servings} \\\n\
         - @./Servings Recipe{2} \\\n\
         - @./Servings Recipe{} \\\n\
         - @./Yield Recipe{750%ml} \\\n\
         - @sugar{2%tsp}\n",
    )
    .unwrap();
}

async fn start_server() -> ServerGuard {
    let dir = TempDir::new().expect("temp dir");
    write_fixture(&dir);

    let port = free_port();
    let child = Command::new(assert_cmd::cargo::cargo_bin("cook"))
        .arg("server")
        .arg(dir.path())
        .arg("--port")
        .arg(port.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn cook server");

    let mut guard = ServerGuard { child, port, dir };

    let client = reqwest::Client::new();
    let url = guard.url("/api/menus");
    for _ in 0..200 {
        if let Some(status) = guard.child.try_wait().expect("poll server") {
            panic!("cook server exited early with {status}");
        }
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                return guard;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("cook server on port {port} never became ready");
}

/// The `scale` of every `recipe_reference` in the menu, in document order.
async fn reference_scales(server: &ServerGuard, query: &str) -> Vec<Option<f64>> {
    let url = server.url(&format!("/api/menus/Plan.menu{query}"));
    let body: Value = reqwest::get(&url)
        .await
        .expect("request menu")
        .error_for_status()
        .expect("menu request succeeded")
        .json()
        .await
        .expect("menu json");

    let mut scales = Vec::new();
    for section in body["sections"].as_array().expect("sections") {
        for meal in section["meals"].as_array().expect("meals") {
            for item in meal["items"].as_array().expect("items") {
                if item["kind"] == "recipe_reference" {
                    scales.push(item["scale"].as_f64());
                }
            }
        }
    }
    scales
}

#[tokio::test]
async fn servings_reference_resolves_against_recipe_servings() {
    let server = start_server().await;
    let scales = reference_scales(&server, "?scale=1").await;

    // `@./Servings Recipe{10%servings}` against `servings: 2` is a ×5 scale,
    // not the raw target of 10.
    assert_eq!(scales[0], Some(5.0));
}

/// Regression test: the query scale used to be applied twice — once by the
/// parser and again by the handler — so `?scale=2` returned 40.0 here.
#[tokio::test]
async fn servings_reference_is_not_scaled_twice() {
    let server = start_server().await;

    let at_one = reference_scales(&server, "?scale=1").await;
    let at_two = reference_scales(&server, "?scale=2").await;

    assert_eq!(at_one[0], Some(5.0));
    assert_eq!(at_two[0], Some(10.0), "?scale=2 must be exactly double");
}

#[tokio::test]
async fn bare_reference_is_a_raw_multiplier() {
    let server = start_server().await;

    // `@./Servings Recipe{2}` has no unit, so 2 is the multiplier itself and
    // the recipe's `servings: 2` is irrelevant.
    assert_eq!(reference_scales(&server, "?scale=1").await[1], Some(2.0));
    assert_eq!(reference_scales(&server, "?scale=2").await[1], Some(4.0));
}

#[tokio::test]
async fn reference_without_quantity_has_no_scale() {
    let server = start_server().await;

    // `@./Servings Recipe{}` carries no target, so the API reports null rather
    // than inventing a multiplier. Pinning the pre-existing behaviour.
    assert_eq!(reference_scales(&server, "?scale=1").await[2], None);
    assert_eq!(reference_scales(&server, "?scale=2").await[2], None);
}

/// The parser normalises units while scaling (2250 ml becomes 2.25 l), so
/// resolving an already-scaled quantity would compare `l` against the recipe's
/// `yield: 750%ml`, find no match, and silently fall back to a raw multiplier
/// of 2.25. Reference quantities are therefore read from a parse at 1.0.
#[tokio::test]
async fn yield_reference_survives_unit_normalisation_under_scaling() {
    let server = start_server().await;

    assert_eq!(reference_scales(&server, "?scale=1").await[3], Some(1.0));
    assert_eq!(reference_scales(&server, "?scale=3").await[3], Some(3.0));
}

/// `GET /api/menus` and `POST /api/shopping_list/add_menu` must agree: both
/// resolve references through the same `reference_scale_factor`.
#[tokio::test]
async fn add_menu_stores_the_same_factors_the_menu_api_reports() {
    let server = start_server().await;
    let list_path = server.dir.path().join(".shopping-list");

    let client = reqwest::Client::new();
    let resp = client
        .post(server.url("/api/shopping_list/add_menu"))
        .json(&serde_json::json!({ "path": "Plan.menu", "scale": 1.0 }))
        .send()
        .await
        .expect("add_menu request");
    assert!(
        resp.status().is_success(),
        "add_menu returned {}",
        resp.status()
    );

    let stored = std::fs::read_to_string(&list_path).expect("read .shopping-list");
    assert_eq!(
        stored,
        "./Plan.menu\n  \
         ./Servings Recipe{5}\n  \
         ./Servings Recipe{2}\n  \
         ./Servings Recipe\n  \
         ./Yield Recipe\n"
    );

    // A menu scale multiplies every stored factor.
    std::fs::remove_file(&list_path).expect("clear .shopping-list");
    let resp = client
        .post(server.url("/api/shopping_list/add_menu"))
        .json(&serde_json::json!({ "path": "Plan.menu", "scale": 3.0 }))
        .send()
        .await
        .expect("add_menu request");
    assert!(
        resp.status().is_success(),
        "add_menu returned {}",
        resp.status()
    );

    let stored = std::fs::read_to_string(&list_path).expect("read .shopping-list");
    assert_eq!(
        stored,
        "./Plan.menu{3}\n  \
         ./Servings Recipe{15}\n  \
         ./Servings Recipe{6}\n  \
         ./Servings Recipe{3}\n  \
         ./Yield Recipe{3}\n"
    );
}
