//! Tag pages: every recipe and menu carrying a tag, whatever its folder.
//!
//! A folder gives a recipe one place; its `tags` give it as many as it has.
//! `/tags` lists the tags of the collection and `/tag/<slug>` the recipes
//! under one, and `cook build web` writes the same pages as `tags.html` and
//! `tag/<slug>.html`.

use crate::web::builders::{natural_cmp, prune_outside, prune_subtree, recipe_item};
use crate::web::language::FeatureFlags;
use crate::web::templates::{RecipeItem, TagSummary, TagTemplate, TagsTemplate, Tr};
use crate::web::viewer::Viewer;
use anyhow::Result;
use camino::Utf8Path;
use std::collections::{BTreeMap, HashSet};
use unic_langid::LanguageIdentifier;

/// The name of a tag in URLs and in the static site's file names.
///
/// Lowercase and trimmed, so `Dinner` and `dinner ` are one tag. Characters a
/// single path segment or a Windows file name cannot hold become `-`, and so
/// does a leading `.`, which would make `.` or `..` a tag page. Spellings that
/// share a slug are grouped as one tag on the server and the static site
/// alike, so the two always agree on which page a recipe is listed on.
pub fn tag_slug(tag: &str) -> String {
    let slug: String = tag
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*' | '#' | '%' => '-',
            c if c.is_control() => '-',
            c => c,
        })
        .collect();
    let slug = match slug.strip_prefix('.') {
        Some(rest) => format!("-{rest}"),
        None => slug,
    };
    // Windows drops trailing dots and spaces from a file name.
    slug.trim_end_matches(['.', ' ']).to_string()
}

/// One tag and the recipes and menus carrying it.
pub struct TagGroup {
    pub slug: String,
    /// The spelling most recipes use; the first in sort order on a tie.
    pub name: String,
    /// Sorted by name, then folder.
    pub items: Vec<RecipeItem>,
}

/// Every tag under `base_path`, sorted by name.
///
/// `static_mode` and `exclude` prune the tree the way the folder listings do:
/// the server leaves out whatever resolves outside the collection, and a
/// static build leaves out its own output directory.
pub fn collect_tags(
    base_path: &Utf8Path,
    url_prefix: &str,
    static_mode: bool,
    exclude: Option<&Utf8Path>,
) -> Result<Vec<TagGroup>> {
    let mut tree = cooklang_find::build_tree(base_path)
        .map_err(|e| anyhow::anyhow!("Failed to build recipe tree: {e}"))?;
    if let Some(dir) = exclude {
        prune_subtree(&mut tree, dir);
    }
    if !static_mode {
        prune_outside(&mut tree, base_path);
    }
    Ok(group_by_tag(&tree, base_path, url_prefix))
}

/// [`collect_tags`] on a tree that is already pruned.
pub fn group_by_tag(
    tree: &cooklang_find::RecipeTree,
    base_path: &Utf8Path,
    url_prefix: &str,
) -> Vec<TagGroup> {
    let mut found: BTreeMap<String, Found> = BTreeMap::new();
    collect(tree, base_path, url_prefix, "", &mut found);

    let mut groups: Vec<TagGroup> = found
        .into_iter()
        .map(|(slug, found)| {
            let name = found
                .spellings
                .into_iter()
                // Most used first; the BTreeMap order breaks ties.
                .fold(None::<(String, usize)>, |best, (spelling, n)| match best {
                    Some((_, best_n)) if best_n >= n => best,
                    _ => Some((spelling, n)),
                })
                .map(|(spelling, _)| spelling)
                .unwrap_or_else(|| slug.clone());
            let mut items = found.items;
            items.sort_by(|a, b| {
                natural_cmp(&a.name, &b.name).then_with(|| a.folder.cmp(&b.folder))
            });
            TagGroup { slug, name, items }
        })
        .collect();
    groups.sort_by(|a, b| natural_cmp(&a.name, &b.name));
    groups
}

#[derive(Default)]
struct Found {
    spellings: BTreeMap<String, usize>,
    items: Vec<RecipeItem>,
}

fn collect(
    tree: &cooklang_find::RecipeTree,
    base_path: &Utf8Path,
    url_prefix: &str,
    folder: &str,
    found: &mut BTreeMap<String, Found>,
) {
    for (name, child) in &tree.children {
        if !child.children.is_empty() {
            let sub = if folder.is_empty() {
                name.to_string()
            } else {
                format!("{folder}/{name}")
            };
            collect(child, base_path, url_prefix, &sub, found);
            continue;
        }
        let Some(recipe) = child.recipe.as_ref() else {
            continue;
        };
        // The URL uses the file stem, not the tree key (which may be a title).
        let stem = recipe
            .file_name()
            .map(|f| {
                f.trim_end_matches(".cook")
                    .trim_end_matches(".menu")
                    .to_string()
            })
            .unwrap_or_else(|| name.to_string());
        let path = if folder.is_empty() {
            stem
        } else {
            format!("{folder}/{stem}")
        };
        let mut item = recipe_item(base_path, url_prefix, name, path, recipe);
        item.folder = (!folder.is_empty()).then(|| folder.to_string());

        // `tags: [Dinner, dinner]` lists the recipe once.
        let mut seen = HashSet::new();
        for tag in &item.tags {
            let slug = tag_slug(tag);
            if slug.is_empty() || !seen.insert(slug.clone()) {
                continue;
            }
            let entry = found.entry(slug).or_default();
            *entry.spellings.entry(tag.trim().to_string()).or_default() += 1;
            entry.items.push(item.clone());
        }
    }
}

/// What the tag pages share with every other page.
pub struct TagsBuildInput<'a> {
    pub url_prefix: &'a str,
    pub lang: LanguageIdentifier,
    pub static_mode: bool,
    pub repo_url: Option<String>,
    pub features: FeatureFlags,
    pub viewer: Viewer,
}

/// The `/tags` page.
pub fn build_tags_template(input: TagsBuildInput<'_>, groups: &[TagGroup]) -> TagsTemplate {
    TagsTemplate {
        active: "recipes".to_string(),
        tags: groups
            .iter()
            .map(|group| TagSummary {
                name: group.name.clone(),
                slug: group.slug.clone(),
                count: group.items.len(),
            })
            .collect(),
        tr: Tr::new(input.lang),
        prefix: input.url_prefix.to_string(),
        static_mode: input.static_mode,
        repo_url: input.repo_url,
        features: input.features,
        viewer: input.viewer,
    }
}

/// The `/tag/<slug>` page of `group`, or the empty page of `slug` when no
/// recipe carries it.
pub fn build_tag_template(
    input: TagsBuildInput<'_>,
    slug: &str,
    group: Option<TagGroup>,
) -> TagTemplate {
    let (name, items) = match group {
        Some(group) => (group.name, group.items),
        None => (slug.to_string(), Vec::new()),
    };
    TagTemplate {
        active: "recipes".to_string(),
        name,
        count: items.len(),
        items,
        tr: Tr::new(input.lang),
        prefix: input.url_prefix.to_string(),
        static_mode: input.static_mode,
        repo_url: input.repo_url,
        features: input.features,
        viewer: input.viewer,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slug_is_the_lowercase_trimmed_tag() {
        assert_eq!(tag_slug("Dinner"), "dinner");
        assert_eq!(tag_slug("  Comfort Food "), "comfort food");
        assert_eq!(tag_slug("Crème"), "crème");
    }

    #[test]
    fn a_slug_is_one_safe_path_segment() {
        assert_eq!(tag_slug("meal/dinner"), "meal-dinner");
        assert_eq!(tag_slug(r"a\b:c?d*e"), "a-b-c-d-e");
        assert_eq!(tag_slug("50%"), "50-");
        assert_eq!(tag_slug(".."), "-");
        assert_eq!(tag_slug(".hidden"), "-hidden");
        assert_eq!(tag_slug("etc."), "etc");
    }

    fn tree_of(files: &[(&str, &str)]) -> (tempfile::TempDir, Vec<TagGroup>) {
        let dir = tempfile::TempDir::new().unwrap();
        let base = Utf8Path::from_path(dir.path()).unwrap().to_path_buf();
        for (file, tags) in files {
            let path = base.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, format!("---\ntags: {tags}\n---\nMix @flour{{1}}.\n")).unwrap();
        }
        let groups = collect_tags(&base, "", false, None).unwrap();
        (dir, groups)
    }

    fn names(groups: &[TagGroup]) -> Vec<(&str, Vec<&str>)> {
        groups
            .iter()
            .map(|group| {
                (
                    group.name.as_str(),
                    group.items.iter().map(|item| item.path.as_str()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn recipes_are_grouped_across_folders_by_tag() {
        let (_dir, groups) = tree_of(&[
            ("Soup.cook", "[dinner, quick]"),
            ("Mains/Stew.cook", "[dinner]"),
            ("Mains/Pasta/Carbonara.cook", "[pasta, dinner]"),
        ]);
        assert_eq!(
            names(&groups),
            [
                (
                    "dinner",
                    vec!["Mains/Pasta/Carbonara", "Soup", "Mains/Stew"]
                ),
                ("pasta", vec!["Mains/Pasta/Carbonara"]),
                ("quick", vec!["Soup"]),
            ]
        );
        let folders: Vec<_> = groups[0]
            .items
            .iter()
            .map(|item| item.folder.as_deref())
            .collect();
        assert_eq!(folders, [Some("Mains/Pasta"), None, Some("Mains")]);
    }

    #[test]
    fn spellings_of_one_tag_are_one_group_named_by_the_most_used() {
        let (_dir, groups) = tree_of(&[
            ("A.cook", "[Dinner]"),
            ("B.cook", "[dinner]"),
            ("C.cook", "[dinner, DINNER]"),
        ]);
        assert_eq!(names(&groups), [("dinner", vec!["A", "B", "C"])]);
        assert_eq!(groups[0].slug, "dinner");
    }

    #[test]
    fn menus_are_grouped_and_untagged_recipes_are_not() {
        let (dir, _) = tree_of(&[("Week.menu", "[plan]")]);
        let base = Utf8Path::from_path(dir.path()).unwrap();
        std::fs::write(base.join("Plain.cook"), "Mix @flour{1}.\n").unwrap();
        let groups = collect_tags(base, "", false, None).unwrap();
        assert_eq!(names(&groups), [("plan", vec!["Week"])]);
        assert!(groups[0].items[0].is_menu);
    }
}
