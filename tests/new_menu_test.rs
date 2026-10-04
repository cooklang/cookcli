//! End-to-end tests for creating a menu from the web UI (#538).
//!
//! `POST /new` takes a `kind`: `recipe` (the default) makes a `.cook` file,
//! `menu` a `.menu` file with a first day and meal to fill in. Anything else
//! is refused, and the path checks are the same for both kinds: a name that
//! resolves out of the collection, through a symlinked sub-folder, is refused
//! without creating or removing anything (#549).

#![cfg(feature = "server")]

use reqwest::{redirect, StatusCode};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

#[path = "common/mod.rs"]
mod common;

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

    fn recipes(&self) -> PathBuf {
        self.dir.path().join("recipes")
    }

    /// `POST /new` as the server's own page would send it, without following
    /// the redirect.
    async fn create(&self, fields: &[(&str, &str)]) -> reqwest::Response {
        client()
            .post(self.url("/new"))
            .header("origin", format!("http://127.0.0.1:{}", self.port))
            .form(fields)
            .send()
            .await
            .expect("new file request")
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

/// A client that reports redirects instead of following them.
fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(redirect::Policy::none())
        .build()
        .unwrap()
}

fn location(resp: &reqwest::Response) -> String {
    resp.headers()["location"].to_str().unwrap().to_string()
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
    let recipes = dir.path().join("recipes");
    std::fs::create_dir_all(recipes.join("Plans")).unwrap();
    std::fs::write(recipes.join("Omelette.cook"), "Beat @eggs{3}.\n").unwrap();

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
async fn a_menu_is_created_with_a_first_day_and_meal() {
    let server = start_server().await;

    let resp = server
        .create(&[("filename", "Plans/Week 12"), ("kind", "menu")])
        .await;

    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&resp), "/edit/Plans/Week 12.menu");
    let content = std::fs::read_to_string(server.recipes().join("Plans/Week 12.menu"))
        .expect("the menu file must exist");
    assert_eq!(
        content,
        "---\ntitle: Week 12\nservings: 2\n---\n\n== Day 1 ==\n\nBreakfast: \\\n- \n"
    );
    assert!(!server.recipes().join("Plans/Week 12.cook").exists());

    // The editor opens it in menu mode.
    let page = client()
        .get(server.url("/edit/Plans/Week%2012.menu"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains(r#"data-mode="menu""#), "{page}");
    assert!(page.contains(r#"data-action="add-recipe""#));
}

/// The listing links menus without their extension, so the menu page's Edit
/// button asks for `/edit/Week`. The editor must still open the file as a
/// menu, under its full name.
#[tokio::test]
async fn the_editor_redirects_a_bare_name_to_the_file() {
    let server = start_server().await;
    std::fs::write(
        server.recipes().join("Plans/Week 12.menu"),
        "== Day 1 ==\n\nDinner: \\\n- @./Omelette{}\n",
    )
    .unwrap();

    for (bare, full) in [
        ("/edit/Plans/Week%2012", "/edit/Plans/Week%2012.menu"),
        ("/edit/Omelette", "/edit/Omelette.cook"),
    ] {
        let resp = client().get(server.url(bare)).send().await.unwrap();
        assert_eq!(resp.status(), StatusCode::SEE_OTHER, "{bare}");
        assert_eq!(location(&resp), full, "{bare}");
    }

    // A full name is served as it is.
    let resp = client()
        .get(server.url("/edit/Plans/Week%2012.menu"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn a_recipe_is_still_the_default() {
    let server = start_server().await;

    for (name, kind) in [("Pancakes", Some("recipe")), ("Waffles", None)] {
        let mut fields = vec![("filename", name)];
        if let Some(kind) = kind {
            fields.push(("kind", kind));
        }
        let resp = server.create(&fields).await;

        assert_eq!(resp.status(), StatusCode::SEE_OTHER, "{name}");
        assert_eq!(location(&resp), format!("/edit/{name}.cook"));
        let content = std::fs::read_to_string(server.recipes().join(format!("{name}.cook")))
            .expect("the recipe file must exist");
        assert_eq!(content, format!("---\ntitle: {name}\n---\n\n"));
        assert!(!server.recipes().join(format!("{name}.menu")).exists());
    }

    let page = client()
        .get(server.url("/edit/Omelette.cook"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains(r#"data-mode="recipe""#));
    assert!(page.contains(r#"data-action="recipe-reference""#));
    assert!(!page.contains(r#"data-action="add-recipe""#));
}

/// Spaces typed around a name or a folder are not part of it: they used to
/// make a folder `Mains ` holding ` Stew .cook`.
#[tokio::test]
async fn spaces_around_names_and_folders_are_dropped() {
    let server = start_server().await;

    for (typed, kind, file) in [
        (" Mains / Beef  Stew ", "recipe", "Mains/Beef  Stew.cook"),
        ("  Plans /Week 13  ", "menu", "Plans/Week 13.menu"),
    ] {
        let resp = server.create(&[("filename", typed), ("kind", kind)]).await;

        assert_eq!(resp.status(), StatusCode::SEE_OTHER, "{typed:?}");
        assert_eq!(location(&resp), format!("/edit/{file}"), "{typed:?}");
        assert!(server.recipes().join(file).is_file(), "{file} must exist");
    }
    // Listed rather than probed with `exists()`: Windows drops trailing
    // spaces when it resolves a path, so `Mains ` would find `Mains`.
    let padded: Vec<String> = std::fs::read_dir(server.recipes())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.trim() != name)
        .collect();
    assert!(padded.is_empty(), "{padded:?}");

    let content = std::fs::read_to_string(server.recipes().join("Mains/Beef  Stew.cook")).unwrap();
    assert_eq!(content, "---\ntitle: Beef  Stew\n---\n\n");

    // Nothing but spaces and slashes is still an empty name.
    let resp = server
        .create(&[("filename", " / "), ("kind", "recipe")])
        .await;
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert!(
        location(&resp).starts_with("/new?error="),
        "{}",
        location(&resp)
    );
}

#[tokio::test]
async fn an_unknown_kind_is_refused() {
    let server = start_server().await;

    for kind in ["report", "", "../menu", "MENU"] {
        let resp = server
            .create(&[("filename", "Mystery"), ("kind", kind)])
            .await;

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "kind {kind:?}");
        let names: Vec<_> = std::fs::read_dir(server.recipes())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert!(
            !names.iter().any(|name| name.starts_with("Mystery")),
            "kind {kind:?} created {names:?}"
        );
    }
}

#[tokio::test]
async fn a_menu_name_cannot_leave_the_collection() {
    let server = start_server().await;

    // Dots are stripped from the name, so this lands inside the collection.
    let resp = server
        .create(&[("filename", "../../Escape"), ("kind", "menu")])
        .await;
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&resp), "/edit/Escape.menu");
    assert!(server.recipes().join("Escape.menu").exists());
    assert!(!server.dir.path().join("Escape.menu").exists());
}

#[cfg(unix)]
#[tokio::test]
async fn a_menu_cannot_be_written_through_a_symlink_out_of_the_collection() {
    let server = start_server().await;
    let outside = server.dir.path().join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("Other.menu"), "").unwrap();
    std::os::unix::fs::symlink(&outside, server.recipes().join("Linked")).unwrap();

    let resp = server
        .create(&[("filename", "Linked/Week"), ("kind", "menu")])
        .await;

    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    let to = location(&resp);
    assert!(to.starts_with("/new?error="), "{to}");
    // Back to the menu form, not the recipe one.
    assert!(to.ends_with("&kind=menu"), "{to}");
    assert!(!outside.join("Week.menu").exists());
    // Refusing is all it does: the link, and the folder it points at, are
    // left alone.
    assert!(std::fs::symlink_metadata(server.recipes().join("Linked"))
        .unwrap()
        .is_symlink());
    assert!(outside.join("Other.menu").exists());
}

/// #549: a sub-folder symlinked out of the collection (a NAS share) held a
/// folder the refused request had not created, and the "clean-up" deleted it.
#[cfg(unix)]
#[tokio::test]
async fn a_refused_recipe_leaves_the_symlinked_folder_alone() {
    let server = start_server().await;
    let nas = server.dir.path().join("nas");
    std::fs::create_dir_all(nas.join("Desserts")).unwrap();
    std::fs::write(
        nas.join("Desserts/Tiramisu.cook"),
        "Soak @ladyfingers{12}.\n",
    )
    .unwrap();
    std::os::unix::fs::symlink(&nas, server.recipes().join("shared")).unwrap();

    let resp = server
        .create(&[("filename", "shared/Desserts/Cheesecake")])
        .await;

    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    let to = location(&resp);
    assert!(to.starts_with("/new?error="), "{to}");
    assert!(!nas.join("Desserts/Cheesecake.cook").exists());
    assert_eq!(
        std::fs::read_to_string(nas.join("Desserts/Tiramisu.cook")).unwrap(),
        "Soak @ladyfingers{12}.\n"
    );
}

/// The check runs before anything is created, so a refused request does not
/// leave new folders behind outside the collection either.
#[cfg(unix)]
#[tokio::test]
async fn a_refused_recipe_creates_no_folder_outside_the_collection() {
    let server = start_server().await;
    let nas = server.dir.path().join("nas");
    std::fs::create_dir_all(&nas).unwrap();
    std::os::unix::fs::symlink(&nas, server.recipes().join("shared")).unwrap();

    let resp = server
        .create(&[("filename", "shared/Cakes/Sponge/Victoria")])
        .await;

    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert!(location(&resp).starts_with("/new?error="));
    assert!(nas.exists());
    assert!(!nas.join("Cakes").exists());
}

/// New folders inside the collection are still created as before.
#[tokio::test]
async fn a_recipe_can_start_a_new_folder() {
    let server = start_server().await;

    let resp = server
        .create(&[("filename", "Cakes/Sponge/Victoria")])
        .await;

    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&resp), "/edit/Cakes/Sponge/Victoria.cook");
    assert!(server.recipes().join("Cakes/Sponge/Victoria.cook").exists());
}

#[tokio::test]
async fn errors_send_the_user_back_to_the_menu_form() {
    let server = start_server().await;
    std::fs::write(server.recipes().join("Taken.menu"), "").unwrap();

    let resp = server
        .create(&[("filename", "Taken"), ("kind", "menu")])
        .await;

    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    let to = location(&resp);
    assert!(
        to.contains("A%20menu%20with%20this%20name%20already%20exists"),
        "{to}"
    );
    assert!(to.ends_with("&kind=menu"), "{to}");
    assert_eq!(
        std::fs::read_to_string(server.recipes().join("Taken.menu")).unwrap(),
        ""
    );
}

#[tokio::test]
async fn the_form_and_the_listing_offer_menus() {
    let server = start_server().await;

    let form = client()
        .get(server.url("/new?kind=menu"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(form.contains(r#"name="kind" value="menu""#));
    assert!(form.contains(".menu</span>"));

    let form = client()
        .get(server.url("/new"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(form.contains(r#"name="kind" value="recipe""#));
    assert!(form.contains(".cook</span>"));

    // The New menu button keeps the folder, like New Recipe.
    let listing = client()
        .get(server.url("/directory/Plans"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(
        listing.contains(r#"href="/new?kind=menu&amp;filename=Plans%2F""#)
            || listing.contains(r#"href="/new?kind=menu&filename=Plans%2F""#),
        "{listing}"
    );
}

// -- Meal plans (#385): a menu with sections on two days or more --

/// The fields the new-plan form posts for a three-day plan from Wednesday
/// 7 October 2026, with breakfast and dinner.
fn plan_fields(filename: &str) -> Vec<(&str, &str)> {
    vec![
        ("filename", filename),
        ("kind", "plan"),
        ("start", "2026-10-07"),
        ("days", "3"),
        ("servings", "4"),
        ("breakfast", "on"),
        ("dinner", "on"),
    ]
}

#[tokio::test]
async fn a_plan_is_created_with_a_section_a_day() {
    let server = start_server().await;

    let resp = server.create(&plan_fields("Plans/Fortnight")).await;

    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&resp), "/edit/Plans/Fortnight.menu");
    let content = std::fs::read_to_string(server.recipes().join("Plans/Fortnight.menu"))
        .expect("the plan file must exist");
    let day = |heading: &str| format!("\n== {heading} ==\n\nBreakfast: \\\n- \n\nDinner: \\\n- \n");
    assert_eq!(
        content,
        format!(
            "---\ntitle: Fortnight\nservings: 4\n---\n{}{}{}",
            day("Wednesday (2026-10-07)"),
            day("Thursday (2026-10-08)"),
            day("Friday (2026-10-09)"),
        )
    );

    // Its empty bullets show as empty meals on the planner.
    let page = client()
        .get(server.url("/recipe/Plans/Fortnight"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(page.matches("Nothing planned").count(), 6, "{page}");
    assert!(!page.contains("Outside this plan"));
}

#[tokio::test]
async fn a_plan_names_its_days_and_meals_in_the_page_language() {
    let server = start_server().await;

    let resp = client()
        .post(server.url("/new"))
        .header("origin", format!("http://127.0.0.1:{}", server.port))
        .header("accept-language", "fr-FR")
        .form(&plan_fields("Semaine"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    let content = std::fs::read_to_string(server.recipes().join("Semaine.menu")).unwrap();
    assert!(
        content.contains("\n== Mercredi (2026-10-07) ==\n\nPetit-déjeuner: \\\n- \n"),
        "{content}"
    );
}

#[tokio::test]
async fn a_plan_the_form_got_wrong_goes_back_with_its_choices() {
    let server = start_server().await;

    for (field, value, error) in [
        ("days", "1", "A%20plan%20lasts%20from%202%20to%2062%20days"),
        ("days", "63", "A%20plan%20lasts%20from%202%20to%2062%20days"),
        (
            "start",
            "2026-02-30",
            "Pick%20the%20day%20the%20plan%20starts",
        ),
        (
            "servings",
            "0",
            "Servings%20must%20be%20a%20whole%20number%20above%20zero",
        ),
    ] {
        let mut fields = plan_fields("Wrong");
        fields.retain(|(name, _)| *name != field);
        fields.push((field, value));

        let resp = server.create(&fields).await;

        assert_eq!(resp.status(), StatusCode::SEE_OTHER);
        let to = location(&resp);
        assert!(to.contains(error), "{field}={value}: {to}");
        assert!(to.contains("&kind=plan"), "{to}");
        assert!(to.contains(&format!("&{field}={value}")), "{to}");
        assert!(
            to.contains("&breakfast=on") && to.contains("&dinner=on"),
            "{to}"
        );
        assert!(!to.contains("lunch"), "{to}");
    }

    let mut no_meals = plan_fields("Wrong");
    no_meals.retain(|(name, _)| !matches!(*name, "breakfast" | "dinner"));
    let to = location(&server.create(&no_meals).await);
    assert!(to.contains("Pick%20at%20least%20one%20meal"), "{to}");

    assert!(!server.recipes().join("Wrong.menu").exists());

    // A taken name keeps the frame too, and leaves the file alone.
    std::fs::write(server.recipes().join("Taken.menu"), "").unwrap();
    let to = location(&server.create(&plan_fields("Taken")).await);
    assert!(
        to.contains("A%20plan%20with%20this%20name%20already%20exists"),
        "{to}"
    );
    assert!(to.contains("&start=2026-10-07&days=3&servings=4"), "{to}");
    assert_eq!(
        std::fs::read_to_string(server.recipes().join("Taken.menu")).unwrap(),
        ""
    );

    // And the form shows them again.
    let form = client()
        .get(server.url(&format!("/new{}", &to[to.find('?').unwrap()..])))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(
        form.contains(r#"name="start" required value="2026-10-07""#),
        "{form}"
    );
    assert!(form.contains(r#"value="3""#), "{form}");
    assert!(form.contains(r#"name="dinner" checked"#), "{form}");
    assert!(!form.contains(r#"name="lunch" checked"#), "{form}");
}

#[tokio::test]
async fn the_planner_lays_a_plan_out_by_day() {
    let server = start_server().await;
    std::fs::write(
        server.recipes().join("Week.menu"),
        "== Wednesday (2026-10-07) ==\n\nBreakfast: \\\n- \n\n\
         == Thursday (2026-10-08) ==\n\nDinner: \\\n- @./Omelette{} \\\n- @salad{1%bowl}\n\n\
         == Day 1 ==\n\nLunch: \\\n- @bread{}\n\n\
         = 2026-10-09 Dinner\n\n- @soup{}\n",
    )
    .unwrap();
    std::fs::write(
        server.recipes().join("Plain.menu"),
        "== Thursday (2026-10-08) ==\n\nDinner: \\\n- @./Omelette{}\n",
    )
    .unwrap();

    let page = client()
        .get(server.url("/recipe/Week"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    assert!(page.contains(r#"id="plan""#), "{page}");
    assert_eq!(
        page.matches(r#"class="plan-day card p-3" data-date="#)
            .count(),
        3
    );
    // The page is in American English, where weeks start on Sunday:
    // Wednesday is the fourth column.
    let plan = &page[page.find(r#"id="plan""#).unwrap()..];
    let cells: Vec<&str> = plan.split("<li class=").skip(1).collect();
    assert!(cells[..3]
        .iter()
        .all(|cell| cell.starts_with(r#""hidden md:block""#)));
    assert!(cells[3].contains(r#"data-date="2026-10-07""#));
    let thursday = cells[4];
    assert!(
        thursday.contains(r#"<time datetime="2026-10-08">Thu 8 Oct</time>"#),
        "{thursday}"
    );
    assert!(
        thursday.contains(r#"href="/recipe/Omelette""#),
        "{thursday}"
    );
    assert!(thursday.contains("salad"), "{thursday}");
    // Every day offers the meals the plan names.
    assert!(thursday.contains(">Breakfast</h3>"), "{thursday}");
    assert!(cells[5].contains(r#"data-date="2026-10-09""#));
    assert!(cells[5].contains(">Dinner</h3>") && cells[5].contains("soup"));
    // Bullets are the file's layout, not something to show.
    assert!(!thursday.contains(">- <"), "{thursday}");
    // The undated section is not lost.
    assert!(page.contains("Outside this plan"));
    assert!(page.contains("Day 1"));
    assert!(page.contains("Meal Plan</span>"));

    // A menu with a single dated day still shows one card a section.
    let page = client()
        .get(server.url("/recipe/Plain"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(!page.contains(r#"id="plan""#));
    assert!(page.contains("Thursday (2026-10-08)"));
    assert!(page.contains(r#"href="/recipe/Omelette""#));
}

#[tokio::test]
async fn empty_dated_sections_are_days_of_the_plan() {
    let server = start_server().await;
    std::fs::write(
        server.recipes().join("Bare.menu"),
        "== Wednesday (2026-10-07) ==\n\nDinner: \\\n- @./Omelette{}\n\n\
         == Thursday (2026-10-08) ==\n\n== Friday (2026-10-09) ==\n",
    )
    .unwrap();
    std::fs::write(
        server.recipes().join("Party.menu"),
        "== Party (2026-12-31) ==\n\n== Food ==\n\n- @./Omelette{}\n",
    )
    .unwrap();

    let page = client()
        .get(server.url("/recipe/Bare"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    // The plan runs to its last heading, even with nothing under it.
    assert_eq!(
        page.matches(r#"class="plan-day card p-3" data-date="#)
            .count(),
        3,
        "{page}"
    );
    assert!(page.contains(r#"data-date="2026-10-09""#));

    // In an ordinary menu an empty section still shows no card.
    let page = client()
        .get(server.url("/recipe/Party"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(!page.contains(r#"id="plan""#));
    assert!(!page.contains("Party (2026-12-31)"), "{page}");
}

#[tokio::test]
async fn the_form_and_the_listing_offer_plans() {
    let server = start_server().await;

    let form = client()
        .get(server.url("/new?kind=plan"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(form.contains(r#"name="kind" value="plan""#));
    assert!(form.contains(".menu</span>"));
    assert!(form.contains(r#"type="date" id="plan-start" name="start""#));
    assert!(form.contains(r#"name="breakfast" checked"#));
    assert!(form.contains(r#"name="dinner" checked"#));
    assert!(!form.contains(r#"name="snacks" checked"#));

    let listing = client()
        .get(server.url("/directory/Plans"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(
        listing.contains(r#"href="/new?kind=plan&amp;filename=Plans%2F""#)
            || listing.contains(r#"href="/new?kind=plan&filename=Plans%2F""#),
        "{listing}"
    );
}
