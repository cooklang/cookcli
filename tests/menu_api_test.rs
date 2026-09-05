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

/// The `scale` of every `recipe_reference` in the menu, in document order.
/// `scale` is always a number — never null — so a missing/!number value here
/// is itself a failure.
async fn reference_scales(server: &ServerGuard, query: &str) -> Vec<f64> {
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
                    scales.push(
                        item["scale"]
                            .as_f64()
                            .unwrap_or_else(|| panic!("scale was not a number: {}", item["scale"])),
                    );
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
    assert_eq!(scales[0], 5.0);
}

/// Regression test: the query scale used to be applied twice — once by the
/// parser and again by the handler — so `?scale=2` returned 40.0 here.
#[tokio::test]
async fn servings_reference_is_not_scaled_twice() {
    let server = start_server().await;

    let at_one = reference_scales(&server, "?scale=1").await;
    let at_two = reference_scales(&server, "?scale=2").await;

    assert_eq!(at_one[0], 5.0);
    assert_eq!(at_two[0], 10.0, "?scale=2 must be exactly double");
}

#[tokio::test]
async fn bare_reference_is_a_raw_multiplier() {
    let server = start_server().await;

    // `@./Servings Recipe{2}` has no unit, so 2 is the multiplier itself and
    // the recipe's `servings: 2` is irrelevant.
    assert_eq!(reference_scales(&server, "?scale=1").await[1], 2.0);
    assert_eq!(reference_scales(&server, "?scale=2").await[1], 4.0);
}

/// `@./Servings Recipe{}` carries no target of its own, so it is ×1 — but the
/// menu scale still applies to it, which is what `add_menu` stores for the same
/// reference. This used to report null, losing the menu scale entirely.
#[tokio::test]
async fn reference_without_quantity_uses_the_menu_scale() {
    let server = start_server().await;

    assert_eq!(reference_scales(&server, "?scale=1").await[2], 1.0);
    assert_eq!(reference_scales(&server, "?scale=2").await[2], 2.0);
}

/// The parser normalises units while scaling (2250 ml becomes 2.25 l), so
/// resolving an already-scaled quantity would compare `l` against the recipe's
/// `yield: 750%ml`, find no match, and silently fall back to a raw multiplier
/// of 2.25. Reference quantities are therefore read from a parse at 1.0.
#[tokio::test]
async fn yield_reference_survives_unit_normalisation_under_scaling() {
    let server = start_server().await;

    assert_eq!(reference_scales(&server, "?scale=1").await[3], 1.0);
    assert_eq!(reference_scales(&server, "?scale=3").await[3], 3.0);
}

/// Factors stored in `.shopping-list`, in file order. A bare `./Name` line is
/// factor 1; `./Name{n}` is factor n. Only top-level (two-space) entries.
fn stored_factors(list: &str) -> Vec<f64> {
    list.lines()
        .filter(|l| l.starts_with("  ") && !l.starts_with("    "))
        .map(|l| {
            let l = l.trim();
            match l.rsplit_once('{') {
                Some((_, rest)) => rest
                    .trim_end_matches('}')
                    .parse()
                    .unwrap_or_else(|_| panic!("unparseable factor in {l:?}")),
                None => 1.0,
            }
        })
        .collect()
}

async fn post_add_menu(server: &ServerGuard, scale: f64) -> String {
    let list_path = server.dir.path().join(".shopping-list");
    let _ = std::fs::remove_file(&list_path);

    let resp = reqwest::Client::new()
        .post(server.url("/api/shopping_list/add_menu"))
        .json(&serde_json::json!({ "path": "Plan.menu", "scale": scale }))
        .send()
        .await
        .expect("add_menu request");
    assert!(
        resp.status().is_success(),
        "add_menu returned {}",
        resp.status()
    );

    std::fs::read_to_string(&list_path).expect("read .shopping-list")
}

/// The exact `.shopping-list` bytes `add_menu` writes. Pinned so a future
/// refactor cannot quietly change what gets stored.
#[tokio::test]
async fn add_menu_stores_the_expected_shopping_list() {
    let server = start_server().await;

    assert_eq!(
        post_add_menu(&server, 1.0).await,
        "./Plan.menu\n  \
         ./Servings Recipe{5}\n  \
         ./Servings Recipe{2}\n  \
         ./Servings Recipe\n  \
         ./Yield Recipe\n"
    );

    // A menu scale multiplies every stored factor, including the `{}` one.
    assert_eq!(
        post_add_menu(&server, 3.0).await,
        "./Plan.menu{3}\n  \
         ./Servings Recipe{15}\n  \
         ./Servings Recipe{6}\n  \
         ./Servings Recipe{3}\n  \
         ./Yield Recipe{3}\n"
    );
}

/// `GET /api/menus` and `POST /api/shopping_list/add_menu` must agree on every
/// reference, including the `{}` one — they share `reference_scale_factor`.
#[tokio::test]
async fn add_menu_and_the_menu_api_report_identical_factors() {
    let server = start_server().await;

    for scale in [1.0, 2.0, 3.0] {
        let from_api = reference_scales(&server, &format!("?scale={scale}")).await;
        let from_store = stored_factors(&post_add_menu(&server, scale).await);

        assert_eq!(
            from_api.len(),
            4,
            "expected 4 references from the API at scale {scale}"
        );
        assert_eq!(
            from_api, from_store,
            "menu API and add_menu disagreed at scale {scale}"
        );
    }
}

/// The HTML menu page (also the `cook build web` static export) renders the
/// same factors the API reports. It used to compute them independently, and
/// wrongly. A x1 badge is suppressed as visual noise, so absence of a badge
/// means exactly 1.0.
#[tokio::test]
async fn html_menu_page_agrees_with_the_menu_api() {
    let server = start_server().await;

    // Every reference link, paired with the badge following it (if any).
    // The badge's classes are deliberately not pinned — this test is about
    // the factors, and hard-coding the styling made a purely visual change
    // to menu.html fail here.
    let re = regex::Regex::new(
        r#"/recipe/(?:[^"?]*?)"[^>]*>\s*[^<]+?\s*</a>\s*(?:<span[^>]*>\(×([0-9.]+)\)</span>)?"#,
    )
    .unwrap();

    for (scale, expected) in [
        (1.0, vec![5.0, 2.0, 1.0, 1.0]),
        (2.0, vec![10.0, 4.0, 2.0, 2.0]),
    ] {
        let html = reqwest::get(server.url(&format!("/recipe/Plan.menu?scale={scale}")))
            .await
            .expect("request menu page")
            .error_for_status()
            .expect("menu page succeeded")
            .text()
            .await
            .expect("menu page body");

        let rendered: Vec<f64> = re
            .captures_iter(&html)
            .map(|c| c.get(1).map_or(1.0, |m| m.as_str().parse().unwrap()))
            .collect();

        assert_eq!(rendered, expected, "HTML menu page at scale {scale}");
        assert_eq!(
            rendered,
            reference_scales(&server, &format!("?scale={scale}")).await,
            "HTML page and menu API disagreed at scale {scale}"
        );
    }
}
