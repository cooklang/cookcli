//! Recipes across folders: grouped by the course their metadata declares, or
//! picked out by a tag. The folder listing shows one directory at a time;
//! these walk the whole collection, as the root listing already does to
//! count its folders.

use crate::web::builders::{listed_recipe_item, natural_cmp, prune_outside};
use crate::web::templates::RecipeItem;
use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use cooklang_find::{RecipeEntry, RecipeTree};
use fluent_templates::Loader;
use std::collections::BTreeMap;
use unic_langid::LanguageIdentifier;

/// Which recipes a list holds.
#[derive(Clone, Copy, Debug)]
pub enum RecipeFilter<'a> {
    /// Recipes declaring this course, compared without case.
    Course(&'a str),
    /// Recipes declaring no course at all.
    NoCourse,
    /// Recipes carrying this tag, compared without case.
    Tag(&'a str),
}

impl RecipeFilter<'_> {
    fn keeps(&self, entry: &RecipeEntry) -> bool {
        match self {
            Self::Course(course) => courses(entry).iter().any(|c| same(c, course)),
            Self::NoCourse => courses(entry).is_empty(),
            Self::Tag(tag) => entry.tags().iter().any(|t| same(t, tag)),
        }
    }
}

fn same(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// The courses a recipe declares in `course` (or its alias `category`): one,
/// a YAML list, or several separated by commas, as `tags` may be written.
pub fn courses(entry: &RecipeEntry) -> Vec<String> {
    let metadata = entry.metadata();
    let Some(value) = metadata
        .get_path("course")
        .or_else(|| metadata.get_path("category"))
    else {
        return Vec::new();
    };
    let values: Vec<&str> = match value {
        serde_yaml::Value::String(s) => s.split(',').collect(),
        serde_yaml::Value::Sequence(items) => items.iter().filter_map(|v| v.as_str()).collect(),
        _ => Vec::new(),
    };
    values
        .into_iter()
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .map(str::to_string)
        .collect()
}

/// The recipe tree of the whole collection. As in the folder listing,
/// nothing reached through a link out of the collection is included.
fn collection_tree(base_path: &Utf8Path) -> Result<RecipeTree> {
    let mut tree = cooklang_find::build_tree(base_path)
        .map_err(|e| anyhow::anyhow!("Failed to build recipe tree: {e}"))?;
    prune_outside(&mut tree, base_path);
    Ok(tree)
}

/// Every recipe and menu in `tree`, with its path from `base_path`.
fn entries<'t>(tree: &'t RecipeTree, base_path: &Utf8Path) -> Vec<(&'t RecipeEntry, Utf8PathBuf)> {
    let mut entries = Vec::new();
    let mut stack = vec![tree];
    while let Some(node) = stack.pop() {
        if let Some(entry) = &node.recipe {
            let relative = entry
                .path()
                .and_then(|path| path.strip_prefix(base_path).ok())
                .map(Utf8Path::to_path_buf);
            if let Some(relative) = relative {
                entries.push((entry, relative));
            }
        }
        stack.extend(node.children.values());
    }
    // The tree keeps its children in a `HashMap`: sorted, so that "the first
    // recipe found" names a course the same way on every load.
    entries.sort_by(|a, b| a.1.cmp(&b.1));
    entries
}

/// A course as a heading: collections tend to write them in lowercase
/// (`course: dessert`), so the first letter is raised.
pub fn course_title(course: &str) -> String {
    let mut chars = course.trim().chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The course index: a card per course, A to Z, then one for the recipes
/// declaring none. A recipe in two courses counts in both. A course is named
/// and linked as the first recipe by path writes it.
///
/// `tree` is the collection's, rooted at `base_path`: the front page has
/// walked it already, and walking a large collection again doubles its time.
pub fn course_cards(
    tree: &RecipeTree,
    base_path: &Utf8Path,
    url_prefix: &str,
    lang: &LanguageIdentifier,
) -> Vec<RecipeItem> {
    let mut by_course: BTreeMap<String, (String, usize)> = BTreeMap::new();
    let mut without = 0;
    for (entry, _) in entries(tree, base_path) {
        let courses = courses(entry);
        if courses.is_empty() {
            without += 1;
        }
        for course in courses {
            by_course
                .entry(course.to_lowercase())
                .or_insert_with(|| (course, 0))
                .1 += 1;
        }
    }

    let mut cards: Vec<RecipeItem> = by_course
        .into_values()
        .map(|(name, count)| {
            let href = format!("{url_prefix}/course/{}", urlencoding::encode(&name));
            course_card(course_title(&name), href, count)
        })
        .collect();
    cards.sort_by(|a, b| natural_cmp(&a.name, &b.name));
    if without > 0 {
        let name = crate::web::i18n::LOCALES.lookup(lang, "course-none");
        cards.push(course_card(
            name,
            format!("{url_prefix}/no-course"),
            without,
        ));
    }
    cards
}

fn course_card(name: String, href: String, count: usize) -> RecipeItem {
    RecipeItem {
        path: String::new(),
        is_directory: true,
        count: Some(count),
        description: None,
        tags: Vec::new(),
        image_path: None,
        is_menu: false,
        modified_at: None,
        created_at: None,
        folder: None,
        href: Some(href),
        name,
    }
}

/// The recipes and menus `filter` keeps, as cards labelled with their
/// folder, A to Z.
pub fn filtered_items(
    base_path: &Utf8Path,
    url_prefix: &str,
    filter: RecipeFilter<'_>,
) -> Result<Vec<RecipeItem>> {
    let tree = collection_tree(base_path)?;
    let mut items: Vec<RecipeItem> = entries(&tree, base_path)
        .into_iter()
        .filter(|(entry, _)| filter.keeps(entry))
        .map(|(entry, relative)| {
            let name = entry.name().clone().unwrap_or_default();
            listed_recipe_item(base_path, url_prefix, Some(entry), &relative, name)
        })
        .collect();
    items.sort_by(|a, b| natural_cmp(&a.name, &b.name).then_with(|| a.path.cmp(&b.path)));
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web::language::EN_US;

    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        let write = |path: &str, text: &str| {
            let path = dir.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        write(
            "Mains/Curry.cook",
            "---\ncourse: dinner\ntags: [quick, Spicy]\n---\nCook @rice{}.\n",
        );
        write(
            "Books/Cakes/Brownies.cook",
            "---\ntitle: Fudgy Brownies\ncourse: Dessert\n---\nBake @cocoa{}.\n",
        );
        write(
            "Sides/Slaw.cook",
            "---\ncourse: [side, lunch]\ntags: quick\n---\nShred @cabbage{}.\n",
        );
        write(
            "Sides/Sorbet.cook",
            "---\ncategory: dessert\n---\nFreeze @lemon{}.\n",
        );
        write("Toast.cook", "Toast @bread{}.\n");
        dir
    }

    fn base(dir: &tempfile::TempDir) -> Utf8PathBuf {
        Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap()
    }

    fn cards(dir: &tempfile::TempDir, url_prefix: &str) -> Vec<RecipeItem> {
        let base = base(dir);
        let tree = collection_tree(&base).unwrap();
        course_cards(&tree, &base, url_prefix, &EN_US)
    }

    fn names(items: &[RecipeItem]) -> Vec<&str> {
        items.iter().map(|i| i.name.as_str()).collect()
    }

    #[test]
    fn courses_are_grouped_without_case_and_the_rest_come_last() {
        let dir = fixture();
        let cards = cards(&dir, "/p");
        let summary: Vec<(&str, Option<usize>, Option<&str>)> = cards
            .iter()
            .map(|c| (c.name.as_str(), c.count, c.href.as_deref()))
            .collect();
        assert_eq!(
            summary,
            vec![
                ("Dessert", Some(2), Some("/p/course/Dessert")),
                ("Dinner", Some(1), Some("/p/course/dinner")),
                ("Lunch", Some(1), Some("/p/course/lunch")),
                ("Side", Some(1), Some("/p/course/side")),
                ("No course", Some(1), Some("/p/no-course")),
            ]
        );
        assert!(cards.iter().all(|c| c.is_directory));
    }

    #[test]
    fn a_course_lists_its_recipes_by_title_with_their_folder() {
        let dir = fixture();
        let items = filtered_items(&base(&dir), "", RecipeFilter::Course("DESSERT")).unwrap();
        assert_eq!(names(&items), vec!["Fudgy Brownies", "Sorbet"]);
        assert_eq!(items[0].folder.as_deref(), Some("Books/Cakes"));
        assert_eq!(items[0].path, "Books/Cakes/Brownies.cook");
    }

    #[test]
    fn no_course_lists_only_recipes_declaring_none() {
        let dir = fixture();
        let items = filtered_items(&base(&dir), "", RecipeFilter::NoCourse).unwrap();
        assert_eq!(names(&items), vec!["Toast"]);
        assert_eq!(items[0].folder, None);
    }

    #[test]
    fn a_tag_matches_without_case_in_lists_and_strings() {
        let dir = fixture();
        let quick = filtered_items(&base(&dir), "", RecipeFilter::Tag("Quick")).unwrap();
        assert_eq!(names(&quick), vec!["Curry", "Slaw"]);
        let spicy = filtered_items(&base(&dir), "", RecipeFilter::Tag("spicy")).unwrap();
        assert_eq!(names(&spicy), vec!["Curry"]);
        let none = filtered_items(&base(&dir), "", RecipeFilter::Tag("vegan")).unwrap();
        assert!(none.is_empty());
    }

    #[test]
    fn a_course_title_raises_only_the_first_letter() {
        assert_eq!(course_title("dessert"), "Dessert");
        assert_eq!(course_title(" main course "), "Main course");
        assert_eq!(course_title("été"), "Été");
        assert_eq!(course_title(""), "");
    }

    #[test]
    fn a_collection_without_courses_has_only_the_no_course_card() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("Toast.cook"), "Toast @bread{}.\n").unwrap();
        let cards = cards(&dir, "");
        assert_eq!(names(&cards), vec!["No course"]);
    }
}
