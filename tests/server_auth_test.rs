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

use reqwest::header::{ACCEPT_LANGUAGE, COOKIE, LOCATION, SET_COOKIE};
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

    /// Where a server started on this fixture writes its standard output.
    fn stdout_path(&self) -> PathBuf {
        self.config_root.path().join("server-stdout.log")
    }

    /// The activity lines the server has printed so far: its standard output
    /// minus the startup banner. `println!` flushes each line, and a handler
    /// prints before it responds, so a change is here once its request
    /// returns.
    fn activity(&self) -> Vec<String> {
        std::fs::read_to_string(self.stdout_path())
            .unwrap_or_default()
            .lines()
            .filter_map(|line| {
                let (time, rest) = (line.get(..19)?, line.get(19..)?);
                chrono::NaiveDateTime::parse_from_str(time, "%Y-%m-%d %H:%M:%S").ok()?;
                Some(rest.strip_prefix(' ')?.to_string())
            })
            .collect()
    }

    fn write_users(&self, users: &[(&str, &str)]) {
        write_users_at(&self.users_file(), users);
    }

    /// Writes users as `(name, password, role)`.
    fn write_roles(&self, users: &[(&str, &str, &str)]) {
        let mut text = String::from("[users]\n");
        for (name, password, role) in users {
            text.push_str(&format!(
                "\"{name}\" = {{ hash = \"{}\", role = \"{role}\" }}\n",
                tiny_hash(password)
            ));
        }
        std::fs::write(self.users_file(), text).unwrap();
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

/// A running server and the fixture it serves.
struct ServerGuard {
    child: KillOnDrop,
    port: u16,
    prefix: String,
    fixture: Fixture,
}

impl ServerGuard {
    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}{}", self.port, self.prefix, path)
    }

    /// Stops the server and hands back its fixture, to start another on it.
    fn stop(self) -> Fixture {
        let ServerGuard { child, fixture, .. } = self;
        drop(child);
        fixture
    }
}

/// Kills the spawned server when the test ends, pass or panic.
struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
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
        .stdout(std::fs::File::create(fixture.stdout_path()).expect("stdout log"))
        .stderr(Stdio::null());
    let mut child = cmd.spawn().expect("spawn cook server");

    // A read open to guests whether or not sign-in or --recipes-only is on.
    let probe = format!("http://127.0.0.1:{port}{prefix}/api/recipes");
    for _ in 0..600 {
        if child.try_wait().expect("poll server").is_some() {
            // The port was taken between reserving and binding it.
            return Err(fixture);
        }
        if let Ok(resp) = client().get(&probe).send().await {
            if resp.status().is_success() {
                return Ok(ServerGuard {
                    child: KillOnDrop(child),
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

async fn sign_out(server: &ServerGuard, cookie: &str) -> Response {
    client()
        .post(server.url("/logout"))
        .header(COOKIE, cookie)
        .send()
        .await
        .expect("sign-out request")
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

/// What the Preferences page tells `cookie`'s viewer about where things are
/// on the server: whether it names the recipe directory, and whether it
/// shows the pantry file as `config/pantry.conf`, relative to it.
async fn preferences_paths(server: &ServerGuard, cookie: Option<&str>) -> (bool, bool) {
    let page = get_page(server, "/preferences", cookie)
        .await
        .text()
        .await
        .unwrap();
    // The directory's own name rather than its full path, which the server
    // may spell differently once it has resolved it.
    let name = server.fixture.recipes.path().file_name().unwrap();
    let relative = Path::new("config").join("pantry.conf");
    (
        page.contains(name.to_str().unwrap()),
        page.contains(&format!(">{}<", relative.display())),
    )
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
    assert_eq!(
        server.fixture.activity(),
        ["guest created recipe \"Open.cook\""]
    );

    let home = get_page(&server, "/", None).await.text().await.unwrap();
    assert!(home.contains("href=\"/new\""), "New Recipe button missing");
    assert!(!home.contains("Sign in"), "no sign-in link without users");

    let login = get_page(&server, "/login", None).await;
    assert_eq!(login.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&login), "/");

    // Everyone runs the server here, so Preferences keeps the full paths.
    assert_eq!(preferences_paths(&server, None).await, (true, false));
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
    // Outside the recipe directory, for Preferences below.
    std::fs::write(fixture.config_dir().join("aisle.conf"), "[baking]\nflour\n").unwrap();
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

    let rename = http
        .post(server.url("/api/recipe_rename/Recipe.cook"))
        .json(&serde_json::json!({ "name": "Renamed" }))
        .send()
        .await
        .unwrap();
    assert_eq!(rename.status(), StatusCode::UNAUTHORIZED);
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
    assert!(recipe_page.contains("window.__CAN_EDIT_LISTS__ = false"));

    // Preferences says where the files are without naming the server's
    // directories.
    assert_eq!(preferences_paths(&server, None).await, (false, true));
    let preferences = get_page(&server, "/preferences", None)
        .await
        .text()
        .await
        .unwrap();
    assert!(preferences.contains(">Global configuration<"));
    let config_root = server.fixture.config_root.path().file_name().unwrap();
    assert!(!preferences.contains(config_root.to_str().unwrap()));
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

    // A second sign-in of the same user, as from another browser.
    let other_browser = signed_in_cookie(&server, "alice", "secret").await;

    // Signing out clears the cookie, and the server forgets the session: a
    // copy of the cookie kept elsewhere is a guest from then on.
    let out = sign_out(&server, &cookie).await;
    assert_eq!(out.status(), StatusCode::SEE_OTHER);
    assert!(session_set_cookie(&out).unwrap().contains("Max-Age=0"));
    assert_eq!(
        put_recipe(&server, "After", Some(&cookie)).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let edit = get_page(&server, "/edit/Recipe.cook", Some(&cookie)).await;
    assert_eq!(edit.status(), StatusCode::SEE_OTHER);
    assert!(location(&edit).starts_with("/login?next="));
    assert!(!server.fixture.recipes.path().join("After.cook").exists());

    // Only that session: alice's other browser, and bob, are still signed in.
    for other in [&other_browser, &bob] {
        assert_eq!(
            put_recipe(&server, "Still", Some(other)).await.status(),
            StatusCode::OK
        );
    }

    // Signing out again, or with a forged cookie, is harmless.
    for again in [cookie.as_str(), "cook_session=alice:99999999999:00:00"] {
        assert_eq!(
            sign_out(&server, again).await.status(),
            StatusCode::SEE_OTHER
        );
    }
}

#[tokio::test]
async fn signing_out_holds_across_a_restart() {
    let fixture = Fixture::new();
    fixture.write_users(&[("alice", "secret")]);
    let server = start(fixture, &[], &[]).await;

    let signed_out = signed_in_cookie(&server, "alice", "secret").await;
    let kept = signed_in_cookie(&server, "alice", "secret").await;
    assert_eq!(
        sign_out(&server, &signed_out).await.status(),
        StatusCode::SEE_OTHER
    );
    let revoked = server.fixture.config_dir().join("revoked-sessions");
    assert!(revoked.exists(), "the sign-out is written down");

    let server = start(server.stop(), &[], &[]).await;
    assert_eq!(
        put_recipe(&server, "Revoked", Some(&signed_out))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        put_recipe(&server, "Kept", Some(&kept)).await.status(),
        StatusCode::OK
    );

    // A list the server cannot make sense of stops it rather than bringing
    // the signed-out session back.
    let fixture = server.stop();
    std::fs::write(&revoked, "not a list\n").unwrap();
    let stderr = startup_failure(&fixture, &[], &[]);
    assert!(stderr.contains("revoked-sessions"), "{stderr}");
}

#[tokio::test]
async fn changes_are_logged_with_who_made_them() {
    let fixture = Fixture::new();
    fixture.write_users(&[("alice", "secret")]);
    let server = start(fixture, &[], &[]).await;

    // A password typed into the name field must not end up in the log.
    let failed = sign_in(&server, "MyPasswordTyped", "x", "/").await;
    assert_eq!(failed.status(), StatusCode::UNAUTHORIZED);
    let cookie = signed_in_cookie(&server, "alice", "secret").await;

    let post = |path: &str, body: serde_json::Value| {
        client()
            .post(server.url(path))
            .header(COOKIE, &cookie)
            .json(&body)
            .send()
    };

    for _ in 0..2 {
        assert_eq!(
            put_recipe(&server, "Logged", Some(&cookie)).await.status(),
            StatusCode::OK
        );
    }
    let deleted = client()
        .delete(server.url("/api/recipes/Logged.cook"))
        .header(COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(deleted.status(), StatusCode::OK);

    let changes = [
        (
            "/api/shopping_list/add",
            serde_json::json!({ "path": "Recipe.cook", "scale": 2.0 }),
        ),
        (
            "/api/shopping_list/check",
            serde_json::json!({ "name": " flour " }),
        ),
        (
            "/api/pantry/add",
            serde_json::json!({ "section": "dairy", "name": "milk" }),
        ),
        ("/api/shopping_list/clear", serde_json::json!({})),
    ];
    for (path, body) in changes {
        assert_eq!(
            post(path, body).await.unwrap().status(),
            StatusCode::OK,
            "{path}"
        );
    }
    // A name that would write a second line to the checked log is refused,
    // and a refused change is not logged.
    let forged = post(
        "/api/shopping_list/check",
        serde_json::json!({ "name": "flour\n2026-01-01 00:00:00 bob deleted it all" }),
    );
    assert_eq!(forged.await.unwrap().status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        sign_out(&server, &cookie).await.status(),
        StatusCode::SEE_OTHER
    );

    assert_eq!(
        server.fixture.activity(),
        [
            "guest failed to sign in",
            "alice signed in",
            "alice created recipe \"Logged.cook\"",
            "alice updated recipe \"Logged.cook\"",
            "alice deleted recipe \"Logged.cook\"",
            "alice added \"Recipe.cook\" ×2 to the shopping list",
            "alice checked off \"flour\" on the shopping list",
            "alice added \"milk\" to the \"dairy\" section of the pantry",
            "alice cleared the shopping list",
            "alice signed out",
        ]
    );
    let stdout = std::fs::read_to_string(server.fixture.stdout_path()).unwrap();
    assert!(!stdout.contains("MyPasswordTyped"), "{stdout}");
}

// --- Roles -------------------------------------------------------------------

/// Whether a signed-in user's request was turned away for their role. Anything
/// else — success, or a handler's own complaint about an empty body — means
/// the middleware let it through.
fn refused_for_role(status: StatusCode) -> bool {
    assert_ne!(status, StatusCode::UNAUTHORIZED, "the session should hold");
    status == StatusCode::FORBIDDEN
}

#[tokio::test]
async fn each_role_can_do_only_what_it_allows() {
    let fixture = Fixture::new();
    fixture.write_roles(&[
        ("rita", "pw", "reader"),
        ("sam", "pw", "shopper"),
        ("eddie", "pw", "editor"),
        ("ada", "pw", "admin"),
    ]);
    let server = start(fixture, &[], &[]).await;
    let http = client();
    let recipe = server.fixture.recipes.path().join("Recipe.cook");

    // (user, lists, recipes, admin)
    for (user, lists, recipes, admin) in [
        ("rita", false, false, false),
        ("sam", true, false, false),
        ("eddie", true, true, false),
        ("ada", true, true, true),
    ] {
        let cookie = signed_in_cookie(&server, user, "pw").await;
        let post = |path: &str, body: serde_json::Value| {
            http.post(server.url(path))
                .header(COOKIE, &cookie)
                .json(&body)
                .send()
        };

        // The shopping list and the pantry.
        let add = post(
            "/api/shopping_list/add",
            serde_json::json!({ "path": "Recipe.cook", "scale": 1.0 }),
        )
        .await
        .unwrap();
        assert_eq!(refused_for_role(add.status()), !lists, "{user} list add");
        if !lists {
            let body: serde_json::Value = add.json().await.unwrap();
            assert_eq!(body["error"], "Your role does not allow this change");
        }
        for path in [
            "/api/shopping_list/clear",
            "/api/pantry/add",
            "/api/aisles/changes",
        ] {
            let resp = post(path, serde_json::json!({})).await.unwrap();
            assert_eq!(refused_for_role(resp.status()), !lists, "{user} {path}");
        }

        // Recipes, and the editor pages.
        std::fs::write(&recipe, "Mix @flour{100%g}.\n").unwrap();
        let save = put_recipe(&server, "Recipe.cook", Some(&cookie)).await;
        assert_eq!(refused_for_role(save.status()), !recipes, "{user} save");
        let saved = std::fs::read_to_string(&recipe).unwrap();
        assert_eq!(saved.contains("water"), recipes, "{user}: {saved}");

        let edit = get_page(&server, "/edit/Recipe.cook", Some(&cookie)).await;
        if recipes {
            assert_eq!(edit.status(), StatusCode::OK, "{user} edit page");
            // Whoever may edit gets the language server's completions.
            let page = edit.text().await.unwrap();
            assert!(page.contains("id=\"lsp-status\""), "{user}");
        } else {
            // Not a redirect to sign in, which would loop.
            assert_eq!(edit.status(), StatusCode::FORBIDDEN, "{user} edit page");
            let page = edit.text().await.unwrap();
            assert!(page.contains("Ask whoever runs this server"), "{user}");
            assert!(page.contains(user), "{user} still shown as signed in");
        }
        let new = get_page(&server, "/new", Some(&cookie)).await;
        assert_eq!(refused_for_role(new.status()), !recipes, "{user} /new");

        // The editor's language server.
        let lsp = get_page(&server, "/api/ws/lsp", Some(&cookie)).await;
        assert_eq!(refused_for_role(lsp.status()), !recipes, "{user} lsp");

        // Renaming, which an editor may undo again.
        let renamed = server.fixture.recipes.path().join("Renamed.cook");
        let rename = post(
            "/api/recipe_rename/Recipe.cook",
            serde_json::json!({ "name": "Renamed" }),
        )
        .await
        .unwrap();
        assert_eq!(refused_for_role(rename.status()), !recipes, "{user} rename");
        assert_eq!(renamed.exists(), recipes, "{user} rename");
        if recipes {
            let back = post(
                "/api/recipe_rename/Renamed.cook",
                serde_json::json!({ "name": "Recipe" }),
            )
            .await
            .unwrap();
            assert_eq!(back.status(), StatusCode::OK, "{user} rename back");
        }
        assert!(recipe.exists(), "{user}");

        // Sync.
        #[cfg(feature = "sync")]
        {
            let status = get_page(&server, "/api/sync/status", Some(&cookie)).await;
            assert_eq!(refused_for_role(status.status()), !admin, "{user} sync");
            let logout = post("/api/sync/logout", serde_json::json!({}))
                .await
                .unwrap();
            assert_eq!(
                refused_for_role(logout.status()),
                !admin,
                "{user} sync logout"
            );
            let preferences = get_page(&server, "/preferences", Some(&cookie))
                .await
                .text()
                .await
                .unwrap();
            assert_eq!(preferences.contains("id=\"sync-section\""), admin, "{user}");
        }

        // Only an admin is told where the server keeps its files.
        assert_eq!(
            preferences_paths(&server, Some(&cookie)).await,
            (admin, !admin),
            "{user} preferences paths"
        );

        // Pages show exactly the controls the role can use.
        let page = get_page(&server, "/recipe/Recipe.cook", Some(&cookie))
            .await
            .text()
            .await
            .unwrap();
        assert_eq!(page.contains("href=\"/edit/"), recipes, "{user} Edit");
        assert_eq!(
            page.contains("onclick=\"addToShoppingList("),
            lists,
            "{user} add to list"
        );
        assert!(page.contains(&format!("window.__CAN_EDIT_LISTS__ = {lists}")));
        let home = get_page(&server, "/", Some(&cookie))
            .await
            .text()
            .await
            .unwrap();
        assert_eq!(home.contains("href=\"/new\""), recipes, "{user} New Recipe");
    }
}

#[tokio::test]
async fn a_changed_role_applies_without_signing_in_again() {
    let fixture = Fixture::new();
    fixture.write_roles(&[("bob", "pw", "editor")]);
    let server = start(fixture, &[], &[]).await;
    let bob = signed_in_cookie(&server, "bob", "pw").await;
    assert!(put_recipe(&server, "Recipe.cook", Some(&bob))
        .await
        .status()
        .is_success());

    assert_success(
        &server
            .fixture
            .run(&["server", "user", "role", "bob", "reader"], ""),
    );
    eventually("bob can no longer edit", || async {
        put_recipe(&server, "Recipe.cook", Some(&bob))
            .await
            .status()
            == StatusCode::FORBIDDEN
    })
    .await;

    assert_success(
        &server
            .fixture
            .run(&["server", "user", "role", "bob", "editor"], ""),
    );
    eventually("bob can edit again", || async {
        put_recipe(&server, "Recipe.cook", Some(&bob))
            .await
            .status()
            .is_success()
    })
    .await;
}

#[tokio::test]
async fn a_refused_page_is_translated_throughout() {
    let fixture = Fixture::new();
    fixture.write_roles(&[("bob", "pw", "reader")]);
    let server = start(fixture, &[], &[]).await;
    let bob = signed_in_cookie(&server, "bob", "pw").await;

    let resp = client()
        .get(server.url("/new"))
        .header(COOKIE, &bob)
        .header(ACCEPT_LANGUAGE, "fr-FR")
        .send()
        .await
        .expect("page request");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let page = resp.text().await.unwrap();
    // The title, the message and the way back, not just the message.
    assert!(page.contains("Une erreur s"), "{page}");
    assert!(page.contains("Votre compte ne permet pas"), "{page}");
    assert!(page.contains("Retour aux recettes"), "{page}");
    assert!(!page.contains("Something went wrong"), "{page}");
    assert!(!page.contains("Back to recipes"), "{page}");
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
fn refuses_a_users_file_with_an_unknown_role() {
    let fixture = Fixture::new();
    fixture.write_roles(&[("alice", "secret", "chef")]);
    let stderr = startup_failure(&fixture, &[], &[]);
    assert!(stderr.contains("unknown role \"chef\""), "{stderr}");
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
    assert_eq!(String::from_utf8_lossy(&list.stdout).trim(), "bob  admin");

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
    assert_eq!(String::from_utf8_lossy(&list.stdout).trim(), "alice  admin");
}

#[test]
fn user_commands_manage_roles() {
    let fixture = Fixture::new();
    let stdout = |output: &Output| String::from_utf8_lossy(&output.stdout).into_owned();

    // Without --role, a user is an admin, as every user was before roles.
    let alice = fixture.run(&["server", "user", "add", "alice"], "a\n");
    assert_success(&alice);
    assert!(stdout(&alice).contains("as admin"), "{}", stdout(&alice));
    assert_success(&fixture.run(
        &["server", "user", "add", "bob", "--role", "shopper"],
        "b\n",
    ));

    let unknown = fixture.run(&["server", "user", "add", "carol", "--role", "chef"], "c\n");
    assert!(!unknown.status.success());
    let stderr = String::from_utf8_lossy(&unknown.stderr);
    assert!(
        stderr.contains("reader, shopper, editor, admin"),
        "{stderr}"
    );

    let text = std::fs::read_to_string(fixture.users_file()).unwrap();
    assert!(
        text.lines()
            .any(|line| line.starts_with("alice = \"$argon2id$")),
        "an admin should stay a bare hash: {text}"
    );

    let list = fixture.run(&["server", "user", "list"], "");
    assert_eq!(stdout(&list), "alice  admin\nbob    shopper\n");

    // A new role keeps the password, and a new password keeps the role.
    let before = std::fs::read_to_string(fixture.users_file()).unwrap();
    let changed = fixture.run(&["server", "user", "role", "bob", "editor"], "");
    assert_success(&changed);
    assert!(stdout(&changed).contains("bob is now editor"));
    let after = std::fs::read_to_string(fixture.users_file()).unwrap();
    let hash_of = |text: &str| {
        text.lines()
            .find(|line| line.starts_with("bob"))
            .and_then(|line| line.split('"').nth(1))
            .map(str::to_owned)
    };
    assert_eq!(hash_of(&before), hash_of(&after));

    assert_success(&fixture.run(&["server", "user", "passwd", "bob"], "new\n"));
    let list = fixture.run(&["server", "user", "list"], "");
    assert_eq!(stdout(&list), "alice  admin\nbob    editor\n");

    let missing = fixture.run(&["server", "user", "role", "nobody", "reader"], "");
    assert!(!missing.status.success());
    let bad = fixture.run(&["server", "user", "role", "bob", "boss"], "");
    assert!(!bad.status.success());
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

// --- --recipes-only -----------------------------------------------------------

/// The fixture, plus a picture, a menu and a shopping list beside the recipe.
fn recipes_only_fixture() -> Fixture {
    let fixture = Fixture::new();
    let recipes = fixture.recipes.path();
    std::fs::write(recipes.join("Recipe.jpg"), b"not really a jpeg").unwrap();
    std::fs::write(
        recipes.join("Week.menu"),
        "== Monday ==\n\nDinner:\n- @./Recipe{}\n",
    )
    .unwrap();
    std::fs::write(recipes.join(".shopping-list"), "./Recipe.cook\n").unwrap();
    fixture
}

async fn status(server: &ServerGuard, path: &str, cookie: Option<&str>) -> StatusCode {
    get_page(server, path, cookie).await.status()
}

/// What a recipes-only visitor may still open.
const RECIPE_PAGES: &[&str] = &[
    "/",
    "/recipe/Recipe.cook",
    "/recipe/Week.menu",
    "/preferences",
    "/rss.xml",
    "/static/css/output.css",
    "/api/recipes",
    "/api/recipes/Recipe.cook",
    "/api/recipes/raw/Recipe.cook",
    "/api/search?q=flour",
    "/api/static/Recipe.jpg",
];

/// What they may not.
const OTHER_PAGES: &[&str] = &[
    "/shopping-list",
    "/pantry",
    "/edit/Recipe.cook",
    "/new",
    "/api-docs",
    "/api/shopping_list/items",
    "/api/pantry",
    "/api/menus",
    "/api/stats",
    "/api/static/.shopping-list",
    "/api/static/config/pantry.conf",
];

#[tokio::test]
async fn recipes_only_without_users_shows_everyone_only_recipes() {
    let server = start(recipes_only_fixture(), &["--recipes-only"], &[]).await;

    for path in RECIPE_PAGES {
        assert_eq!(status(&server, path, None).await, StatusCode::OK, "{path}");
    }
    // Nobody can sign in, so to the visitor the rest does not exist.
    for path in OTHER_PAGES {
        assert_eq!(
            status(&server, path, None).await,
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
    let add = client()
        .post(server.url("/api/shopping_list/add"))
        .json(&serde_json::json!([]))
        .send()
        .await
        .unwrap();
    assert_eq!(add.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        std::fs::read_to_string(server.fixture.recipes.path().join(".shopping-list")).unwrap(),
        "./Recipe.cook\n"
    );

    // No way to the rest from the pages either.
    for path in ["/", "/recipe/Recipe.cook", "/recipe/Week.menu"] {
        let html = get_page(&server, path, None).await.text().await.unwrap();
        for hidden in [
            "<a href=\"/shopping-list\"",
            "<a href=\"/pantry\"",
            "href=\"/edit/",
            "href=\"/new",
            "onclick=\"addToShoppingList",
        ] {
            assert!(!html.contains(hidden), "{path} shows {hidden}");
        }
        assert!(html.contains("window.__RECIPES_ONLY__ = true"), "{path}");
    }

    // The preferences page is down to the language picker.
    let preferences = get_page(&server, "/preferences", None)
        .await
        .text()
        .await
        .unwrap();
    assert!(preferences.contains("setLanguage("));
    let base_path = server.fixture.recipes.path().to_str().unwrap();
    for hidden in [
        base_path,
        "onclick=\"toggleFeature(",
        "CookCloud",
        "/api-docs",
    ] {
        assert!(!preferences.contains(hidden), "preferences show {hidden}");
    }
}

#[tokio::test]
async fn recipes_only_asks_guests_to_sign_in_for_the_rest() {
    let fixture = recipes_only_fixture();
    fixture.write_roles(&[("alice", "secret", "shopper")]);
    let server = start(fixture, &["--recipes-only"], &[]).await;

    for path in RECIPE_PAGES {
        assert_eq!(status(&server, path, None).await, StatusCode::OK, "{path}");
    }
    let page = get_page(&server, "/shopping-list", None).await;
    assert_eq!(page.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&page), "/login?next=%2Fshopping-list");
    assert_eq!(
        status(&server, "/api/shopping_list/items", None).await,
        StatusCode::UNAUTHORIZED
    );
    let home = get_page(&server, "/", None).await.text().await.unwrap();
    assert!(home.contains("Sign in"), "no sign-in link");
    assert!(!home.contains("<a href=\"/shopping-list\""));

    // Signed in, the role decides as usual.
    let cookie = signed_in_cookie(&server, "alice", "secret").await;
    for path in ["/shopping-list", "/pantry", "/api/menus"] {
        assert_eq!(
            status(&server, path, Some(&cookie)).await,
            StatusCode::OK,
            "{path}"
        );
    }
    let home = get_page(&server, "/", Some(&cookie))
        .await
        .text()
        .await
        .unwrap();
    assert!(home.contains("<a href=\"/shopping-list\""));
    assert!(home.contains("window.__RECIPES_ONLY__ = false"));
    let recipe = get_page(&server, "/recipe/Recipe.cook", Some(&cookie))
        .await
        .text()
        .await
        .unwrap();
    assert!(recipe.contains("onclick=\"addToShoppingList"));
    // A shopper still may not edit.
    assert_eq!(
        status(&server, "/edit/Recipe.cook", Some(&cookie)).await,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn recipes_only_can_come_from_the_environment() {
    let server = start(
        recipes_only_fixture(),
        &[],
        &[("COOK_RECIPES_ONLY", "true")],
    )
    .await;
    assert_eq!(
        status(&server, "/pantry", None).await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(status(&server, "/", None).await, StatusCode::OK);

    let server = start(recipes_only_fixture(), &[], &[("COOK_RECIPES_ONLY", "")]).await;
    assert_eq!(status(&server, "/pantry", None).await, StatusCode::OK);
}

// --- Files that are not recipes (#658) ----------------------------------------

/// Stands for a token in a dot-file, which no response may contain.
const TOKEN: &str = "gho_FAKE_token_marker";

/// The fixture, plus the dot-files a recipe directory that is a git checkout
/// or part of a home directory holds, an aisle file, and a picture.
fn hidden_files_fixture() -> Fixture {
    let fixture = Fixture::new();
    let recipes = fixture.recipes.path();
    std::fs::write(recipes.join("Recipe.jpg"), b"not really a jpeg").unwrap();
    std::fs::create_dir_all(recipes.join(".git")).unwrap();
    std::fs::write(
        recipes.join(".git/config"),
        format!("[remote \"origin\"]\nurl = https://me:{TOKEN}@github.com/me/recipes\n"),
    )
    .unwrap();
    std::fs::write(recipes.join(".git/shot.png"), TOKEN).unwrap();
    std::fs::create_dir_all(recipes.join(".config/gh")).unwrap();
    std::fs::write(
        recipes.join(".config/gh/hosts.yml"),
        format!("github.com:\n    oauth_token: {TOKEN}\n"),
    )
    .unwrap();
    std::fs::write(recipes.join(".env"), format!("TOKEN={TOKEN}\n")).unwrap();
    std::fs::write(
        recipes.join("config/aisle.conf"),
        format!("[{TOKEN}]\nflour\n"),
    )
    .unwrap();
    std::fs::write(recipes.join("notes.html"), TOKEN).unwrap();
    fixture
}

/// Requests for the fixture's other files, through each route that takes a
/// path. The pantry file holds `flour = "1%kg"`.
const NOT_RECIPES: &[&str] = &[
    "/api/static/.git/config",
    "/api/static/%2Egit/config",
    "/api/static/.git/shot.png",
    "/api/static/.config/gh/hosts.yml",
    "/api/static/.env",
    "/api/static/config/pantry.conf",
    "/api/static/config/aisle.conf",
    "/api/static/notes.html",
    "/api/static/Recipe.cook",
    "/recipe/.git/config",
    "/recipe/.config/gh/hosts.yml",
    "/recipe/.env",
    "/recipe/config/pantry.conf",
    "/recipe/config/aisle.conf",
    "/recipe/notes.html",
    "/api/recipes/.config/gh/hosts.yml",
    "/api/recipes/config/pantry.conf",
    "/api/recipes/config/aisle.conf",
    "/api/recipes/notes.html",
];

async fn assert_not_served(server: &ServerGuard, cookie: Option<&str>) {
    for path in NOT_RECIPES {
        let response = get_page(server, path, cookie).await;
        let status = response.status();
        let body = response.text().await.unwrap();
        assert!(
            !body.contains(TOKEN) && !body.contains("1%kg") && !body.contains("1 kg"),
            "{path} ({status}) served the file: {body}"
        );
        if path.starts_with("/api/") {
            assert!(status.is_client_error(), "{path}: {status}");
        }
    }
}

/// What must still be served beside them.
async fn assert_recipes_served(server: &ServerGuard, cookie: Option<&str>) {
    for path in [
        "/recipe/Recipe",
        "/recipe/Recipe.cook",
        "/api/recipes/Recipe",
        "/api/recipes/Recipe.cook",
        "/api/static/Recipe.jpg",
    ] {
        assert_eq!(status(server, path, cookie).await, StatusCode::OK, "{path}");
    }
}

#[tokio::test]
async fn only_recipes_and_pictures_are_served_without_sign_in() {
    let server = start(hidden_files_fixture(), &[], &[]).await;
    assert_not_served(&server, None).await;
    assert_recipes_served(&server, None).await;
}

#[tokio::test]
async fn only_recipes_and_pictures_are_served_with_sign_in() {
    let fixture = hidden_files_fixture();
    fixture.write_roles(&[("root", "secret", "admin")]);
    let server = start(fixture, &[], &[]).await;

    assert_not_served(&server, None).await;
    assert_recipes_served(&server, None).await;
    // Not even to an admin: what the server is for is recipes and pictures.
    let cookie = signed_in_cookie(&server, "root", "secret").await;
    assert_not_served(&server, Some(&cookie)).await;
    assert_recipes_served(&server, Some(&cookie)).await;
}

#[tokio::test]
async fn only_recipes_and_pictures_are_served_to_recipes_only_guests() {
    let server = start(hidden_files_fixture(), &["--recipes-only"], &[]).await;
    assert_not_served(&server, None).await;
    assert_recipes_served(&server, None).await;
}
