//! Characterization tests for `cook report`.
//!
//! # Why this file exists
//!
//! `cook report` had no end-to-end coverage at all before this file: the only
//! test naming it, `basic_test.rs::test_report_templates`, writes two fixture
//! files and asserts they exist without ever running the command. These tests
//! were added ahead of extracting the command into `cookcli-core`, so that
//! refactor could be verified as behaviour-preserving rather than done blind.
//!
//! # These tests assert what the output IS, not what it should be
//!
//! `a_bare_recipe_name_is_not_resolved` in particular records behaviour that is
//! arguably a bug. It is pinned so that changing it has to be deliberate.
//!
//! # Machine independence
//!
//! `report` falls back to a *global* `~/.config/cook/aisle.conf` and
//! `pantry.conf` when no local `./config/` equivalent exists, so output would
//! otherwise depend on the developer's home directory. Every fixture here
//! writes both local files, the empty ones purely to shadow any global copy.

use assert_cmd::Command;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

const PANCAKES: &str = "---\ntitle: Pancakes\n---\n\n\
     Mix @eggs{3%large} with @milk{250%ml} and @flour{125%g}.\n";

/// A working directory with a recipe, a template, and local aisle and pantry
/// files that shadow whatever the developer has in their home directory.
fn fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    let path = dir.path();
    fs::create_dir(path.join("config")).unwrap();
    fs::write(
        path.join("config").join("aisle.conf"),
        "[dairy]\nmilk\neggs\n",
    )
    .unwrap();
    fs::write(path.join("config").join("pantry.conf"), "").unwrap();
    fs::write(path.join("pancakes.cook"), PANCAKES).unwrap();
    write_template(
        path,
        "list.jinja",
        "{{ metadata.title }} x{{ scale }}\
         {% for i in ingredients %}|{{ i.name }}={{ i.quantity }}{% endfor %}",
    );
    dir
}

fn write_template(dir: &Path, name: &str, body: &str) {
    fs::write(dir.join(name), body).unwrap();
}

fn cook(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("cook").unwrap();
    cmd.current_dir(dir.path());
    cmd
}

#[test]
fn a_template_renders_against_the_named_recipe() {
    let dir = fixture();
    let assert = cook(&dir)
        .args(["report", "-t", "list.jinja", "pancakes.cook"])
        .assert()
        .success()
        .stdout("Pancakes x1.0|eggs=3 large|milk=250 ml|flour=125 g\n");

    // The command still announces itself as a prototype, on stderr so that
    // piping the report somewhere does not pick it up.
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("prototype feature"),
        "expected the prototype warning, got: {stderr}"
    );
}

/// The `name:factor` suffix must reach both the `scale` variable and the
/// quantities, or a template that prints them side by side contradicts itself.
#[test]
fn the_scaling_suffix_scales_the_variable_and_the_quantities() {
    let dir = fixture();
    cook(&dir)
        .args(["report", "-t", "list.jinja", "pancakes.cook:2"])
        .assert()
        .success()
        .stdout("Pancakes x2.0|eggs=6 large|milk=500 ml|flour=250 g\n");
}

/// Recorded, not endorsed: `report` opens the recipe argument as a plain path
/// relative to the working directory, where every other command resolves it
/// through `cooklang-find`. So `cook recipe pancakes` works and this does not.
#[test]
fn a_bare_recipe_name_is_not_resolved() {
    let dir = fixture();
    let output = cook(&dir)
        .args(["report", "-t", "list.jinja", "pancakes"])
        .assert()
        .failure()
        .code(1)
        .to_string();
    assert!(
        output.contains("Failed to read recipe file: pancakes"),
        "expected a plain read failure, got: {output}"
    );
}

/// The point of keeping `std::process::exit(1)` in the CLI: the template
/// engine's own report, with the offending line and its hints, is printed
/// as-is rather than wrapped in an anyhow chain.
#[test]
fn a_broken_template_prints_the_engine_report_and_exits_1() {
    let dir = fixture();
    // Missing `%}` on the endfor.
    write_template(
        dir.path(),
        "bad.jinja",
        "{% for i in ingredients %}{{ i.name }}{% endfor",
    );

    let assert = cook(&dir)
        .args(["report", "-t", "bad.jinja", "pancakes.cook"])
        .assert()
        .failure()
        .code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();

    assert!(
        stderr.contains("unexpected end of input"),
        "expected the engine's message, got: {stderr}"
    );
    assert!(
        stderr.contains("{% endfor"),
        "expected the offending line quoted, got: {stderr}"
    );
    assert!(
        stderr.contains("Missing closing tags"),
        "expected the engine's hints, got: {stderr}"
    );
    assert!(
        !stderr.contains("Caused by"),
        "the report must not be wrapped in an error chain, got: {stderr}"
    );
    assert!(
        assert.get_output().stdout.is_empty(),
        "nothing should be printed on failure"
    );
}

/// A recipe `cooklang-reports` cannot parse fails the same way a broken
/// template does, because that crate parses the recipe from inside the render
/// call. Pinned because it is surprising.
#[test]
fn an_unparseable_recipe_fails_the_render() {
    let dir = fixture();
    fs::write(dir.path().join("broken.cook"), "Add @{1%tsp} to the pot.\n").unwrap();

    let assert = cook(&dir)
        .args(["report", "-t", "list.jinja", "broken.cook"])
        .assert()
        .failure()
        .code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("error parsing recipe"),
        "expected the parse failure, got: {stderr}"
    );
}

/// The discovered `config/aisle.conf` reaches `aisled()`, and `--aisle`
/// overrides it. Asserting both in one test pins the precedence, which
/// asserting either alone would not.
#[test]
fn the_local_aisle_is_used_and_an_explicit_one_overrides_it() {
    let dir = fixture();
    write_template(
        dir.path(),
        "aisled.jinja",
        "{% for aisle, items in aisled(ingredients) | items %}\
         {{ aisle }}:{% for i in items %}{{ i.name }},{% endfor %}|{% endfor %}",
    );
    fs::write(dir.path().join("other.conf"), "[baking]\nflour\n").unwrap();

    // config/aisle.conf files milk and eggs under dairy; flour is unlisted.
    cook(&dir)
        .args(["report", "-t", "aisled.jinja", "pancakes.cook"])
        .assert()
        .success()
        .stdout("dairy:milk,eggs,|other:flour,|\n");

    cook(&dir)
        .args([
            "report",
            "-t",
            "aisled.jinja",
            "-a",
            "other.conf",
            "pancakes.cook",
        ])
        .assert()
        .success()
        .stdout("baking:flour,|other:eggs,milk,|\n");
}

/// The pantry half of the same contract: the discovered `config/pantry.conf`
/// reaches `excluding_pantry()`, and `--pantry` overrides it.
#[test]
fn the_local_pantry_is_used_and_an_explicit_one_overrides_it() {
    let dir = fixture();
    write_template(
        dir.path(),
        "shopping.jinja",
        "{% for i in excluding_pantry(ingredients) %}{{ i.name }},{% endfor %}",
    );
    // Replace the empty pantry the fixture writes to shadow the global one.
    fs::write(
        dir.path().join("config").join("pantry.conf"),
        "[baking]\nflour = \"2%kg\"\n",
    )
    .unwrap();
    fs::write(dir.path().join("other.conf"), "[dairy]\nmilk = \"1%l\"\n").unwrap();

    // The recipe lists eggs, milk, flour; the local pantry already has flour.
    cook(&dir)
        .args(["report", "-t", "shopping.jinja", "pancakes.cook"])
        .assert()
        .success()
        .stdout("eggs,milk,\n");

    cook(&dir)
        .args([
            "report",
            "-t",
            "shopping.jinja",
            "-p",
            "other.conf",
            "pancakes.cook",
        ])
        .assert()
        .success()
        .stdout("eggs,flour,\n");
}

/// A `--aisle` in a subdirectory is found, and its text reaches the template
/// as `aisle_content` verbatim.
#[test]
fn a_relative_aisle_path_in_a_subdirectory_is_read() {
    let dir = fixture();
    fs::create_dir(dir.path().join("shop")).unwrap();
    fs::write(
        dir.path().join("shop").join("aisle.conf"),
        "[baking]\nflour\n",
    )
    .unwrap();
    write_template(dir.path(), "raw.jinja", "{{ aisle_content }}");

    cook(&dir)
        .args([
            "report",
            "-t",
            "raw.jinja",
            "-a",
            "shop/aisle.conf",
            "pancakes.cook",
        ])
        .assert()
        .success()
        .stdout("[baking]\nflour\n\n");
}

/// `--base-path` is made absolute before it is handed on, and unlike the aisle
/// and pantry paths that is observable: the template sees it as `base_path`,
/// and it is what recipe references inside the template resolve against.
#[test]
fn a_relative_base_path_reaches_the_template_absolute() {
    let dir = fixture();
    fs::create_dir(dir.path().join("sub")).unwrap();
    write_template(dir.path(), "base.jinja", "{{ base_path }}");

    let assert = cook(&dir)
        .args(["report", "-t", "base.jinja", "-b", "sub", "pancakes.cook"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let base_path = stdout.trim_end();

    assert!(
        Path::new(base_path).is_absolute(),
        "expected an absolute base path, got: {base_path:?}"
    );
    assert!(
        base_path.ends_with("/sub"),
        "expected the argument on the end, got: {base_path:?}"
    );
}

#[test]
fn a_missing_template_file_is_reported_as_such() {
    let dir = fixture();
    let output = cook(&dir)
        .args(["report", "-t", "absent.jinja", "pancakes.cook"])
        .assert()
        .failure()
        .code(1)
        .to_string();
    assert!(
        output.contains("Failed to read template file: absent.jinja"),
        "expected the template read failure, got: {output}"
    );
}
