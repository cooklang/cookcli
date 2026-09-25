//! End-to-end tests for `cook server`'s opt-in sign-in (`src/server/auth/`).
//!
//! Every server here gets a configuration directory of its own, outside the
//! recipe directory: the server refuses a users file or session key the
//! recipe directory would publish, and `common::with_isolated_config` keeps
//! the developer's real one out of reach.
//!
//! Users are written with tiny argon2 parameters so signing in stays fast in
//! debug builds; the server verifies against whatever parameters a hash
//! carries. Only the `user` / `hash-password` commands use the real defaults.

#![cfg(feature = "server")]

#[path = "common/mod.rs"]
mod common;

use reqwest::header::{COOKIE, LOCATION, SET_COOKIE};
use reqwest::{redirect, Client, Response, StatusCode};
use std::io::Write;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// A recipe directory and, beside it, a configuration directory.
struct Fixture {
    recipes: TempDir,
    config_root: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let recipes = TempDir::new().expect("temp dir");
        std::fs::write(
            recipes.path().join("Recipe.cook"),
            "Mix @flour{100%g} and @water{100%ml}.\n",
        )
        .unwrap();
        let config = recipes.path().join("config");
        std::fs::create_dir_all(&config).unwrap();
        std::fs::write(config.join("pantry.conf"), "[pantry]\nflour = \"1%kg\"\n").unwrap();

        Self {
            recipes,
            config_root: TempDir::new().expect("temp dir"),
        }
    }

    /// The global configuration directory the spawned `cook` sees, as set up
    /// by `common::with_isolated_config`.
    fn config_dir(&self) -> PathBuf {
        let dir = self.config_root.path().join(".cook-config");
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Where the server looks for users by default.
    fn users_file(&self) -> PathBuf {
        self.config_dir().join("users.toml")
    }

    fn write_users(&self, users: &[(&str, &str)]) {
        write_users_at(&self.users_file(), users);
    }

    /// A `cook` command isolated to this fixture's configuration directory.
    fn cook(&self) -> Command {
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin("cook"));
        common::with_isolated_config(&mut cmd, self.config_root.path());
        cmd
    }

    /// Runs a `cook` subcommand with `stdin` piped in.
    fn run(&self, args: &[&str], stdin: &str) -> Output {
        let mut child = self
            .cook()
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn cook");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
        child.wait_with_output().expect("wait for cook")
    }
}

fn write_users_at(path: &Path, users: &[(&str, &str)]) {
    let mut text = String::from("[users]\n");
    for (name, password) in users {
        text.push_str(&format!("\"{name}\" = \"{}\"\n", tiny_hash(password)));
    }
    std::fs::write(path, text).unwrap();
}

fn tiny_hash(password: &str) -> String {
    use argon2::password_hash::{PasswordHasher, SaltString};
    use argon2::{Algorithm, Argon2, Params, Version};
    let params = Params::new(64, 1, 1, None).unwrap();
    let salt = SaltString::encode_b64(b"fixture-salt").unwrap();
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string()
}

/// Kills the spawned server when the test ends, pass or panic.
struct ServerGuard {
    child: Child,
    port: u16,
    prefix: String,
    fixture: Fixture,
}

impl ServerGuard {
    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}{}", self.port, self.prefix, path)
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

/// No redirects followed: the tests look at the `303`s and their cookies.
fn client() -> Client {
    Client::builder()
        .redirect(redirect::Policy::none())
        .build()
        .unwrap()
}

async fn start(fixture: Fixture, args: &[&str], envs: &[(&str, &str)]) -> ServerGuard {
    let mut fixture = Some(fixture);
    for _ in 0..5 {
        match try_start(fixture.take().unwrap(), args, envs).await {
            Ok(server) => return server,
            Err(returned) => fixture = Some(returned),
        }
    }
    panic!("could not start cook server on a free port after 5 attempts");
}

async fn try_start(
    fixture: Fixture,
    args: &[&str],
    envs: &[(&str, &str)],
) -> Result<ServerGuard, Fixture> {
    let port = free_port();
    let prefix = args
        .iter()
        .position(|arg| *arg == "--url-prefix")
        .map(|i| args[i + 1].to_string())
        .unwrap_or_default();
    let mut cmd = fixture.cook();
    cmd.arg("server")
        .arg(fixture.recipes.path())
        .arg("--port")
        .arg(port.to_string())
        .args(args)
        .envs(envs.iter().copied())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = cmd.spawn().expect("spawn cook server");

    // A read: open to guests whether or not sign-in is on.
    let probe = format!("http://127.0.0.1:{port}{prefix}/api/menus");
    for _ in 0..600 {
        if child.try_wait().expect("poll server").is_some() {
            // The port was taken between reserving and binding it.
            return Err(fixture);
        }
        if let Ok(resp) = client().get(&probe).send().await {
            if resp.status().is_success() {
                return Ok(ServerGuard {
                    child,
                    port,
                    prefix,
                    fixture,
                });
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let _ = child.kill();
    panic!("cook server on port {port} never became ready");
}

/// Runs `cook server` expecting it to stop before it serves anything.
fn startup_failure(fixture: &Fixture, args: &[&str], envs: &[(&str, &str)]) -> String {
    let output = fixture
        .cook()
        .arg("server")
        .arg(fixture.recipes.path())
        .arg("--port")
        .arg(free_port().to_string())
        .args(args)
        .envs(envs.iter().copied())
        .output()
        .expect("run cook server");
    assert!(
        !output.status.success(),
        "server should have refused to start"
    );
    String::from_utf8_lossy(&output.stderr).into_owned()
}

async fn sign_in(server: &ServerGuard, user: &str, password: &str, next: &str) -> Response {
    client()
        .post(server.url("/login"))
        .form(&[("username", user), ("password", password), ("next", next)])
        .send()
        .await
        .expect("sign-in request")
}

/// The response's `Set-Cookie` for the session, if any. Every response also
/// sets the feature-flag cookies, so the header has to be picked out.
fn session_set_cookie(resp: &Response) -> Option<String> {
    resp.headers()
        .get_all(SET_COOKIE)
        .iter()
        .map(|value| value.to_str().unwrap().to_string())
        .find(|value| value.starts_with("cook_session="))
}

/// `name=value` from the session's `Set-Cookie`, ready for a `Cookie` header.
fn session_cookie(resp: &Response) -> String {
    let set_cookie = session_set_cookie(resp).expect("session Set-Cookie");
    set_cookie.split(';').next().unwrap().to_string()
}

async fn signed_in_cookie(server: &ServerGuard, user: &str, password: &str) -> String {
    let resp = sign_in(server, user, password, "/").await;
    assert_eq!(resp.status(), StatusCode::SEE_OTHER, "sign-in failed");
    session_cookie(&resp)
}

async fn put_recipe(server: &ServerGuard, name: &str, cookie: Option<&str>) -> Response {
    let mut req = client()
        .put(server.url(&format!("/api/recipes/{name}")))
        .body("Boil @water{1%l}.\n");
    if let Some(cookie) = cookie {
        req = req.header(COOKIE, cookie);
    }
    req.send().await.expect("save request")
}

async fn get_page(server: &ServerGuard, path: &str, cookie: Option<&str>) -> Response {
    let mut req = client().get(server.url(path));
    if let Some(cookie) = cookie {
        req = req.header(COOKIE, cookie);
    }
    req.send().await.expect("page request")
}

fn location(resp: &Response) -> &str {
    resp.headers()
        .get(LOCATION)
        .expect("Location")
        .to_str()
        .unwrap()
}

// --- Without a users file ---------------------------------------------------

#[tokio::test]
async fn without_users_the_server_stays_open() {
    let server = start(Fixture::new(), &[], &[]).await;

    let resp = put_recipe(&server, "Open", None).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(server.fixture.recipes.path().join("Open.cook").exists());

    let home = get_page(&server, "/", None).await.text().await.unwrap();
    assert!(home.contains("href=\"/new\""), "New Recipe button missing");
    assert!(!home.contains("Sign in"), "no sign-in link without users");

    let login = get_page(&server, "/login", None).await;
    assert_eq!(login.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&login), "/");
}

#[tokio::test]
async fn an_empty_users_file_variable_counts_as_unset() {
    let server = start(Fixture::new(), &[], &[("COOK_USERS_FILE", "")]).await;
    assert_eq!(
        put_recipe(&server, "Open", None).await.status(),
        StatusCode::OK
    );
}

// --- Guests -----------------------------------------------------------------

#[tokio::test]
async fn guests_can_browse_but_not_change_anything() {
    let fixture = Fixture::new();
    fixture.write_users(&[("alice", "secret")]);
    let server = start(fixture, &[], &[]).await;
    let http = client();

    // Reads stay open, including the POST that only computes a list.
    for path in [
        "/",
        "/recipe/Recipe.cook",
        "/shopping-list",
        "/pantry",
        "/api/recipes",
    ] {
        assert_eq!(
            get_page(&server, path, None).await.status(),
            StatusCode::OK,
            "{path}"
        );
    }
    let list = http
        .post(server.url("/api/shopping_list"))
        .json(&serde_json::json!([{ "recipe": "Recipe.cook" }]))
        .send()
        .await
        .unwrap();
    assert_eq!(list.status(), StatusCode::OK);

    // API writes: 401 with the usual error shape, and nothing changes.
    let resp = put_recipe(&server, "Recipe.cook", None).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "Sign in to make changes");
    let recipe = server.fixture.recipes.path().join("Recipe.cook");
    assert!(std::fs::read_to_string(&recipe).unwrap().contains("flour"));

    let delete = http
        .delete(server.url("/api/recipes/Recipe.cook"))
        .send()
        .await
        .unwrap();
    assert_eq!(delete.status(), StatusCode::UNAUTHORIZED);
    assert!(recipe.exists());

    for path in [
        "/api/shopping_list/add",
        "/api/shopping_list/check",
        "/api/shopping_list/clear",
        "/api/pantry/add",
    ] {
        let resp = http
            .post(server.url(path))
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "{path}");
    }

    // The editor's language server and the sync controls.
    assert_eq!(
        get_page(&server, "/api/ws/lsp", None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    #[cfg(feature = "sync")]
    assert_eq!(
        get_page(&server, "/api/sync/status", None).await.status(),
        StatusCode::UNAUTHORIZED
    );

    // Editor pages send the browser to sign in, and back afterwards.
    let edit = get_page(&server, "/edit/Recipe.cook", None).await;
    assert_eq!(edit.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&edit), "/login?next=%2Fedit%2FRecipe.cook");
    let new = get_page(&server, "/new", None).await;
    assert_eq!(location(&new), "/login?next=%2Fnew");
    let create = http
        .post(server.url("/new"))
        .form(&[("filename", "Sneaky")])
        .send()
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::SEE_OTHER);
    assert!(location(&create).starts_with("/login"));
    assert!(!server.fixture.recipes.path().join("Sneaky.cook").exists());

    // The pages offer a sign-in link instead of editing controls.
    let home = get_page(&server, "/", None).await.text().await.unwrap();
    assert!(home.contains("Sign in"), "no sign-in link");
    assert!(
        !home.contains("href=\"/new\""),
        "New Recipe shown to a guest"
    );
    let recipe_page = get_page(&server, "/recipe/Recipe.cook", None)
        .await
        .text()
        .await
        .unwrap();
    assert!(
        !recipe_page.contains("href=\"/edit/"),
        "Edit shown to a guest"
    );
    assert!(recipe_page.contains("window.__CAN_EDIT__ = false"));
}

#[tokio::test]
async fn an_empty_user_list_makes_the_site_read_only() {
    let fixture = Fixture::new();
    std::fs::write(fixture.users_file(), "# nobody yet\n[users]\n").unwrap();
    let server = start(fixture, &[], &[]).await;

    assert_eq!(
        put_recipe(&server, "Recipe.cook", None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        sign_in(&server, "alice", "secret", "/").await.status(),
        StatusCode::UNAUTHORIZED
    );
}

// --- Signing in ---------------------------------------------------------------

#[tokio::test]
async fn signing_in_allows_changes_until_signing_out() {
    let fixture = Fixture::new();
    fixture.write_users(&[("alice", "secret"), ("bob@home", "other")]);
    let server = start(fixture, &[], &[]).await;

    // Wrong password, unknown user: the form again, no cookie.
    for (user, password) in [("alice", "wrong"), ("mallory", "secret")] {
        let resp = sign_in(&server, user, password, "/").await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "{user}");
        assert!(session_set_cookie(&resp).is_none(), "{user}");
        let body = resp.text().await.unwrap();
        assert!(body.contains("Wrong username or password"), "{user}");
    }

    let resp = sign_in(&server, "alice", "secret", "/edit/Recipe.cook").await;
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&resp), "/edit/Recipe.cook");
    let set_cookie = session_set_cookie(&resp).unwrap();
    for attribute in ["HttpOnly", "SameSite=Lax", "Path=/;", "Max-Age=2592000"] {
        assert!(set_cookie.contains(attribute), "{set_cookie}");
    }
    let cookie = session_cookie(&resp);

    assert_eq!(
        put_recipe(&server, "Signed", Some(&cookie)).await.status(),
        StatusCode::OK
    );
    assert!(server.fixture.recipes.path().join("Signed.cook").exists());
    assert_eq!(
        get_page(&server, "/edit/Recipe.cook", Some(&cookie))
            .await
            .status(),
        StatusCode::OK
    );
    let home = get_page(&server, "/", Some(&cookie))
        .await
        .text()
        .await
        .unwrap();
    assert!(home.contains("Sign out"));
    assert!(home.contains("alice"));
    assert!(home.contains("href=\"/new\""));

    // Another user, whose name needs quoting in TOML.
    let bob = signed_in_cookie(&server, "bob@home", "other").await;
    assert_eq!(
        put_recipe(&server, "Bob", Some(&bob)).await.status(),
        StatusCode::OK
    );

    // A forged or altered cookie is a guest.
    let tampered = cookie.replacen("alice:", "bob@home:", 1);
    for bad in [tampered.as_str(), "cook_session=alice:99999999999:00"] {
        assert_eq!(
            put_recipe(&server, "Recipe.cook", Some(bad)).await.status(),
            StatusCode::UNAUTHORIZED,
            "{bad}"
        );
    }

    // Signing out clears the cookie.
    let out = client()
        .post(server.url("/logout"))
        .header(COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(out.status(), StatusCode::SEE_OTHER);
    assert!(session_set_cookie(&out).unwrap().contains("Max-Age=0"));
}

#[tokio::test]
async fn sign_in_only_redirects_within_the_server() {
    let fixture = Fixture::new();
    fixture.write_users(&[("alice", "secret")]);
    let server = start(fixture, &[], &[]).await;

    for next in ["//evil.test", "https://evil.test/", "/\\evil.test"] {
        let resp = sign_in(&server, "alice", "secret", next).await;
        assert_eq!(location(&resp), "/", "{next}");
    }
}

#[tokio::test]
async fn sign_in_works_under_a_url_prefix() {
    let fixture = Fixture::new();
    fixture.write_users(&[("alice", "secret")]);
    let server = start(fixture, &["--url-prefix", "/cook"], &[]).await;

    let edit = get_page(&server, "/edit/Recipe.cook", None).await;
    assert_eq!(location(&edit), "/cook/login?next=%2Fedit%2FRecipe.cook");

    let resp = sign_in(&server, "alice", "secret", "/").await;
    assert_eq!(location(&resp), "/cook/");
    let set_cookie = session_set_cookie(&resp).unwrap();
    // `/cook`, not `/cook/`: the home page is `/cook`.
    assert!(set_cookie.contains("Path=/cook;"), "{set_cookie}");
    let cookie = session_cookie(&resp);

    assert_eq!(
        put_recipe(&server, "Prefixed", Some(&cookie))
            .await
            .status(),
        StatusCode::OK
    );
    let home = get_page(&server, "", Some(&cookie))
        .await
        .text()
        .await
        .unwrap();
    assert!(home.contains("Sign out"));
}

#[tokio::test]
async fn users_file_can_come_from_the_environment() {
    let fixture = Fixture::new();
    let elsewhere = fixture.config_root.path().join("elsewhere.toml");
    write_users_at(&elsewhere, &[("alice", "secret")]);
    let server = start(
        fixture,
        &[],
        &[("COOK_USERS_FILE", elsewhere.to_str().unwrap())],
    )
    .await;

    assert_eq!(
        put_recipe(&server, "Recipe.cook", None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    signed_in_cookie(&server, "alice", "secret").await;
}

// --- Refusing to start ----------------------------------------------------------

#[test]
fn refuses_a_named_users_file_that_does_not_exist() {
    let fixture = Fixture::new();
    let missing = fixture.config_root.path().join("missing.toml");
    let stderr = startup_failure(&fixture, &["--users-file", missing.to_str().unwrap()], &[]);
    assert!(stderr.contains("does not exist"), "{stderr}");
}

#[test]
fn refuses_a_users_file_it_cannot_use() {
    let fixture = Fixture::new();
    std::fs::write(fixture.users_file(), "[users]\nalice = \"hunter2\"\n").unwrap();
    let stderr = startup_failure(&fixture, &[], &[]);
    assert!(stderr.contains("not an argon2 hash"), "{stderr}");
}

#[test]
fn refuses_a_users_file_the_server_would_publish() {
    let fixture = Fixture::new();
    let inside = fixture.recipes.path().join("users.toml");
    write_users_at(&inside, &[("alice", "secret")]);
    let stderr = startup_failure(&fixture, &["--users-file", inside.to_str().unwrap()], &[]);
    assert!(stderr.contains("inside the recipe directory"), "{stderr}");
}

// --- Managing users -------------------------------------------------------------

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn user_commands_edit_the_users_file() {
    let fixture = Fixture::new();

    let added = fixture.run(&["server", "user", "add", "alice"], "first\n");
    assert_success(&added);
    let stdout = String::from_utf8_lossy(&added.stdout);
    assert!(stdout.contains("Added alice"), "{stdout}");
    assert!(stdout.contains("Restart cook server"), "{stdout}");
    assert!(fixture.users_file().exists());

    // Comments survive the rewrites.
    let text = std::fs::read_to_string(fixture.users_file()).unwrap();
    std::fs::write(fixture.users_file(), format!("# kitchen crew\n{text}")).unwrap();

    assert_success(&fixture.run(&["server", "user", "add", "bob"], "second\n"));
    let duplicate = fixture.run(&["server", "user", "add", "bob"], "again\n");
    assert!(!duplicate.status.success());
    assert!(String::from_utf8_lossy(&duplicate.stderr).contains("already exists"));

    let invalid = fixture.run(&["server", "user", "add", "no spaces"], "pw\n");
    assert!(!invalid.status.success());
    let empty = fixture.run(&["server", "user", "add", "carol"], "\n");
    assert!(!empty.status.success());

    let before = std::fs::read_to_string(fixture.users_file()).unwrap();
    assert_success(&fixture.run(&["server", "user", "passwd", "bob"], "changed\n"));
    let after = std::fs::read_to_string(fixture.users_file()).unwrap();
    assert_ne!(before, after, "passwd should rewrite bob's hash");

    assert_success(&fixture.run(&["server", "user", "remove", "alice"], ""));
    let list = fixture.run(&["server", "user", "list"], "");
    assert_success(&list);
    assert_eq!(String::from_utf8_lossy(&list.stdout).trim(), "bob");

    let text = std::fs::read_to_string(fixture.users_file()).unwrap();
    assert!(text.starts_with("# kitchen crew"), "{text}");

    let missing = fixture.run(&["server", "user", "remove", "alice"], "");
    assert!(!missing.status.success());
}

#[test]
fn user_commands_honour_the_users_file_flag() {
    let fixture = Fixture::new();
    let elsewhere = fixture.config_root.path().join("team.toml");
    let path = elsewhere.to_str().unwrap();

    assert_success(&fixture.run(
        &["server", "user", "add", "alice", "--users-file", path],
        "pw\n",
    ));
    assert!(elsewhere.exists());
    assert!(!fixture.users_file().exists());

    let list = fixture.run(&["server", "user", "--users-file", path, "list"], "");
    assert_eq!(String::from_utf8_lossy(&list.stdout).trim(), "alice");
}

#[tokio::test]
async fn users_added_from_the_command_line_can_sign_in() {
    let fixture = Fixture::new();
    assert_success(&fixture.run(&["server", "user", "add", "alice"], "from-cli\n"));

    // And a hand-edited file with a hash from `hash-password`.
    let hashed = fixture.run(&["server", "hash-password"], "by-hand\n");
    assert_success(&hashed);
    let hash = String::from_utf8_lossy(&hashed.stdout).trim().to_string();
    assert!(hash.starts_with("$argon2id$"), "{hash}");
    let mut text = std::fs::read_to_string(fixture.users_file()).unwrap();
    text.push_str(&format!("bob = \"{hash}\"\n"));
    std::fs::write(fixture.users_file(), text).unwrap();

    let server = start(fixture, &[], &[]).await;
    signed_in_cookie(&server, "alice", "from-cli").await;
    signed_in_cookie(&server, "bob", "by-hand").await;
}

// --- Live reload ------------------------------------------------------------------

/// Polls `check` until it holds, for up to 15 seconds: the server notices a
/// change through a debounced filesystem watcher.
async fn eventually<F, Fut>(what: &str, mut check: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if check().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("timed out waiting for: {what}");
}

#[tokio::test]
async fn a_running_server_picks_up_user_changes() {
    let fixture = Fixture::new();
    fixture.write_users(&[("alice", "secret")]);
    let server = start(fixture, &[], &[]).await;
    let alice = signed_in_cookie(&server, "alice", "secret").await;

    // Added: can sign in without a restart.
    assert_success(
        &server
            .fixture
            .run(&["server", "user", "add", "bob"], "bobpw\n"),
    );
    eventually("bob can sign in", || async {
        sign_in(&server, "bob", "bobpw", "/").await.status() == StatusCode::SEE_OTHER
    })
    .await;

    // Removed: signed out everywhere.
    assert_success(
        &server
            .fixture
            .run(&["server", "user", "remove", "alice"], ""),
    );
    eventually("alice's session ends", || async {
        put_recipe(&server, "Recipe.cook", Some(&alice))
            .await
            .status()
            == StatusCode::UNAUTHORIZED
    })
    .await;

    // A broken edit is ignored: bob keeps working.
    std::fs::write(server.fixture.users_file(), "[users\nthis is not toml").unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(
        sign_in(&server, "bob", "bobpw", "/").await.status(),
        StatusCode::SEE_OTHER
    );
}
