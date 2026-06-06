#[allow(unused_imports)] // used by the next task's tree-walk glue
use anyhow::Result;
#[allow(unused_imports)] // used by the next task's tree-walk glue
use camino::Utf8Path;
use chrono::NaiveDate;

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
        out.push_str(&format!("    <loc>{}</loc>\n", build_loc(base, &entry.relpath)));
        if let Some(date) = entry.lastmod {
            out.push_str(&format!("    <lastmod>{}</lastmod>\n", format_lastmod(date)));
        }
        out.push_str("  </url>\n");
    }
    out.push_str("</urlset>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
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
            SitemapUrl { relpath: String::new(), lastmod: None },
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
