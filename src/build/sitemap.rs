use anyhow::Result;
use camino::Utf8Path;
use chrono::NaiveDate;
use cooklang_find::RecipeTree;

/// One entry in the sitemap.
///
/// `relpath` is the page path relative to the output root, e.g.
/// "recipe/Breakfast/Pancakes.html". The empty string represents the homepage
/// and renders as `<base>/`.
struct SitemapUrl {
    relpath: String,
    lastmod: Option<NaiveDate>,
}

/// XML-escape element text: `&`, `<`, `>`.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Percent-encode each `/`-separated path segment, preserving the separators.
fn encode_path(relpath: &str) -> String {
    relpath
        .split('/')
        .map(|seg| urlencoding::encode(seg).into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Build the `<loc>` text: trimmed base + "/" + encoded path, XML-escaped.
/// The empty relpath yields `<base>/`.
fn build_loc(base: &str, relpath: &str) -> String {
    let base = base.trim_end_matches('/');
    let loc = format!("{base}/{}", encode_path(relpath));
    xml_escape(&loc)
}

/// Format a date as W3C `YYYY-MM-DD`.
fn format_lastmod(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// Render the full sitemap XML document.
fn render_sitemap_xml(base: &str, entries: &[SitemapUrl]) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    for entry in entries {
        out.push_str("  <url>\n");
        out.push_str(&format!(
            "    <loc>{}</loc>\n",
            build_loc(base, &entry.relpath)
        ));
        if let Some(date) = entry.lastmod {
            out.push_str(&format!(
                "    <lastmod>{}</lastmod>\n",
                format_lastmod(date)
            ));
        }
        out.push_str("  </url>\n");
    }
    out.push_str("</urlset>\n");
    out
}

/// Read a file's modification time as a UTC `NaiveDate`, best-effort.
///
/// UTC (rather than the local timezone) keeps `<lastmod>` reproducible across
/// machines: the same source file yields the same date regardless of where the
/// site is built.
fn file_lastmod(path: &Utf8Path) -> Option<NaiveDate> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    let datetime: chrono::DateTime<chrono::Utc> = modified.into();
    Some(datetime.date_naive())
}

/// Walk the recipe tree into a flat list of sitemap entries: the homepage, one
/// per directory listing page, and one per recipe/menu page.
///
/// Only pages that were actually written under `output` are listed: the render
/// pass skips recipes whose templates fail to build (warn + continue), so a
/// purely tree-derived list could point at pages that don't exist. We run after
/// rendering, so an on-disk existence check keeps the sitemap free of 404s.
/// Entries are sorted by path so repeated builds produce a stable, diff-friendly
/// file (the underlying tree iteration order is non-deterministic).
fn build_sitemap_entries(tree: &RecipeTree, output: &Utf8Path) -> Vec<SitemapUrl> {
    let mut out = vec![SitemapUrl {
        relpath: String::new(),
        lastmod: None,
    }];
    collect(tree, String::new(), output, &mut out);
    out.sort_by(|a, b| a.relpath.cmp(&b.relpath));
    out
}

/// The on-disk file backing a page entry, relative to `output`
/// (the homepage's empty relpath maps to `index.html`).
fn page_file(relpath: &str) -> &str {
    if relpath.is_empty() {
        "index.html"
    } else {
        relpath
    }
}

fn collect(tree: &RecipeTree, prefix: String, output: &Utf8Path, out: &mut Vec<SitemapUrl>) {
    for (name, child) in &tree.children {
        if child.children.is_empty() {
            let Some(recipe) = child.recipe.as_ref() else {
                continue;
            };
            // URL path uses the on-disk file stem, not the tree key (which may
            // be the title from metadata) — consistent with index.rs.
            let stem = recipe
                .file_name()
                .as_deref()
                .map(|f| {
                    f.trim_end_matches(".cook")
                        .trim_end_matches(".menu")
                        .to_string()
                })
                .unwrap_or_else(|| name.clone());
            let sub = if prefix.is_empty() {
                stem
            } else {
                format!("{prefix}/{stem}")
            };
            let relpath = if recipe.is_menu() {
                format!("menu/{sub}.html")
            } else {
                format!("recipe/{sub}.html")
            };
            // Skip recipes the render pass failed to write (avoids dead links).
            if !output.join(page_file(&relpath)).exists() {
                continue;
            }
            out.push(SitemapUrl {
                relpath,
                lastmod: file_lastmod(&child.path),
            });
        } else {
            let sub = if prefix.is_empty() {
                name.to_string()
            } else {
                format!("{prefix}/{name}")
            };
            let relpath = format!("directory/{sub}.html");
            // List the directory page only if it was written, but always recurse:
            // a missing directory page doesn't mean its recipes are missing.
            if output.join(page_file(&relpath)).exists() {
                out.push(SitemapUrl {
                    relpath,
                    lastmod: None,
                });
            }
            collect(child, sub, output, out);
        }
    }
}

/// Build and write `sitemap.xml` to the output root.
pub fn write_sitemap(output: &Utf8Path, base: &str, tree: &RecipeTree) -> Result<()> {
    let entries = build_sitemap_entries(tree, output);
    let xml = render_sitemap_xml(base, &entries);
    crate::build::writer::write_bytes(output, Utf8Path::new("sitemap.xml"), xml.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn build_entries_filters_missing_pages_and_sorts() {
        use std::fs;
        use tempfile::TempDir;

        let src = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();
        let src_root = Utf8Path::from_path(src.path()).unwrap();
        let out_root = Utf8Path::from_path(out.path()).unwrap();

        // Source recipes: one nested under a directory, one at the root.
        fs::create_dir_all(src_root.join("Breakfast")).unwrap();
        fs::write(src_root.join("Breakfast/Pancakes.cook"), "Mix @flour{1}.\n").unwrap();
        fs::write(src_root.join("Soup.cook"), "Boil @water{1}.\n").unwrap();

        let tree = cooklang_find::build_tree(src_root).unwrap();

        // Simulate the render pass writing the homepage, the directory page, and
        // Pancakes — but NOT Soup (e.g. it failed to render).
        fs::write(out_root.join("index.html"), "x").unwrap();
        fs::create_dir_all(out_root.join("recipe/Breakfast")).unwrap();
        fs::write(out_root.join("recipe/Breakfast/Pancakes.html"), "x").unwrap();
        fs::create_dir_all(out_root.join("directory")).unwrap();
        fs::write(out_root.join("directory/Breakfast.html"), "x").unwrap();

        let entries = build_sitemap_entries(&tree, out_root);
        let paths: Vec<&str> = entries.iter().map(|e| e.relpath.as_str()).collect();

        // Homepage first, then sorted; Soup is excluded because no file was written.
        assert_eq!(
            paths,
            vec![
                "",
                "directory/Breakfast.html",
                "recipe/Breakfast/Pancakes.html"
            ]
        );
    }

    #[test]
    fn escapes_xml_specials() {
        assert_eq!(xml_escape("a & b < c > d"), "a &amp; b &lt; c &gt; d");
    }

    #[test]
    fn encodes_spaces_per_segment_preserving_slashes() {
        assert_eq!(
            encode_path("recipe/Root Vegetables/Mash Up.html"),
            "recipe/Root%20Vegetables/Mash%20Up.html"
        );
    }

    #[test]
    fn loc_trims_trailing_slash_on_base() {
        assert_eq!(
            build_loc("https://x.test/recipes/", "recipe/A.html"),
            "https://x.test/recipes/recipe/A.html"
        );
        assert_eq!(
            build_loc("https://x.test/recipes", "recipe/A.html"),
            "https://x.test/recipes/recipe/A.html"
        );
    }

    #[test]
    fn loc_homepage_is_base_slash() {
        assert_eq!(build_loc("https://x.test", ""), "https://x.test/");
        assert_eq!(build_loc("https://x.test/", ""), "https://x.test/");
    }

    #[test]
    fn loc_percent_encodes_ampersand_in_name() {
        let loc = build_loc("https://x.test", "recipe/Mac & Cheese.html");
        assert!(loc.contains("Mac%20%26%20Cheese"), "got: {loc}");
        assert!(!loc.contains(" & "), "raw ampersand leaked: {loc}");
    }

    #[test]
    fn formats_lastmod_w3c() {
        assert_eq!(format_lastmod(date(2026, 6, 6)), "2026-06-06");
        assert_eq!(format_lastmod(date(2026, 1, 2)), "2026-01-02");
    }

    #[test]
    fn renders_well_formed_document() {
        let entries = vec![
            SitemapUrl {
                relpath: String::new(),
                lastmod: None,
            },
            SitemapUrl {
                relpath: "recipe/Pancakes.html".to_string(),
                lastmod: Some(date(2026, 6, 6)),
            },
        ];
        let xml = render_sitemap_xml("https://x.test", &entries);
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">"));
        assert!(xml.contains("<loc>https://x.test/</loc>"));
        assert!(xml.contains("<loc>https://x.test/recipe/Pancakes.html</loc>"));
        assert!(xml.contains("<lastmod>2026-06-06</lastmod>"));
        assert!(xml.trim_end().ends_with("</urlset>"));
        assert_eq!(xml.matches("<lastmod>").count(), 1);
        assert_eq!(xml.matches("<url>").count(), 2);
    }
}
