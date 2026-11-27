use super::*;

fn temp() -> tempfile::TempDir {
    tempfile::TempDir::new().unwrap()
}

fn base(dir: &tempfile::TempDir) -> Utf8PathBuf {
    Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap()
}

fn store(dir: &tempfile::TempDir) -> ShoppingListStore {
    ShoppingListStore::new(&base(dir))
}

/// A plain recipe entry, as a caller of [`ShoppingListStore::add`] builds one.
fn entry(path: &str, scale: f64) -> StoredEntry {
    StoredEntry {
        name: recipe_display_name(path),
        path: path.to_string(),
        scale,
        included_references: None,
        included_reference_names: None,
        included_reference_scales: None,
        recipes: None,
        optional_ingredients: Vec::new(),
    }
}

fn list_file(dir: &tempfile::TempDir) -> String {
    std::fs::read_to_string(base(dir).join(".shopping-list").as_std_path()).unwrap()
}

fn checked_file(dir: &tempfile::TempDir) -> String {
    std::fs::read_to_string(base(dir).join(".shopping-checked").as_std_path()).unwrap()
}

fn write(path: &Utf8Path, contents: &str) {
    std::fs::write(path.as_std_path(), contents).unwrap();
}

/// Names in the collection directory, sorted, so a stray temporary file shows
/// up rather than being missed.
fn names_in(dir: &tempfile::TempDir) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

// -- Round trips ------------------------------------------------------------

#[test]
fn a_collection_with_no_list_reads_as_an_empty_one() {
    let dir = temp();
    assert!(store(&dir).load().expect("loads").is_empty());
}

#[test]
fn an_added_recipe_comes_back_with_its_path_name_and_scale() {
    let dir = temp();
    let store = store(&dir);
    store
        .add(entry("Breakfast/Easy Pancakes.cook", 2.0))
        .unwrap();

    let items = store.load().expect("loads");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].path, "Breakfast/Easy Pancakes.cook");
    assert_eq!(items[0].name, "Easy Pancakes");
    assert_eq!(items[0].scale, 2.0);
    assert_eq!(items[0].included_references.as_deref(), Some(&[][..]));
    assert!(items[0].recipes.is_none());
}

/// The multiplier is the whole point of storing a scale: losing it on the way
/// to disk or back buys the wrong amount of everything.
#[test]
fn a_multiplier_survives_the_round_trip() {
    let dir = temp();
    let store = store(&dir);
    store.add(entry("Soup.cook", 2.5)).unwrap();

    assert!(
        list_file(&dir).contains("{2.5}"),
        "the multiplier must be written: {:?}",
        list_file(&dir)
    );
    assert_eq!(store.load().unwrap()[0].scale, 2.5);
}

/// A scale of 1 is written as a bare path — the format's implicit ×1 — and has
/// to read back as 1 rather than as nothing.
#[test]
fn a_scale_of_one_is_stored_without_a_multiplier() {
    let dir = temp();
    let store = store(&dir);
    store.add(entry("Soup.cook", 1.0)).unwrap();

    assert!(
        !list_file(&dir).contains('{'),
        "no multiplier expected: {:?}",
        list_file(&dir)
    );
    assert_eq!(store.load().unwrap()[0].scale, 1.0);
}

#[test]
fn included_references_are_stored_as_children_and_read_back() {
    let dir = temp();
    let store = store(&dir);
    store
        .add(StoredEntry {
            included_references: Some(vec!["./Sauce".to_string(), "Sides/Rice".to_string()]),
            ..entry("Dinner.cook", 1.0)
        })
        .unwrap();

    let items = store.load().expect("loads");
    assert_eq!(
        items[0].included_references.as_deref(),
        Some(&["Sauce".to_string(), "Sides/Rice".to_string()][..]),
        "the leading ./ is stripped on the way in; the writer puts it back"
    );
}

fn selection(name: &str, quantity: Option<&str>) -> OptionalSelection {
    OptionalSelection {
        name: name.to_string(),
        quantity: quantity.map(str::to_string),
    }
}

/// Chosen optional ingredients are `? name{quantity}` selection lines under
/// the recipe, written before its references, and read back as they were.
#[test]
fn optional_ingredients_are_stored_as_selection_lines_and_read_back() {
    let dir = temp();
    let store = store(&dir);
    let chosen = vec![
        selection("chilli flakes", Some("2%pinch")),
        selection("chives", None),
    ];
    store
        .add(StoredEntry {
            included_references: Some(vec!["Components/Salsa".to_string()]),
            optional_ingredients: chosen.clone(),
            ..entry("Breakfast/Eggs on toast.cook", 2.0)
        })
        .unwrap();

    assert_eq!(
        list_file(&dir),
        "./Breakfast/Eggs on toast.cook{2}\n  ? chilli flakes{2%pinch}\n  ? chives\n  ./Components/Salsa\n"
    );

    let items = store.load().expect("loads");
    assert_eq!(items[0].optional_ingredients, chosen);
    assert_eq!(
        items[0].included_references.as_deref(),
        Some(&["Components/Salsa".to_string()][..]),
        "a selection line is not a reference"
    );
}

/// A list written by another app may accept an optional recipe with a
/// `? ./path` line. This store does not write those, and must not mistake one
/// for a reference to include.
#[test]
fn an_optional_recipe_selection_is_not_an_included_reference() {
    let dir = temp();
    write(
        &base(&dir).join(".shopping-list"),
        "./Steak.cook\n  ? ./sauces/chimichurri{2}\n  ./Sides/Rice\n",
    );

    let items = store(&dir).load().expect("loads");
    assert_eq!(
        items[0].included_references.as_deref(),
        Some(&["Sides/Rice".to_string()][..])
    );
    assert!(items[0].optional_ingredients.is_empty());
}

#[test]
fn a_menu_recipe_keeps_its_optional_ingredients() {
    let dir = temp();
    let store = store(&dir);
    store
        .add_menu(
            "Plans/Week 1.menu".to_string(),
            1.0,
            vec![StoredEntry {
                optional_ingredients: vec![selection("parmesan", Some("50%g"))],
                ..entry("Risotto.cook", 1.0)
            }],
        )
        .unwrap();

    let items = store.load().expect("loads");
    let recipes = items[0].recipes.as_ref().expect("a menu carries recipes");
    assert_eq!(
        recipes[0].optional_ingredients,
        vec![selection("parmesan", Some("50%g"))]
    );
}

#[test]
fn a_menu_reads_back_with_its_recipes_nested() {
    let dir = temp();
    let store = store(&dir);
    store
        .add_menu(
            "Plans/Week 1.menu".to_string(),
            2.0,
            vec![
                StoredEntry {
                    included_references: Some(vec!["./Sauce".to_string()]),
                    ..entry("Dinner.cook", 3.0)
                },
                entry("Soup.cook", 1.0),
            ],
        )
        .unwrap();

    let items = store.load().expect("loads");
    assert_eq!(items.len(), 1, "a menu is one entry, not one per recipe");
    assert_eq!(items[0].name, "Week 1");
    assert_eq!(items[0].scale, 2.0);

    let recipes = items[0].recipes.as_ref().expect("a menu carries recipes");
    assert_eq!(recipes.len(), 2);
    assert_eq!(recipes[0].path, "Dinner.cook");
    assert_eq!(
        recipes[0].scale, 3.0,
        "each recipe keeps its own multiplier"
    );
    assert_eq!(
        recipes[0].included_references.as_deref(),
        Some(&["Sauce".to_string()][..])
    );
    assert_eq!(recipes[1].scale, 1.0);
}

#[test]
fn adding_the_same_recipe_twice_stores_it_twice() {
    let dir = temp();
    let store = store(&dir);
    store.add(entry("Soup.cook", 1.0)).unwrap();
    store.add(entry("Soup.cook", 1.0)).unwrap();

    assert_eq!(store.load().unwrap().len(), 2);
}

#[test]
fn remove_takes_the_named_entry_and_leaves_the_others_in_order() {
    let dir = temp();
    let store = store(&dir);
    store.add(entry("Soup.cook", 1.0)).unwrap();
    store.add(entry("Dinner.cook", 4.0)).unwrap();
    store.add(entry("Cake.cook", 1.0)).unwrap();

    store.remove("Dinner.cook").expect("removes");

    let paths: Vec<_> = store.load().unwrap().into_iter().map(|i| i.path).collect();
    assert_eq!(paths, ["Soup.cook", "Cake.cook"]);
}

#[test]
fn removing_something_that_is_not_there_changes_nothing() {
    let dir = temp();
    let store = store(&dir);
    store.add(entry("Soup.cook", 1.0)).unwrap();

    store.remove("Nowhere.cook").expect("removes nothing");

    let paths: Vec<_> = store.load().unwrap().into_iter().map(|i| i.path).collect();
    assert_eq!(paths, ["Soup.cook"]);
}

/// The same recipe twice, the first with its stock and the second without —
/// what a page that adds a recipe, unticks a sub-recipe and adds it again
/// stores.
fn soup_twice(store: &ShoppingListStore) {
    store
        .add(StoredEntry {
            included_references: Some(vec!["./Stock.cook".into()]),
            ..entry("Soup.cook", 1.0)
        })
        .unwrap();
    store
        .add(StoredEntry {
            included_references: Some(vec![]),
            ..entry("Soup.cook", 2.0)
        })
        .unwrap();
}

#[test]
fn remove_at_takes_that_entry_and_not_the_first_with_its_path() {
    let dir = temp();
    let store = store(&dir);
    soup_twice(&store);

    assert!(store.remove_at(1, "Soup.cook").expect("removes"));

    let items = store.load().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].scale, 1.0, "the first entry stays");
    assert_eq!(
        items[0].included_references.as_deref(),
        Some(&["Stock.cook".to_string()][..]),
        "with its sub-recipe"
    );
}

#[test]
fn remove_at_changes_nothing_when_the_entry_is_not_there_any_more() {
    let dir = temp();
    let store = store(&dir);
    soup_twice(&store);
    store.add(entry("Cake.cook", 1.0)).unwrap();
    let before = list_file(&dir);

    // Another path at that position: the list changed under the caller.
    assert!(!store.remove_at(2, "Soup.cook").unwrap());
    // Past the end.
    assert!(!store.remove_at(3, "Soup.cook").unwrap());

    assert_eq!(list_file(&dir), before);
}

#[test]
fn remove_at_counts_the_entries_load_lists() {
    let dir = temp();
    let store = store(&dir);
    // A free-hand ingredient, written by one of the apps, comes first in the
    // file but is not one of the entries `load` returns.
    write(
        &base(&dir).join(".shopping-list"),
        "salt\n./Soup.cook\n./Cake.cook\n",
    );

    assert!(store.remove_at(1, "Cake.cook").expect("removes"));

    assert_eq!(list_file(&dir), "salt\n./Soup.cook\n");
}

#[test]
fn clear_empties_the_list_and_the_checked_log() {
    let dir = temp();
    let store = store(&dir);
    store.add(entry("Soup.cook", 1.0)).unwrap();
    store.check("Onion").unwrap();

    store.clear().expect("clears");

    assert!(store.load().unwrap().is_empty());
    assert!(store.checked_set().unwrap().is_empty());
    assert_eq!(
        std::fs::read_to_string(base(&dir).join(".shopping-checked").as_std_path()).unwrap(),
        "",
        "the checked log is emptied, not removed, so it keeps its owner"
    );
}

/// A list that cannot be parsed must be reported, not silently treated as an
/// empty one — that would replace what the user has with nothing.
#[test]
fn an_unparseable_list_is_an_error_naming_the_file() {
    let dir = temp();
    let path = base(&dir).join(".shopping-list");
    write(&path, "  ./Soup\n");

    match store(&dir).load() {
        Err(CoreError::InvalidShoppingList { path: named, .. }) => assert_eq!(named, path),
        other => panic!("expected InvalidShoppingList, got {other:?}"),
    }
}

// -- Checked state ----------------------------------------------------------

#[test]
fn a_checked_ingredient_is_remembered_lowercased() {
    let dir = temp();
    let store = store(&dir);
    store.check("Onion").expect("checks");

    assert_eq!(checked_file(&dir), "+ Onion\n");
    assert!(store.checked_set().unwrap().contains("onion"));
}

#[test]
fn unchecking_wins_over_an_earlier_check() {
    let dir = temp();
    let store = store(&dir);
    store.check("Onion").unwrap();
    store.check("Salt").unwrap();
    store.uncheck("Onion").unwrap();

    let checked = store.checked_set().expect("reads");
    assert!(!checked.contains("onion"), "the later entry wins");
    assert!(checked.contains("salt"));
}

/// The log is append-only, so a check survives a process that never gets to
/// write anything else.
#[test]
fn checks_accumulate_across_stores() {
    let dir = temp();
    store(&dir).check("Onion").unwrap();
    store(&dir).check("Salt").unwrap();

    assert_eq!(checked_file(&dir), "+ Onion\n+ Salt\n");
    assert_eq!(store(&dir).checked_set().unwrap().len(), 2);
}

/// Compaction replays the log and rewrites it as one `+ name` line per
/// ingredient that is still checked — lowercased, since matching is
/// case-insensitive, and sorted, so the file does not churn between runs.
#[test]
fn compact_keeps_checks_for_ingredients_still_on_the_list() {
    let dir = temp();
    let store = store(&dir);
    store.check("Onion").unwrap();
    store.check("Butter").unwrap();
    store.check("Salt").unwrap();
    store.uncheck("Salt").unwrap();

    store
        .compact(["Onion", "butter", "flour"])
        .expect("compacts");

    assert_eq!(
        checked_file(&dir),
        "+ butter\n+ onion\n",
        "the unchecked and the absent both go; what is left is sorted"
    );
    assert!(store.checked_set().unwrap().contains("onion"));
}

#[test]
fn compact_drops_checks_for_ingredients_that_are_gone() {
    let dir = temp();
    let store = store(&dir);
    store.check("Onion").unwrap();

    store.compact(["flour"]).expect("compacts");

    assert_eq!(checked_file(&dir), "");
    assert!(store.checked_set().unwrap().is_empty());
}

// -- Migration --------------------------------------------------------------

/// The tab-delimited `.shopping_list.txt` older CookCLI versions wrote:
/// path, display name, scale.
const LEGACY: &str = "\
# a comment
Breakfast/Easy Pancakes.cook\tEasy Pancakes\t2

Soup.cook\tSoup\t1
";

#[test]
fn a_legacy_list_is_migrated_on_first_read() {
    let dir = temp();
    write(&base(&dir).join(".shopping_list.txt"), LEGACY);

    let items = store(&dir).load().expect("loads");

    let paths: Vec<_> = items.iter().map(|i| i.path.as_str()).collect();
    assert_eq!(paths, ["Breakfast/Easy Pancakes.cook", "Soup.cook"]);
    assert_eq!(items[0].scale, 2.0, "the legacy scale is carried over");
    assert_eq!(items[1].scale, 1.0);
}

#[test]
fn migration_renames_the_legacy_file_so_it_is_not_read_again() {
    let dir = temp();
    write(&base(&dir).join(".shopping_list.txt"), LEGACY);

    store(&dir).load().expect("loads");
    assert_eq!(
        names_in(&dir),
        [".shopping-list", ".shopping_list.txt.bak"],
        "the legacy file is kept as a backup, under a name nothing reads"
    );

    // A second store adding to the migrated list must not resurrect the legacy
    // entries or lose the new one.
    let store = store(&dir);
    store.add(entry("Cake.cook", 1.0)).unwrap();
    let paths: Vec<_> = store.load().unwrap().into_iter().map(|i| i.path).collect();
    assert_eq!(
        paths,
        ["Breakfast/Easy Pancakes.cook", "Soup.cook", "Cake.cook"]
    );
}

/// Adding is the other entry point that has to migrate first: writing the new
/// file without migrating would hide the legacy one for good.
#[test]
fn adding_to_a_legacy_list_migrates_it_rather_than_replacing_it() {
    let dir = temp();
    write(&base(&dir).join(".shopping_list.txt"), LEGACY);

    let store = store(&dir);
    store.add(entry("Cake.cook", 1.0)).expect("adds");

    let paths: Vec<_> = store.load().unwrap().into_iter().map(|i| i.path).collect();
    assert_eq!(
        paths,
        ["Breakfast/Easy Pancakes.cook", "Soup.cook", "Cake.cook"],
        "the legacy entries must still be there"
    );
}

/// Migration is for a collection that has no new-format list. One that does is
/// the authority, whatever an old file beside it says.
#[test]
fn an_existing_list_is_not_overwritten_by_a_legacy_one() {
    let dir = temp();
    write(&base(&dir).join(".shopping_list.txt"), LEGACY);
    store(&dir).add(entry("Cake.cook", 1.0)).unwrap();
    // Both files now exist: the legacy one was renamed by that first add, so
    // put it back to stand for a stale copy someone restored.
    write(&base(&dir).join(".shopping_list.txt"), LEGACY);

    let paths: Vec<_> = store(&dir)
        .load()
        .unwrap()
        .into_iter()
        .map(|i| i.path)
        .collect();
    assert!(
        paths.contains(&"Cake.cook".to_string()),
        "the new-format list survives: {paths:?}"
    );
    assert!(
        base(&dir).join(".shopping_list.txt").exists(),
        "and the legacy file is left alone rather than renamed again"
    );
}

// -- Atomic writes ----------------------------------------------------------

/// The property every rewrite in here is for: a failure leaves the list that
/// was there.
///
/// The failure is forced by making `.shopping-list` a non-empty directory,
/// which nothing can be renamed onto — no permission bits, so it means the
/// same thing under root and in a container.
#[test]
fn a_failed_list_write_leaves_no_temporary_file_behind() {
    let dir = temp();
    let path = base(&dir).join(".shopping-list");
    std::fs::create_dir_all(path.join("child").as_std_path()).unwrap();

    match store(&dir).add(entry("Soup.cook", 1.0)) {
        Err(CoreError::Io { path: named, .. }) => assert_eq!(named, path),
        other => panic!("expected Io, got {other:?}"),
    }

    assert_eq!(
        names_in(&dir),
        [".shopping-list"],
        "no temporary file is left behind"
    );
}

#[test]
fn a_failed_compact_leaves_the_checked_log_alone() {
    let dir = temp();
    let path = base(&dir).join(".shopping-checked");
    std::fs::create_dir_all(path.join("child").as_std_path()).unwrap();

    match store(&dir).compact(["onion"]) {
        Err(CoreError::Io { path: named, .. }) => assert_eq!(named, path),
        other => panic!("expected Io, got {other:?}"),
    }

    assert_eq!(names_in(&dir), [".shopping-checked"]);
}

/// A rewrite must never be visible half-done: the previous list stays readable
/// right up to the rename, so a reader either sees all of the old file or all
/// of the new one.
#[test]
fn a_rewrite_never_shortens_the_file_in_place() {
    let dir = temp();
    let store = store(&dir);
    for i in 0..40 {
        store.add(entry(&format!("Recipe {i}.cook"), 1.0)).unwrap();
    }
    let long = list_file(&dir);

    // Replacing 40 entries with 1 shrinks the file by an order of magnitude.
    // Were it truncated in place, the inode holding `long` would shrink too.
    let before = std::fs::metadata(base(&dir).join(".shopping-list").as_std_path()).unwrap();
    store.clear().expect("clears");
    let after = std::fs::metadata(base(&dir).join(".shopping-list").as_std_path()).unwrap();

    assert!(long.len() > 200, "the fixture has to be worth shrinking");
    assert_eq!(list_file(&dir), "", "the list really was replaced");
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        assert_ne!(
            before.ino(),
            after.ino(),
            "the new contents must arrive as a different file, renamed into \
             place — the same inode would mean the old list was truncated"
        );
    }
    #[cfg(not(unix))]
    let _ = (before, after);
}

// -- Free-hand items -------------------------------------------------------

fn free_hand(name: &str, quantity: Option<&str>) -> FreeHandItem {
    FreeHandItem {
        name: name.to_string(),
        quantity: quantity.map(String::from),
    }
}

#[test]
fn add_all_appends_recipes_then_free_hand_items_in_one_write() {
    let dir = temp();
    let store = store(&dir);
    store.add(entry("Soup.cook", 1.0)).unwrap();

    store
        .add_all(
            vec![entry("Risotto.cook", 2.0), entry("Salad.cook", 1.0)],
            vec![
                free_hand("almonds", Some("100%g")),
                free_hand("paper towels", None),
            ],
        )
        .expect("adds");

    assert_eq!(
        list_file(&dir),
        "./Soup.cook\n./Risotto.cook{2}\n./Salad.cook\nalmonds{100%g}\npaper towels\n"
    );
    let paths: Vec<String> = store.load().unwrap().into_iter().map(|e| e.path).collect();
    assert_eq!(paths, ["Soup.cook", "Risotto.cook", "Salad.cook"]);
    assert_eq!(
        store.load_free_hand().unwrap(),
        [
            free_hand("almonds", Some("100%g")),
            free_hand("paper towels", None)
        ]
    );
}

#[test]
fn free_hand_lines_written_by_another_app_are_read() {
    let dir = temp();
    write(
        &base(&dir).join(".shopping-list"),
        "salt\n./Pancakes{2}\n  flour{500%g}\nmilk{1%l}\n",
    );
    let store = store(&dir);

    // Only the top level: `flour` belongs to the pancakes.
    assert_eq!(
        store.load_free_hand().unwrap(),
        [free_hand("salt", None), free_hand("milk", Some("1%l"))]
    );
    assert_eq!(store.load().unwrap().len(), 1);
}

#[test]
fn remove_free_hand_takes_the_first_item_with_that_name_and_amount() {
    let dir = temp();
    let store = store(&dir);
    store
        .add_all(
            vec![entry("Soup.cook", 1.0)],
            vec![
                free_hand("coffee", Some("1%cup")),
                free_hand("coffee", Some("2%cup")),
                free_hand("coffee", Some("1%cup")),
            ],
        )
        .unwrap();

    store
        .remove_free_hand(&free_hand("coffee", Some("1%cup")))
        .expect("removes");
    store
        .remove_free_hand(&free_hand("coffee", None))
        .expect("nothing to remove is fine");

    assert_eq!(
        list_file(&dir),
        "./Soup.cook\ncoffee{2%cup}\ncoffee{1%cup}\n"
    );
}

#[test]
fn removing_a_recipe_leaves_free_hand_items_alone() {
    let dir = temp();
    let store = store(&dir);
    store
        .add_all(
            vec![entry("Soup.cook", 1.0)],
            vec![free_hand("Soup.cook", None)],
        )
        .unwrap();

    store.remove("Soup.cook").unwrap();

    assert_eq!(list_file(&dir), "Soup.cook\n");
}

#[test]
fn a_free_hand_item_spec_is_its_shopping_list_line() {
    assert_eq!(free_hand("almonds", Some("50%g")).spec(), "almonds{50%g}");
    assert_eq!(free_hand("paper towels", None).spec(), "paper towels{}");
}

/// Names that would be Cooklang markup in a recipe — a tag, a timer, an
/// ingredient sigil — are only names on the list, and are ticked as such.
#[test]
fn free_hand_names_that_look_like_markup_round_trip_and_tick() {
    let dir = temp();
    let store = store(&dir);
    let items = vec![
        free_hand("#1 flour", Some("2")),
        free_hand("~ramen", None),
        free_hand("@milk", Some("1%l")),
    ];
    store.add_all(Vec::new(), items.clone()).unwrap();

    assert_eq!(list_file(&dir), "#1 flour{2}\n~ramen\n@milk{1%l}\n");
    assert_eq!(store.load_free_hand().unwrap(), items);

    for item in &items {
        store.check(&item.name).unwrap();
    }
    store.compact(items.iter().map(|i| i.name.clone())).unwrap();
    let checked = store.checked_set().unwrap();
    for name in ["#1 flour", "~ramen", "@milk"] {
        assert!(checked.contains(name), "{name} ticked: {checked:?}");
    }
    for item in &items {
        store.uncheck(&item.name).unwrap();
    }
    assert!(store.checked_set().unwrap().is_empty());
}

/// A quantity is matched by what it reads as, so a client holding another
/// spelling of the stored amount still finds the line; a rewrite then stores
/// the amount in the format's own spelling.
#[test]
fn a_free_hand_quantity_is_matched_by_value_and_normalised_on_rewrite() {
    let dir = temp();
    write(
        &base(&dir).join(".shopping-list"),
        "sugar{0.50%kg}\nsalt{2}\n",
    );
    let store = store(&dir);

    assert_eq!(
        store.load_free_hand().unwrap()[0],
        free_hand("sugar", Some("0.5%kg"))
    );
    store
        .remove_free_hand(&free_hand("sugar", Some("0.50%kg")))
        .unwrap();
    assert_eq!(list_file(&dir), "salt{2}\n");
}

/// Text that is no quantity — or that the format could not hold between
/// braces — never matches a line without one.
#[test]
fn an_unreadable_quantity_does_not_match_a_bare_line() {
    let dir = temp();
    write(&base(&dir).join(".shopping-list"), "salt\n");
    let store = store(&dir);

    for text in ["%", "[- open", "a -- b"] {
        store
            .remove_free_hand(&free_hand("salt", Some(text)))
            .unwrap();
        assert_eq!(list_file(&dir), "salt\n", "{text:?}");
    }
}

// -- Typed items ---------------------------------------------------------------

fn free(name: &str, quantity: Option<&str>) -> FreeHandItem {
    FreeHandItem {
        name: name.to_string(),
        quantity: quantity.map(str::to_string),
    }
}

#[test]
fn typed_items_are_written_as_free_hand_lines() {
    let dir = temp();
    let store = store(&dir);
    store.add(entry("Soup.cook", 1.0)).unwrap();
    let written = store
        .add_typed(vec![free("sugar", Some("1.5%kg")), free("bread", None)])
        .unwrap();
    assert_eq!(
        written,
        [free("sugar", Some("1.5%kg")), free("bread", None)]
    );
    assert_eq!(list_file(&dir), "./Soup.cook\nsugar{1.5%kg}\nbread\n");
}

#[test]
fn the_same_name_and_unit_add_up_ignoring_case() {
    let dir = temp();
    let store = store(&dir);
    store
        .add_typed(vec![free("Sugar", Some("1.5%kg"))])
        .unwrap();
    let written = store.add_typed(vec![free("sugar", Some("1%kg"))]).unwrap();
    assert_eq!(written, [free("Sugar", Some("2.5%kg"))]);
    assert_eq!(list_file(&dir), "Sugar{2.5%kg}\n");
}

#[test]
fn a_different_unit_is_a_second_line() {
    let dir = temp();
    let store = store(&dir);
    store.add_typed(vec![free("milk", Some("2%l"))]).unwrap();
    store
        .add_typed(vec![free("milk", Some("500%ml")), free("milk", Some("2"))])
        .unwrap();
    assert_eq!(list_file(&dir), "milk{2%l}\nmilk{500%ml}\nmilk{2}\n");
}

#[test]
fn two_lines_without_an_amount_are_one() {
    let dir = temp();
    let store = store(&dir);
    store
        .add_typed(vec![free("bread", None), free("Bread", None)])
        .unwrap();
    assert_eq!(list_file(&dir), "bread\n");
}

#[test]
fn a_typed_item_merges_into_a_menu_day_line_too() {
    let dir = temp();
    let store = store(&dir);
    store
        .add_all(Vec::new(), vec![free("almonds", Some("50%g"))])
        .unwrap();
    store
        .add_typed(vec![free("almonds", Some("100%g"))])
        .unwrap();
    assert_eq!(list_file(&dir), "almonds{150%g}\n");
}

#[test]
fn an_edit_replaces_the_line_in_place() {
    let dir = temp();
    let store = store(&dir);
    store
        .add_typed(vec![free("milk", Some("2%l")), free("bread", None)])
        .unwrap();
    let found = store
        .replace_free_hand(
            &free("milk", Some("2%l")),
            free("oat milk", Some("1%l")),
            true,
        )
        .unwrap();
    assert_eq!(found, Some(free("oat milk", Some("1%l"))));
    assert_eq!(list_file(&dir), "oat milk{1%l}\nbread\n");
}

#[test]
fn an_edit_that_matches_another_line_folds_into_it() {
    let dir = temp();
    let store = store(&dir);
    store
        .add_typed(vec![
            free("sugar", Some("1%kg")),
            free("flour", Some("2%kg")),
        ])
        .unwrap();
    store
        .replace_free_hand(
            &free("flour", Some("2%kg")),
            free("Sugar", Some("2%kg")),
            true,
        )
        .unwrap();
    assert_eq!(list_file(&dir), "sugar{3%kg}\n");
}

#[test]
fn an_edit_of_a_line_that_is_gone_changes_nothing() {
    let dir = temp();
    let store = store(&dir);
    store.add_typed(vec![free("bread", None)]).unwrap();
    let found = store
        .replace_free_hand(&free("milk", Some("2%l")), free("milk", Some("3%l")), true)
        .unwrap();
    assert_eq!(found, None);
    assert_eq!(list_file(&dir), "bread\n");
}

#[test]
fn an_unchanged_edit_writes_nothing() {
    let dir = temp();
    let store = store(&dir);
    store.add_typed(vec![free("bread", None)]).unwrap();
    let path = base(&dir).join(".shopping-list");
    let before = std::fs::metadata(path.as_std_path())
        .unwrap()
        .modified()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    assert_eq!(
        store
            .replace_free_hand(&free("bread", None), free("bread", None), true)
            .unwrap(),
        Some(free("bread", None))
    );
    let after = std::fs::metadata(path.as_std_path())
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(before, after);
}

fn mtime(dir: &tempfile::TempDir) -> std::time::SystemTime {
    std::fs::metadata(base(dir).join(".shopping-list").as_std_path())
        .unwrap()
        .modified()
        .unwrap()
}

#[test]
fn a_batch_is_written_once_and_an_empty_one_not_at_all() {
    let dir = temp();
    let store = store(&dir);
    store
        .add_typed(vec![
            free("a", None),
            free("b", Some("1")),
            free("c", Some("2%g")),
        ])
        .unwrap();
    assert_eq!(list_file(&dir), "a\nb{1}\nc{2%g}\n");
    let before = mtime(&dir);
    std::thread::sleep(std::time::Duration::from_millis(20));
    assert!(store.add_typed(vec![]).unwrap().is_empty());
    assert_eq!(mtime(&dir), before);
}

#[test]
fn an_item_without_a_name_is_skipped() {
    let dir = temp();
    let store = store(&dir);
    let written = store
        .add_typed(vec![
            free("", Some("2")),
            free("  ", None),
            free("tea", None),
        ])
        .unwrap();
    assert_eq!(written, [free("tea", None)]);
    assert_eq!(list_file(&dir), "tea\n");

    let dir = temp();
    let store = self::store(&dir);
    assert!(store
        .add_typed(vec![free("", Some("2"))])
        .unwrap()
        .is_empty());
    assert!(!base(&dir).join(".shopping-list").exists());
}

#[test]
fn an_edit_to_no_name_is_refused() {
    let dir = temp();
    let store = store(&dir);
    store.add_typed(vec![free("milk", Some("2"))]).unwrap();
    let err = store
        .replace_free_hand(&free("milk", Some("2")), free(" ", Some("2")), true)
        .unwrap_err();
    assert!(matches!(err, CoreError::EmptyItemName), "{err:?}");
    assert_eq!(list_file(&dir), "milk{2}\n");
}

#[test]
fn names_compare_ignoring_unicode_composition() {
    let dir = temp();
    let store = store(&dir);
    store.add_typed(vec![free("caf\u{e9}", Some("1"))]).unwrap();
    store
        .add_typed(vec![free("cafe\u{301}", Some("1"))])
        .unwrap();
    assert_eq!(list_file(&dir), "caf\u{e9}{2}\n");
}

#[test]
fn a_recipes_nested_ingredient_is_left_alone() {
    let dir = temp();
    write(
        &base(&dir).join(".shopping-list"),
        "./Soup.cook\n  milk{1%l}\n",
    );
    let store = store(&dir);
    store.add_typed(vec![free("milk", Some("1%l"))]).unwrap();
    assert_eq!(list_file(&dir), "./Soup.cook\n  milk{1%l}\nmilk{1%l}\n");
}

#[test]
fn an_edit_keeping_a_text_quantity_does_not_fold() {
    let dir = temp();
    let store = store(&dir);
    write(
        &base(&dir).join(".shopping-list"),
        "salt{a pinch}\nsea salt{1%tbsp}\n",
    );
    let old = free("salt", Some("a pinch"));
    let typed = crate::shopping_list::edited_item(&old, "sea salt", None).unwrap();
    assert_eq!(typed.item, free("sea salt", Some("a pinch")));
    assert!(typed.keeps_quantity);
    let written = store
        .replace_free_hand(&old, typed.item, !typed.keeps_quantity)
        .unwrap();
    assert_eq!(written, Some(free("sea salt", Some("a pinch"))));
    assert_eq!(list_file(&dir), "sea salt{a pinch}\nsea salt{1%tbsp}\n");
}

#[test]
fn a_kept_quantity_is_not_folded_into_a_same_name_line() {
    let dir = temp();
    let store = store(&dir);
    write(
        &base(&dir).join(".shopping-list"),
        "parsley{2%bunches}\nflat parsley{1%bunches}\n",
    );
    let old = free("parsley", Some("2%bunches"));
    let typed = crate::shopping_list::edited_item(&old, "flat parsley", None).unwrap();
    store
        .replace_free_hand(&old, typed.item, !typed.keeps_quantity)
        .unwrap();
    assert_eq!(
        list_file(&dir),
        "flat parsley{2%bunches}\nflat parsley{1%bunches}\n"
    );
}

#[test]
fn a_fold_target_after_the_edited_line_takes_the_sum() {
    let dir = temp();
    let store = store(&dir);
    store
        .add_typed(vec![
            free("flour", Some("2%kg")),
            free("Sugar", Some("1%kg")),
        ])
        .unwrap();
    let written = store
        .replace_free_hand(
            &free("flour", Some("2%kg")),
            free("sugar", Some("2%kg")),
            true,
        )
        .unwrap();
    assert_eq!(written, Some(free("Sugar", Some("3%kg"))));
    assert_eq!(list_file(&dir), "Sugar{3%kg}\n");
}

#[test]
fn a_fold_skips_a_first_line_with_another_unit() {
    let dir = temp();
    let store = store(&dir);
    store
        .add_typed(vec![
            free("milk", Some("2%l")),
            free("bread", None),
            free("milk", Some("1%ml")),
        ])
        .unwrap();
    let written = store
        .replace_free_hand(&free("bread", None), free("Milk", Some("500%ml")), true)
        .unwrap();
    assert_eq!(written, Some(free("milk", Some("501%ml"))));
    assert_eq!(list_file(&dir), "milk{2%l}\nmilk{501%ml}\n");
}
