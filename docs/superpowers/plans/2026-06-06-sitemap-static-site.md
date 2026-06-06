# Sitemap Generation for `cook build web` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Optionally emit a sitemaps.org-compliant `sitemap.xml` at the static-site output root when `cook build web --sitemap <URL>` is given.

**Architecture:** A new `src/build/sitemap.rs` module with pure, unit-tested rendering helpers (`build_loc`, `xml_escape`, `format_lastmod`, `render_sitemap_xml`) plus thin tree-walking glue (`build_sitemap_entries`, `write_sitemap`) that mirrors the existing `index.rs` walk. `mod.rs` gains a `--sitemap` flag, validates it with the `url` crate, and calls the writer after the search index is built using the already-pruned tree.

**Tech Stack:** Rust, clap (args), `url` (validation), `urlencoding` (percent-encode path segments), `chrono` (W3C date), `camino` (paths). Output via existing `writer::write_bytes`.

**Spec:** `docs/superpowers/specs/2026-06-06-sitemap-static-site-design.md`

---

### Task 1: Sitemap module — pure rendering core (TDD)

Create the module with the data type and pure helpers, fully unit-tested. The
tree-walking glue (`build_sitemap_entries`, `write_sitemap`) is added in Task 2
together with the wiring, so the functions are exercised end-to-end then. Between
this task and Task 2 the new functions are unused by non-test code, so
`cargo build` will print `dead_code` warnings — that is expected and is cleared
by Task 2. `cargo test` compiles the tests that use them, so tests pass.

**Files:**
- Create: `src/build/sitemap.rs`
- Modify: `src/build/mod.rs:1-4` (add `mod sitemap;` to the module declarations)

- [ ] **Step 1: Declare the module**

In `src/build/mod.rs`, the top currently reads:

```rust
mod index;
mod links;
mod renderer;
mod writer;
```

Change it to (keep alphabetical-ish grouping; add `sitemap`):

```rust
mod index;
mod links;
mod renderer;
mod sitemap;
mod writer;
```

- [ ] **Step 2: Write the module skeleton with the data type and the failing tests**

Create `src/build/sitemap.rs` with the type, helper signatures (unimplemented
bodies using `todo!()`), and the full test module:

```rust
use anyhow::Result;
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
    todo!()
}

/// Percent-encode each `/`-separated path segment, preserving the separators.
fn encode_path(relpath: &str) -> String {
    todo!()
}

/// Build the `<loc>` text: trimmed base + "/" + encoded path, XML-escaped.
/// The empty relpath yields `<base>/`.
fn build_loc(base: &str, relpath: &str) -> String {
    todo!()
}

/// Format a date as W3C `YYYY-MM-DD`.
fn format_lastmod(date: NaiveDate) -> String {
    todo!()
}

/// Render the full sitemap XML document.
fn render_sitemap_xml(base: &str, entries: &[SitemapUrl]) -> String {
    todo!()
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
        // Homepage entry has no lastmod; recipe entry has exactly one.
        assert_eq!(xml.matches("<lastmod>").count(), 1);
        assert_eq!(xml.matches("<url>").count(), 2);
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p cookcli sitemap::`
Expected: compiles, tests panic with `not yet implemented` (`todo!()`).

- [ ] **Step 4: Implement the pure helpers**

Replace the four `todo!()` bodies:

```rust
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn encode_path(relpath: &str) -> String {
    relpath
        .split('/')
        .map(|seg| urlencoding::encode(seg).into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn build_loc(base: &str, relpath: &str) -> String {
    let base = base.trim_end_matches('/');
    let loc = format!("{base}/{}", encode_path(relpath));
    xml_escape(&loc)
}

fn format_lastmod(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}
```

Note: `encode_path("")` returns `""` (a single empty segment), so
`build_loc(base, "")` yields `"{base}/"` — the homepage. `urlencoding::encode`
encodes a space as `%20` and `&` as `%26`, so no raw XML-special characters
survive in the path; `xml_escape` then covers anything in `base`.

- [ ] **Step 5: Implement `render_sitemap_xml`**

```rust
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
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p cookcli sitemap::`
Expected: all sitemap tests PASS.

- [ ] **Step 7: Commit**

```bash
git add src/build/sitemap.rs src/build/mod.rs
git commit -m "feat(build): add sitemap XML rendering core"
```

---

### Task 2: Tree walk + wiring into `cook build web`

Add the tree-walking glue and the `--sitemap` flag, then call it from `run_web`.
This clears the dead-code warnings from Task 1 by putting every function to use.

**Files:**
- Modify: `src/build/sitemap.rs` (add `build_sitemap_entries` and `write_sitemap`)
- Modify: `src/build/mod.rs:37-64` (add `sitemap` arg field), `:107-148` (validation + call + summary)

- [ ] **Step 1: Add the tree-walk and writer glue to `sitemap.rs`**

Append to `src/build/sitemap.rs` (before the `#[cfg(test)]` module). It walks the
`RecipeTree` exactly like `index.rs::collect` / `walk_directories`: leaves are
recipe/menu pages, non-leaves are directory pages. The homepage entry is pushed
once at the top. `lastmod` comes from the node's on-disk `path` mtime
(best-effort).

```rust
use cooklang_find::RecipeTree;

/// Read a file's modification time as a local `NaiveDate`, best-effort.
fn file_lastmod(path: &Utf8Path) -> Option<NaiveDate> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    let datetime: chrono::DateTime<chrono::Local> = modified.into();
    Some(datetime.date_naive())
}

/// Walk the recipe tree into a flat list of sitemap entries: the homepage, one
/// per directory listing page, and one per recipe/menu page.
fn build_sitemap_entries(tree: &RecipeTree) -> Vec<SitemapUrl> {
    let mut out = vec![SitemapUrl { relpath: String::new(), lastmod: None }];
    collect(tree, String::new(), &mut out);
    out
}

fn collect(tree: &RecipeTree, prefix: String, out: &mut Vec<SitemapUrl>) {
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
            out.push(SitemapUrl {
                relpath: format!("directory/{sub}.html"),
                lastmod: None,
            });
            collect(child, sub, out);
        }
    }
}

/// Build and write `sitemap.xml` to the output root.
pub fn write_sitemap(output: &Utf8Path, base: &str, tree: &RecipeTree) -> Result<()> {
    let entries = build_sitemap_entries(tree);
    let xml = render_sitemap_xml(base, &entries);
    crate::build::writer::write_bytes(output, Utf8Path::new("sitemap.xml"), xml.as_bytes())
}
```

- [ ] **Step 2: Confirm it still compiles and tests pass**

Run: `cargo test -p cookcli sitemap::`
Expected: PASS (the existing pure-function tests still pass; new glue compiles).

- [ ] **Step 3: Add the `--sitemap` flag to `WebBuildArgs`**

In `src/build/mod.rs`, the `WebBuildArgs` struct currently ends with the `lang`
field (around lines 57-64). Add a new field after `lang`:

```rust
    /// UI language for the generated site (default: en-US)
    ///
    /// Accepts a BCP-47 tag like `de-DE`, or a bare language code like `de`
    /// that matches a supported region. Supported: en-US, de-DE, nl-NL,
    /// fr-FR, es-ES, eu-ES, sv-SE.
    #[arg(long, value_parser = parse_lang_arg)]
    pub lang: Option<LanguageIdentifier>,

    /// Full base URL of the deployed site to generate a sitemap.xml
    ///
    /// When set (e.g. https://recipes.example.com or
    /// https://example.com/recipes), writes a sitemaps.org-compliant
    /// sitemap.xml at the output root listing every page with absolute URLs.
    /// This is the complete URL prefix (scheme + host + optional subpath) and
    /// is independent of --base-url. Omit it to skip sitemap generation.
    #[arg(long)]
    pub sitemap: Option<String>,
```

- [ ] **Step 4: Validate the URL and write the sitemap in `run_web`**

In `src/build/mod.rs`, locate the block in `run_web` that writes the search
index (currently lines ~137-144):

```rust
    let entries = index::build_search_index(&tree);
    let entry_count = entries.len();
    let json = serde_json::to_string(&entries)?;
    writer::write_bytes(
        &output,
        camino::Utf8Path::new("static/search-index.json"),
        json.as_bytes(),
    )?;
```

Immediately after that block (before the final `println!`), add:

```rust
    let sitemap_written = if let Some(base) = args.sitemap.as_deref() {
        let parsed = url::Url::parse(base)
            .with_context(|| format!("Invalid --sitemap URL: {base}"))?;
        if parsed.cannot_be_a_base() || parsed.host().is_none() {
            bail!("--sitemap must be an absolute URL with a host, e.g. https://recipes.example.com");
        }
        sitemap::write_sitemap(&output, base, &tree)?;
        true
    } else {
        false
    };
```

- [ ] **Step 5: Update the summary line to mention the sitemap**

The final `println!` in `run_web` currently reads (lines ~146-148):

```rust
    println!(
        "Wrote index, directories, {recipe_count} recipe pages, {image_count} images, {asset_count} static assets, {entry_count} search entries"
    );
    Ok(())
```

Replace it with:

```rust
    let sitemap_note = if sitemap_written { ", sitemap.xml" } else { "" };
    println!(
        "Wrote index, directories, {recipe_count} recipe pages, {image_count} images, {asset_count} static assets, {entry_count} search entries{sitemap_note}"
    );
    Ok(())
```

- [ ] **Step 6: Build and run the full test suite**

Run: `cargo test -p cookcli`
Expected: PASS, no compile errors, no remaining `dead_code` warnings from `sitemap.rs`.

- [ ] **Step 7: Commit**

```bash
git add src/build/sitemap.rs src/build/mod.rs
git commit -m "feat(build): generate sitemap.xml with --sitemap flag"
```

---

### Task 3: Manual end-to-end verification

**Files:** none (verification only).

- [ ] **Step 1: Build a site with a sitemap against the seed recipes**

Run:

```bash
cargo run -- build web /tmp/cooksite --base-path ./seed --sitemap https://recipes.example.com
```

Expected: the summary line ends with `, sitemap.xml`, and
`/tmp/cooksite/sitemap.xml` exists.

- [ ] **Step 2: Inspect the sitemap**

Run:

```bash
head -20 /tmp/cooksite/sitemap.xml
```

Expected: starts with the XML prolog and `<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">`,
contains `<loc>https://recipes.example.com/</loc>` for the homepage, plus
`<loc>` entries under `https://recipes.example.com/recipe/...` and
`https://recipes.example.com/directory/...`, with `<lastmod>YYYY-MM-DD</lastmod>`
on recipe entries.

- [ ] **Step 3: Verify it is well-formed XML**

Run:

```bash
xmllint --noout /tmp/cooksite/sitemap.xml && echo "well-formed"
```

Expected: prints `well-formed` (no parser errors). If `xmllint` is unavailable,
skip this step.

- [ ] **Step 4: Verify omitting the flag skips the sitemap**

Run:

```bash
rm -rf /tmp/cooksite2 && cargo run -- build web /tmp/cooksite2 --base-path ./seed
test ! -e /tmp/cooksite2/sitemap.xml && echo "no sitemap (correct)"
```

Expected: prints `no sitemap (correct)` and the summary line does NOT mention sitemap.

- [ ] **Step 5: Verify an invalid URL is rejected**

Run:

```bash
cargo run -- build web /tmp/cooksite3 --base-path ./seed --sitemap not-a-url; echo "exit: $?"
```

Expected: a clear error mentioning the invalid `--sitemap` URL and a non-zero exit code.

---

### Task 4: Pre-PR checks

**Files:** none.

- [ ] **Step 1: Format**

Run: `cargo fmt`
Expected: no diff / clean.

- [ ] **Step 2: Lint**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: no warnings or errors.

- [ ] **Step 3: Test**

Run: `cargo test`
Expected: all tests pass.

- [ ] **Step 4: Commit any formatting changes (if any)**

```bash
git add -A
git commit -m "chore: fmt" || echo "nothing to commit"
```
```
