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
        recipes: None,
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

#[test]
fn clear_empties_the_list_and_the_checked_log() {
    let dir = temp();
    let store = store(&dir);
    store.add(entry("Soup.cook", 1.0)).unwrap();
    store.check("Onion").unwrap();

    store.clear().expect("clears");

    assert!(store.load().unwrap().is_empty());
    assert!(store.checked_set().unwrap().is_empty());
    assert!(
        !base(&dir).join(".shopping-checked").exists(),
        "the checked log is removed, not emptied"
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
