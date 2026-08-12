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

#[test]
fn one_line_keeps_every_part_of_a_multi_line_message() {
    assert_eq!(one_line("first\n\n  second  \nthird"), "first second third");
    assert_eq!(one_line("already one line"), "already one line");
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
