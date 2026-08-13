//! Characterization (golden master) tests for `cook shopping-list`.
//!
//! # Why this file exists
//!
//! `shopping-list` is the most intricate command in CookCLI — recursive recipe
//! reference expansion, aisle and pantry config discovery, ingredient
//! aggregation with unit conversion, and four output formats — and it had
//! effectively zero regression coverage: every `#[ignore]`d test in the
//! repository was a shopping-list test, and most of them failed when run.
//!
//! Those have since been repaired and un-ignored
//! (<https://github.com/cooklang/cookcli/issues/415>), so the two now divide
//! the work: `shopping_list_test.rs` asserts *intent* — that references are
//! expanded, that quantities combine, that a flag does what it says — while
//! this file pins exact output, and is what notices a change nobody meant.
//!
//! These snapshots were added ahead of extracting the command into the
//! `cookcli-core` library crate, so that refactor can be verified as
//! behaviour-preserving rather than done blind.
//!
//! # These tests assert what the output IS, not what it should be
//!
//! They are deliberately uncritical. If a snapshot here looks wrong, that is a
//! product bug to be fixed on purpose in its own change — not a reason to edit
//! the snapshot. Known oddities recorded on purpose are called out in comments
//! on the individual tests below.
//!
//! # Machine independence
//!
//! `cook shopping-list` falls back to a *global* `~/.config/cook/aisle.conf`
//! and `pantry.conf` when no local `./config/` equivalent exists, so output
//! would otherwise depend on the developer's home directory. Every fixture
//! here therefore writes an explicit local `config/aisle.conf` *and* a local
//! `config/pantry.conf` — the latter is empty in the non-pantry fixtures purely
//! to shadow any global file.

use assert_cmd::Command;
use insta::assert_snapshot;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Recipes shared by every fixture.
///
/// Chosen to exercise the aggregation machinery rather than just render:
/// - `tomatoes` and `salt` appear unitless / same-unit in two recipes and must merge.
/// - `milk` appears as `500%ml` and `1%l` — different units that *do* convert.
/// - `flour` appears as `200%g` and `1%cup` — different units that do *not* convert.
/// - `water` is deliberately absent from `aisle.conf`, to exercise the `[other]` bucket.
/// - `main.cook` references `@./sauce{}`, to exercise reference expansion.
fn write_recipes(dir: &Path) {
    fs::write(
        dir.join("pasta.cook"),
        "---\ntitle: Pasta\nservings: 2\n---\n\n\
         Boil @pasta{200%g} in @water{2%l} with @salt{1%tsp}.\n\
         Fry @garlic{2%cloves} in @olive oil{2%tbsp}.\n\
         Add @tomatoes{3} and @flour{200%g}.\n\
         Finish with @milk{500%ml}.\n",
    )
    .unwrap();

    fs::write(
        dir.join("salad.cook"),
        "---\ntitle: Salad\n---\n\n\
         Chop @tomatoes{2} and @lettuce{1%head}.\n\
         Dress with @olive oil{1%tbsp} and @salt{1%tsp}.\n\
         Dust with @flour{1%cup} and pour @milk{1%l}.\n",
    )
    .unwrap();

    fs::write(
        dir.join("sauce.cook"),
        "---\ntitle: Sauce\n---\n\n\
         Simmer @tomatoes{4} with @garlic{3%cloves} in @olive oil{1%tbsp}.\n\
         Season with @black pepper{1%tsp}.\n",
    )
    .unwrap();

    fs::write(
        dir.join("main.cook"),
        "---\ntitle: Main\n---\n\n\
         Prepare @./sauce{}.\n\
         Serve with @rice{300%g} and @butter{50%g}.\n",
    )
    .unwrap();
}

/// A well-formed aisle config with several categories, in a deliberate
/// non-alphabetical order so the snapshots record that category ordering
/// follows the file rather than being sorted.
const AISLE_CONF: &str = "\
[produce]
tomatoes
lettuce
garlic

[dairy]
milk
butter

[dry goods]
pasta
flour
rice

[condiments]
olive oil | olive_oil

[spices]
salt
black pepper
";

/// Only present to shadow a possible global `~/.config/cook/pantry.conf`.
const EMPTY_PANTRY_CONF: &str = "# intentionally empty: shadows the global pantry.conf\n";

/// Recipes + a valid `config/aisle.conf` + an empty `config/pantry.conf`.
fn base_fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write_recipes(root);
    fs::create_dir(root.join("config")).unwrap();
    fs::write(root.join("config/aisle.conf"), AISLE_CONF).unwrap();
    fs::write(root.join("config/pantry.conf"), EMPTY_PANTRY_CONF).unwrap();
    dir
}

/// Same, but with a populated pantry that overlaps the recipes in three ways:
/// exactly covers an ingredient (`salt`), partially covers one (`tomatoes`,
/// `olive oil`), and covers one whose recipe quantity is split across
/// convertible and non-convertible units (`flour`).
fn pantry_fixture() -> TempDir {
    let dir = base_fixture();
    fs::write(
        dir.path().join("config/pantry.conf"),
        "[freezer]\n\
         salt = \"2%tsp\"\n\
         \"olive oil\" = \"1%tbsp\"\n\
         tomatoes = \"2\"\n\
         flour = \"100%g\"\n",
    )
    .unwrap();
    dir
}

/// Same recipes, but `config/aisle.conf` is garbage.
fn malformed_aisle_fixture() -> TempDir {
    let dir = base_fixture();
    fs::write(
        dir.path().join("config/aisle.conf"),
        "this is not [ a valid ((( aisle config\n===\n%%%\n",
    )
    .unwrap();
    dir
}

/// Runs `cook shopping-list <args>` in `dir`, asserts it succeeded, and
/// returns stdout with ANSI escapes stripped.
///
/// Colour is stripped rather than snapshotted because `build_human_table`
/// paints category headings green via `yansi`, and whether that is emitted
/// depends on TTY detection rather than on shopping-list logic.
fn run(dir: &TempDir, args: &[&str]) -> String {
    let output = Command::cargo_bin("cook")
        .unwrap()
        .current_dir(dir.path())
        .arg("shopping-list")
        .args(args)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "`cook shopping-list {}` failed with {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
        args.join(" "),
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    String::from_utf8(strip_ansi_escapes::strip(&output.stdout)).unwrap()
}

// ---------------------------------------------------------------------------
// 1. Single recipe, default (human) output
// ---------------------------------------------------------------------------

#[test]
fn single_recipe_human() {
    assert_snapshot!(run(&base_fixture(), &["pasta.cook"]));
}

// ---------------------------------------------------------------------------
// 2. Multiple recipes aggregated
// ---------------------------------------------------------------------------

/// `tomatoes` (3 + 2), `salt` (1 tsp + 1 tsp) and `olive oil` (2 tbsp + 1 tbsp)
/// must each appear once, merged. `milk` (500 ml + 1 l) records that
/// convertible units are summed into one quantity; `flour` (200 g + 1 cup)
/// records that inconvertible units are listed side by side instead.
#[test]
fn multiple_recipes_aggregated() {
    assert_snapshot!(run(&base_fixture(), &["pasta.cook", "salad.cook"]));
}

// ---------------------------------------------------------------------------
// 3. Aisle categorisation picked up automatically from ./config/aisle.conf
// ---------------------------------------------------------------------------

/// Run over every recipe so all five configured categories are populated.
/// Records that category order follows `aisle.conf` rather than being sorted,
/// and that ingredients missing from the config land in a trailing `[other]`.
#[test]
fn aisle_categorization_from_local_config() {
    assert_snapshot!(run(
        &base_fixture(),
        &["pasta.cook", "salad.cook", "sauce.cook", "main.cook"]
    ));
}

// ---------------------------------------------------------------------------
// 4. --plain
// ---------------------------------------------------------------------------

/// Same input as `multiple_recipes_aggregated`; the difference between the two
/// snapshots is exactly what `--plain` suppresses.
#[test]
fn plain_suppresses_categories() {
    assert_snapshot!(run(
        &base_fixture(),
        &["--plain", "pasta.cook", "salad.cook"]
    ));
}

// ---------------------------------------------------------------------------
// 5. -f json --pretty
// ---------------------------------------------------------------------------

#[test]
fn json_pretty_output() {
    assert_snapshot!(run(
        &base_fixture(),
        &["-f", "json", "--pretty", "pasta.cook", "salad.cook"]
    ));
}

// ---------------------------------------------------------------------------
// 6. -f yaml
// ---------------------------------------------------------------------------

#[test]
fn yaml_output() {
    assert_snapshot!(run(
        &base_fixture(),
        &["-f", "yaml", "pasta.cook", "salad.cook"]
    ));
}

/// `-f yaml --plain` drops the categories, like `-f json --plain` and
/// `-f markdown --plain`. The YAML writer used to take no `plain` parameter at
/// all, so the flag silently did nothing here (#419).
#[test]
fn yaml_plain_output() {
    assert_snapshot!(run(
        &base_fixture(),
        &["-f", "yaml", "--plain", "pasta.cook", "salad.cook"]
    ));
}

// ---------------------------------------------------------------------------
// 7. -f markdown
// ---------------------------------------------------------------------------

#[test]
fn markdown_output() {
    assert_snapshot!(run(
        &base_fixture(),
        &["-f", "markdown", "pasta.cook", "salad.cook"]
    ));
}

// ---------------------------------------------------------------------------
// 8. Scaling via recipe.cook:3
// ---------------------------------------------------------------------------

/// Compare against `single_recipe_human`. Note two rescaling side effects
/// recorded here on purpose: `1 tsp` salt scaled by 3 is reported as `1 tbsp`,
/// and `500 ml` milk scaled by 3 is reported as `1.5 l` — the formatter
/// promotes units once the value crosses a threshold.
#[test]
fn scaling_factor_in_recipe_name() {
    assert_snapshot!(run(&base_fixture(), &["pasta.cook:3"]));
}

// ---------------------------------------------------------------------------
// 9. Recipe-reference expansion (@./sauce{})
// ---------------------------------------------------------------------------

/// `main.cook` names only `rice` and `butter` directly; everything else in this
/// snapshot comes from expanding `@./sauce{}`.
#[test]
fn recipe_reference_expansion() {
    assert_snapshot!(run(&base_fixture(), &["main.cook"]));
}

// ---------------------------------------------------------------------------
// 10. --ignore-references
// ---------------------------------------------------------------------------

/// Compare against `recipe_reference_expansion`. Records a surprising current
/// behaviour: suppressing expansion does not drop the reference, it degrades
/// it into a quantity-less shopping item literally named `sauce`, filed under
/// `[other]`.
#[test]
fn ignore_references() {
    assert_snapshot!(run(&base_fixture(), &["--ignore-references", "main.cook"]));
}

// ---------------------------------------------------------------------------
// 11. --ingredients-only
// ---------------------------------------------------------------------------

/// Names only, one per line, with no amounts and no category headings — even
/// though an aisle config is present and loaded.
#[test]
fn ingredients_only() {
    assert_snapshot!(run(
        &base_fixture(),
        &["--ingredients-only", "pasta.cook", "salad.cook"]
    ));
}

// ---------------------------------------------------------------------------
// 12. Pantry subtraction from ./config/pantry.conf
// ---------------------------------------------------------------------------

/// Compare against `multiple_recipes_aggregated`. `salt` is fully covered by
/// the pantry and disappears — taking the whole `[spices]` heading with it —
/// while `tomatoes` and `olive oil` are reduced. `flour` records the partial
/// case: the `200 g` component is reduced to `100 g` while the inconvertible
/// `1 c` component is left untouched (with a unit-mismatch warning on stderr,
/// which this stdout snapshot does not capture).
#[test]
fn pantry_subtraction() {
    assert_snapshot!(run(&pantry_fixture(), &["pasta.cook", "salad.cook"]));
}

// ---------------------------------------------------------------------------
// 13. --ignore-pantry
// ---------------------------------------------------------------------------

/// Same fixture as `pantry_subtraction`; this should match
/// `multiple_recipes_aggregated` exactly, i.e. the pantry is not consulted.
#[test]
fn ignore_pantry() {
    assert_snapshot!(run(
        &pantry_fixture(),
        &["--ignore-pantry", "pasta.cook", "salad.cook"]
    ));
}

// ---------------------------------------------------------------------------
// 14. Malformed config/aisle.conf
// ---------------------------------------------------------------------------

/// Records today's silent-fallback behaviour, which a later task deliberately
/// changes: an unparseable aisle config is *not* an error. The command exits 0
/// (asserted by `run`), logs warnings to stderr, and quietly falls back to an
/// empty aisle config — so every ingredient ends up in `[other]` and the user
/// sees a plausible-looking but uncategorised list.
#[test]
fn malformed_aisle_config_falls_back_silently() {
    assert_snapshot!(run(
        &malformed_aisle_fixture(),
        &["pasta.cook", "salad.cook"]
    ));
}
