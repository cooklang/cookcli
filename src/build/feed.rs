//! Web feeds (`atom.xml` and `rss.xml`), one item per recipe or menu page.
//!
//! Shared by `cook build web --feed` (written next to the static site) and
//! `cook server` (rendered per request), which differ only in page URLs.

use crate::build::sitemap::{build_loc, xml_escape};
use anyhow::Result;
use camino::Utf8Path;
use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use cooklang_find::RecipeTree;

/// One recipe or menu in the feed.
///
/// `relpath` is the page path relative to the output root, e.g.
/// "recipe/Breakfast/Pancakes.html".
struct FeedItem {
    relpath: String,
    title: String,
    updated: DateTime<Utc>,
    summary: Option<String>,
    author: Option<String>,
    tags: Vec<String>,
}

/// Channel-level fields shared by both feed formats.
struct FeedInfo<'a> {
    /// Full base URL of the deployed site.
    base: &'a str,
    title: &'a str,
    /// Human-readable publisher name (Atom requires a feed-level author when
    /// entries may lack one).
    author: &'a str,
    /// BCP-47 language tag.
    lang: &'a str,
}

/// Date of the item: the `date` metadata field when it parses, else the
/// source file's modification time (UTC, like the sitemap's `<lastmod>`).
///
/// An explicit `date` matters for feeds built in CI, where a fresh checkout
/// gives every file the same mtime and readers would see no ordering.
fn item_date(metadata: &cooklang_find::Metadata, path: &Utf8Path) -> Option<DateTime<Utc>> {
    metadata
        .get("date")
        .and_then(|v| v.as_str())
        .and_then(parse_date)
        .or_else(|| file_mtime(path))
}

/// Parse an RFC 3339 timestamp or a bare `YYYY-MM-DD` date (midnight UTC).
fn parse_date(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc())
}

fn file_mtime(path: &Utf8Path) -> Option<DateTime<Utc>> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(modified.into())
}

/// Read a string metadata field; for `author`, also accept `{ name: ... }`
/// and fall back to `source.author`, matching the recipe page.
fn string_field(metadata: &cooklang_find::Metadata, key: &str) -> Option<String> {
    let value = metadata.get(key)?;
    value
        .as_str()
        .or_else(|| value.get("name").and_then(|n| n.as_str()))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn item_author(metadata: &cooklang_find::Metadata) -> Option<String> {
    string_field(metadata, "author").or_else(|| {
        metadata
            .get("source")
            .and_then(|s| s.get("author"))
            .and_then(|a| a.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    })
}

/// Where the pages an item links to live.
#[derive(Clone, Copy)]
pub(crate) enum PageUrls<'a> {
    /// Static site: `recipe/<path>.html` and `menu/<path>.html`. Only pages
    /// actually written under `output` are listed, like the sitemap, so the
    /// feed never links to a recipe that failed to render.
    Static { output: &'a Utf8Path },
    /// `cook server`: `recipe/<path>` for recipes and menus alike.
    Server,
}

impl PageUrls<'_> {
    /// Page path relative to the site root, or `None` to leave the item out.
    fn relpath(self, sub: &str, is_menu: bool) -> Option<String> {
        match self {
            PageUrls::Static { output } => {
                let kind = if is_menu { "menu" } else { "recipe" };
                let relpath = format!("{kind}/{sub}.html");
                output.join(&relpath).exists().then_some(relpath)
            }
            PageUrls::Server => Some(format!("recipe/{sub}")),
        }
    }
}

/// Walk the recipe tree into feed items, newest first. Ties on date are broken
/// by path so repeated builds produce a stable file.
fn build_feed_items(tree: &RecipeTree, urls: PageUrls) -> Vec<FeedItem> {
    let mut out = Vec::new();
    collect(tree, String::new(), urls, &mut out);
    out.sort_by(|a, b| {
        b.updated
            .cmp(&a.updated)
            .then_with(|| a.relpath.cmp(&b.relpath))
    });
    out
}

fn collect(tree: &RecipeTree, prefix: String, urls: PageUrls, out: &mut Vec<FeedItem>) {
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
            let Some(relpath) = urls.relpath(&sub, recipe.is_menu()) else {
                continue;
            };
            let metadata = recipe.metadata();
            out.push(FeedItem {
                relpath,
                title: recipe.name().clone().unwrap_or_else(|| name.clone()),
                // Fall back to the epoch rather than dropping the item: a
                // recipe whose mtime can't be read still belongs in the feed.
                updated: item_date(metadata, &child.path).unwrap_or_default(),
                summary: string_field(metadata, "description"),
                author: item_author(metadata),
                tags: recipe.tags(),
            });
        } else {
            let sub = if prefix.is_empty() {
                name.to_string()
            } else {
                format!("{prefix}/{name}")
            };
            collect(child, sub, urls, out);
        }
    }
}

/// The feed's own last-updated time: its newest item, so an unchanged recipe
/// set yields a byte-identical feed. Empty feeds use the epoch.
fn feed_updated(items: &[FeedItem]) -> DateTime<Utc> {
    items.iter().map(|i| i.updated).max().unwrap_or_default()
}

fn rfc3339(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Render an Atom 1.0 (RFC 4287) document.
fn render_atom(info: &FeedInfo, items: &[FeedItem]) -> String {
    let home = build_loc(info.base, "");
    let self_url = build_loc(info.base, "atom.xml");
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<feed xmlns=\"http://www.w3.org/2005/Atom\" xml:lang=\"{}\">\n",
        xml_escape(info.lang)
    ));
    out.push_str(&format!("  <title>{}</title>\n", xml_escape(info.title)));
    out.push_str(&format!("  <id>{home}</id>\n"));
    out.push_str(&format!(
        "  <link rel=\"alternate\" type=\"text/html\" href=\"{home}\"/>\n"
    ));
    out.push_str(&format!(
        "  <link rel=\"self\" type=\"application/atom+xml\" href=\"{self_url}\"/>\n"
    ));
    out.push_str(&format!(
        "  <updated>{}</updated>\n",
        rfc3339(feed_updated(items))
    ));
    out.push_str(&format!(
        "  <author><name>{}</name></author>\n",
        xml_escape(info.author)
    ));
    out.push_str("  <generator uri=\"https://cooklang.org\">CookCLI</generator>\n");
    for item in items {
        let link = build_loc(info.base, &item.relpath);
        out.push_str("  <entry>\n");
        out.push_str(&format!("    <title>{}</title>\n", xml_escape(&item.title)));
        out.push_str(&format!("    <id>{link}</id>\n"));
        out.push_str(&format!(
            "    <link rel=\"alternate\" type=\"text/html\" href=\"{link}\"/>\n"
        ));
        out.push_str(&format!(
            "    <updated>{}</updated>\n",
            rfc3339(item.updated)
        ));
        if let Some(author) = &item.author {
            out.push_str(&format!(
                "    <author><name>{}</name></author>\n",
                xml_escape(author)
            ));
        }
        for tag in &item.tags {
            out.push_str(&format!("    <category term=\"{}\"/>\n", xml_escape(tag)));
        }
        if let Some(summary) = &item.summary {
            out.push_str(&format!("    <summary>{}</summary>\n", xml_escape(summary)));
        }
        out.push_str("  </entry>\n");
    }
    out.push_str("</feed>\n");
    out
}

/// Render an RSS 2.0 document.
fn render_rss(info: &FeedInfo, items: &[FeedItem]) -> String {
    let home = build_loc(info.base, "");
    let self_url = build_loc(info.base, "rss.xml");
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(
        "<rss version=\"2.0\" xmlns:atom=\"http://www.w3.org/2005/Atom\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n",
    );
    out.push_str("  <channel>\n");
    out.push_str(&format!("    <title>{}</title>\n", xml_escape(info.title)));
    out.push_str(&format!("    <link>{home}</link>\n"));
    out.push_str(&format!(
        "    <description>{}</description>\n",
        xml_escape(info.title)
    ));
    out.push_str(&format!(
        "    <language>{}</language>\n",
        xml_escape(info.lang)
    ));
    out.push_str(&format!(
        "    <lastBuildDate>{}</lastBuildDate>\n",
        feed_updated(items).to_rfc2822()
    ));
    out.push_str("    <generator>CookCLI</generator>\n");
    out.push_str(&format!(
        "    <atom:link rel=\"self\" type=\"application/rss+xml\" href=\"{self_url}\"/>\n"
    ));
    for item in items {
        let link = build_loc(info.base, &item.relpath);
        out.push_str("    <item>\n");
        out.push_str(&format!(
            "      <title>{}</title>\n",
            xml_escape(&item.title)
        ));
        out.push_str(&format!("      <link>{link}</link>\n"));
        out.push_str(&format!("      <guid isPermaLink=\"true\">{link}</guid>\n"));
        out.push_str(&format!(
            "      <pubDate>{}</pubDate>\n",
            item.updated.to_rfc2822()
        ));
        // RSS <author> must be an email address; dc:creator takes a name.
        if let Some(author) = &item.author {
            out.push_str(&format!(
                "      <dc:creator>{}</dc:creator>\n",
                xml_escape(author)
            ));
        }
        for tag in &item.tags {
            out.push_str(&format!("      <category>{}</category>\n", xml_escape(tag)));
        }
        if let Some(summary) = &item.summary {
            out.push_str(&format!(
                "      <description>{}</description>\n",
                xml_escape(summary)
            ));
        }
        out.push_str("    </item>\n");
    }
    out.push_str("  </channel>\n");
    out.push_str("</rss>\n");
    out
}

/// Feed format served or written.
#[derive(Clone, Copy)]
pub(crate) enum FeedFormat {
    Atom,
    Rss,
}

impl FeedFormat {
    pub(crate) fn content_type(self) -> &'static str {
        match self {
            FeedFormat::Atom => "application/atom+xml; charset=utf-8",
            FeedFormat::Rss => "application/rss+xml; charset=utf-8",
        }
    }
}

/// The host is the most meaningful publisher name we know about.
fn publisher(base: &str) -> String {
    url::Url::parse(base)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string))
        .unwrap_or_else(|| base.to_string())
}

/// Render one feed document for the pages `urls` points at.
///
/// `base` is the absolute URL of the site root (scheme, host and any subpath).
pub(crate) fn render_feed(
    format: FeedFormat,
    tree: &RecipeTree,
    urls: PageUrls,
    base: &str,
    title: &str,
    lang: &str,
) -> String {
    let items = build_feed_items(tree, urls);
    let author = publisher(base);
    let info = FeedInfo {
        base,
        title,
        author: &author,
        lang,
    };
    match format {
        FeedFormat::Atom => render_atom(&info, &items),
        FeedFormat::Rss => render_rss(&info, &items),
    }
}

/// Build and write `atom.xml` and `rss.xml` to the output root.
///
/// `base` must already be validated as an absolute http(s) URL.
pub fn write_feeds(
    output: &Utf8Path,
    base: &str,
    title: &str,
    lang: &str,
    tree: &RecipeTree,
) -> Result<usize> {
    let items = build_feed_items(tree, PageUrls::Static { output });
    let author = publisher(base);
    let info = FeedInfo {
        base,
        title,
        author: &author,
        lang,
    };
    crate::build::writer::write_bytes(
        output,
        Utf8Path::new("atom.xml"),
        render_atom(&info, &items).as_bytes(),
    )?;
    crate::build::writer::write_bytes(
        output,
        Utf8Path::new("rss.xml"),
        render_rss(&info, &items).as_bytes(),
    )?;
    Ok(items.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
    }

    fn info() -> FeedInfo<'static> {
        FeedInfo {
            base: "https://x.test/recipes/",
            title: "All Recipes",
            author: "x.test",
            lang: "en-US",
        }
    }

    fn items() -> Vec<FeedItem> {
        vec![
            FeedItem {
                relpath: "recipe/Mac & Cheese.html".to_string(),
                title: "Mac & Cheese".to_string(),
                updated: at(2026, 6, 6),
                summary: Some("Creamy <3".to_string()),
                author: Some("Jane".to_string()),
                tags: vec!["comfort".to_string(), "\"quick\"".to_string()],
            },
            FeedItem {
                relpath: "menu/Week.html".to_string(),
                title: "Week".to_string(),
                updated: at(2026, 1, 2),
                summary: None,
                author: None,
                tags: vec![],
            },
        ]
    }

    #[test]
    fn parses_bare_and_rfc3339_dates() {
        assert_eq!(parse_date("2026-06-06"), Some(at(2026, 6, 6)));
        assert_eq!(
            parse_date(" 2026-06-06T12:30:00+02:00 "),
            Some(at(2026, 6, 6) + chrono::Duration::minutes(630))
        );
        assert_eq!(parse_date("June 6th"), None);
    }

    #[test]
    fn metadata_date_wins_over_mtime() {
        let entry = cooklang_find::RecipeEntry::from_content(
            "---\ndate: 2020-03-04\nauthor:\n  name: Jane\n---\nMix @flour{1}.\n".to_string(),
            Some("x".to_string()),
        )
        .unwrap();
        let metadata = entry.metadata();
        assert_eq!(
            item_date(metadata, Utf8Path::new("does-not-exist.cook")),
            Some(at(2020, 3, 4))
        );
        assert_eq!(item_author(metadata).as_deref(), Some("Jane"));
    }

    #[test]
    fn build_items_filters_missing_pages_and_sorts_newest_first() {
        use std::fs;
        use tempfile::TempDir;

        let src = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();
        let src_root = Utf8Path::from_path(src.path()).unwrap();
        let out_root = Utf8Path::from_path(out.path()).unwrap();

        fs::create_dir_all(src_root.join("Breakfast")).unwrap();
        fs::write(
            src_root.join("Breakfast/Pancakes.cook"),
            "---\ntitle: Fluffy Pancakes\ndate: 2026-01-01\ndescription: Sunday treat\ntags: [sweet]\n---\nMix @flour{1}.\n",
        )
        .unwrap();
        fs::write(
            src_root.join("Omelette.cook"),
            "---\ndate: 2026-05-01\n---\nBeat @eggs{2}.\n",
        )
        .unwrap();
        fs::write(src_root.join("Soup.cook"), "Boil @water{1}.\n").unwrap();

        let tree = cooklang_find::build_tree(src_root).unwrap();

        // Soup "failed to render": no page on disk.
        fs::create_dir_all(out_root.join("recipe/Breakfast")).unwrap();
        fs::write(out_root.join("recipe/Breakfast/Pancakes.html"), "x").unwrap();
        fs::write(out_root.join("recipe/Omelette.html"), "x").unwrap();

        let items = build_feed_items(&tree, PageUrls::Static { output: out_root });
        let paths: Vec<&str> = items.iter().map(|i| i.relpath.as_str()).collect();
        assert_eq!(
            paths,
            vec!["recipe/Omelette.html", "recipe/Breakfast/Pancakes.html"]
        );
        let pancakes = &items[1];
        assert_eq!(pancakes.title, "Fluffy Pancakes");
        assert_eq!(pancakes.summary.as_deref(), Some("Sunday treat"));
        assert_eq!(pancakes.tags, vec!["sweet".to_string()]);
    }

    #[test]
    fn server_urls_list_every_recipe_without_extension() {
        use std::fs;
        use tempfile::TempDir;

        let src = TempDir::new().unwrap();
        let src_root = Utf8Path::from_path(src.path()).unwrap();
        fs::create_dir_all(src_root.join("Breakfast")).unwrap();
        fs::write(
            src_root.join("Breakfast/Pancakes.cook"),
            "Mix @flour{1}.
",
        )
        .unwrap();
        fs::write(
            src_root.join("Week.menu"),
            "== Monday ==
",
        )
        .unwrap();

        let tree = cooklang_find::build_tree(src_root).unwrap();
        let mut paths: Vec<String> = build_feed_items(&tree, PageUrls::Server)
            .into_iter()
            .map(|i| i.relpath)
            .collect();
        paths.sort();
        assert_eq!(paths, vec!["recipe/Breakfast/Pancakes", "recipe/Week"]);
    }

    #[test]
    fn renders_atom_document() {
        let xml = render_atom(&info(), &items());
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<feed xmlns=\"http://www.w3.org/2005/Atom\" xml:lang=\"en-US\">"));
        assert!(xml.contains("<id>https://x.test/recipes/</id>"));
        assert!(xml.contains("href=\"https://x.test/recipes/atom.xml\""));
        // Feed updated is the newest item.
        assert!(xml.contains("  <updated>2026-06-06T00:00:00Z</updated>"));
        assert!(xml.contains("<title>Mac &amp; Cheese</title>"));
        assert!(xml.contains("<id>https://x.test/recipes/recipe/Mac%20%26%20Cheese.html</id>"));
        assert!(xml.contains("<summary>Creamy &lt;3</summary>"));
        assert!(xml.contains("<category term=\"&quot;quick&quot;\"/>"));
        assert!(xml.contains("<author><name>Jane</name></author>"));
        assert_eq!(xml.matches("<entry>").count(), 2);
        assert!(xml.trim_end().ends_with("</feed>"));
    }

    #[test]
    fn renders_rss_document() {
        let xml = render_rss(&info(), &items());
        assert!(xml.contains("<rss version=\"2.0\""));
        assert!(xml.contains("<link>https://x.test/recipes/</link>"));
        assert!(xml.contains("href=\"https://x.test/recipes/rss.xml\""));
        assert!(xml.contains("<language>en-US</language>"));
        assert!(xml.contains("<lastBuildDate>Sat, 6 Jun 2026 00:00:00 +0000</lastBuildDate>"));
        assert!(
            xml.contains("<guid isPermaLink=\"true\">https://x.test/recipes/menu/Week.html</guid>")
        );
        assert!(xml.contains("<pubDate>Fri, 2 Jan 2026 00:00:00 +0000</pubDate>"));
        assert!(xml.contains("<dc:creator>Jane</dc:creator>"));
        assert!(xml.contains("<category>comfort</category>"));
        assert_eq!(xml.matches("<item>").count(), 2);
        assert!(xml.trim_end().ends_with("</rss>"));
    }

    #[test]
    fn empty_feed_is_well_formed() {
        let xml = render_atom(&info(), &[]);
        assert!(xml.contains("<updated>1970-01-01T00:00:00Z</updated>"));
        assert_eq!(xml.matches("<entry>").count(), 0);
    }
}
