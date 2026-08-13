use super::*;
use crate::{ConfigSource, Severity};
use camino::Utf8PathBuf;

/// A pantry with one of everything the queries care about.
const PANTRY: &str = r#"
[dairy]
milk = { quantity = "1%l", expire = "2025-06-05", low = "500%ml" }
eggs = { quantity = "12", expire = "2025-06-03", low = "6" }
butter = { quantity = "200%g", low = "50%g" }

[produce]
tomatoes = { quantity = "5", expire = "2025-06-04" }
garlic = { quantity = "10", low = "3" }

[depleted]
honey = { quantity = "0", low = "100%g" }
vinegar = { quantity = "50%ml", low = "200%ml" }
oregano = { quantity = "50%g" }
water = "always available"
"#;

fn ctx_with(pantry: &str) -> Context {
    Context::new(Utf8PathBuf::from("/nowhere"))
        .with_pantry(ConfigSource::Inline(pantry.to_string()))
}

fn temp() -> tempfile::TempDir {
    tempfile::TempDir::new().unwrap()
}

fn base(dir: &tempfile::TempDir) -> Utf8PathBuf {
    Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap()
}

fn write(path: &Utf8Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn names(items: &[PantryItem]) -> Vec<&str> {
    items.iter().map(|item| item.name.as_str()).collect()
}

// ---------------------------------------------------------------------------
// load
// ---------------------------------------------------------------------------

#[test]
fn load_keeps_sections_and_items_in_file_order() {
    let contents = load(&ctx_with(PANTRY)).expect("loads").into_value();

    assert_eq!(
        contents
            .sections
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["dairy", "produce", "depleted"],
        "sections must stay in the order the file wrote them"
    );
    assert_eq!(
        names(&contents.sections[0].items),
        ["milk", "eggs", "butter"]
    );
}

#[test]
fn load_carries_every_attribute_as_written() {
    let contents = load(&ctx_with(PANTRY)).expect("loads").into_value();
    let milk = contents
        .items()
        .find(|item| item.name == "milk")
        .expect("milk is in the pantry");

    assert_eq!(milk.section, "dairy");
    assert_eq!(milk.quantity.as_deref(), Some("1%l"));
    assert_eq!(milk.low.as_deref(), Some("500%ml"));
    assert_eq!(milk.expire.as_deref(), Some("2025-06-05"));
    assert_eq!(milk.bought, None);

    // Every item knows its own section, even taken out of one.
    for section in &contents.sections {
        for item in &section.items {
            assert_eq!(item.section, section.name, "{} is misfiled", item.name);
        }
    }
}

#[test]
fn load_reads_a_file_when_the_source_is_a_path() {
    let dir = temp();
    let path = base(&dir).join("pantry.conf");
    write(&path, "[dairy]\nmilk = { quantity = \"1%l\" }\n");

    let ctx = Context::new(base(&dir)).with_pantry(ConfigSource::Path(path));
    let contents = load(&ctx).expect("loads").into_value();
    assert_eq!(names(&contents.sections[0].items), ["milk"]);
}

#[test]
fn without_a_pantry_every_query_that_needs_one_says_so() {
    let ctx = Context::new(Utf8PathBuf::from("/nowhere"));

    let missing = |result: Result<(), CoreError>| match result {
        Err(CoreError::MissingConfig { kind }) => assert_eq!(kind, "pantry"),
        other => panic!("expected MissingConfig, got {other:?}"),
    };

    missing(load(&ctx).map(|_| ()));
    missing(list(&ctx, ListRequest::default()).map(|_| ()));
    missing(depleted(&ctx, DepletedRequest::default()).map(|_| ()));
    missing(expiring(&ctx, ExpiringRequest::default()).map(|_| ()));
    missing(recipes(&ctx, RecipesRequest::default()).map(|_| ()));
}

#[test]
fn a_pantry_file_that_cannot_be_read_is_an_io_error_naming_it() {
    let missing = Utf8PathBuf::from("/nonexistent/pantry.conf");
    let ctx = Context::new(Utf8PathBuf::from("/nowhere"))
        .with_pantry(ConfigSource::Path(missing.clone()));

    match load(&ctx) {
        Err(CoreError::Io { path, .. }) => assert_eq!(path, missing),
        other => panic!("expected CoreError::Io, got {other:?}"),
    }
}

/// A pantry that will not parse at all is fatal, and says why on one line —
/// TOML reports its syntax errors over several.
#[test]
fn an_unparseable_pantry_is_an_error_that_names_the_file_and_the_cause() {
    let dir = temp();
    let path = base(&dir).join("pantry.conf");
    write(&path, "[dairy\nmilk = 1\n");

    let ctx = Context::new(base(&dir)).with_pantry(ConfigSource::Path(path.clone()));
    match load(&ctx) {
        Err(error @ CoreError::Config { .. }) => {
            let CoreError::Config {
                path: ref reported,
                ref message,
            } = error
            else {
                unreachable!()
            };
            assert_eq!(reported.as_deref(), Some(path.as_path()));
            assert!(!message.is_empty());
            assert!(
                !error.to_string().contains('\n'),
                "Display must stay one line: {error}"
            );
        }
        other => panic!("expected CoreError::Config, got {other:?}"),
    }
}

/// A pantry that parses with something to say still loads: the warnings are
/// the payload, not a failure.
#[test]
fn warnings_come_back_as_diagnostics_rather_than_failing_the_load() {
    let outcome = load(&ctx_with(
        "[freezer]\nice = { quantity = \"1%kg\", colour = \"white\" }\n",
    ))
    .expect("an unknown attribute is not fatal");

    assert_eq!(names(&outcome.value.sections[0].items), ["ice"]);
    assert!(
        !outcome.diagnostics.is_empty(),
        "the unknown attribute must be reported"
    );
    for diagnostic in &outcome.diagnostics {
        assert_eq!(diagnostic.severity, Severity::Warning, "{diagnostic:?}");
    }
    assert!(!outcome.has_errors());
}

#[test]
fn a_diagnostic_from_a_pantry_file_points_at_that_file() {
    let dir = temp();
    let path = base(&dir).join("pantry.conf");
    write(
        &path,
        "[freezer]\nice = { quantity = \"1%kg\", colour = \"x\" }\n",
    );

    let ctx = Context::new(base(&dir)).with_pantry(ConfigSource::Path(path.clone()));
    let outcome = load(&ctx).expect("loads");
    let location = outcome.diagnostics[0]
        .location
        .as_ref()
        .expect("located in the file it came from");
    assert_eq!(location.file.as_deref(), Some(path.as_path()));
}

// ---------------------------------------------------------------------------
// list
// ---------------------------------------------------------------------------

#[test]
fn listing_without_a_filter_returns_the_whole_pantry() {
    let contents = list(&ctx_with(PANTRY), ListRequest::default())
        .expect("lists")
        .into_value();
    assert_eq!(contents.sections.len(), 3);
    assert_eq!(contents.items().count(), 9);
}

#[test]
fn a_section_filter_keeps_only_that_section_whatever_its_case() {
    for filter in ["dairy", "DAIRY", "DaIrY"] {
        let contents = list(
            &ctx_with(PANTRY),
            ListRequest {
                section: Some(filter.to_string()),
            },
        )
        .expect("lists")
        .into_value();

        assert_eq!(
            contents
                .sections
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>(),
            ["dairy"],
            "filtering by {filter:?} must keep only the dairy section"
        );
        assert_eq!(
            names(&contents.sections[0].items),
            ["milk", "eggs", "butter"]
        );
    }
}

/// Core reports what is there; whether an empty result is an error is the
/// caller's policy, and the CLI's answer is that it is.
#[test]
fn a_section_filter_matching_nothing_lists_nothing_rather_than_failing() {
    let contents = list(
        &ctx_with(PANTRY),
        ListRequest {
            section: Some("nonexistent".to_string()),
        },
    )
    .expect("an unmatched filter is not an error")
    .into_value();
    assert!(contents.sections.is_empty());
    assert_eq!(contents.items().count(), 0);
}

// ---------------------------------------------------------------------------
// depleted
// ---------------------------------------------------------------------------

fn depleted_names(pantry: &str, all: bool) -> Vec<String> {
    depleted(&ctx_with(pantry), DepletedRequest { all })
        .expect("reports")
        .into_value()
        .iter()
        .map(|item| item.name.clone())
        .collect()
}

#[test]
fn depleted_reports_what_is_at_or_below_its_own_threshold() {
    let reported = depleted_names(PANTRY, false);

    // honey: 0 <= 100%g... in different units, so the fallback catches it
    // anyway; vinegar: 50%ml <= 200%ml; oregano: 50%g <= 100 by the built-in
    // rule for grams.
    assert!(reported.contains(&"honey".to_string()), "{reported:?}");
    assert!(reported.contains(&"vinegar".to_string()), "{reported:?}");
    assert!(reported.contains(&"oregano".to_string()), "{reported:?}");

    // Well stocked, and above their own thresholds.
    assert!(!reported.contains(&"milk".to_string()), "{reported:?}");
    assert!(!reported.contains(&"garlic".to_string()), "{reported:?}");
    assert!(!reported.contains(&"butter".to_string()), "{reported:?}");
}

#[test]
fn depleted_reports_in_file_order_with_the_section_on_each_item() {
    let items = depleted(&ctx_with(PANTRY), DepletedRequest { all: true })
        .expect("reports")
        .into_value();

    assert_eq!(
        items
            .iter()
            .map(|item| (item.section.as_str(), item.name.as_str()))
            .collect::<Vec<_>>(),
        [
            ("dairy", "eggs"),
            ("dairy", "butter"),
            ("produce", "garlic"),
            ("depleted", "honey"),
            ("depleted", "vinegar"),
            ("depleted", "oregano"),
        ],
        "items must come back grouped as the file wrote them"
    );
}

/// <https://github.com/cooklang/cookcli/issues/228>: an explicit threshold has
/// to beat the built-in rules in *both* directions.
#[test]
fn an_explicit_threshold_in_matching_units_decides_on_its_own() {
    let pantry = r#"
[test]
below = { quantity = "15%g", low = "20%g" }
above = { quantity = "85%g", low = "20%g" }
no_threshold_low = "50%g"
no_threshold_high = "200%g"
"#;

    assert_eq!(
        depleted_names(pantry, false),
        ["below", "no_threshold_low"],
        "85g is above its own 20g threshold, even though the built-in rule for \
         grams would call it low"
    );
    assert_eq!(
        depleted_names(pantry, true),
        ["below", "above", "no_threshold_low"],
        "`all` adds the item held back by its own threshold — but not \
         `no_threshold_high`, which nothing but the built-in rule ever judged"
    );
}

/// `all` is narrower than it sounds: it brings back the items whose stock
/// *could* be compared and came out fine, not every item in the pantry. An
/// item with no threshold of its own is judged by the built-in rule and by
/// nothing else, so a well-stocked one stays out either way. Pinned because it
/// reads like a bug — it is CookCLI's long-standing behaviour, and this crate
/// must not change it by accident.
#[test]
fn all_does_not_mean_every_item() {
    let pantry = r#"
[test]
plenty_with_threshold = { quantity = "500%g", low = "20%g" }
plenty_without_threshold = "500%g"
"#;
    assert!(depleted_names(pantry, false).is_empty());
    assert_eq!(depleted_names(pantry, true), ["plenty_with_threshold"]);
}

/// A threshold that cannot be compared with the quantity is no threshold at
/// all, so the built-in rules decide.
#[test]
fn a_threshold_in_other_units_falls_back_to_the_built_in_rules() {
    let pantry = r#"
[test]
low_in_other_units = { quantity = "50%g", low = "2%kg" }
high_in_other_units = { quantity = "500%g", low = "2%kg" }
"#;
    assert_eq!(depleted_names(pantry, false), ["low_in_other_units"]);
}

/// An item with no quantity at all has nothing to judge, so `all` is the only
/// thing that brings it back. A quantity that is not a number — `"always
/// available"` — is *not* the same case: it is a quantity the built-in rule
/// simply reads as fine, and it stays out even with `all`.
#[test]
fn items_with_no_quantity_are_reported_only_with_all() {
    let pantry = r#"
[test]
no_quantity = { low = "200%g" }
quantity_that_is_not_a_number = "always available"
"#;
    assert!(depleted_names(pantry, false).is_empty());
    assert_eq!(depleted_names(pantry, true), ["no_quantity"]);
}

#[test]
fn is_low_reads_only_the_quantity_and_the_threshold() {
    let item = |quantity: Option<&str>, low: Option<&str>| PantryItem {
        name: "x".to_string(),
        section: "s".to_string(),
        quantity: quantity.map(ToOwned::to_owned),
        bought: None,
        expire: None,
        low: low.map(ToOwned::to_owned),
    };

    assert!(item(Some("50%ml"), Some("200%ml")).is_low());
    assert!(item(Some("200%ml"), Some("200%ml")).is_low(), "at is low");
    assert!(!item(Some("500%ml"), Some("200%ml")).is_low());
    assert!(
        !item(Some("50%ml"), None).is_low(),
        "no threshold, no answer"
    );
    assert!(!item(None, Some("200%ml")).is_low(), "no stock, no answer");
    assert!(
        !item(Some("50%g"), Some("2%kg")).is_low(),
        "units that do not match are not compared"
    );
}

// ---------------------------------------------------------------------------
// expiring
// ---------------------------------------------------------------------------

fn on(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2025, 6, day).unwrap()
}

fn expiring_names(pantry: &str, req: ExpiringRequest, today: NaiveDate) -> Vec<(String, i64)> {
    let contents = load(&ctx_with(pantry)).expect("loads").into_value();
    expiring_on(&contents, &req, today)
        .into_iter()
        .map(|item| (item.item.name, item.days_until_expiry.unwrap_or(i64::MAX)))
        .collect()
}

/// Everything up to and including the last day of the window, soonest first,
/// with what has already expired at the front.
#[test]
fn expiring_returns_the_window_soonest_first() {
    let pantry = r#"
[test]
expired = { expire = "2025-06-01" }
today = { expire = "2025-06-02" }
tomorrow = { expire = "2025-06-03" }
on_the_last_day = { expire = "2025-06-09" }
just_outside = { expire = "2025-06-10" }
"#;

    assert_eq!(
        expiring_names(pantry, ExpiringRequest::default(), on(2)),
        [
            ("expired".to_string(), -1),
            ("today".to_string(), 0),
            ("tomorrow".to_string(), 1),
            ("on_the_last_day".to_string(), 7),
        ],
        "the window is inclusive of its last day, and one day past it is out"
    );
}

#[test]
fn zero_days_returns_only_what_has_expired_or_expires_today() {
    let pantry = r#"
[test]
expired = { expire = "2025-06-01" }
today = { expire = "2025-06-02" }
tomorrow = { expire = "2025-06-03" }
"#;
    assert_eq!(
        expiring_names(
            pantry,
            ExpiringRequest {
                days: 0,
                include_unknown: false
            },
            on(2)
        ),
        [("expired".to_string(), -1), ("today".to_string(), 0)]
    );
}

#[test]
fn items_without_a_readable_date_come_last_and_only_when_asked_for() {
    let pantry = r#"
[test]
no_date = { quantity = "1%kg" }
unreadable_date = { expire = "next tuesday" }
soon = { expire = "2025-06-03" }
"#;

    assert_eq!(
        expiring_names(pantry, ExpiringRequest::default(), on(2)),
        [("soon".to_string(), 1)]
    );

    let with_unknown = expiring_names(
        pantry,
        ExpiringRequest {
            days: 7,
            include_unknown: true,
        },
        on(2),
    );
    assert_eq!(
        with_unknown,
        [
            ("soon".to_string(), 1),
            ("no_date".to_string(), i64::MAX),
            ("unreadable_date".to_string(), i64::MAX),
        ],
        "an unreadable date is as good as no date, and both sort last"
    );
}

#[test]
fn the_expiry_date_is_normalised_to_iso_whatever_the_file_wrote() {
    let contents = load(&ctx_with("[test]\nmilk = { expire = \"05.06.2025\" }\n"))
        .expect("loads")
        .into_value();
    let items = expiring_on(&contents, &ExpiringRequest::default(), on(2));

    assert_eq!(items[0].expire_date.as_deref(), Some("2025-06-05"));
    assert_eq!(items[0].days_until_expiry, Some(3));
    assert_eq!(
        items[0].item.expire.as_deref(),
        Some("05.06.2025"),
        "the item itself must keep what the file said"
    );
}

/// A window wider than the calendar used to panic — `cook pantry expiring -d
/// 4294967295` — which in a library called from a NAPI addon crosses into
/// JavaScript.
#[test]
fn a_window_wider_than_the_calendar_returns_everything_instead_of_panicking() {
    let contents = load(&ctx_with("[test]\nmilk = { expire = \"2025-06-05\" }\n"))
        .expect("loads")
        .into_value();
    let items = expiring_on(
        &contents,
        &ExpiringRequest {
            days: u32::MAX,
            include_unknown: false,
        },
        on(2),
    );
    assert_eq!(items.len(), 1);
}

#[test]
fn dates_are_read_in_the_documented_order() {
    assert_eq!(parse_date("2025-06-01"), Some(on(1)));
    assert_eq!(parse_date("01.06.2025"), Some(on(1)));
    assert_eq!(parse_date("2025.06.01"), Some(on(1)));
    assert_eq!(parse_date("01-06-2025"), Some(on(1)));
    // Day-first wins over month-first, so this is the 1st of June.
    assert_eq!(parse_date("01/06/2025"), Some(on(1)));
    // Only readable month-first, so it is the 6th of January.
    assert_eq!(
        parse_date("06/13/2025"),
        NaiveDate::from_ymd_opt(2025, 6, 13)
    );

    for unreadable in ["", "next tuesday", "2025-13-01", "06/2025", "2025"] {
        assert_eq!(parse_date(unreadable), None, "{unreadable:?}");
    }
}

// ---------------------------------------------------------------------------
// quantities
// ---------------------------------------------------------------------------

#[test]
fn the_built_in_thresholds_are_per_unit() {
    // Grams and millilitres: at or below 100.
    assert!(is_low_quantity("100%g"));
    assert!(is_low_quantity("100%ml"));
    assert!(!is_low_quantity("101%g"));
    assert!(!is_low_quantity("101%ml"));

    // Kilos and litres: below half.
    assert!(is_low_quantity("0.4%kg"));
    assert!(is_low_quantity("0.4%l"));
    assert!(!is_low_quantity("0.5%kg"), "half a kilo is not low");
    assert!(!is_low_quantity("0.5%l"));

    // Everything else, including a bare count: at or below one.
    assert!(is_low_quantity("1"));
    assert!(is_low_quantity("1%item"));
    assert!(is_low_quantity("1%loaf"));
    assert!(!is_low_quantity("2"));
    assert!(!is_low_quantity("2%items"));

    // Case and the `%` are both optional.
    assert!(is_low_quantity("100 G"));
    assert!(!is_low_quantity("101 G"));

    // Not a quantity at all.
    for not_a_quantity in ["", "always available", "a few", "-1%g"] {
        assert!(!is_low_quantity(not_a_quantity), "{not_a_quantity:?}");
    }
}

#[test]
fn units_match_compares_only_the_unit() {
    assert!(units_match("500%ml", "200%ml"));
    assert!(
        units_match("500 ml", "200%ML"),
        "case and spacing are noise"
    );
    assert!(units_match("5", "2"), "two bare counts are comparable");
    assert!(!units_match("500%g", "2%kg"));
    assert!(!units_match("5", "2%kg"));
    assert!(!units_match("always available", "200%g"));
    assert!(!units_match("200%g", "always available"));
}

// ---------------------------------------------------------------------------
// recipes
// ---------------------------------------------------------------------------

/// A collection covering every branch: fully stocked, partly stocked,
/// referencing another recipe, hiding an ingredient, and listing none.
fn collection() -> tempfile::TempDir {
    let dir = temp();
    let base = base(&dir);
    write(&base.join("toast.cook"), "Toast @bread{2%slices}.\n");
    write(
        &base.join("Breakfast").join("eggs on toast.cook"),
        "Fry @eggs{2} and toast @bread{2%slices}.\n",
    );
    write(
        &base.join("cake.cook"),
        "Mix @flour{200%g}, @sugar{100%g} and @eggs{2}.\n",
    );
    write(&base.join("boiled water.cook"), "Boil some water.\n");
    dir
}

const STOCKED: &str = "[test]\nBread = \"2\"\neggs = \"12\"\n";

fn matches(dir: &tempfile::TempDir, pantry: &str, threshold: u8) -> RecipeMatches {
    let ctx = Context::new(base(dir)).with_pantry(ConfigSource::Inline(pantry.to_string()));
    recipes(&ctx, RecipesRequest { threshold })
        .expect("reports")
        .into_value()
}

#[test]
fn a_recipe_matches_fully_when_every_ingredient_is_in_stock() {
    let dir = collection();
    let found = matches(&dir, STOCKED, 75);

    assert_eq!(
        found.full,
        ["eggs on toast", "toast"],
        "names are titles, in alphabetical order"
    );
}

/// Names are compared lowercased on both sides, so a pantry writing `Bread`
/// stocks a recipe asking for `bread`.
#[test]
fn stock_is_matched_ignoring_case() {
    let dir = temp();
    write(&base(&dir).join("toast.cook"), "Toast @Bread{2%slices}.\n");
    assert_eq!(matches(&dir, "[test]\nbread = \"2\"\n", 75).full, ["toast"]);
}

#[test]
fn a_partly_stocked_recipe_is_reported_with_what_is_missing() {
    let dir = collection();
    let found = matches(&dir, STOCKED, 30);

    assert_eq!(
        found.partial,
        [PartialMatch {
            name: "cake".to_string(),
            percentage: 33,
            missing: vec!["flour".to_string(), "sugar".to_string()],
        }],
        "one of three ingredients in stock, and the missing two in order"
    );
}

#[test]
fn the_threshold_is_inclusive_and_keeps_thinner_matches_out() {
    let dir = collection();
    // Cake is 33% covered.
    assert_eq!(matches(&dir, STOCKED, 33).partial.len(), 1);
    assert!(matches(&dir, STOCKED, 34).partial.is_empty());
}

#[test]
fn a_recipe_that_lists_no_ingredients_matches_nothing() {
    let dir = collection();
    let found = matches(&dir, STOCKED, 0);
    assert!(
        !found.full.contains(&"boiled water".to_string()),
        "{:?}",
        found.full
    );
    assert!(
        !found.partial.iter().any(|m| m.name == "boiled water"),
        "{:?}",
        found.partial
    );
}

/// A reference is a recipe to make, not a thing to have in, so it is neither
/// wanted nor missing.
#[test]
fn a_reference_to_another_recipe_is_not_counted() {
    let dir = temp();
    let base = base(&dir);
    write(&base.join("sauce.cook"), "Simmer @tomatoes{1%kg}.\n");
    write(
        &base.join("pasta.cook"),
        "Cook @pasta{200%g} with @./sauce{}.\n",
    );

    let found = matches(&dir, "[test]\npasta = \"1%kg\"\n", 75);
    assert_eq!(
        found.full,
        ["pasta"],
        "pasta is the only ingredient that counts: {found:?}"
    );
}

/// With CookCLI's parser configuration there is no such thing as a hidden
/// ingredient: `@-salt{}` parses as an ingredient *named* `-salt`, and counts
/// like any other. Pinned because the code filters on `should_be_listed`,
/// which reads as though it would leave this out.
#[test]
fn a_dash_prefixed_ingredient_is_wanted_like_any_other() {
    let dir = temp();
    write(
        &base(&dir).join("pasta.cook"),
        "Cook @pasta{200%g} with a pinch of @-salt{}.\n",
    );

    let found = matches(&dir, "[test]\npasta = \"1%kg\"\n", 50);
    assert!(found.full.is_empty(), "{found:?}");
    assert_eq!(
        found.partial,
        [PartialMatch {
            name: "pasta".to_string(),
            percentage: 50,
            missing: vec!["-salt".to_string()],
        }]
    );
}

/// One broken recipe must not cost the caller the answer for the rest.
#[test]
fn an_unparseable_recipe_is_skipped_with_a_warning() {
    let dir = collection();
    let base = base(&dir);
    write(&base.join("broken.cook"), "Add @{1%tsp} and @{2%tsp}.\n");

    let ctx = Context::new(base.clone()).with_pantry(ConfigSource::Inline(STOCKED.to_string()));
    let outcome = recipes(&ctx, RecipesRequest::default()).expect("reports");

    assert_eq!(outcome.value.full, ["eggs on toast", "toast"]);
    let skipped: Vec<&Diagnostic> = outcome
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("broken.cook"))
        .collect();
    assert_eq!(skipped.len(), 1, "{:?}", outcome.diagnostics);
    assert_eq!(skipped[0].severity, Severity::Warning);
    assert_eq!(
        skipped[0].location.as_ref().and_then(|l| l.file.as_deref()),
        Some(base.join("broken.cook").as_path())
    );
    assert!(!outcome.has_errors(), "a skipped recipe is not a failure");
}

/// `cooklang-find` holds a directory's children in a `HashMap`, so the walk
/// order changes between runs. A single run can come out sorted by luck.
#[test]
fn matches_come_back_in_the_same_order_every_time() {
    let dir = collection();
    let expected = matches(&dir, STOCKED, 0);
    for _ in 0..8 {
        assert_eq!(matches(&dir, STOCKED, 0), expected, "order must not vary");
    }
}

/// Results are ordered by what they are called, not by where the files sit.
/// The two are told apart by titling the files against their names.
#[test]
fn matches_are_ordered_by_name_rather_than_by_path() {
    let dir = temp();
    let base = base(&dir);
    write(
        &base.join("a.cook"),
        "---\ntitle: Zebra\n---\n\nToast @bread{2%slices}.\n",
    );
    write(
        &base.join("b.cook"),
        "---\ntitle: Apple\n---\n\nBoil @eggs{2}.\n",
    );

    assert_eq!(matches(&dir, STOCKED, 0).full, ["Apple", "Zebra"]);
}

/// The warnings a walk raises are in path order too, so a caller printing
/// them prints the same report twice running. Three files in one directory,
/// because two could come out in order by luck.
#[test]
fn skipped_recipes_are_reported_in_path_order() {
    let dir = temp();
    let base = base(&dir);
    for name in ["a", "b", "c"] {
        write(&base.join(format!("{name}.cook")), "Add @{1%tsp}.\n");
    }
    let ctx = Context::new(base).with_pantry(ConfigSource::Inline(STOCKED.to_string()));

    for _ in 0..8 {
        let outcome = recipes(&ctx, RecipesRequest::default()).expect("reports");
        let files: Vec<String> = outcome
            .diagnostics
            .iter()
            .filter_map(|d| d.location.as_ref()?.file.as_ref())
            .filter_map(|file| file.file_name().map(ToOwned::to_owned))
            .collect();
        assert_eq!(files, ["a.cook", "b.cook", "c.cook"], "order must not vary");
    }
}

#[test]
fn a_collection_that_cannot_be_walked_is_reported() {
    let ctx = Context::new(Utf8PathBuf::from("/nonexistent/recipes"))
        .with_pantry(ConfigSource::Inline(STOCKED.to_string()));

    match recipes(&ctx, RecipesRequest::default()) {
        Err(CoreError::Search { base_dir, message }) => {
            assert_eq!(base_dir, "/nonexistent/recipes");
            assert_eq!(message, "no such directory");
        }
        other => panic!(
            "expected CoreError::Search, got {:?}",
            other.map(|o| o.value)
        ),
    }
}

// ---------------------------------------------------------------------------
// plan
// ---------------------------------------------------------------------------

fn planned(dir: &tempfile::TempDir, req: PlanRequest) -> PantryPlan {
    plan(&Context::new(base(dir)), req)
        .expect("plans")
        .into_value()
}

fn steps(plan: &PantryPlan) -> Vec<(&str, usize, usize)> {
    plan.steps
        .iter()
        .map(|step| {
            (
                step.name.as_str(),
                step.new_recipes_unlocked,
                step.total_cookable,
            )
        })
        .collect()
}

/// Three recipes over four ingredients: flour is in two of them, so it goes
/// first even though it unlocks nothing on its own.
fn plan_collection() -> tempfile::TempDir {
    let dir = temp();
    let base = base(&dir);
    write(
        &base.join("bread.cook"),
        "Mix @flour{1%kg} and @water{1%l}.\n",
    );
    write(
        &base.join("cake.cook"),
        "Mix @flour{1%kg} and @sugar{1%kg}.\n",
    );
    write(&base.join("tea.cook"), "Steep @tea{1}.\n");
    dir
}

#[test]
fn the_plan_takes_the_most_wanted_ingredient_first() {
    let plan = planned(&plan_collection(), PlanRequest::default());

    assert_eq!(
        steps(&plan),
        [
            ("flour", 0, 0),
            ("sugar", 1, 1),
            ("tea", 1, 2),
            ("water", 1, 3),
        ],
        "flour is wanted twice so it leads; the rest are one each, and ties \
         are broken alphabetically"
    );
    assert_eq!(plan.total_recipes, 3);
    assert_eq!(plan.cookable_recipes(), 3);
    assert_eq!(plan.coverage_percentage(), 100);
}

#[test]
fn max_ingredients_stops_the_plan_short() {
    let plan = planned(
        &plan_collection(),
        PlanRequest {
            max_ingredients: Some(2),
            allow_missing: 0,
        },
    );

    assert_eq!(steps(&plan), [("flour", 0, 0), ("sugar", 1, 1)]);
    assert_eq!(plan.total_recipes, 3);
    assert_eq!(plan.cookable_recipes(), 1);
    assert_eq!(plan.coverage_percentage(), 33, "one of three, rounded down");
}

#[test]
fn no_ingredients_at_all_covers_nothing() {
    let plan = planned(
        &plan_collection(),
        PlanRequest {
            max_ingredients: Some(0),
            allow_missing: 0,
        },
    );
    assert!(plan.steps.is_empty());
    assert_eq!(plan.total_recipes, 3);
    assert_eq!(plan.cookable_recipes(), 0);
    assert_eq!(plan.coverage_percentage(), 0);
}

#[test]
fn allow_missing_counts_a_recipe_before_it_is_fully_covered() {
    let plan = planned(
        &plan_collection(),
        PlanRequest {
            max_ingredients: None,
            allow_missing: 1,
        },
    );

    assert_eq!(
        steps(&plan),
        [("flour", 3, 3)],
        "flour leaves bread and cake one ingredient short each, which is close \
         enough — and tea, which never wanted flour, is one short too, so the \
         first step covers everything"
    );
    assert_eq!(plan.coverage_percentage(), 100);
}

#[test]
fn menus_and_recipes_without_ingredients_are_left_out_of_the_plan() {
    let dir = temp();
    let base = base(&dir);
    write(&base.join("bread.cook"), "Mix @flour{1%kg}.\n");
    write(&base.join("week.menu"), "Have @bread{} on Monday.\n");
    write(&base.join("nothing.cook"), "Boil some water.\n");

    let plan = planned(&dir, PlanRequest::default());
    assert_eq!(plan.total_recipes, 1, "only bread.cook counts");
    assert_eq!(steps(&plan), [("flour", 1, 1)]);
}

#[test]
fn an_empty_collection_plans_nothing() {
    let dir = temp();
    let plan = planned(&dir, PlanRequest::default());
    assert!(plan.steps.is_empty());
    assert_eq!(plan.total_recipes, 0);
    assert_eq!(plan.cookable_recipes(), 0);
    assert_eq!(
        plan.coverage_percentage(),
        0,
        "no recipes is 0% covered, not a division by zero"
    );
}

/// Unlike every other query here, this one needs no pantry at all.
#[test]
fn planning_needs_no_pantry_configuration() {
    let dir = plan_collection();
    let ctx = Context::new(base(&dir));
    assert!(ctx.pantry().is_unset());
    assert_eq!(
        plan(&ctx, PlanRequest::default())
            .unwrap()
            .value
            .steps
            .len(),
        4
    );
}

#[test]
fn the_plan_is_the_same_every_time() {
    let dir = plan_collection();
    let expected = planned(&dir, PlanRequest::default());
    for _ in 0..8 {
        assert_eq!(
            planned(&dir, PlanRequest::default()),
            expected,
            "the tie-break must not vary"
        );
    }
}

#[test]
fn ties_are_broken_alphabetically_rather_than_by_walk_order() {
    // Every ingredient is wanted exactly once, so only the tie-break decides.
    let dir = temp();
    let base = base(&dir);
    write(&base.join("z.cook"), "Add @zucchini{1}.\n");
    write(&base.join("a.cook"), "Add @apple{1}.\n");
    write(&base.join("m.cook"), "Add @mango{1}.\n");

    assert_eq!(
        planned(&dir, PlanRequest::default())
            .steps
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["apple", "mango", "zucchini"]
    );
}

/// `plan` compares ingredients as recipes write them, where `recipes`
/// lowercases. Pinned because the two differing is surprising.
#[test]
fn the_plan_treats_differently_cased_ingredients_as_different() {
    let dir = temp();
    let base = base(&dir);
    write(&base.join("a.cook"), "Add @Flour{1%kg}.\n");
    write(&base.join("b.cook"), "Add @flour{1%kg}.\n");

    let plan = planned(&dir, PlanRequest::default());
    assert_eq!(steps(&plan), [("Flour", 1, 1), ("flour", 1, 2)]);
}

#[test]
fn a_collection_that_cannot_be_walked_fails_the_plan() {
    match plan(
        &Context::new(Utf8PathBuf::from("/nonexistent/recipes")),
        PlanRequest::default(),
    ) {
        Err(CoreError::Search { base_dir, .. }) => {
            assert_eq!(base_dir, "/nonexistent/recipes")
        }
        other => panic!(
            "expected CoreError::Search, got {:?}",
            other.map(|o| o.value)
        ),
    }
}

// ---------------------------------------------------------------------------
// add, remove, update
// ---------------------------------------------------------------------------

/// Where `Context::discover` would put a pantry, and where `add` creates one.
fn pantry_path(base: &Utf8Path) -> Utf8PathBuf {
    base.join("config").join("pantry.conf")
}

/// A context whose pantry is a real file, which is the only kind that can be
/// written.
fn ctx_at(base: &Utf8Path) -> Context {
    Context::new(base.to_owned()).with_pantry(ConfigSource::Path(pantry_path(base)))
}

/// A temporary directory holding `pantry`, and a context pointing at it.
fn planted(pantry: &str) -> (tempfile::TempDir, Context) {
    let dir = temp();
    let base = base(&dir);
    write(&pantry_path(&base), pantry);
    let ctx = ctx_at(&base);
    (dir, ctx)
}

fn read_back(ctx: &Context) -> String {
    std::fs::read_to_string(ctx.pantry().path().expect("a file-backed pantry")).unwrap()
}

const SMALL: &str = "[pantry]\nflour = { quantity = \"1%kg\", low = \"200%g\" }\n";

#[test]
fn add_puts_a_new_item_at_the_end_of_an_existing_section() {
    let (_dir, ctx) = planted(SMALL);

    add(
        &ctx,
        AddRequest {
            section: "pantry".to_string(),
            name: "sugar".to_string(),
            ..Default::default()
        },
    )
    .expect("adds");

    let written = read_back(&ctx);
    assert!(written.contains("sugar"), "{written}");
    assert!(
        written.find("flour") < written.find("sugar"),
        "a new item goes after the ones already there\n{written}"
    );
}

/// An item added with no attributes is written as an empty one, and reads
/// back with nothing invented.
#[test]
fn add_without_attributes_writes_an_empty_item() {
    let (_dir, ctx) = planted(SMALL);

    let contents = add(
        &ctx,
        AddRequest {
            section: "pantry".to_string(),
            name: "sugar".to_string(),
            ..Default::default()
        },
    )
    .expect("adds")
    .into_value();

    let sugar = contents
        .items()
        .find(|item| item.name == "sugar")
        .expect("sugar is in the returned pantry");
    assert_eq!(sugar.quantity, None);
    assert_eq!(sugar.expire, None);
    assert!(
        read_back(&ctx).contains("sugar = {}"),
        "{}",
        read_back(&ctx)
    );
}

#[test]
fn add_writes_every_attribute_it_is_given() {
    let (_dir, ctx) = planted(SMALL);

    add(
        &ctx,
        AddRequest {
            section: "dairy".to_string(),
            name: "milk".to_string(),
            quantity: Some("2%l".to_string()),
            bought: Some("2025-05-01".to_string()),
            expire: Some("2025-12-01".to_string()),
            low: Some("500%ml".to_string()),
        },
    )
    .expect("adds");

    let reread = load(&ctx).expect("reads back").into_value();
    let milk = reread
        .items()
        .find(|item| item.name == "milk")
        .expect("milk survived the round trip");
    assert_eq!(milk.section, "dairy");
    assert_eq!(milk.quantity.as_deref(), Some("2%l"));
    assert_eq!(milk.bought.as_deref(), Some("2025-05-01"));
    assert_eq!(milk.expire.as_deref(), Some("2025-12-01"));
    assert_eq!(milk.low.as_deref(), Some("500%ml"));
}

#[test]
fn add_creates_a_section_the_file_does_not_have() {
    let (_dir, ctx) = planted(SMALL);

    add(
        &ctx,
        AddRequest {
            section: "spices".to_string(),
            name: "cumin".to_string(),
            quantity: Some("50%g".to_string()),
            ..Default::default()
        },
    )
    .expect("adds");

    assert!(read_back(&ctx).contains("[spices]"));
}

/// Sections are matched exactly, so this makes a second one rather than
/// adding to the first. Pinned because `list` filters case-insensitively and
/// the difference is easy to trip over.
#[test]
fn add_matches_the_section_name_exactly() {
    let (_dir, ctx) = planted(SMALL);

    add(
        &ctx,
        AddRequest {
            section: "Pantry".to_string(),
            name: "sugar".to_string(),
            ..Default::default()
        },
    )
    .expect("adds");

    let written = read_back(&ctx);
    assert!(written.contains("[pantry]"), "{written}");
    assert!(written.contains("[Pantry]"), "{written}");
}

#[test]
fn add_creates_the_pantry_file_when_the_context_has_none() {
    let dir = temp();
    let base = base(&dir);
    let ctx = Context::new(base.clone());
    assert!(ctx.pantry().is_unset());

    add(
        &ctx,
        AddRequest {
            section: "produce".to_string(),
            name: "apples".to_string(),
            quantity: Some("6".to_string()),
            ..Default::default()
        },
    )
    .expect("adds");

    let written = std::fs::read_to_string(pantry_path(&base)).expect("the file was created");
    assert!(written.contains("apples"), "{written}");
}

#[test]
fn add_refuses_an_item_the_section_already_holds() {
    let (_dir, ctx) = planted(SMALL);

    match add(
        &ctx,
        AddRequest {
            section: "pantry".to_string(),
            name: "flour".to_string(),
            quantity: Some("9%kg".to_string()),
            ..Default::default()
        },
    ) {
        Err(CoreError::PantryEdit { message }) => assert_eq!(
            message, "item 'flour' already exists in section 'pantry'",
            "the message must name both the item and the section"
        ),
        other => panic!("expected PantryEdit, got {:?}", other.map(|o| o.value)),
    }

    assert_eq!(
        read_back(&ctx),
        SMALL,
        "a refused add must leave the file exactly as it was"
    );
}

/// Item names are compared exactly too, so `Flour` and `flour` are two items
/// in one section rather than a clash. TOML keeps them apart as keys, so this
/// survives a round trip.
#[test]
fn add_matches_the_item_name_exactly() {
    let (_dir, ctx) = planted(SMALL);

    add(
        &ctx,
        AddRequest {
            section: "pantry".to_string(),
            name: "Flour".to_string(),
            ..Default::default()
        },
    )
    .expect("a differently cased name is not a duplicate");

    let reread = load(&ctx).expect("reads back").into_value();
    assert_eq!(names(&reread.sections[0].items), ["flour", "Flour"]);
}

/// The same name in another section is a different item.
#[test]
fn add_allows_the_same_name_in_a_different_section() {
    let (_dir, ctx) = planted(SMALL);

    add(
        &ctx,
        AddRequest {
            section: "baking".to_string(),
            name: "flour".to_string(),
            ..Default::default()
        },
    )
    .expect("adds");

    let reread = load(&ctx).expect("reads back").into_value();
    assert_eq!(
        reread
            .items()
            .filter(|item| item.name == "flour")
            .map(|item| item.section.as_str())
            .collect::<Vec<_>>(),
        ["pantry", "baking"]
    );
}

#[test]
fn remove_takes_the_item_out_and_leaves_the_rest() {
    let (_dir, ctx) = planted("[pantry]\nflour = \"1%kg\"\nsugar = \"2%kg\"\n");

    let contents = remove(
        &ctx,
        RemoveRequest {
            section: "pantry".to_string(),
            name: "flour".to_string(),
        },
    )
    .expect("removes")
    .into_value();

    assert_eq!(names(&contents.sections[0].items), ["sugar"]);
    let written = read_back(&ctx);
    assert!(!written.contains("flour"), "{written}");
    assert!(written.contains("sugar"), "{written}");
}

#[test]
fn remove_deletes_a_section_it_empties_and_leaves_the_others_in_order() {
    // Four sections, and the one removed is neither first nor second-to-last:
    // with three, taking out the middle one would look right even if the last
    // were swapped into its place.
    let (_dir, ctx) = planted(
        "[dairy]\nmilk = \"1%l\"\n\n[produce]\napple = \"5\"\n\n[bakery]\nbread = \"1\"\n\n[freezer]\npeas = \"1%kg\"\n",
    );

    remove(
        &ctx,
        RemoveRequest {
            section: "produce".to_string(),
            name: "apple".to_string(),
        },
    )
    .expect("removes");

    let written = read_back(&ctx);
    assert!(!written.contains("[produce]"), "{written}");
    let at = |needle: &str| written.find(needle).unwrap_or_else(|| panic!("{needle}"));
    assert!(
        at("[dairy]") < at("[bakery]") && at("[bakery]") < at("[freezer]"),
        "the sections that remain keep their order\n{written}"
    );
}

#[test]
fn remove_refuses_an_item_that_is_not_there() {
    let (_dir, ctx) = planted(SMALL);

    match remove(
        &ctx,
        RemoveRequest {
            section: "pantry".to_string(),
            name: "sugar".to_string(),
        },
    ) {
        Err(CoreError::PantryEdit { message }) => {
            assert_eq!(message, "item 'sugar' not found in section 'pantry'")
        }
        other => panic!("expected PantryEdit, got {:?}", other.map(|o| o.value)),
    }
    assert_eq!(read_back(&ctx), SMALL, "nothing may be written");
}

#[test]
fn remove_refuses_a_section_that_is_not_there() {
    let (_dir, ctx) = planted(SMALL);

    match remove(
        &ctx,
        RemoveRequest {
            section: "freezer".to_string(),
            name: "flour".to_string(),
        },
    ) {
        Err(CoreError::PantryEdit { message }) => {
            assert_eq!(message, "section 'freezer' not found")
        }
        other => panic!("expected PantryEdit, got {:?}", other.map(|o| o.value)),
    }
    assert_eq!(read_back(&ctx), SMALL, "nothing may be written");
}

#[test]
fn update_changes_only_the_attributes_it_is_given() {
    let (_dir, ctx) = planted(SMALL);

    update(
        &ctx,
        UpdateRequest {
            section: "pantry".to_string(),
            name: "flour".to_string(),
            quantity: Some("2%kg".to_string()),
            ..Default::default()
        },
    )
    .expect("updates");

    let reread = load(&ctx).expect("reads back").into_value();
    let flour = reread.items().next().expect("flour is still there");
    assert_eq!(flour.quantity.as_deref(), Some("2%kg"), "the new quantity");
    assert_eq!(
        flour.low.as_deref(),
        Some("200%g"),
        "an attribute not named by the request is left alone"
    );
}

#[test]
fn update_gives_a_bare_item_its_first_attribute() {
    let (_dir, ctx) = planted("[pantry]\nsugar = {}\n");

    update(
        &ctx,
        UpdateRequest {
            section: "pantry".to_string(),
            name: "sugar".to_string(),
            expire: Some("2025-12-31".to_string()),
            ..Default::default()
        },
    )
    .expect("updates");

    let reread = load(&ctx).expect("reads back").into_value();
    let sugar = reread.items().next().expect("sugar is still there");
    assert_eq!(sugar.expire.as_deref(), Some("2025-12-31"));
    assert_eq!(sugar.quantity, None, "nothing else is invented");
}

#[test]
fn update_refuses_a_request_that_sets_nothing() {
    let (_dir, ctx) = planted(SMALL);

    match update(
        &ctx,
        UpdateRequest {
            section: "pantry".to_string(),
            name: "flour".to_string(),
            ..Default::default()
        },
    ) {
        Err(CoreError::PantryEdit { message }) => assert_eq!(
            message,
            "no attributes given to update on item 'flour' in section 'pantry'"
        ),
        other => panic!("expected PantryEdit, got {:?}", other.map(|o| o.value)),
    }
    assert_eq!(read_back(&ctx), SMALL, "nothing may be written");
}

/// Checked before the pantry is even resolved, so a request that sets nothing
/// is refused for what it is rather than for having nowhere to go.
#[test]
fn update_refuses_a_request_that_sets_nothing_before_looking_for_a_pantry() {
    let ctx = Context::new(Utf8PathBuf::from("/nowhere"));

    assert!(matches!(
        update(
            &ctx,
            UpdateRequest {
                section: "pantry".to_string(),
                name: "flour".to_string(),
                ..Default::default()
            },
        ),
        Err(CoreError::PantryEdit { .. })
    ));
}

#[test]
fn update_refuses_an_item_that_is_not_there() {
    let (_dir, ctx) = planted(SMALL);

    match update(
        &ctx,
        UpdateRequest {
            section: "pantry".to_string(),
            name: "sugar".to_string(),
            quantity: Some("1%kg".to_string()),
            ..Default::default()
        },
    ) {
        Err(CoreError::PantryEdit { message }) => {
            assert_eq!(message, "item 'sugar' not found in section 'pantry'")
        }
        other => panic!("expected PantryEdit, got {:?}", other.map(|o| o.value)),
    }
    assert_eq!(read_back(&ctx), SMALL, "nothing may be written");
}

#[test]
fn update_refuses_a_section_that_is_not_there() {
    let (_dir, ctx) = planted(SMALL);

    match update(
        &ctx,
        UpdateRequest {
            section: "freezer".to_string(),
            name: "flour".to_string(),
            quantity: Some("1%kg".to_string()),
            ..Default::default()
        },
    ) {
        Err(CoreError::PantryEdit { message }) => {
            assert_eq!(message, "section 'freezer' not found")
        }
        other => panic!("expected PantryEdit, got {:?}", other.map(|o| o.value)),
    }
    assert_eq!(read_back(&ctx), SMALL, "nothing may be written");
}

/// A change lands in the section it names, not in whichever one happens to
/// hold an item of that name.
#[test]
fn update_changes_the_item_in_the_section_it_names() {
    let (_dir, ctx) = planted("[dairy]\nmilk = \"1%l\"\n\n[freezer]\nmilk = \"2%l\"\n");

    update(
        &ctx,
        UpdateRequest {
            section: "freezer".to_string(),
            name: "milk".to_string(),
            quantity: Some("9%l".to_string()),
            ..Default::default()
        },
    )
    .expect("updates");

    let reread = load(&ctx).expect("reads back").into_value();
    let quantities: Vec<_> = reread
        .items()
        .map(|item| (item.section.as_str(), item.quantity.as_deref()))
        .collect();
    assert_eq!(
        quantities,
        [("dairy", Some("1%l")), ("freezer", Some("9%l"))]
    );
}

// ---------------------------------------------------------------------------
// Where a change may and may not be written
// ---------------------------------------------------------------------------

/// Writing an editor's unsaved buffer to a path this crate made up would be
/// worse than refusing, so all three refuse.
#[test]
fn nothing_writes_to_an_inline_pantry() {
    let dir = temp();
    let base = base(&dir);
    let ctx = Context::new(base.clone()).with_pantry(ConfigSource::Inline(SMALL.to_string()));

    let refused = |result: Result<Outcome<PantryContents>, CoreError>, what: &str| match result {
        Err(CoreError::ReadOnlyConfig { kind }) => assert_eq!(kind, "pantry", "{what}"),
        other => panic!(
            "expected ReadOnlyConfig from {what}, got {:?}",
            other.map(|o| o.value)
        ),
    };

    refused(
        add(
            &ctx,
            AddRequest {
                section: "pantry".to_string(),
                name: "sugar".to_string(),
                ..Default::default()
            },
        ),
        "add",
    );
    refused(
        remove(
            &ctx,
            RemoveRequest {
                section: "pantry".to_string(),
                name: "flour".to_string(),
            },
        ),
        "remove",
    );
    refused(
        update(
            &ctx,
            UpdateRequest {
                section: "pantry".to_string(),
                name: "flour".to_string(),
                quantity: Some("2%kg".to_string()),
                ..Default::default()
            },
        ),
        "update",
    );

    assert!(
        !base.join("config").exists(),
        "a refused write must not leave a file behind"
    );
}

/// `add` creates a pantry where there is none; the other two have nothing to
/// change and say so.
#[test]
fn remove_and_update_need_a_pantry_that_exists() {
    let dir = temp();
    let ctx = Context::new(base(&dir));

    for result in [
        remove(
            &ctx,
            RemoveRequest {
                section: "pantry".to_string(),
                name: "flour".to_string(),
            },
        ),
        update(
            &ctx,
            UpdateRequest {
                section: "pantry".to_string(),
                name: "flour".to_string(),
                quantity: Some("2%kg".to_string()),
                ..Default::default()
            },
        ),
    ] {
        match result {
            Err(CoreError::MissingConfig { kind }) => assert_eq!(kind, "pantry"),
            other => panic!("expected MissingConfig, got {:?}", other.map(|o| o.value)),
        }
    }
}

/// A path-backed pantry that is not there is a different thing from no pantry
/// at all: something named a file and it could not be read.
#[test]
fn removing_from_a_pantry_path_that_does_not_exist_is_an_io_error() {
    let dir = temp();
    let base = base(&dir);
    let ctx = ctx_at(&base);

    match remove(
        &ctx,
        RemoveRequest {
            section: "pantry".to_string(),
            name: "flour".to_string(),
        },
    ) {
        Err(CoreError::Io { path, source }) => {
            assert_eq!(path, pantry_path(&base));
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
        }
        other => panic!("expected Io, got {:?}", other.map(|o| o.value)),
    }
}

#[test]
fn a_pantry_that_cannot_be_parsed_stops_a_change() {
    let (_dir, ctx) = planted("[unclosed\n");

    match add(
        &ctx,
        AddRequest {
            section: "pantry".to_string(),
            name: "sugar".to_string(),
            ..Default::default()
        },
    ) {
        Err(CoreError::Config { path, message }) => {
            assert_eq!(path.as_deref(), ctx.pantry().path());
            assert!(!message.is_empty());
            assert!(!message.contains('\n'), "one line: {message:?}");
        }
        other => panic!("expected Config, got {:?}", other.map(|o| o.value)),
    }
    assert_eq!(read_back(&ctx), "[unclosed\n", "nothing may be written");
}

// ---------------------------------------------------------------------------
// What a write keeps, and what it throws away
// ---------------------------------------------------------------------------

/// Pins the loss documented on the module. Comments do not survive a change,
/// because the file is re-serialised from a model that has never seen them.
#[test]
fn a_write_throws_away_comments_and_layout() {
    let (_dir, ctx) =
        planted("# my pantry\n\n[pantry]\n# staple\n  flour   =   \"1%kg\"   # a whole bag\n");

    add(
        &ctx,
        AddRequest {
            section: "pantry".to_string(),
            name: "sugar".to_string(),
            ..Default::default()
        },
    )
    .expect("adds");

    assert_eq!(
        read_back(&ctx),
        "[pantry]\nflour = { quantity = \"1%kg\" }\nsugar = {}\n",
        "comments, indentation and the shorthand item shape are all rewritten; \
         only what cooklang models survives"
    );
}

/// An attribute `cooklang` does not model is dropped, and the caller is told
/// so — the warning is the only sign that anything was lost.
#[test]
fn a_write_throws_away_attributes_cooklang_does_not_model() {
    let (_dir, ctx) = planted("[pantry]\nflour = { quantity = \"1%kg\", shelf = \"top\" }\n");

    let outcome = add(
        &ctx,
        AddRequest {
            section: "pantry".to_string(),
            name: "sugar".to_string(),
            ..Default::default()
        },
    )
    .expect("adds");

    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Warning && d.message.contains("shelf")),
        "the unknown attribute must be reported: {:?}",
        outcome.diagnostics
    );
    let written = read_back(&ctx);
    assert!(!written.contains("shelf"), "and it is gone\n{written}");
    assert!(written.contains("1%kg"), "{written}");
}

/// An attribute whose value is not a string is dropped with no warning at all.
#[test]
fn a_write_throws_away_a_non_string_attribute_silently() {
    let (_dir, ctx) = planted("[pantry]\nflour = { quantity = 2 }\n");

    let outcome = add(
        &ctx,
        AddRequest {
            section: "pantry".to_string(),
            name: "sugar".to_string(),
            ..Default::default()
        },
    )
    .expect("adds");

    assert!(
        outcome.diagnostics.is_empty(),
        "nothing warns about this: {:?}",
        outcome.diagnostics
    );
    let written = read_back(&ctx);
    assert!(
        !written.contains('2'),
        "the quantity is gone all the same\n{written}"
    );
}

#[test]
fn a_write_keeps_section_and_item_order() {
    let (_dir, ctx) = planted(
        "[dairy]\nzucchini = \"2\"\napple = \"5\"\nmango = \"3\"\n\n[produce]\nleek = \"1\"\n\n[bakery]\nbread = \"1\"\n",
    );

    add(
        &ctx,
        AddRequest {
            section: "dairy".to_string(),
            name: "banana".to_string(),
            ..Default::default()
        },
    )
    .expect("adds");

    let written = read_back(&ctx);
    let at = |needle: &str| written.find(needle).unwrap_or_else(|| panic!("{needle}"));
    assert!(
        at("zucchini") < at("apple") && at("apple") < at("mango") && at("mango") < at("banana"),
        "items keep file order, with the new one last\n{written}"
    );
    assert!(
        at("[dairy]") < at("[produce]") && at("[produce]") < at("[bakery]"),
        "sections keep file order\n{written}"
    );
}

/// Items above the first section header stay above it rather than being
/// wrapped in a `[general]` section — but they can only be written as
/// `name = "quantity"`, so everything else about them is lost.
#[test]
fn a_write_keeps_general_items_at_the_top_but_loses_their_other_attributes() {
    let (_dir, ctx) = planted(
        "salt = \"1%kg\"\n\n[dairy]\nmilk = { quantity = \"2%l\", expire = \"2025-12-01\" }\n",
    );

    // `salt` is a general item; give it an expiry, which cannot be written.
    update(
        &ctx,
        UpdateRequest {
            section: "general".to_string(),
            name: "salt".to_string(),
            expire: Some("2025-11-11".to_string()),
            ..Default::default()
        },
    )
    .expect("updates");

    let written = read_back(&ctx);
    assert!(
        written.find("salt").unwrap() < written.find('[').unwrap(),
        "a general item stays above the first section header\n{written}"
    );
    assert!(!written.contains("[general]"), "{written}");
    assert!(
        !written.contains("2025-11-11"),
        "a general item's expiry cannot be written and is lost\n{written}"
    );
    assert!(
        written.contains("2025-12-01"),
        "a sectioned item's is not\n{written}"
    );
}

/// A bare general item gains an empty quantity, because the only shape the
/// top level can be written in is `name = "quantity"`.
#[test]
fn a_bare_general_item_comes_back_with_an_empty_quantity() {
    let (_dir, ctx) = planted("salt = \"\"\n\n[dairy]\nmilk = \"1%l\"\n");

    let contents = add(
        &ctx,
        AddRequest {
            section: "dairy".to_string(),
            name: "cheese".to_string(),
            ..Default::default()
        },
    )
    .expect("adds")
    .into_value();

    let salt = contents
        .items()
        .find(|item| item.name == "salt")
        .expect("salt survived");
    assert_eq!(
        salt.quantity.as_deref(),
        Some(""),
        "not None, which is what a bare sectioned item gives"
    );
}

/// A non-ASCII item name is not a bare TOML key, so a file that is going to
/// hold one has to quote it — which `add` does, and a person writing the file
/// by hand has to.
#[test]
fn a_name_that_is_not_a_bare_toml_key_has_to_be_quoted() {
    let (_dir, ctx) = planted("[mejeri]\nsmörgås = \"1\"\n");

    assert!(matches!(
        add(
            &ctx,
            AddRequest {
                section: "mejeri".to_string(),
                name: "äpple".to_string(),
                ..Default::default()
            },
        ),
        Err(CoreError::Config { .. })
    ));
}

/// A section written as an array of item names comes back as a table. The
/// items survive; the syntax does not.
#[test]
fn a_section_written_as_an_array_comes_back_as_a_table() {
    let (_dir, ctx) = planted("fridge = [\"milk\", \"eggs\"]\n");

    add(
        &ctx,
        AddRequest {
            section: "fridge".to_string(),
            name: "butter".to_string(),
            ..Default::default()
        },
    )
    .expect("adds");

    let written = read_back(&ctx);
    assert!(
        written.contains("[fridge]"),
        "the array became a section table\n{written}"
    );
    assert!(
        !written.contains("[\"milk\""),
        "and the array syntax is gone\n{written}"
    );
    assert_eq!(
        names(&load(&ctx).expect("reads back").into_value().sections[0].items),
        ["milk", "eggs", "butter"],
        "the items themselves survive"
    );
}

#[test]
fn a_write_keeps_non_ascii_names() {
    let (_dir, ctx) = planted("[mejeri]\n\"smörgås\" = \"1\"\n");

    add(
        &ctx,
        AddRequest {
            section: "mejeri".to_string(),
            name: "äpple".to_string(),
            quantity: Some("3".to_string()),
            ..Default::default()
        },
    )
    .expect("adds");

    let reread = load(&ctx).expect("reads back").into_value();
    assert_eq!(names(&reread.sections[0].items), ["smörgås", "äpple"]);
}

/// What the three return is what the file now says, so a caller need not read
/// it back.
#[test]
fn a_change_returns_the_pantry_it_wrote() {
    let (_dir, ctx) = planted(SMALL);

    let returned = add(
        &ctx,
        AddRequest {
            section: "dairy".to_string(),
            name: "milk".to_string(),
            quantity: Some("1%l".to_string()),
            ..Default::default()
        },
    )
    .expect("adds")
    .into_value();

    assert_eq!(returned, load(&ctx).expect("reads back").into_value());
}

// ---------------------------------------------------------------------------
// Writing the file itself
// ---------------------------------------------------------------------------

#[test]
fn a_write_leaves_no_temporary_file_behind() {
    let (_dir, ctx) = planted(SMALL);

    add(
        &ctx,
        AddRequest {
            section: "pantry".to_string(),
            name: "sugar".to_string(),
            ..Default::default()
        },
    )
    .expect("adds");

    let config = ctx.pantry().path().unwrap().parent().unwrap().to_owned();
    let left: Vec<_> = std::fs::read_dir(&config)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(left, ["pantry.conf"], "only the pantry itself is left");
}

/// A pantry symlinked in from a dotfiles repository must be written through,
/// not replaced by a regular file.
#[cfg(unix)]
#[test]
fn a_write_follows_a_symlinked_pantry() {
    let dir = temp();
    let base = base(&dir);
    let real = base.join("dotfiles").join("pantry.conf");
    write(&real, SMALL);
    std::fs::create_dir_all(base.join("config")).unwrap();
    std::os::unix::fs::symlink(real.as_std_path(), pantry_path(&base).as_std_path()).unwrap();

    let ctx = ctx_at(&base);
    add(
        &ctx,
        AddRequest {
            section: "pantry".to_string(),
            name: "sugar".to_string(),
            ..Default::default()
        },
    )
    .expect("adds");

    assert!(
        std::fs::symlink_metadata(pantry_path(&base).as_std_path())
            .unwrap()
            .file_type()
            .is_symlink(),
        "the link itself must survive"
    );
    assert!(
        std::fs::read_to_string(real.as_std_path())
            .unwrap()
            .contains("sugar"),
        "and the change must land on the file it points at"
    );
}

/// A temporary file is created from the process umask, which is usually more
/// permissive than a pantry someone has locked down.
#[cfg(unix)]
#[test]
fn a_write_keeps_the_pantry_file_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let (_dir, ctx) = planted(SMALL);
    let path = ctx.pantry().path().unwrap().to_owned();
    std::fs::set_permissions(path.as_std_path(), std::fs::Permissions::from_mode(0o600)).unwrap();

    add(
        &ctx,
        AddRequest {
            section: "pantry".to_string(),
            name: "sugar".to_string(),
            ..Default::default()
        },
    )
    .expect("adds");

    let mode = std::fs::metadata(path.as_std_path())
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600, "the pantry must not become readable");
}

/// The property the whole write dance is for: whatever goes wrong, the pantry
/// that was there is still there.
///
/// The failure is forced by making the directory unwritable, so the test can
/// only run as a user the permission applies to — root is not one, and the
/// probe below says so rather than the test quietly passing.
#[cfg(unix)]
#[test]
fn a_failed_write_leaves_the_pantry_untouched() {
    use std::os::unix::fs::PermissionsExt;

    let (_dir, ctx) = planted(SMALL);
    let config = ctx.pantry().path().unwrap().parent().unwrap().to_owned();

    std::fs::set_permissions(config.as_std_path(), std::fs::Permissions::from_mode(0o500)).unwrap();
    let unwritable = std::fs::File::create(config.join("probe").as_std_path()).is_err();

    let result = add(
        &ctx,
        AddRequest {
            section: "pantry".to_string(),
            name: "sugar".to_string(),
            ..Default::default()
        },
    );

    // Restore before asserting, so a failure does not also break the cleanup.
    std::fs::set_permissions(config.as_std_path(), std::fs::Permissions::from_mode(0o700)).unwrap();

    assert!(
        unwritable,
        "this test needs a user that a read-only directory applies to; \
         running as root makes it prove nothing"
    );
    match result {
        Err(CoreError::Io { path, .. }) => assert_eq!(path, *ctx.pantry().path().unwrap()),
        other => panic!("expected Io, got {:?}", other.map(|o| o.value)),
    }
    assert_eq!(read_back(&ctx), SMALL, "the pantry is exactly as it was");
    let left: Vec<_> = std::fs::read_dir(config.as_std_path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(left, ["pantry.conf"], "and no temporary file is left");
}

/// A pantry that exists but cannot be read must stop the change. Treating an
/// unreadable file as an empty one would replace it with a one-item pantry —
/// the worst thing any of this can do.
///
/// Unreadable is arranged with a mode, so the test only means anything as a
/// user that mode applies to; the probe says so rather than passing quietly
/// under root.
#[cfg(unix)]
#[test]
fn add_refuses_a_pantry_it_cannot_read_rather_than_replacing_it() {
    use std::os::unix::fs::PermissionsExt;

    let (_dir, ctx) = planted(SMALL);
    let path = ctx.pantry().path().unwrap().to_owned();
    std::fs::set_permissions(path.as_std_path(), std::fs::Permissions::from_mode(0o000)).unwrap();
    let unreadable = std::fs::read_to_string(path.as_std_path()).is_err();

    let result = add(
        &ctx,
        AddRequest {
            section: "pantry".to_string(),
            name: "sugar".to_string(),
            ..Default::default()
        },
    );

    std::fs::set_permissions(path.as_std_path(), std::fs::Permissions::from_mode(0o600)).unwrap();

    assert!(
        unreadable,
        "this test needs a user that a 000 file applies to; running as root \
         makes it prove nothing"
    );
    match result {
        Err(CoreError::Io { path: named, .. }) => assert_eq!(named, path),
        other => panic!("expected Io, got {:?}", other.map(|o| o.value)),
    }
    assert_eq!(read_back(&ctx), SMALL, "the pantry is exactly as it was");
}

/// [`write_atomically`] cleans up after itself when the rename fails, which is
/// the one failure that happens after the temporary file exists.
///
/// Renaming a file onto a directory cannot succeed, which is a portable way to
/// reach that branch — the pantry commands themselves stop earlier than this,
/// because a directory cannot be read as a pantry either.
#[test]
fn a_write_that_fails_after_writing_its_temporary_file_still_removes_it() {
    let dir = temp();
    let base = base(&dir);
    let target = base.join("a-directory");
    std::fs::create_dir_all(target.join("child").as_std_path()).unwrap();

    match write_atomically(&target, "contents") {
        Err(CoreError::Io { path, .. }) => assert_eq!(path, target),
        other => panic!("expected Io, got {other:?}"),
    }

    let left: Vec<_> = std::fs::read_dir(base.as_std_path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(left, ["a-directory"], "no temporary file is left behind");
}
