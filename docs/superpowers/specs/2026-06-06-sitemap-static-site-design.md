# Sitemap generation for `cook build web`

Date: 2026-06-06
Issue: https://github.com/cooklang/cookcli/issues/346

## Goal

Optionally generate a `sitemap.xml` (per the [sitemaps.org protocol](https://www.sitemaps.org/protocol.html))
when building the static site with `cook build web`, so the generated site is
more discoverable by search engines.

## Background

`cook build web` (added in #344, nested under `build` in #348) renders recipes
to a self-contained static site. The output tree contains:

- `index.html` — root recipe listing
- `directory/<sub>.html` — one per non-leaf directory node
- `recipe/<sub>.html` and `menu/<sub>.html` — one per recipe / menu leaf
- `static/...` assets, `api/static/...` images, `recipe/<sub>.cook` sources
- `static/search-index.json`

The sitemaps.org protocol requires each `<loc>` to be a **fully-qualified
absolute URL** (scheme + host). The existing `--base-url` flag is only a *path*
prefix for internal links (e.g. `/recipes/`) and cannot supply the domain, so a
new input is required.

## Interface

A new opt-in flag on `WebBuildArgs`:

```
--sitemap <URL>    Full base URL of the deployed site
                   (e.g. https://recipes.example.com or https://example.com/recipes).
                   When set, writes sitemap.xml at the output root listing all
                   pages with absolute URLs.
```

- The value is the **complete base URL prefix** (origin + optional subpath).
  Sitemap entry URLs are formed as `<sitemap>/<relpath>`. This mirrors the
  `--base` argument of the `static-sitemap-cli` tool referenced in the issue:
  one value carries scheme + host + subpath.
- It is **independent of `--base-url`** (which only controls internal link
  prefixes within pages). The two can be combined freely.
- Validated with the `url` crate. A non-absolute / unparseable URL is a hard
  error (`bail!`) with a clear message, so users never get a silently broken
  sitemap.
- When omitted, **no sitemap is produced** — no change to existing behavior.

## Module: `src/build/sitemap.rs`

Data:

```rust
struct SitemapUrl {
    relpath: String,                  // e.g. "recipe/Breakfast/Pancakes.html"; "" for the homepage
    lastmod: Option<chrono::NaiveDate>,
}
```

Functions:

- `build_sitemap_entries(tree: &RecipeTree, source: &Utf8Path) -> Vec<SitemapUrl>`
  Walks the `RecipeTree` with the same traversal shape as
  `index.rs::build_search_index` / `walk_directories`, producing:
  - the homepage → `relpath = ""` (rendered as `<base>/`), no `lastmod`
  - each non-leaf node → `directory/<sub>.html`, no `lastmod`
  - each recipe/menu leaf → `recipe/<sub>.html` or `menu/<sub>.html`, with
    `lastmod` from the on-disk `.cook`/`.menu` source file's modification time
    (`std::fs::metadata().modified()`), converted to a local `NaiveDate`. If the
    mtime cannot be read, `lastmod` is `None` (best-effort, never fatal).
  Uses the on-disk file stem (not the tree key, which may be the metadata title),
  consistent with `index.rs` and `walk_recipes`.

- `render_sitemap_xml(base: &str, entries: &[SitemapUrl]) -> String`
  Builds the XML document:
  - `<?xml version="1.0" encoding="UTF-8"?>`
  - `<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">`
  - one `<url>` per entry with:
    - `<loc>`: `base` with any trailing slash trimmed, joined to the relpath.
      Each path **segment** is percent-encoded with `urlencoding::encode`
      (slashes between segments preserved), then the whole `<loc>` text is
      XML-escaped (`&` → `&amp;`, `<` → `&lt;`, `>` → `&gt;`). The homepage
      relpath `""` yields `<base>/`.
    - `<lastmod>` (only when present): `YYYY-MM-DD`.

- `write_sitemap(output: &Utf8Path, base: &str, tree: &RecipeTree, source: &Utf8Path) -> Result<()>`
  Ties the above together and writes `sitemap.xml` to the output root via
  `writer::write_bytes`.

## Wiring in `mod.rs`

- Add `sitemap: Option<String>` to `WebBuildArgs` (with the doc comment above).
- Declare `mod sitemap;`.
- In `run_web`, after the search index is written (reusing the already-pruned
  `tree`):
  - if `args.sitemap` is `Some(base)`:
    - validate with `url::Url::parse(base)`; `bail!` on error or non-absolute
      (no scheme/host).
    - call `sitemap::write_sitemap(&output, base, &tree, &source)`.
  - reflect it in the final summary line (e.g. append `, sitemap` when written).

## Edge cases & decisions

- **Homepage URL**: emitted as `<base>/` (canonical homepage), not
  `<base>/index.html`. Static hosts serve `index.html` at the directory root.
- **All HTML pages included**: index, directory listings, recipe and menu pages
  (per design decision). `.cook` sources, images, JSON index, and static assets
  are not page content and are excluded.
- **Escaping**: per-segment percent-encoding handles spaces/Unicode in recipe
  names; XML-escaping handles `&`/`<`/`>` in the resulting URL.
- **Pruned tree**: the same pruned tree used for rendering is passed in, so a
  `_site` output nested inside the source directory is not listed.
- **lastmod is best-effort**: a missing/unreadable mtime omits `<lastmod>` for
  that entry rather than failing the build.

## Testing (unit tests in `sitemap.rs`)

- `render_sitemap_xml` produces a well-formed document: XML prolog, `urlset`
  with the correct `xmlns`, one `<url>`/`<loc>` per entry.
- `<loc>` joins base + relpath correctly, with the homepage rendered as
  `<base>/`.
- Base URL with and without a trailing slash produce identical `<loc>` values.
- A relpath segment containing a space is percent-encoded.
- A name containing `&` is XML-escaped in the output.
- `<lastmod>` is rendered as `YYYY-MM-DD` when present and omitted when absent.

## Out of scope

- `changefreq` / `priority` hints (optional in the protocol; omitted — YAGNI).
- Sitemap index files / 50k-URL splitting (not needed at recipe-collection scale).
- `robots.txt` generation.
