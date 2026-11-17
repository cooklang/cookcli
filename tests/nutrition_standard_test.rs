//! End-to-end tests for the nutrition standard: the standard in the `cook_standard`
//! cookie decides how a recipe's nutrition is laid out and rated, and the preferences page
//! offers one button per standard file.

#![cfg(feature = "server")]

use reqwest::StatusCode;
use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

#[path = "common/mod.rs"]
mod common;

/// Kills the spawned server when the test ends, pass or panic.
struct ServerGuard {
    child: Child,
    port: u16,
    #[allow(dead_code)] // keeps the fixture directory alive for the server's lifetime
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

/// A recipe that gives carbohydrates with their fibre, salt, cholesterol and a few vitamins.
fn write_fixture(recipes: &Path) {
    std::fs::create_dir_all(recipes).unwrap();
    std::fs::write(
        recipes.join("Bowl.cook"),
        "---\nservings: 2\nweight: 720g\nnutrition:\n  calories: 685\n  fatContent: 36\n  saturatedFatContent: 6.2\n  \
         carbohydrateContent: 58\n  sugarContent: 7.5\n  fiberContent: 14.5\n  proteinContent: 38\n  salt: 1.4\n  \
         cholesterolContent: 78\n  calcium: 142\n  vitaminD: 12.5\n---\nCook @quinoa{160%g}.\n",
    )
    .unwrap();
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
    write_fixture(&dir.path().join("recipes"));

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

/// The page, as seen by a browser that sends `cookie`.
async fn get(
    server: &ServerGuard,
    path: &str,
    cookie: Option<&str>,
) -> (StatusCode, Vec<String>, String) {
    let mut request = reqwest::Client::new().get(server.url(path));
    if let Some(cookie) = cookie {
        request = request.header("Cookie", cookie);
    }
    let resp = request.send().await.unwrap();
    let status = resp.status();
    let cookies = resp
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok().map(str::to_string))
        .collect();
    (status, cookies, resp.text().await.unwrap())
}

/// The depth the table gives the row of `key`: the row, not the card that shares its key.
fn depth_of(html: &str, key: &str) -> u32 {
    let marker = format!(r#"data-key="{key}""#);
    let (at, row_start) = html
        .match_indices(&marker)
        .map(|(at, _)| (at, html[..at].rfind('<').expect("a tag")))
        .find(|(_, start)| html[*start..].starts_with("<tr"))
        .unwrap_or_else(|| panic!("no row {key} in the page"));
    let row = &html[row_start..at];
    let d = row.split(r#"data-d=""#).nth(1).expect("a depth");
    d.split('"').next().unwrap().parse().unwrap()
}

#[tokio::test]
async fn the_fibre_sits_where_the_standard_in_the_cookie_puts_it() {
    let server = start_server().await;
    let page = "/recipe/Bowl.cook?servings=2";

    // The EU is the default: the fibre is next to the carbohydrates.
    let (status, _, eu) = get(&server, page, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        depth_of(&eu, "fiberContent"),
        depth_of(&eu, "carbohydrateContent")
    );
    assert!(
        eu.contains("eu.yaml"),
        "the table names the standard it follows"
    );
    // under the EU the carbohydrates are shown without the fibre, and the page says so
    assert!(
        eu.contains("43.5 g"),
        "58 g of carbohydrates less 14.5 g of fibre"
    );

    // The US puts it inside.
    let (_, _, us) = get(&server, page, Some("cook_standard=us")).await;
    assert_eq!(
        depth_of(&us, "fiberContent"),
        depth_of(&us, "carbohydrateContent") + 1
    );
    assert!(us.contains("us.yaml"));
    assert!(us.contains("58.0 g"));
    // and the US label gives sodium: the recipe's salt shows as sodium too (1,4 g is 560 mg)
    assert!(us.contains(r#"data-key="sodiumContent""#) && us.contains("560 mg"));
}

#[tokio::test]
async fn the_percentages_follow_the_reference_intakes_of_the_standard() {
    let server = start_server().await;
    let page = "/recipe/Bowl.cook?servings=2";
    // 142 mg of calcium: 18 % of the EU's 800 mg, 11 % of the US's 1300 mg
    let (_, _, eu) = get(&server, page, Some("cook_standard=eu")).await;
    let (_, _, us) = get(&server, page, Some("cook_standard=us")).await;
    assert!(
        eu.contains(r#"<span class="nutri-share">18 %</span>"#),
        "{eu}"
    );
    assert!(
        us.contains(r#"<span class="nutri-share">11 %</span>"#),
        "{us}"
    );
}

#[tokio::test]
async fn a_cookie_that_names_no_standard_falls_back_to_the_default_one() {
    let server = start_server().await;
    let (status, cookies, html) =
        get(&server, "/recipe/Bowl.cook", Some("cook_standard=nowhere")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("eu.yaml"));
    // the answer keeps the cookie alive with a standard that exists
    assert!(
        cookies.iter().any(|c| c.starts_with("cook_standard=eu")),
        "{cookies:?}"
    );
}

#[tokio::test]
async fn the_preferences_page_offers_one_button_per_standard_file() {
    let server = start_server().await;
    let (status, _, html) = get(&server, "/preferences", Some("cook_standard=us")).await;
    assert_eq!(status, StatusCode::OK);
    for id in ["eu", "us", "uk"] {
        assert!(
            html.contains(&format!("setStandard('{id}')")),
            "no button for {id}"
        );
    }
    // the chosen one is the pressed one
    assert!(
        html.contains(r#"setStandard('us')" aria-pressed="true""#),
        "{html}"
    );
    assert!(
        html.contains(r#"setStandard('eu')" aria-pressed="false""#),
        "{html}"
    );
}
