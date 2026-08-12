use super::*;
use crate::{ConfigSource, Severity};
use camino::Utf8Path;

const AISLE: &str = "\
[produce]
tomatoes
lettuce

[dairy]
milk
";

fn dir_with(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    for (name, text) in files {
        let path = dir.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    dir
}

fn base(dir: &tempfile::TempDir) -> Utf8PathBuf {
    Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap()
}

/// A context that reads nothing ambient: `Context::new` leaves both
/// configurations unset, so these tests cannot pick up the developer's
/// `~/.config/cook/aisle.conf`.
fn ctx(dir: &tempfile::TempDir) -> Context {
    Context::new(base(dir))
}

fn request(names: &[&str]) -> GenerateRequest {
    GenerateRequest {
        recipes: names.iter().map(|n| ScaledRecipe::new(*n)).collect(),
        ignore_references: false,
    }
}

/// The rendered quantities of one ingredient, or `None` if it is not listed.
fn quantities(list: &AggregatedList, name: &str) -> Option<Vec<String>> {
    list.items
        .iter()
        .find(|i| i.name == name)
        .map(|i| i.quantities.clone())
}

// ---------------------------------------------------------------------------
// Aggregation
// ---------------------------------------------------------------------------

/// The core of the command: the same ingredient named by two recipes becomes
/// one line with the amounts added together.
#[test]
fn duplicate_ingredients_are_merged_into_one_item() {
    let dir = dir_with(&[
        ("a.cook", "Add @tomatoes{3} and @salt{1%tsp}.\n"),
        ("b.cook", "Add @tomatoes{2} and @salt{2%tsp}.\n"),
    ]);

    let list = generate(&ctx(&dir), request(&["a.cook", "b.cook"]))
        .expect("generates")
        .value;

    assert_eq!(
        list.items.len(),
        2,
        "two ingredients across two recipes, merged: {:?}",
        list.items
    );
    assert_eq!(quantities(&list, "tomatoes"), Some(vec!["5".to_string()]));
    // Merging adds the values and keeps the unit; unlike scaling, it does not
    // re-fit `3 tsp` up to `1 tbsp`.
    assert_eq!(quantities(&list, "salt"), Some(vec!["3 tsp".to_string()]));
}

/// Units that convert are summed into one quantity; units that do not are kept
/// side by side rather than silently added.
#[test]
fn convertible_units_combine_and_others_stay_separate() {
    let dir = dir_with(&[
        ("a.cook", "Pour @milk{500%ml} and @flour{200%g}.\n"),
        ("b.cook", "Pour @milk{1%l} and @flour{1%cup}.\n"),
    ]);

    let list = generate(&ctx(&dir), request(&["a.cook", "b.cook"]))
        .expect("generates")
        .value;

    assert_eq!(quantities(&list, "milk"), Some(vec!["1500 ml".to_string()]));
    // Two entries rather than one, because grams and cups do not convert. The
    // order within an ingredient is `cooklang`'s, not the recipes'.
    let flour = quantities(&list, "flour").expect("flour is listed");
    assert_eq!(flour.len(), 2, "{flour:?}");
    assert!(flour.contains(&"200 g".to_string()), "{flour:?}");
    assert!(flour.contains(&"1 c".to_string()), "{flour:?}");
}

#[test]
fn the_request_scale_is_applied_per_recipe() {
    let dir = dir_with(&[
        ("a.cook", "Add @tomatoes{3}.\n"),
        ("b.cook", "Add @tomatoes{2}.\n"),
    ]);

    let list = generate(
        &ctx(&dir),
        GenerateRequest {
            recipes: vec![
                ScaledRecipe::scaled("a.cook", 2.0),
                ScaledRecipe::scaled("b.cook", 10.0),
            ],
            ignore_references: false,
        },
    )
    .expect("generates")
    .value;

    // 3 * 2 + 2 * 10, not (3 + 2) * anything.
    assert_eq!(quantities(&list, "tomatoes"), Some(vec!["26".to_string()]));
}

// ---------------------------------------------------------------------------
// Aisle configuration
// ---------------------------------------------------------------------------

/// Categories follow the configuration's order, not the alphabet, and
/// ingredients it does not mention fall into a trailing `other`.
#[test]
fn ingredients_are_categorised_by_the_aisle_configuration() {
    let dir = dir_with(&[("a.cook", "Add @tomatoes{3}, @milk{1%l} and @sand{1%kg}.\n")]);
    let ctx = ctx(&dir).with_aisle(ConfigSource::Inline(AISLE.to_string()));

    let list = generate(&ctx, request(&["a.cook"]))
        .expect("generates")
        .value;

    let names: Vec<&str> = list.categories.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["produce", "dairy", "other"]);
    assert_eq!(list.categories[0].items[0].name, "tomatoes");
    assert_eq!(list.categories[2].items[0].name, "sand");
}

/// Without an aisle configuration everything lands in `other`, and the caller
/// is told why rather than being left to wonder.
#[test]
fn a_missing_aisle_configuration_warns_and_leaves_everything_uncategorised() {
    let dir = dir_with(&[("a.cook", "Add @tomatoes{3}.\n")]);

    let outcome = generate(&ctx(&dir), request(&["a.cook"])).expect("generates");

    let names: Vec<&str> = outcome
        .value
        .categories
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(names, vec!["other"]);

    let diagnostic = outcome
        .diagnostics
        .iter()
        .find(|d| d.message.contains("no aisle configuration"))
        .unwrap_or_else(|| panic!("expected a warning, got {:?}", outcome.diagnostics));
    assert_eq!(diagnostic.severity, Severity::Warning);
    assert!(
        diagnostic.hints.iter().any(|h| h.contains("cooklang.org")),
        "the warning should say where the format is documented: {diagnostic:?}"
    );
}

/// Aisle synonyms are folded onto the configuration's first spelling, so two
/// recipes naming the same thing differently still merge.
#[test]
fn aisle_synonyms_fold_onto_one_name() {
    let dir = dir_with(&[
        ("a.cook", "Add @olive oil{2%tbsp}.\n"),
        ("b.cook", "Add @olive_oil{1%tbsp}.\n"),
    ]);
    let ctx = ctx(&dir).with_aisle(ConfigSource::Inline(
        "[condiments]\nolive oil | olive_oil\n".to_string(),
    ));

    let list = generate(&ctx, request(&["a.cook", "b.cook"]))
        .expect("generates")
        .value;

    assert_eq!(list.items.len(), 1, "{:?}", list.items);
    assert_eq!(list.items[0].name, "olive oil");
    assert_eq!(list.items[0].quantities, vec!["3 tbsp".to_string()]);
}

/// Records today's silent fallback, which
/// <https://github.com/cooklang/cookcli/issues/416> changes later: an
/// unparseable aisle file is a warning, not a failure.
#[test]
fn a_malformed_aisle_configuration_warns_and_falls_back() {
    let dir = dir_with(&[("a.cook", "Add @tomatoes{3}.\n")]);
    let path = base(&dir).join("aisle.conf");
    std::fs::write(&path, "this is not [ a valid ((( aisle config\n===\n%%%\n").unwrap();
    let ctx = ctx(&dir).with_aisle(ConfigSource::Path(path.clone()));

    let outcome = generate(&ctx, request(&["a.cook"])).expect("must not fail");

    assert_eq!(
        outcome
            .value
            .categories
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        vec!["other"]
    );
    let diagnostic = outcome
        .diagnostics
        .first()
        .unwrap_or_else(|| panic!("expected a warning about the aisle file"));
    assert_eq!(diagnostic.severity, Severity::Warning);
    assert!(
        diagnostic.message.contains("aisle configuration"),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.location.as_ref().and_then(|l| l.file.as_deref()),
        Some(path.as_path()),
        "a configuration warning must name the file it came from"
    );
}

#[test]
fn an_unreadable_aisle_file_is_an_io_error_naming_it() {
    let dir = dir_with(&[("a.cook", "Add @tomatoes{3}.\n")]);
    let missing = Utf8PathBuf::from("/nonexistent/aisle.conf");
    let ctx = ctx(&dir).with_aisle(ConfigSource::Path(missing.clone()));

    match generate(&ctx, request(&["a.cook"])) {
        Err(CoreError::Io { path, .. }) => assert_eq!(path, missing),
        other => panic!("expected CoreError::Io, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Pantry
// ---------------------------------------------------------------------------

/// What the pantry already holds is taken off the list, and an ingredient it
/// fully covers disappears.
#[test]
fn pantry_quantities_are_subtracted() {
    let dir = dir_with(&[(
        "a.cook",
        "Add @tomatoes{5}, @salt{1%tsp} and @rice{300%g}.\n",
    )]);
    let ctx = ctx(&dir).with_pantry(ConfigSource::Inline(
        "[cupboard]\ntomatoes = \"2\"\nsalt = \"2%tsp\"\n".to_string(),
    ));

    let list = generate(&ctx, request(&["a.cook"]))
        .expect("generates")
        .value;

    assert_eq!(
        quantities(&list, "tomatoes"),
        Some(vec!["3".to_string()]),
        "a partially stocked ingredient is reduced"
    );
    assert_eq!(
        quantities(&list, "salt"),
        None,
        "a fully stocked ingredient drops off the list entirely"
    );
    assert_eq!(
        quantities(&list, "rice"),
        Some(vec!["300 g".to_string()]),
        "an ingredient the pantry does not mention is untouched"
    );
}

/// The pantry is only consulted when one is configured — this is how
/// `--ignore-pantry` works.
#[test]
fn no_pantry_source_means_nothing_is_subtracted() {
    let dir = dir_with(&[("a.cook", "Add @tomatoes{5}.\n")]);

    let list = generate(&ctx(&dir), request(&["a.cook"]))
        .expect("generates")
        .value;
    assert_eq!(quantities(&list, "tomatoes"), Some(vec!["5".to_string()]));
}

#[test]
fn an_unreadable_pantry_file_is_an_io_error_naming_it() {
    let dir = dir_with(&[("a.cook", "Add @tomatoes{3}.\n")]);
    let missing = Utf8PathBuf::from("/nonexistent/pantry.conf");
    let ctx = ctx(&dir).with_pantry(ConfigSource::Path(missing.clone()));

    match generate(&ctx, request(&["a.cook"])) {
        Err(CoreError::Io { path, .. }) => assert_eq!(path, missing),
        other => panic!("expected CoreError::Io, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Recipe references
// ---------------------------------------------------------------------------

fn referencing_fixture() -> tempfile::TempDir {
    dir_with(&[
        (
            "sauce.cook",
            "Simmer @tomatoes{4} with @garlic{3%cloves}.\n",
        ),
        (
            "main.cook",
            "Prepare @./sauce{}.\nServe with @rice{300%g}.\n",
        ),
    ])
}

#[test]
fn references_are_expanded_into_their_ingredients() {
    let dir = referencing_fixture();

    let list = generate(&ctx(&dir), request(&["main.cook"]))
        .expect("generates")
        .value;

    assert_eq!(quantities(&list, "tomatoes"), Some(vec!["4".to_string()]));
    assert_eq!(
        quantities(&list, "garlic"),
        Some(vec!["3 cloves".to_string()])
    );
    assert_eq!(quantities(&list, "rice"), Some(vec!["300 g".to_string()]));
    assert_eq!(
        quantities(&list, "sauce"),
        None,
        "an expanded reference must not also appear as an item"
    );
}

/// Records today's surprising behaviour: suppressing expansion does not drop
/// the reference, it leaves it on the list as a quantity-less item.
#[test]
fn ignore_references_leaves_the_reference_as_a_bare_item() {
    let dir = referencing_fixture();

    let list = generate(
        &ctx(&dir),
        GenerateRequest {
            recipes: vec![ScaledRecipe::new("main.cook")],
            ignore_references: true,
        },
    )
    .expect("generates")
    .value;

    assert_eq!(
        quantities(&list, "sauce"),
        Some(Vec::new()),
        "the reference stays, with no quantity: {:?}",
        list.items
    );
    assert_eq!(
        quantities(&list, "tomatoes"),
        None,
        "the referenced recipe's ingredients must not be expanded"
    );
    assert_eq!(quantities(&list, "rice"), Some(vec!["300 g".to_string()]));
}

/// A quantity on the reference scales the referenced recipe to that target
/// rather than multiplying the referring recipe's own factor into it.
#[test]
fn a_quantity_on_a_reference_scales_the_referenced_recipe_to_it() {
    let dir = dir_with(&[
        (
            "sauce.cook",
            "---\nservings: 2\n---\n\nSimmer @tomatoes{4}.\n",
        ),
        ("main.cook", "Prepare @./sauce{6%servings}.\n"),
    ]);

    let list = generate(&ctx(&dir), request(&["main.cook"]))
        .expect("generates")
        .value;

    // 2 servings makes 4 tomatoes, so 6 servings makes 12.
    assert_eq!(quantities(&list, "tomatoes"), Some(vec!["12".to_string()]));
}

/// A reference to a recipe that is not there names the recipe that is missing,
/// not the one that asked for it.
#[test]
fn a_missing_referenced_recipe_is_not_found() {
    let dir = dir_with(&[("main.cook", "Prepare @./absent{}.\n")]);

    match generate(&ctx(&dir), request(&["main.cook"])) {
        Err(CoreError::RecipeNotFound { name }) => assert_eq!(name, "absent"),
        other => panic!("expected RecipeNotFound, got {other:?}"),
    }
}

/// Expansion is bounded rather than recursive, so two recipes referencing each
/// other terminate instead of looping. This pins the behaviour that makes the
/// `CircularReference` guard in `extract_into` unreachable today — if expansion
/// ever becomes recursive, this test is the one that has to change.
#[test]
fn mutually_referencing_recipes_terminate() {
    let dir = dir_with(&[
        ("a.cook", "Prepare @./b{}.\nAdd @salt{1%tsp}.\n"),
        ("b.cook", "Prepare @./a{}.\nAdd @pepper{1%tsp}.\n"),
    ]);

    let list = generate(&ctx(&dir), request(&["a.cook"]))
        .expect("a cycle must not fail or hang today")
        .value;

    assert_eq!(quantities(&list, "pepper"), Some(vec!["1 tsp".to_string()]));
    // `a` contributes its salt twice: once as the recipe asked for, and once
    // more when `b`'s reference back to it is expanded a level down. Recorded
    // rather than endorsed — a recursive expansion that errored on the cycle
    // would not double-count.
    assert_eq!(quantities(&list, "salt"), Some(vec!["2 tsp".to_string()]));
}

/// `included_references` is what the web server uses to let a shopper drop one
/// sub-recipe from a menu without dropping the rest.
#[test]
fn included_references_selects_which_references_to_follow() {
    let dir = dir_with(&[
        ("sauce.cook", "Simmer @tomatoes{4}.\n"),
        ("stock.cook", "Simmer @bones{1%kg}.\n"),
        ("main.cook", "Prepare @./sauce{} and @./stock{}.\n"),
    ]);

    let mut list = IngredientList::new();
    let included = ["sauce".to_string()];
    extract_ingredients(
        &ctx(&dir),
        &ScaledRecipe::new("main.cook"),
        &ExtractOptions {
            ignore_references: false,
            included_references: Some(&included),
        },
        &mut list,
    )
    .expect("extracts");

    let names: Vec<&String> = list.iter().map(|(name, _)| name).collect();
    assert!(names.contains(&&"tomatoes".to_string()), "{names:?}");
    assert!(
        !names.contains(&&"bones".to_string()),
        "an excluded reference must not be expanded: {names:?}"
    );
}

/// `None` follows every reference — the difference from the test above.
#[test]
fn no_included_references_follows_all_of_them() {
    let dir = dir_with(&[
        ("sauce.cook", "Simmer @tomatoes{4}.\n"),
        ("stock.cook", "Simmer @bones{1%kg}.\n"),
        ("main.cook", "Prepare @./sauce{} and @./stock{}.\n"),
    ]);

    let mut list = IngredientList::new();
    extract_ingredients(
        &ctx(&dir),
        &ScaledRecipe::new("main.cook"),
        &ExtractOptions::default(),
        &mut list,
    )
    .expect("extracts");

    let names: Vec<&String> = list.iter().map(|(name, _)| name).collect();
    assert!(names.contains(&&"tomatoes".to_string()), "{names:?}");
    assert!(names.contains(&&"bones".to_string()), "{names:?}");
}

/// Several calls accumulate into one list — the reason `extract_ingredients` is
/// public at all.
#[test]
fn extract_ingredients_accumulates_across_calls() {
    let dir = dir_with(&[
        ("a.cook", "Add @tomatoes{3}.\n"),
        ("b.cook", "Add @tomatoes{2}.\n"),
    ]);
    let ctx = ctx(&dir);

    let mut list = IngredientList::new();
    for name in ["a.cook", "b.cook"] {
        extract_ingredients(
            &ctx,
            &ScaledRecipe::new(name),
            &ExtractOptions::default(),
            &mut list,
        )
        .expect("extracts");
    }

    let items: Vec<(&String, String)> = list
        .iter()
        .map(|(name, q)| {
            (
                name,
                q.iter().map(quantity_fmt).collect::<Vec<_>>().join(", "),
            )
        })
        .collect();
    assert_eq!(items.len(), 1, "{items:?}");
    assert_eq!(items[0].1, "5");
}

// ---------------------------------------------------------------------------
// Diagnostics and errors
// ---------------------------------------------------------------------------

/// With several recipes going into one list, a warning that does not say which
/// recipe it came from is useless.
#[test]
fn warnings_are_attributed_to_the_recipe_that_raised_them() {
    let dir = dir_with(&[
        ("clean.cook", "Add @tomatoes{3}.\n"),
        // Deprecated `>>` metadata parses, but warns.
        ("old.cook", ">> title: Old Style\n\nAdd @salt{1%tsp}.\n"),
    ]);

    let ctx = ctx(&dir).with_aisle(ConfigSource::Inline(AISLE.to_string()));
    let outcome =
        generate(&ctx, request(&["clean.cook", "old.cook"])).expect("parses despite warning");

    assert!(!outcome.diagnostics.is_empty(), "expected a diagnostic");
    for diagnostic in &outcome.diagnostics {
        assert_eq!(diagnostic.severity, Severity::Warning, "{diagnostic:?}");
        assert_eq!(
            diagnostic
                .location
                .as_ref()
                .and_then(|l| l.file.as_deref())
                .and_then(Utf8Path::file_name),
            Some("old.cook"),
            "every warning must name the recipe it came from, and only the \
             recipe that raised it: {diagnostic:?}"
        );
    }
}

#[test]
fn a_missing_recipe_is_not_found() {
    let dir = dir_with(&[("a.cook", "Add @tomatoes{3}.\n")]);

    match generate(&ctx(&dir), request(&["absent.cook"])) {
        Err(CoreError::RecipeNotFound { name }) => assert_eq!(name, "absent.cook"),
        other => panic!("expected RecipeNotFound, got {other:?}"),
    }
}

#[test]
fn a_recipe_that_does_not_parse_is_a_parse_error_naming_its_file() {
    let dir = dir_with(&[("broken.cook", "Add @{1%tsp}.\n")]);

    match generate(&ctx(&dir), request(&["broken.cook"])) {
        Err(CoreError::Parse {
            name, diagnostics, ..
        }) => {
            assert!(name.ends_with("broken.cook"), "{name}");
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].severity, Severity::Error);
        }
        other => panic!("expected CoreError::Parse, got {other:?}"),
    }
}

/// A reference the referenced recipe cannot satisfy is its own error, so a
/// caller can tell it apart from the recipe being missing or broken.
#[test]
fn an_unscalable_reference_is_a_reference_error() {
    let dir = dir_with(&[
        // No `yield` metadata, so scaling to a yield target cannot work.
        ("sauce.cook", "Simmer @tomatoes{4}.\n"),
        ("main.cook", "Prepare @./sauce{500%ml}.\n"),
    ]);

    match generate(&ctx(&dir), request(&["main.cook"])) {
        Err(CoreError::Reference { name, message }) => {
            assert!(name.contains("sauce"), "{name}");
            assert!(message.contains("500"), "{message}");
        }
        other => panic!("expected CoreError::Reference, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// The two views of the result
// ---------------------------------------------------------------------------

/// `items` and `categories` must describe the same list, or `--plain` and the
/// default output would disagree about what to buy.
#[test]
fn both_views_hold_the_same_ingredients() {
    let dir = dir_with(&[("a.cook", "Add @tomatoes{3}, @milk{1%l} and @sand{1%kg}.\n")]);
    let ctx = ctx(&dir).with_aisle(ConfigSource::Inline(AISLE.to_string()));

    let list = generate(&ctx, request(&["a.cook"]))
        .expect("generates")
        .value;

    let mut flat: Vec<&ListItem> = list.items.iter().collect();
    let mut grouped: Vec<&ListItem> = list.categories.iter().flat_map(|c| &c.items).collect();
    flat.sort_by(|a, b| a.name.cmp(&b.name));
    grouped.sort_by(|a, b| a.name.cmp(&b.name));
    assert_eq!(flat, grouped);
    assert!(!list.is_empty());
}

#[test]
fn no_recipes_gives_an_empty_list() {
    let dir = dir_with(&[("a.cook", "Add @tomatoes{3}.\n")]);

    let list = generate(&ctx(&dir), request(&[])).expect("generates").value;
    assert!(list.is_empty());
    assert!(list.categories.is_empty());
}

/// `items` keeps the order the recipes introduced the ingredients in, which is
/// what `--plain` shows.
#[test]
fn items_keep_recipe_order() {
    let dir = dir_with(&[
        ("a.cook", "Add @zucchini{1} then @apple{1}.\n"),
        ("b.cook", "Add @banana{1}.\n"),
    ]);

    let list = generate(&ctx(&dir), request(&["a.cook", "b.cook"]))
        .expect("generates")
        .value;

    let names: Vec<&str> = list.items.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, vec!["zucchini", "apple", "banana"]);
}
