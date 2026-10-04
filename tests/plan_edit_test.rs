//! End-to-end tests for changing a meal plan from its calendar (#385):
//! `POST /api/plans/{*path}` and the controls the plan's page carries.

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

const PLAN: &str = "---\nservings: 2\n---\n\n\
== Wednesday (2026-10-07) ==\n\n\
-- bring the good olive oil\n\n\
Dinner: \\\n- @./Risotto{} \\\n- @salad{1%bowl}\n\n\
== Friday (2026-10-09) ==\n\n\
Breakfast: \\\n- \n\nDinner: \\\n- \n";

struct ServerGuard {
    child: Child,
    port: u16,
    dir: TempDir,
}

impl ServerGuard {
    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", self.port, path)
    }

    fn plan_file(&self) -> PathBuf {
        self.dir.path().join("recipes/Plans/Week.menu")
    }

    fn plan(&self) -> String {
        std::fs::read_to_string(self.plan_file()).unwrap()
    }

    async fn change(&self, body: Value) -> (StatusCode, Value) {
        let resp = reqwest::Client::new()
            .post(self.url("/api/plans/Plans/Week.menu"))
            .json(&body)
            .send()
            .await
            .expect("plan request");
        let status = resp.status();
        (status, resp.json().await.unwrap_or(Value::Null))
    }

    async fn page(&self) -> String {
        reqwest::get(self.url("/recipe/Plans/Week.menu"))
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
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
    std::fs::create_dir_all(recipes.join("Plans")).unwrap();
    std::fs::create_dir_all(recipes.join("Salads")).unwrap();
    std::fs::write(recipes.join("Risotto.cook"), "Cook @rice{300%g}.\n").unwrap();
    std::fs::write(recipes.join("Salads/Caprese.cook"), "Slice @tomato{2}.\n").unwrap();
    std::fs::write(recipes.join("Plans/Week.menu"), PLAN).unwrap();
    std::fs::write(recipes.join("Plain.menu"), "== Day ==\n\n- @./Risotto{}\n").unwrap();

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

fn version(text: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[tokio::test]
async fn the_page_carries_what_the_calendar_needs_to_change_the_plan() {
    let server = start_server().await;

    let page = server.page().await;

    assert!(page.contains(&format!("\"{}\"", version(PLAN))), "{page}");
    // An Add on each of the plan's meals, every day.
    assert_eq!(page.matches("data-plan-add").count(), 6);
    // The two lines of Wednesday's dinner, with the file's text.
    assert!(page.contains(r#"data-index="0" data-text="@./Risotto{}""#));
    assert!(page.contains(r#"data-index="1" data-text="@salad{1%bowl}""#));
    assert!(page.contains(r#"id="recipe-picker""#));
    assert!(page.contains(r#"id="plan-line-menu""#));

    // A menu with no dated days has no calendar to change.
    let plain = reqwest::get(server.url("/recipe/Plain.menu"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(!plain.contains("data-plan-add"));
}

#[tokio::test]
async fn a_recipe_is_added_to_a_new_day_in_date_order() {
    let server = start_server().await;

    let (status, body) = server
        .change(json!({
            "version": version(PLAN),
            "op": "add",
            "date": "2026-10-08",
            "meal": "Breakfast",
            "recipe": "Salads/Caprese.cook",
            "servings": 2,
        }))
        .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    // Thursday goes before Friday, the first day after it.
    let expected = PLAN.replace(
        "== Friday",
        "== Thursday (2026-10-08) ==\n\nBreakfast: \\\n- @./Salads/Caprese{2%servings}\n\n== Friday",
    );
    assert_eq!(server.plan(), expected);
    assert_eq!(body["version"], version(&expected));
}

#[tokio::test]
async fn a_line_is_moved_and_copied_leaving_the_rest_alone() {
    let server = start_server().await;

    let (status, body) = server
        .change(json!({
            "version": version(PLAN),
            "op": "copy",
            "date": "2026-10-07",
            "meal": "Dinner",
            "index": 1,
            "text": "@salad{1%bowl}",
            "to": { "date": "2026-10-07", "meal": "Breakfast" },
        }))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    // Breakfast comes before Dinner in the plan, so it goes before it.
    let copied = PLAN.replacen(
        "Dinner: \\\n",
        "Breakfast: \\\n- @salad{1%bowl}\n\nDinner: \\\n",
        1,
    );
    assert_eq!(server.plan(), copied);

    let (status, body) = server
        .change(json!({
            "version": body["version"],
            "op": "move",
            "date": "2026-10-07",
            "meal": "Dinner",
            "index": 0,
            "text": "@./Risotto{}",
            "to": { "date": "2026-10-08", "meal": "Dinner" },
        }))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    // Thursday is made before Friday, the first day after it.
    assert_eq!(
        server.plan(),
        copied.replace("- @./Risotto{} \\\n", "").replace(
            "== Friday",
            "== Thursday (2026-10-08) ==\n\nDinner: \\\n- @./Risotto{}\n\n== Friday"
        )
    );
    // The comment is still there, byte for byte.
    assert!(server.plan().contains("\n-- bring the good olive oil\n"));
}

#[tokio::test]
async fn a_change_to_an_older_version_is_refused() {
    let server = start_server().await;

    let (status, body) = server
        .change(json!({
            "version": "0".repeat(64),
            "op": "remove",
            "date": "2026-10-07",
            "meal": "Dinner",
            "index": 0,
            "text": "@./Risotto{}",
        }))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["version"], version(PLAN));

    // The right version but a line that reads otherwise: refused too.
    let (status, _) = server
        .change(json!({
            "version": version(PLAN),
            "op": "remove",
            "date": "2026-10-07",
            "meal": "Dinner",
            "index": 0,
            "text": "@./Soup{}",
        }))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(server.plan(), PLAN);
}

#[tokio::test]
async fn bad_requests_change_nothing() {
    let server = start_server().await;
    let v = version(PLAN);

    for (body, expected) in [
        (
            json!({ "version": v, "op": "add", "date": "2026-10-08", "meal": "Dinner",
                    "recipe": "Missing.cook" }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "version": v, "op": "add", "date": "2026-10-08", "meal": "Dinner",
                    "recipe": "../outside.cook" }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "version": v, "op": "add", "date": "2026-10-08", "meal": "Tea: time",
                    "recipe": "Risotto.cook" }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "version": v, "op": "add", "date": "next week", "meal": "Dinner",
                    "recipe": "Risotto.cook" }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "version": v, "op": "add", "date": "2026-10-08", "meal": "Dinner",
                    "recipe": "Risotto.cook", "servings": 0 }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "version": v, "op": "explode" }),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ] {
        let (status, response) = server.change(body.clone()).await;
        assert_eq!(status, expected, "{body} -> {response}");
    }
    assert_eq!(server.plan(), PLAN);

    // Only a plan can be changed this way.
    let resp = reqwest::Client::new()
        .post(server.url("/api/plans/Plain.menu"))
        .json(
            &json!({ "version": version("== Day ==\n\n- @./Risotto{}\n"), "op": "add",
                       "date": "2026-10-08", "meal": "Dinner", "recipe": "Risotto.cook" }),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let resp = reqwest::Client::new()
        .post(server.url("/api/plans/Nope.menu"))
        .json(
            &json!({ "version": v, "op": "remove", "date": "2026-10-07", "meal": "Dinner",
                       "index": 0, "text": "x" }),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
