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

/// A path-backed recipe at its authored scale — the form the CLI builds.
fn at_path(name: &str) -> ScaledRecipe {
    ScaledRecipe::new(RecipeSource::Path(name.into()))
}

fn request(names: &[&str]) -> GenerateRequest {
    GenerateRequest {
        recipes: names.iter().map(|n| at_path(n)).collect(),
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

/// Quantities that add up are summed into one; quantities that do not are kept
/// side by side rather than silently added, ordered by unit name.
///
/// `cooklang`'s unit database is off, so quantities keep the unit they were
/// authored in and only *identical* units add up: `ml` and `l` now stay apart
/// just as `g` and `cup` always did.
#[test]
fn same_unit_quantities_combine_and_others_stay_separate() {
    let dir = dir_with(&[
        ("a.cook", "Pour @milk{500%ml} and @flour{200%g}.\n"),
        ("b.cook", "Pour @milk{250%ml} and @flour{1%cup}.\n"),
    ]);

    let list = generate(&ctx(&dir), request(&["a.cook", "b.cook"]))
        .expect("generates")
        .value;

    assert_eq!(quantities(&list, "milk"), Some(vec!["750 ml".to_string()]));
    // Two entries rather than one, because grams and cups do not convert, in
    // unit-name order — not the order the recipes were written in, which the
    // grouping has already lost. See `format::quantity::ordered_components`.
    assert_eq!(
        quantities(&list, "flour"),
        Some(vec!["1 cup".to_string(), "200 g".to_string()])
    );
}

/// The same ingredient measured two ways renders in one fixed order, whichever
/// recipe is read first.
///
/// Without this, the order comes out of a `HashMap` and changes from run to
/// run. Asserting the exact string rather than "one of two orders" is the
/// point: a single run of a weaker assertion passes by luck.
#[test]
fn inconvertible_quantities_render_in_the_same_order_either_way_round() {
    let cup_first = dir_with(&[
        ("a.cook", "Mix @flour{1%cup}.\n"),
        ("b.cook", "Mix @flour{100%g}.\n"),
    ]);
    let gram_first = dir_with(&[
        ("a.cook", "Mix @flour{100%g}.\n"),
        ("b.cook", "Mix @flour{1%cup}.\n"),
    ]);

    for dir in [&cup_first, &gram_first] {
        let list = generate(&ctx(dir), request(&["a.cook", "b.cook"]))
            .expect("generates")
            .value;
        assert_eq!(
            quantities(&list, "flour"),
            Some(vec!["1 cup".to_string(), "100 g".to_string()])
        );
    }
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
                ScaledRecipe::scaled(RecipeSource::Path("a.cook".into()), 2.0),
                ScaledRecipe::scaled(RecipeSource::Path("b.cook".into()), 10.0),
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
            recipes: vec![at_path("main.cook")],
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
/// other terminate instead of looping — and silently double-count, which is
/// <https://github.com/cooklang/cookcli/issues/424>. This is the test that has
/// to change when that is fixed: a recursive expansion would error on the cycle
/// instead, and would need a `CoreError` variant reintroduced for it.
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
        &at_path("main.cook"),
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
        &at_path("main.cook"),
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
        extract_ingredients(&ctx, &at_path(name), &ExtractOptions::default(), &mut list)
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

// ---------------------------------------------------------------------------
// In-memory recipes
//
// The case a path-only API cannot serve: an editor putting the buffer someone
// is typing into onto a shopping list, before it has ever been saved.
// ---------------------------------------------------------------------------

/// Recipe text with no file behind it at all — the directory is empty, so a
/// lookup could not have found anything even if one happened.
#[test]
fn in_memory_recipe_text_reaches_the_list() {
    let dir = dir_with(&[]);

    let list = generate(
        &ctx(&dir),
        GenerateRequest {
            recipes: vec![ScaledRecipe::new(RecipeSource::Content {
                text: "Add @tomatoes{3} and @salt{1%tsp}.\n".to_string(),
                name: "unsaved buffer".to_string(),
            })],
            ignore_references: false,
        },
    )
    .expect("generates")
    .value;

    assert_eq!(quantities(&list, "tomatoes"), Some(vec!["3".to_string()]));
    assert_eq!(quantities(&list, "salt"), Some(vec!["1 tsp".to_string()]));
}

/// Two buffers merge into one list exactly as two files would. This is the
/// editor's `generateShoppingList`, which takes an array of recipe texts.
#[test]
fn two_in_memory_recipes_aggregate_into_one_list() {
    let dir = dir_with(&[]);

    let list = generate(
        &ctx(&dir),
        GenerateRequest {
            recipes: vec![
                ScaledRecipe::new(RecipeSource::Content {
                    text: "Add @tomatoes{3} and @milk{500%ml}.\n".to_string(),
                    name: "first".to_string(),
                }),
                ScaledRecipe::new(RecipeSource::Content {
                    text: "Add @tomatoes{2} and @milk{1%l}.\n".to_string(),
                    name: "second".to_string(),
                }),
            ],
            ignore_references: false,
        },
    )
    .expect("generates")
    .value;

    assert_eq!(quantities(&list, "tomatoes"), Some(vec!["5".to_string()]));
    // `ml` and `l` do not add up without a unit database, so both survive, in
    // unit-name order.
    assert_eq!(
        quantities(&list, "milk"),
        Some(vec!["1 l".to_string(), "500 ml".to_string()])
    );
}

/// A path recipe and a buffer in the same request, aggregating together —
/// nothing about `Content` puts it in a separate list.
#[test]
fn a_buffer_and_a_file_aggregate_together() {
    let dir = dir_with(&[("a.cook", "Add @tomatoes{3}.\n")]);

    let list = generate(
        &ctx(&dir),
        GenerateRequest {
            recipes: vec![
                at_path("a.cook"),
                ScaledRecipe::new(RecipeSource::Content {
                    text: "Add @tomatoes{2}.\n".to_string(),
                    name: "buffer".to_string(),
                }),
            ],
            ignore_references: false,
        },
    )
    .expect("generates")
    .value;

    assert_eq!(quantities(&list, "tomatoes"), Some(vec!["5".to_string()]));
}

/// The documented boundary: only the *starting* recipe comes from memory. A
/// buffer that references `@./sauce{}` still has that reference resolved from
/// disk under `base_path`, and the referenced recipe's ingredients land on the
/// list.
#[test]
fn an_in_memory_recipe_expands_references_from_disk() {
    let dir = dir_with(&[("sauce.cook", "Simmer @tomatoes{4} in @oil{1%tbsp}.\n")]);

    let list = generate(
        &ctx(&dir),
        GenerateRequest {
            recipes: vec![ScaledRecipe::new(RecipeSource::Content {
                text: "Prepare @./sauce{} and add @basil{1}.\n".to_string(),
                name: "buffer".to_string(),
            })],
            ignore_references: false,
        },
    )
    .expect("generates")
    .value;

    assert_eq!(quantities(&list, "tomatoes"), Some(vec!["4".to_string()]));
    assert_eq!(quantities(&list, "oil"), Some(vec!["1 tbsp".to_string()]));
    assert_eq!(quantities(&list, "basil"), Some(vec!["1".to_string()]));
    // Expanded, not listed: `sauce` itself is not something to buy. That is
    // the difference from `ignore_references`, and it must not depend on
    // whether the recipe came from a file or a buffer.
    assert_eq!(quantities(&list, "sauce"), None, "{:?}", list.items);
}

/// The other half of that boundary, stated as a failure: a buffer referencing
/// a recipe that exists only in another buffer cannot work, and says so in the
/// same words a path recipe would.
#[test]
fn a_reference_from_a_buffer_to_an_unsaved_recipe_is_not_found() {
    let dir = dir_with(&[]);

    match generate(
        &ctx(&dir),
        GenerateRequest {
            recipes: vec![ScaledRecipe::new(RecipeSource::Content {
                text: "Prepare @./sauce{}.\n".to_string(),
                name: "buffer".to_string(),
            })],
            ignore_references: false,
        },
    ) {
        Err(CoreError::RecipeNotFound { name }) => assert!(name.contains("sauce"), "{name}"),
        other => panic!("expected RecipeNotFound, got {other:?}"),
    }
}

/// Mutation guard: `Content` must use the text it was handed and never look
/// its `name` up. `decoy.cook` exists on disk with entirely different
/// ingredients, so an implementation that resolved the name would be caught.
#[test]
fn in_memory_text_is_used_and_the_name_is_never_looked_up() {
    let dir = dir_with(&[("decoy.cook", "Simmer @bones{1%kg}.\n")]);

    let list = generate(
        &ctx(&dir),
        GenerateRequest {
            recipes: vec![ScaledRecipe::new(RecipeSource::Content {
                text: "Add @tomatoes{3}.\n".to_string(),
                name: "decoy.cook".to_string(),
            })],
            ignore_references: false,
        },
    )
    .expect("generates")
    .value;

    assert_eq!(quantities(&list, "tomatoes"), Some(vec!["3".to_string()]));
    assert_eq!(
        quantities(&list, "bones"),
        None,
        "the file named by `name` must not be read: {:?}",
        list.items
    );
}

#[test]
fn the_request_scale_applies_to_an_in_memory_recipe() {
    let dir = dir_with(&[]);

    let list = generate(
        &ctx(&dir),
        GenerateRequest {
            recipes: vec![ScaledRecipe::scaled(
                RecipeSource::Content {
                    text: "Add @tomatoes{3}.\n".to_string(),
                    name: "buffer".to_string(),
                },
                4.0,
            )],
            ignore_references: false,
        },
    )
    .expect("generates")
    .value;

    assert_eq!(quantities(&list, "tomatoes"), Some(vec!["12".to_string()]));
}

/// Mutation guard: the supplied `name` is what identifies a buffer that will
/// not parse. Nothing else can — there is no file to name — so an
/// implementation that dropped it, or substituted a placeholder, would leave
/// the caller unable to say which of several buffers was at fault.
#[test]
fn a_broken_buffer_is_a_parse_error_naming_the_supplied_name() {
    let dir = dir_with(&[]);

    match generate(
        &ctx(&dir),
        GenerateRequest {
            recipes: vec![ScaledRecipe::new(RecipeSource::Content {
                text: "Add @{1%tsp}.\n".to_string(),
                name: "Untitled-1".to_string(),
            })],
            ignore_references: false,
        },
    ) {
        Err(CoreError::Parse {
            name,
            diagnostics,
            rendered,
        }) => {
            assert_eq!(name, "Untitled-1");
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].severity, Severity::Error);
            assert!(
                rendered.contains("Untitled-1"),
                "the rendered report should point at the buffer: {rendered}"
            );
        }
        other => panic!("expected CoreError::Parse, got {other:?}"),
    }
}

/// A warning from a buffer carries a span but no file, because there is no
/// file to open — the same shape `recipe::read` produces for `Content`. Pinned
/// so that a future change cannot quietly invent a path here.
#[test]
fn a_warning_from_a_buffer_carries_no_file() {
    let dir = dir_with(&[]);

    let outcome = generate(
        &ctx(&dir),
        GenerateRequest {
            // Deprecated `>>` metadata parses, but warns.
            recipes: vec![ScaledRecipe::new(RecipeSource::Content {
                text: ">> title: Old Style\n\nAdd @salt{1%tsp}.\n".to_string(),
                name: "buffer".to_string(),
            })],
            ignore_references: false,
        },
    )
    .expect("parses despite warning");

    let diagnostic = outcome
        .diagnostics
        .iter()
        .find(|d| d.severity == Severity::Warning && d.location.is_some())
        .unwrap_or_else(|| panic!("expected a recipe warning, got {:?}", outcome.diagnostics));
    let location = diagnostic.location.as_ref().expect("filtered for Some");
    assert_eq!(location.file, None, "{diagnostic:?}");
    assert!(location.span.is_some(), "{diagnostic:?}");
}
