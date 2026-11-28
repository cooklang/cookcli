//! Source metadata must survive both the shared builders and HTML rendering.

use askama::Template;
use camino::Utf8Path;
use cookcli::web::builders::{build_recipe_template, RecipeBuildInput, RecipeBuildOutput};
use cookcli::web::language::{FeatureFlags, EN_US};
use cookcli::web::viewer::Viewer;
use tempfile::TempDir;

fn render(metadata: &str, extension: &str, static_mode: bool) -> String {
    let dir = TempDir::new().unwrap();
    let path = format!("Example.{extension}");
    let body = if extension == "menu" {
        "- @flour{1%kg}\n"
    } else {
        "Mix @flour{1%kg}.\n"
    };
    std::fs::write(
        dir.path().join(&path),
        format!("---\n{metadata}\n---\n\n{body}"),
    )
    .unwrap();
    let output = build_recipe_template(RecipeBuildInput {
        base_path: Utf8Path::from_path(dir.path()).unwrap(),
        url_prefix: "",
        recipe_path: &path,
        aisle_path: None,
        scale: 1.0,
        servings: None,
        lang: EN_US,
        static_mode,
        repo_url: None,
        features: FeatureFlags::default(),
        viewer: Viewer::default(),
    })
    .unwrap();
    match output {
        RecipeBuildOutput::Recipe(template) => template.render().unwrap(),
        RecipeBuildOutput::Menu(template) => template.render().unwrap(),
    }
}

fn source_pill(html: &str) -> &str {
    html.split("metadata-pill metadata-source\"")
        .nth(1)
        .expect("source pill must be rendered")
        .split("</span>")
        .next()
        .unwrap()
}

fn check_named_source(metadata: &str) {
    for extension in ["cook", "menu"] {
        for static_mode in [false, true] {
            let html = render(metadata, extension, static_mode);
            let pill = source_pill(&html);
            assert!(
                pill.contains("href=\"https://example.org/recipe?a=1&amp;b=2\""),
                "{pill}"
            );
            assert!(pill.contains(">Chef &amp; Co</a>"), "{pill}");
            assert_eq!(
                html.matches("href=\"https://example.org/recipe?a=1&amp;b=2\"")
                    .count(),
                1
            );
        }
    }
}

#[test]
fn nested_sources_link_the_name_on_recipe_and_menu_pages() {
    check_named_source("source:\n  name: Chef & Co\n  url: https://example.org/recipe?a=1&b=2");
}

#[test]
fn flat_sources_link_the_name_on_recipe_and_menu_pages() {
    check_named_source("source.name: Chef & Co\nsource.url: https://example.org/recipe?a=1&b=2");
}

#[test]
fn source_with_only_a_name_or_url_still_renders() {
    for extension in ["cook", "menu"] {
        for metadata in ["source:\n  name: My cookbook", "source.name: My cookbook"] {
            let html = render(metadata, extension, false);
            let pill = source_pill(&html);
            assert!(pill.contains("My cookbook"), "{pill}");
            assert!(!pill.contains("href="), "{pill}");
        }
        for metadata in [
            "source:\n  url: https://example.org/recipe",
            "source.url: https://example.org/recipe",
        ] {
            let html = render(metadata, extension, true);
            let pill = source_pill(&html);
            assert!(
                pill.contains("href=\"https://example.org/recipe\""),
                "{pill}"
            );
            assert!(pill.contains(">example.org</a>"), "{pill}");
        }
    }
}

#[test]
fn plain_text_and_url_sources_still_render() {
    for extension in ["cook", "menu"] {
        let text = render("source: My cookbook & notes", extension, false);
        let pill = source_pill(&text);
        assert!(pill.contains("My cookbook &amp; notes"), "{pill}");
        assert!(!pill.contains("<a "), "{pill}");

        let url = render("source: https://example.org/recipe", extension, true);
        let pill = source_pill(&url);
        assert!(
            pill.contains("href=\"https://example.org/recipe\""),
            "{pill}"
        );
        assert!(pill.contains(">example.org</a>"), "{pill}");
    }
}

#[test]
fn source_names_are_escaped_and_unsafe_urls_are_never_links() {
    for extension in ["cook", "menu"] {
        for metadata in [
            "source:\n  name: '<img src=x onerror=alert(1)>'\n  url: javascript:alert(1)",
            "source.name: '<img src=x onerror=alert(1)>'\nsource.url: javascript:alert(1)",
        ] {
            let html = render(metadata, extension, false);
            let pill = source_pill(&html);
            assert!(pill.contains("&lt;img"), "{pill}");
            assert!(!pill.contains("<img"), "{pill}");
            assert!(!pill.contains("href="), "{pill}");
        }
    }
}
