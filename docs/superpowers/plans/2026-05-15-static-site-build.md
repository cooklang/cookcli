# Static Site Build Command Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `cook build` CLI command that renders the recipe collection as a self-contained static HTML site, reusing the existing server templates.

**Architecture:** New top-level module `src/build/` orchestrates a single-pass render. We add a `static_mode: bool` flag to existing Askama template structs and gate dynamic UI in template files behind it. Recipe/menu rendering logic is first extracted from `src/server/ui.rs` handlers into `src/server/builders.rs` so both the server and the static build use the same code path. The build writes HTML files mirroring the source tree, copies embedded static assets and recipe images, and generates a JSON search index plus a small client-side `search.js`.

**Tech Stack:** Rust, Askama (templates), `cooklang_find` (recipe tree), `RustEmbed` (static asset embedding), `tempfile`/`assert_cmd` (tests).

**Spec:** `docs/superpowers/specs/2026-05-15-static-site-build-design.md`

---

## File Structure

**New files (Rust):**
- `src/build/mod.rs` — `BuildArgs`, `run(ctx, args)`, orchestrator
- `src/build/renderer.rs` — render functions for index/directory/recipe/menu pages
- `src/build/writer.rs` — file writes, static asset copying, image copying
- `src/build/links.rs` — relative `prefix` computation per page
- `src/build/index.rs` — search index generation
- `src/server/builders.rs` — extracted template-builder functions (used by both server and build)
- `tests/build.rs` — integration smoke tests

**New file (static asset):**
- `static/js/search.js` — client-side search using `search-index.json`

**Modified files:**
- `src/args.rs` — add `Build` command variant
- `src/main.rs` — add `build` module + dispatch + base_path handling for build command
- `src/server/mod.rs` — wire `builders` module
- `src/server/templates.rs` — add `static_mode: bool` to all relevant template structs
- `src/server/ui.rs` — call new builders, pass `static_mode: false`
- `templates/base.html` — gate dynamic nav, switch search JS source
- `templates/recipes.html` — gate shopping-list & menu buttons; append `.html` to links
- `templates/recipe.html` — gate edit/shopping-list/pantry/scaling; append `.html`
- `templates/menu.html` — gate edit/shopping-list; append `.html`
- `Cargo.toml` — none expected; reuse existing deps

---

## Task 1: CLI skeleton (`cook build` stub)

**Files:**
- Create: `src/build/mod.rs`
- Modify: `src/args.rs`, `src/main.rs`
- Test: `tests/build.rs`

- [ ] **Step 1: Write the failing test**

Create `tests/build.rs`:

```rust
use assert_cmd::Command;

#[test]
fn build_command_help_works() {
    let mut cmd = Command::cargo_bin("cook").unwrap();
    cmd.args(["build", "--help"]).assert().success();
}
```

- [ ] **Step 2: Run test, verify it fails**

Run: `cargo test --test build build_command_help_works`
Expected: FAIL with unrecognized subcommand `build`.

- [ ] **Step 3: Add stub module `src/build/mod.rs`**

```rust
use crate::Context;
use anyhow::Result;
use camino::Utf8PathBuf;
use clap::Args;

#[derive(Debug, Args)]
pub struct BuildArgs {
    /// Output directory for the generated static site
    ///
    /// Defaults to ./_site if not specified. The directory is created if
    /// missing. Existing files in the directory are overwritten as needed
    /// but not wiped wholesale.
    #[arg(value_hint = clap::ValueHint::DirPath)]
    pub output_dir: Option<Utf8PathBuf>,

    /// Root directory containing your recipe files
    #[arg(long, value_hint = clap::ValueHint::DirPath)]
    pub base_path: Option<Utf8PathBuf>,

    /// Absolute URL prefix for hosting under a subpath (e.g. /recipes/)
    ///
    /// When set, internal links use this absolute prefix instead of
    /// page-relative paths. Useful when you know the deployed subpath.
    #[arg(long)]
    pub base_url: Option<String>,
}

impl BuildArgs {
    pub fn get_base_path(&self) -> Option<Utf8PathBuf> {
        self.base_path.clone()
    }
}

pub fn run(_ctx: &Context, _args: BuildArgs) -> Result<()> {
    println!("cook build: not yet implemented");
    Ok(())
}
```

- [ ] **Step 4: Wire into `src/args.rs`**

Add `build` to the imports at top:

```rust
use crate::{build, doctor, import, lsp, pantry, recipe, report, search, seed, server, shopping_list};
```

Add the command variant in the `Command` enum (after `Server`):

```rust
/// Generate a self-contained static website from your recipe collection
///
/// Renders your recipes as static HTML files browsable on any static-file
/// host or directly from disk via file://. Excludes dynamic features
/// (shopping list, pantry, editing).
///
/// Examples:
///   cook build                         # Build to ./_site
///   cook build out                     # Build to ./out
///   cook build --base-path ~/recipes   # Use specific source directory
///   cook build --base-url /recipes/    # Absolute URL prefix for subpath hosting
#[command(
    long_about = "Generate a static HTML website from your recipe collection"
)]
Build(build::BuildArgs),
```

- [ ] **Step 5: Wire into `src/main.rs`**

Add module declaration near the other `mod` lines:

```rust
mod build;
```

Add match arm in `main()`:

```rust
Command::Build(args) => build::run(&ctx, args),
```

Add the `Build` case in `configure_context()` so its `--base-path` flag is honored:

```rust
Command::Build(ref build_args) => build_args
    .get_base_path()
    .unwrap_or_else(|| Utf8PathBuf::from(".")),
```

- [ ] **Step 6: Run test, verify it passes**

Run: `cargo test --test build build_command_help_works`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat(build): add cook build command skeleton"
```

---

## Task 2: Output directory resolution and basic invocation

**Files:**
- Modify: `src/build/mod.rs`
- Test: `tests/build.rs`

- [ ] **Step 1: Write the failing test**

Append to `tests/build.rs`:

```rust
use std::path::PathBuf;
use tempfile::TempDir;

fn seed_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("seed")
}

#[test]
fn build_creates_output_dir() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("_site");
    let seed = seed_dir();

    let mut cmd = Command::cargo_bin("cook").unwrap();
    cmd.args([
        "build",
        out.to_str().unwrap(),
        "--base-path",
        seed.to_str().unwrap(),
    ])
    .assert()
    .success();

    assert!(out.is_dir(), "output dir should exist after build");
}
```

- [ ] **Step 2: Run test, verify it fails**

Run: `cargo test --test build build_creates_output_dir`
Expected: FAIL — output dir not created (stub just prints).

- [ ] **Step 3: Implement output directory creation in `src/build/mod.rs`**

Replace `run`:

```rust
use crate::util::resolve_to_absolute_path;
use crate::Context;
use anyhow::{bail, Context as _, Result};
use camino::Utf8PathBuf;
use clap::Args;

#[derive(Debug, Args)]
pub struct BuildArgs {
    /// Output directory for the generated static site
    #[arg(value_hint = clap::ValueHint::DirPath)]
    pub output_dir: Option<Utf8PathBuf>,

    /// Root directory containing your recipe files
    #[arg(long, value_hint = clap::ValueHint::DirPath)]
    pub base_path: Option<Utf8PathBuf>,

    /// Absolute URL prefix for hosting under a subpath (e.g. /recipes/)
    #[arg(long)]
    pub base_url: Option<String>,
}

impl BuildArgs {
    pub fn get_base_path(&self) -> Option<Utf8PathBuf> {
        self.base_path.clone()
    }
}

pub fn run(ctx: &Context, args: BuildArgs) -> Result<()> {
    let source = resolve_to_absolute_path(ctx.base_path())?;
    if !source.is_dir() {
        bail!("Source base path is not a directory: {source}");
    }

    let output = args
        .output_dir
        .clone()
        .unwrap_or_else(|| Utf8PathBuf::from("_site"));
    let output = resolve_to_absolute_path(&output)?;

    std::fs::create_dir_all(&output)
        .with_context(|| format!("Failed to create output directory: {output}"))?;

    tracing::info!("Building static site from {source} into {output}");
    println!("Building static site from {source} into {output}");
    Ok(())
}
```

- [ ] **Step 4: Run test, verify it passes**

Run: `cargo test --test build`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(build): resolve source and output paths"
```

---

## Task 3: Add `static_mode` field to template structs

**Files:**
- Modify: `src/server/templates.rs`
- Modify: `src/server/ui.rs`

- [ ] **Step 1: Add `static_mode` to template structs**

In `src/server/templates.rs`, add `pub static_mode: bool` as the last field of these structs:
- `ErrorTemplate`
- `RecipesTemplate`
- `RecipeTemplate`
- `MenuTemplate`

(Skip `ShoppingListTemplate`, `PreferencesTemplate`, `PantryTemplate`, `EditTemplate`, `NewTemplate` — never rendered in static mode.)

Example for `RecipesTemplate`:

```rust
#[derive(Template)]
#[template(path = "recipes.html")]
pub struct RecipesTemplate {
    pub active: String,
    pub current_name: String,
    pub breadcrumbs: Vec<Breadcrumb>,
    pub items: Vec<RecipeItem>,
    pub todays_menu: Option<TodaysMenu>,
    pub tr: Tr,
    pub prefix: String,
    pub static_mode: bool,
}
```

- [ ] **Step 2: Pass `static_mode: false` from every server handler**

In `src/server/ui.rs`, find each construction of `RecipesTemplate`, `RecipeTemplate`, `MenuTemplate`, and `ErrorTemplate` (in `error_page`). Add `static_mode: false,` to each struct literal.

Use `grep` to find every occurrence:
```bash
grep -n "RecipesTemplate\|RecipeTemplate\|MenuTemplate\|ErrorTemplate" src/server/ui.rs src/server/handlers/*.rs src/server/mod.rs
```

For each match, append `static_mode: false,` to the field list. Do not miss any — askama will fail to compile if `static_mode` is referenced in templates and missing from struct literals.

- [ ] **Step 3: Run `cargo build` to verify compilation**

Run: `cargo build`
Expected: compiles cleanly. If any struct literal is missing `static_mode`, the compiler points at the line.

- [ ] **Step 4: Run existing tests**

Run: `cargo test`
Expected: all existing tests pass (no behavior change yet).

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor(server): add static_mode field to template structs"
```

---

## Task 4: Gate dynamic UI in `templates/base.html`

**Files:**
- Modify: `templates/base.html`

- [ ] **Step 1: Gate dynamic nav links**

In `templates/base.html`, wrap each of these elements in `{% if !static_mode %} ... {% endif %}`:

1. The shopping-list nav link (the `<a href="{{ prefix }}/shopping-list" ...>` block, lines ~777-779).
2. The pantry nav link (`<a href="{{ prefix }}/pantry" ...>`, lines ~780-782).
3. The preferences nav link (`<a href="{{ prefix }}/preferences" ...>`, lines ~784-786).
4. The mobile overflow dropdown's preferences link inside `#more-dropdown` (lines ~808-810).

Each looks like:
```html
{% if !static_mode %}
<a href="{{ prefix }}/shopping-list" ...>...</a>
{% endif %}
```

- [ ] **Step 2: Conditional search JS source**

Find the inline search script (around line 877 with `fetch(\`{{ prefix }}/api/search?q=...\`)`).

Wrap the entire `<script>` block that defines the search behavior (from `const searchInput = document.getElementById('search-input');` through the closing `</script>` containing the `document.addEventListener('click', ...)` block) in:

```html
{% if !static_mode %}
<script>
  // ... existing dynamic search script unchanged ...
</script>
{% else %}
<script src="{{ prefix }}/static/js/search.js"></script>
{% endif %}
```

Keep the `translations` object (which is currently at the top of that script block) inside the `!static_mode` branch — search.js for static mode will hardcode strings.

- [ ] **Step 3: Verify template compiles**

Run: `cargo build`
Expected: compiles cleanly. Askama validates template syntax at compile time.

- [ ] **Step 4: Run existing tests**

Run: `cargo test`
Expected: PASS (server still passes `static_mode: false`, so nothing changes at runtime).

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(templates): gate dynamic nav and search behind static_mode"
```

---

## Task 5: Gate dynamic UI in remaining templates

**Files:**
- Modify: `templates/recipes.html`
- Modify: `templates/recipe.html`
- Modify: `templates/menu.html`

For each file:

- [ ] **Step 1: Search for elements that mutate state or call dynamic APIs**

Run for each file:
```bash
grep -n "/api/\|shopping-list\|pantry\|/edit\|/new\|scale-input\|reload\|onclick" templates/recipes.html
grep -n "/api/\|shopping-list\|pantry\|/edit\|/new\|scale-input\|reload\|onclick" templates/recipe.html
grep -n "/api/\|shopping-list\|pantry\|/edit\|/new\|scale-input\|reload\|onclick" templates/menu.html
```

- [ ] **Step 2: Wrap each dynamic-only block in `{% if !static_mode %} ... {% endif %}`**

Specifically gate:

In `templates/recipes.html`:
- "Add menu to shopping list" buttons and links to `/shopping-list`.
- Any link to `/edit/...` or `/new`.

In `templates/recipe.html`:
- Edit button / link to `/edit/...`
- "Add to shopping list" button(s) and any `/api/shopping_list*` form submission targets.
- "In pantry" badges / pantry-related elements.
- Scale input control (the entire scaling form/UI block).
- Any `<script>` block that calls `/api/shopping_list*` or `/api/pantry*`.

In `templates/menu.html`:
- Edit / shopping-list / pantry actions, same as recipe.html.

Leave intact: ingredient/cookware/timer badges, recipe metadata, sections/steps, the cooking-mode JSON script (which is read-only), images.

- [ ] **Step 3: Verify template compiles**

Run: `cargo build`
Expected: cleanly compiles.

- [ ] **Step 4: Run existing tests**

Run: `cargo test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(templates): gate dynamic actions in recipe/menu/recipes templates"
```

---

## Task 6: Append `.html` to internal links when `static_mode`

**Files:**
- Modify: `templates/base.html`
- Modify: `templates/recipes.html`
- Modify: `templates/recipe.html`
- Modify: `templates/menu.html`

The server uses URLs like `{{ prefix }}/recipe/{{ path }}`. The static site needs the same but with `.html` appended. We do this with an inline conditional next to each href.

- [ ] **Step 1: Find every relevant href**

Run:
```bash
grep -n 'href="{{ prefix }}/recipe/\|href="{{ prefix }}/directory/\|href="{{ prefix }}/menu/' templates/*.html
```

- [ ] **Step 2: Add `.html` suffix conditionally**

For each match, change:
```html
href="{{ prefix }}/recipe/{{ item.path }}"
```
to:
```html
href="{{ prefix }}/recipe/{{ item.path }}{% if static_mode %}.html{% endif %}"
```

Do the same for `/directory/...` and `/menu/...` href patterns.

Also handle the root navigation: `href="{{ prefix }}/"` in base.html. In static mode the root is `index.html` — the existing trailing-slash form works on web servers but not under `file://`. Change to:
```html
href="{{ prefix }}/{% if static_mode %}index.html{% endif %}"
```

For breadcrumbs that link to a directory path (e.g. `href="{{ prefix }}/directory/{{ crumb.path }}"`), apply the same `.html` suffix rule.

- [ ] **Step 3: Verify template compiles**

Run: `cargo build`
Expected: cleanly compiles.

- [ ] **Step 4: Run existing tests**

Run: `cargo test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(templates): append .html to internal links in static_mode"
```

---

## Task 7: Extract template builders from `src/server/ui.rs`

The goal is to share recipe / menu / recipes template construction between the server handler and the static-site renderer without copy-pasting.

**Files:**
- Create: `src/server/builders.rs`
- Modify: `src/server/mod.rs`
- Modify: `src/server/ui.rs`

- [ ] **Step 1: Create `src/server/builders.rs` with extracted functions**

Move (don't copy) the template-construction body of each handler in `src/server/ui.rs` into new functions:

```rust
use crate::server::templates::*;
use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use unic_langid::LanguageIdentifier;

pub struct RecipesBuildInput<'a> {
    pub base_path: &'a Utf8Path,
    pub url_prefix: &'a str,
    pub sub_path: Option<&'a str>,
    pub lang: LanguageIdentifier,
    pub static_mode: bool,
}

/// Build a RecipesTemplate for either the root or a subdirectory.
pub fn build_recipes_template(input: RecipesBuildInput<'_>) -> Result<RecipesTemplate> {
    // ... move the body of `recipes_handler` here, returning RecipesTemplate instead of Response ...
}

pub struct RecipeBuildInput<'a> {
    pub base_path: &'a Utf8Path,
    pub url_prefix: &'a str,
    pub recipe_path: &'a str,
    pub aisle_path: Option<&'a Utf8PathBuf>,
    pub scale: f64,
    pub lang: LanguageIdentifier,
    pub static_mode: bool,
}

pub enum RecipeBuildOutput {
    Recipe(RecipeTemplate),
    Menu(MenuTemplate),
}

/// Build a RecipeTemplate or MenuTemplate for the given recipe path.
pub fn build_recipe_template(input: RecipeBuildInput<'_>) -> Result<RecipeBuildOutput> {
    // ... move the body of `recipe_page` (excluding axum response handling) here ...
}
```

The functions return template structs (`Result<RecipesTemplate>` etc.) rather than `axum::Response`. Move the parsing, image-path resolution, ingredient grouping, and section building. Set `static_mode` from the input.

- [ ] **Step 2: Register `builders` module**

In `src/server/mod.rs`, add:
```rust
pub mod builders;
```

- [ ] **Step 3: Update `src/server/ui.rs` handlers to use builders**

Each handler becomes a thin wrapper:

```rust
async fn recipes_handler(
    state: Arc<AppState>,
    path: Option<String>,
    lang: LanguageIdentifier,
) -> axum::response::Response {
    let input = crate::server::builders::RecipesBuildInput {
        base_path: &state.base_path,
        url_prefix: &state.url_prefix,
        sub_path: path.as_deref(),
        lang: lang.clone(),
        static_mode: false,
    };
    match crate::server::builders::build_recipes_template(input) {
        Ok(template) => template.into_response(),
        Err(e) => error_page(lang, &state.url_prefix, &e),
    }
}
```

Apply equivalent rewrites for `recipe_page` (handling both `RecipeBuildOutput::Recipe` and `Menu`), and remove now-unused helpers from `ui.rs` (`count_recipes_tree`, `get_image_path`, etc.) by moving them into `builders.rs` if still needed.

- [ ] **Step 4: Verify everything compiles**

Run: `cargo build`
Expected: clean build.

- [ ] **Step 5: Run existing tests**

Run: `cargo test`
Expected: PASS. Server behavior unchanged.

- [ ] **Step 6: Manual smoke test**

Run:
```bash
cargo run -- server ./seed &
SERVER_PID=$!
sleep 2
curl -sf http://127.0.0.1:9080/ > /dev/null && echo "root OK"
curl -sf http://127.0.0.1:9080/recipe/Breakfast/Easy%20Pancakes > /dev/null && echo "recipe OK"
kill $SERVER_PID
```
Expected: both "root OK" and "recipe OK" print.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "refactor(server): extract template builders for reuse"
```

---

## Task 8: Implement relative-prefix helper

**Files:**
- Create: `src/build/links.rs`
- Modify: `src/build/mod.rs`
- Test: inline `#[cfg(test)]` module in `src/build/links.rs`

- [ ] **Step 1: Write the failing test**

Create `src/build/links.rs`:

```rust
use camino::Utf8Path;

/// Given an output file path like "index.html" or "recipe/Breakfast/Pancakes.html",
/// return the relative prefix that resolves from that file back to the output root.
/// Examples:
///   "index.html"                            -> "."
///   "directory/Breakfast.html"              -> ".."
///   "recipe/Breakfast/Pancakes.html"        -> "../.."
///   "menu/Sunday/Brunch.html"               -> "../.."
pub fn relative_prefix(output_relpath: &Utf8Path) -> String {
    let depth = output_relpath
        .components()
        .count()
        .saturating_sub(1); // last component is the filename
    if depth == 0 {
        ".".to_string()
    } else {
        std::iter::repeat("..")
            .take(depth)
            .collect::<Vec<_>>()
            .join("/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_index() {
        assert_eq!(relative_prefix(Utf8Path::new("index.html")), ".");
    }

    #[test]
    fn one_level_deep() {
        assert_eq!(relative_prefix(Utf8Path::new("directory/Breakfast.html")), "..");
    }

    #[test]
    fn two_levels_deep() {
        assert_eq!(relative_prefix(Utf8Path::new("recipe/Breakfast/Pancakes.html")), "../..");
    }

    #[test]
    fn three_levels_deep() {
        assert_eq!(relative_prefix(Utf8Path::new("recipe/A/B/C.html")), "../../..");
    }
}
```

- [ ] **Step 2: Register module in `src/build/mod.rs`**

Add at the top:
```rust
mod links;
```

- [ ] **Step 3: Run tests, verify they pass**

Run: `cargo test --lib build::links`
Expected: PASS for all four tests.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(build): add relative-prefix helper"
```

---

## Task 9: Implement file writer module

**Files:**
- Create: `src/build/writer.rs`
- Modify: `src/build/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `src/build/writer.rs`:

```rust
use anyhow::{Context, Result};
use camino::Utf8Path;
use std::fs;

/// Write `contents` to `output_root/relpath`, creating parent directories.
pub fn write_html(output_root: &Utf8Path, relpath: &Utf8Path, contents: &str) -> Result<()> {
    let dest = output_root.join(relpath);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent dir: {parent}"))?;
    }
    fs::write(&dest, contents).with_context(|| format!("Failed to write: {dest}"))?;
    Ok(())
}

/// Copy `bytes` to `output_root/relpath`, creating parent directories.
pub fn write_bytes(output_root: &Utf8Path, relpath: &Utf8Path, bytes: &[u8]) -> Result<()> {
    let dest = output_root.join(relpath);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent dir: {parent}"))?;
    }
    fs::write(&dest, bytes).with_context(|| format!("Failed to write: {dest}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn write_html_creates_nested_dirs() {
        let tmp = TempDir::new().unwrap();
        let root = camino::Utf8Path::from_path(tmp.path()).unwrap();
        let rel = Utf8Path::new("a/b/c.html");
        write_html(root, rel, "<html></html>").unwrap();
        let contents = std::fs::read_to_string(root.join(rel)).unwrap();
        assert_eq!(contents, "<html></html>");
    }
}
```

- [ ] **Step 2: Register module in `src/build/mod.rs`**

Add:
```rust
mod writer;
```

- [ ] **Step 3: Run test**

Run: `cargo test --lib build::writer`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(build): add output writer with nested dir support"
```

---

## Task 10: Copy embedded static assets

**Files:**
- Modify: `src/build/writer.rs`
- Modify: `src/build/mod.rs`

Note: The server's `StaticFiles` rust-embed is declared as `struct StaticFiles` in `src/server/mod.rs`. We need access to it. Make it `pub` so build can use it.

- [ ] **Step 1: Make `StaticFiles` pub in `src/server/mod.rs`**

Find:
```rust
#[derive(RustEmbed)]
#[folder = "static/"]
struct StaticFiles;
```

Change to:
```rust
#[derive(RustEmbed)]
#[folder = "static/"]
pub struct StaticFiles;
```

- [ ] **Step 2: Add `copy_static_assets` to `src/build/writer.rs`**

Append:

```rust
use rust_embed::RustEmbed;

/// Copy every file in the rust-embed `StaticFiles` to `output_root/static/<path>`.
pub fn copy_static_assets(output_root: &Utf8Path) -> Result<usize> {
    let mut count = 0;
    for path in crate::server::StaticFiles::iter() {
        let rel = Utf8Path::new("static").join(path.as_ref());
        let file = crate::server::StaticFiles::get(path.as_ref())
            .with_context(|| format!("Embedded file vanished: {path}"))?;
        write_bytes(output_root, &rel, &file.data)?;
        count += 1;
    }
    Ok(count)
}
```

- [ ] **Step 3: Write test**

Append to the `#[cfg(test)] mod tests` block in `src/build/writer.rs`:

```rust
#[test]
fn copy_static_assets_writes_known_file() {
    let tmp = TempDir::new().unwrap();
    let root = camino::Utf8Path::from_path(tmp.path()).unwrap();
    let count = copy_static_assets(root).unwrap();
    assert!(count > 0, "should copy at least one static asset");
    assert!(root.join("static/css/output.css").is_file());
}
```

- [ ] **Step 4: Run test**

Run: `cargo test --lib build::writer::tests::copy_static_assets_writes_known_file`
Expected: PASS. (Prerequisite: `make css` was run at some point so `static/css/output.css` exists. CI runs this. Locally run `make css` first if it fails.)

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(build): copy embedded static assets to output"
```

---

## Task 11: Render index and directory pages

**Files:**
- Create: `src/build/renderer.rs`
- Modify: `src/build/mod.rs`

- [ ] **Step 1: Create `src/build/renderer.rs`**

```rust
use crate::build::links::relative_prefix;
use crate::build::writer::write_html;
use crate::server::builders::{build_recipes_template, RecipesBuildInput};
use anyhow::Result;
use askama::Template;
use camino::{Utf8Path, Utf8PathBuf};
use unic_langid::LanguageIdentifier;

/// Render the root index page (recipes listing).
pub fn render_index(
    source: &Utf8Path,
    output: &Utf8Path,
    base_url: Option<&str>,
    lang: &LanguageIdentifier,
) -> Result<()> {
    let relpath = Utf8PathBuf::from("index.html");
    let prefix = compute_prefix(base_url, &relpath);
    let template = build_recipes_template(RecipesBuildInput {
        base_path: source,
        url_prefix: &prefix,
        sub_path: None,
        lang: lang.clone(),
        static_mode: true,
    })?;
    let html = template.render()?;
    write_html(output, &relpath, &html)
}

/// Render one directory listing page.
pub fn render_directory(
    source: &Utf8Path,
    output: &Utf8Path,
    sub_path: &str,
    base_url: Option<&str>,
    lang: &LanguageIdentifier,
) -> Result<()> {
    let relpath = Utf8PathBuf::from(format!("directory/{sub_path}.html"));
    let prefix = compute_prefix(base_url, &relpath);
    let template = build_recipes_template(RecipesBuildInput {
        base_path: source,
        url_prefix: &prefix,
        sub_path: Some(sub_path),
        lang: lang.clone(),
        static_mode: true,
    })?;
    let html = template.render()?;
    write_html(output, &relpath, &html)
}

fn compute_prefix(base_url: Option<&str>, relpath: &Utf8Path) -> String {
    match base_url {
        Some(b) => b.trim_end_matches('/').to_string(),
        None => relative_prefix(relpath),
    }
}
```

- [ ] **Step 2: Register module**

In `src/build/mod.rs`:
```rust
mod renderer;
```

- [ ] **Step 3: Walk tree in `run()` and render listings**

In `src/build/mod.rs`, replace the body of `run()`:

```rust
pub fn run(ctx: &Context, args: BuildArgs) -> Result<()> {
    let source = resolve_to_absolute_path(ctx.base_path())?;
    if !source.is_dir() {
        bail!("Source base path is not a directory: {source}");
    }

    let output = args
        .output_dir
        .clone()
        .unwrap_or_else(|| Utf8PathBuf::from("_site"));
    let output = resolve_to_absolute_path(&output)?;
    std::fs::create_dir_all(&output)
        .with_context(|| format!("Failed to create output directory: {output}"))?;

    println!("Building static site from {source} into {output}");

    let lang: unic_langid::LanguageIdentifier = "en-US".parse().unwrap();
    let base_url = args.base_url.as_deref();

    renderer::render_index(&source, &output, base_url, &lang)?;

    let tree = cooklang_find::build_tree(&source)?;
    walk_directories(&tree, &source, &output, base_url, &lang, String::new())?;

    let asset_count = writer::copy_static_assets(&output)?;
    println!("Wrote index, directories, and {asset_count} static assets");
    Ok(())
}

fn walk_directories(
    tree: &cooklang_find::RecipeTree,
    source: &camino::Utf8Path,
    output: &camino::Utf8Path,
    base_url: Option<&str>,
    lang: &unic_langid::LanguageIdentifier,
    prefix_path: String,
) -> Result<()> {
    for (name, child) in &tree.children {
        if child.children.is_empty() {
            continue; // it's a recipe file, handled in next task
        }
        let sub = if prefix_path.is_empty() {
            name.to_string()
        } else {
            format!("{prefix_path}/{name}")
        };
        renderer::render_directory(source, output, &sub, base_url, lang)?;
        walk_directories(child, source, output, base_url, lang, sub)?;
    }
    Ok(())
}
```

- [ ] **Step 4: Write integration test**

Append to `tests/build.rs`:

```rust
#[test]
fn build_writes_index_and_static_assets() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("_site");
    let seed = seed_dir();

    Command::cargo_bin("cook")
        .unwrap()
        .args([
            "build",
            out.to_str().unwrap(),
            "--base-path",
            seed.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(out.join("index.html").is_file(), "index.html should exist");
    assert!(out.join("static/css/output.css").is_file(), "css should exist");

    let index = std::fs::read_to_string(out.join("index.html")).unwrap();
    assert!(!index.contains("/api/search"), "static index should not reference api search");
    assert!(!index.contains("Add to shopping list"), "no shopping list UI");
}
```

- [ ] **Step 5: Run test**

Run: `cargo test --test build build_writes_index_and_static_assets`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(build): render index and directory listings"
```

---

## Task 12: Render recipe and menu pages

**Files:**
- Modify: `src/build/renderer.rs`
- Modify: `src/build/mod.rs`

- [ ] **Step 1: Add render functions for recipes and menus**

Append to `src/build/renderer.rs`:

```rust
use crate::server::builders::{build_recipe_template, RecipeBuildInput, RecipeBuildOutput};

/// Render a single recipe (or menu) page.
///
/// Both `recipe/<path>.html` and `menu/<path>.html` sit at the same depth in
/// the output tree, so the page-relative `prefix` is identical regardless of
/// which one we end up writing. We render once and pick the destination path
/// based on whether the entry turned out to be a menu.
pub fn render_recipe(
    source: &Utf8Path,
    output: &Utf8Path,
    recipe_relpath: &str,
    aisle_path: Option<&Utf8PathBuf>,
    base_url: Option<&str>,
    lang: &LanguageIdentifier,
) -> Result<()> {
    let trimmed = recipe_relpath.trim_end_matches(".cook");
    let provisional = Utf8PathBuf::from(format!("recipe/{trimmed}.html"));
    let prefix = compute_prefix(base_url, &provisional);

    let kind = build_recipe_template(RecipeBuildInput {
        base_path: source,
        url_prefix: &prefix,
        recipe_path: recipe_relpath,
        aisle_path,
        scale: 1.0,
        lang: lang.clone(),
        static_mode: true,
    })?;

    match kind {
        RecipeBuildOutput::Recipe(t) => {
            let html = t.render()?;
            write_html(output, &provisional, &html)
        }
        RecipeBuildOutput::Menu(t) => {
            let menu_relpath = Utf8PathBuf::from(format!("menu/{trimmed}.html"));
            let html = t.render()?;
            write_html(output, &menu_relpath, &html)
        }
    }
}
```

- [ ] **Step 2: Walk all `.cook` files in `run()`**

In `src/build/mod.rs`, add a recipe-walk helper and call it from `run()` (after `walk_directories` but before asset copy):

```rust
fn walk_recipes(
    tree: &cooklang_find::RecipeTree,
    source: &camino::Utf8Path,
    output: &camino::Utf8Path,
    aisle_path: Option<&camino::Utf8PathBuf>,
    base_url: Option<&str>,
    lang: &unic_langid::LanguageIdentifier,
    prefix_path: String,
) -> Result<usize> {
    let mut count = 0;
    for (name, child) in &tree.children {
        let sub = if prefix_path.is_empty() {
            name.to_string()
        } else {
            format!("{prefix_path}/{name}")
        };

        if child.children.is_empty() {
            // Recipe file
            if let Err(e) = renderer::render_recipe(source, output, &sub, aisle_path, base_url, lang) {
                tracing::warn!("Skipping recipe {sub}: {e:#}");
                continue;
            }
            count += 1;
        } else {
            count += walk_recipes(child, source, output, aisle_path, base_url, lang, sub)?;
        }
    }
    Ok(count)
}
```

In `run()`, after the `walk_directories(...)` call:

```rust
let aisle = ctx.aisle();
let recipe_count = walk_recipes(&tree, &source, &output, aisle.as_ref(), base_url, &lang, String::new())?;
```

Update the summary `println!` to include `{recipe_count}`.

- [ ] **Step 3: Write integration test**

Append to `tests/build.rs`:

```rust
#[test]
fn build_writes_recipe_pages() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("_site");
    let seed = seed_dir();

    Command::cargo_bin("cook")
        .unwrap()
        .args([
            "build",
            out.to_str().unwrap(),
            "--base-path",
            seed.to_str().unwrap(),
        ])
        .assert()
        .success();

    // The seed contains "Easy Pancakes.cook" under Breakfast.
    let pancakes = out.join("recipe/Breakfast/Easy Pancakes.html");
    assert!(pancakes.is_file(), "pancakes html should exist at {pancakes:?}");

    let html = std::fs::read_to_string(&pancakes).unwrap();
    assert!(html.contains("Pancakes"), "title should be present");
    assert!(!html.contains("/api/shopping_list"), "no shopping-list api references");
}
```

(If the seed directory doesn't contain that exact filename, run `ls seed/Breakfast/` first and adjust to the actual filename present.)

- [ ] **Step 4: Run tests**

Run: `cargo test --test build`
Expected: PASS for all build tests.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(build): render recipe and menu pages"
```

---

## Task 13: Copy recipe images

**Files:**
- Modify: `src/build/writer.rs`
- Modify: `src/build/mod.rs`

Recipe images live alongside `.cook` files (e.g. `Pancakes.jpg` next to `Pancakes.cook`). The server serves them via `/api/static/<path>`. We copy them into `_site/api/static/<same-path>` so the template-generated URLs continue to resolve.

- [ ] **Step 1: Add image copy function**

Append to `src/build/writer.rs`:

```rust
/// Copy a single source file into `output_root/api/static/<relpath>`.
pub fn copy_image(output_root: &Utf8Path, source_root: &Utf8Path, abs_image: &Utf8Path) -> Result<()> {
    let rel = abs_image
        .strip_prefix(source_root)
        .with_context(|| format!("Image {abs_image} not under source {source_root}"))?;
    let dest = output_root.join("api/static").join(rel);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(abs_image, &dest)
        .with_context(|| format!("Failed to copy {abs_image} → {dest}"))?;
    Ok(())
}
```

- [ ] **Step 2: Discover and copy images during the recipe walk**

In `src/build/mod.rs`, the recipe walk has access to each recipe entry. Modify `walk_recipes` so that after rendering a recipe, it also discovers and copies image files.

Use the recipe-find API's `title_image()` (which the renderer already uses) plus a manual scan for step images. Simpler approach for v1: walk the source directory for any file with an image extension and copy it:

Add a helper `copy_all_images(source, output)`:

```rust
fn copy_all_images(source: &camino::Utf8Path, output: &camino::Utf8Path) -> Result<usize> {
    let mut count = 0;
    for entry in walkdir_utf8(source)? {
        let path = entry;
        if path.is_file() {
            match path.extension().map(|e| e.to_ascii_lowercase()).as_deref() {
                Some("jpg" | "jpeg" | "png" | "gif" | "webp" | "avif") => {
                    writer::copy_image(output, source, &path)?;
                    count += 1;
                }
                _ => {}
            }
        }
    }
    Ok(count)
}
```

The project already uses `camino` and standard library. Implement `walkdir_utf8` inline using `std::fs::read_dir` recursively (no new dependency):

```rust
fn walkdir_utf8(root: &camino::Utf8Path) -> Result<Vec<camino::Utf8PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = camino::Utf8PathBuf::try_from(entry.path())
                .map_err(|e| anyhow::anyhow!("Non-UTF-8 path: {e}"))?;
            if path.is_dir() {
                // Skip hidden + output-like dirs at the source root
                if let Some(name) = path.file_name() {
                    if name.starts_with('.') {
                        continue;
                    }
                }
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    Ok(out)
}
```

Call from `run()`:

```rust
let image_count = copy_all_images(&source, &output)?;
println!("Wrote {recipe_count} recipes, {image_count} images, {asset_count} static assets");
```

- [ ] **Step 3: Integration test**

Append to `tests/build.rs` (assuming the seed has at least one image):

```rust
#[test]
fn build_copies_images_when_present() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("_site");
    let seed = seed_dir();

    Command::cargo_bin("cook")
        .unwrap()
        .args([
            "build",
            out.to_str().unwrap(),
            "--base-path",
            seed.to_str().unwrap(),
        ])
        .assert()
        .success();

    // At minimum, no panic. If the seed has images, api/static exists.
    let images_dir = out.join("api/static");
    if images_dir.is_dir() {
        let entries: Vec<_> = std::fs::read_dir(&images_dir).unwrap().collect();
        assert!(!entries.is_empty(), "api/static is empty");
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --test build`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(build): copy recipe images to output"
```

---

## Task 14: Generate search index JSON

**Files:**
- Create: `src/build/index.rs`
- Modify: `src/build/mod.rs`

- [ ] **Step 1: Write search index module**

Create `src/build/index.rs`:

```rust
use anyhow::Result;
use camino::Utf8Path;
use serde::Serialize;

#[derive(Serialize)]
pub struct SearchEntry {
    pub title: String,
    pub path: String,        // e.g. "recipe/Breakfast/Pancakes.html"
    pub tags: Vec<String>,
    pub ingredients: Vec<String>,
}

/// Build a flat list of search entries by walking the recipe tree.
pub fn build_search_index(
    source: &Utf8Path,
    tree: &cooklang_find::RecipeTree,
) -> Result<Vec<SearchEntry>> {
    let mut out = Vec::new();
    collect(source, tree, String::new(), &mut out);
    Ok(out)
}

fn collect(
    source: &Utf8Path,
    tree: &cooklang_find::RecipeTree,
    prefix: String,
    out: &mut Vec<SearchEntry>,
) {
    for (name, child) in &tree.children {
        let sub = if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{prefix}/{name}")
        };
        if child.children.is_empty() {
            if let Some(ref recipe) = child.recipe {
                let url_path = if recipe.is_menu() {
                    format!("menu/{sub}.html")
                } else {
                    format!("recipe/{sub}.html")
                };
                let tags = recipe.tags();
                // Best-effort ingredient extraction; failures degrade silently.
                let ingredients = match crate::util::parse_recipe_from_entry(recipe, 1.0) {
                    Ok(parsed) => parsed
                        .group_ingredients(crate::util::PARSER.converter())
                        .into_iter()
                        .map(|e| e.ingredient.display_name().to_string())
                        .collect(),
                    Err(_) => Vec::new(),
                };
                out.push(SearchEntry {
                    title: name.clone(),
                    path: url_path,
                    tags,
                    ingredients,
                });
            }
        } else {
            collect(source, child, sub, out);
        }
    }
    let _ = source; // currently unused; reserved for future use
}
```

- [ ] **Step 2: Register module and write index in `run()`**

In `src/build/mod.rs`:

```rust
mod index;
```

After the recipe walk in `run()`:

```rust
let entries = index::build_search_index(&source, &tree)?;
let json = serde_json::to_string(&entries)?;
writer::write_bytes(&output, camino::Utf8Path::new("static/search-index.json"), json.as_bytes())?;
```

- [ ] **Step 3: Integration test**

Append to `tests/build.rs`:

```rust
#[test]
fn build_writes_search_index() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("_site");
    let seed = seed_dir();

    Command::cargo_bin("cook")
        .unwrap()
        .args([
            "build",
            out.to_str().unwrap(),
            "--base-path",
            seed.to_str().unwrap(),
        ])
        .assert()
        .success();

    let idx = out.join("static/search-index.json");
    assert!(idx.is_file(), "search-index.json should exist");

    let json: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(&idx).unwrap()).unwrap();
    let arr = json.as_array().expect("index is array");
    assert!(!arr.is_empty(), "index should not be empty for seed");

    let first = &arr[0];
    assert!(first.get("title").is_some());
    assert!(first.get("path").is_some());
    assert!(first.get("tags").is_some());
    assert!(first.get("ingredients").is_some());
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --test build`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(build): generate client-side search index JSON"
```

---

## Task 15: Add client-side search script

**Files:**
- Create: `static/js/search.js`

- [ ] **Step 1: Write `static/js/search.js`**

This script is loaded by `base.html` when `static_mode` is true. It fetches `search-index.json`, filters as the user types, and renders results into the existing `#search-results` element. It also rewires keyboard navigation to mirror the dynamic version.

```javascript
(function () {
  var prefix = window.__PREFIX__ || ".";
  var input = document.getElementById("search-input");
  var results = document.getElementById("search-results");
  if (!input || !results) return;

  var index = null;
  var selectedIndex = -1;

  function loadIndex() {
    if (index !== null) return Promise.resolve(index);
    return fetch(prefix + "/static/search-index.json")
      .then(function (r) { return r.json(); })
      .then(function (data) {
        index = data;
        return data;
      })
      .catch(function (e) {
        console.error("search-index load failed", e);
        index = [];
        return index;
      });
  }

  function score(entry, q) {
    var ql = q.toLowerCase();
    if (entry.title.toLowerCase().indexOf(ql) !== -1) return 3;
    if (entry.tags.some(function (t) { return t.toLowerCase().indexOf(ql) !== -1; })) return 2;
    if (entry.ingredients.some(function (i) { return i.toLowerCase().indexOf(ql) !== -1; })) return 1;
    return 0;
  }

  function render(matches) {
    if (matches.length === 0) {
      results.innerHTML = '<div class="p-4 text-gray-500 text-center">No recipes found</div>';
    } else {
      results.innerHTML = matches.map(function (m) {
        var href = prefix + "/" + m.path;
        return '<a href="' + href + '" class="search-result block px-4 py-3 hover:bg-gradient-to-r hover:from-purple-50 hover:to-pink-50 transition-colors border-b border-gray-100 last:border-b-0">' +
          '<div class="font-medium text-gray-800">' + escapeHtml(m.title) + '</div>' +
          '</a>';
      }).join("");
    }
    results.classList.remove("hidden");
  }

  function escapeHtml(s) {
    return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
  }

  function updateSearchSelection() {
    var items = results.querySelectorAll("a");
    items.forEach(function (item, i) {
      if (i === selectedIndex) {
        item.classList.add("search-selected");
        item.scrollIntoView({ block: "nearest" });
      } else {
        item.classList.remove("search-selected");
      }
    });
  }

  var timeout;
  input.addEventListener("input", function () {
    clearTimeout(timeout);
    var q = this.value.trim();
    selectedIndex = -1;
    if (q.length < 2) {
      results.classList.add("hidden");
      return;
    }
    timeout = setTimeout(function () {
      loadIndex().then(function (idx) {
        var matches = idx
          .map(function (e) { return { e: e, s: score(e, q) }; })
          .filter(function (x) { return x.s > 0; })
          .sort(function (a, b) { return b.s - a.s; })
          .slice(0, 20)
          .map(function (x) { return x.e; });
        render(matches);
      });
    }, 150);
  });

  input.addEventListener("keydown", function (e) {
    var items = results.querySelectorAll("a");
    if (items.length === 0 || results.classList.contains("hidden")) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, items.length - 1);
      updateSearchSelection();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, -1);
      updateSearchSelection();
    } else if (e.key === "Enter") {
      if (selectedIndex >= 0 && selectedIndex < items.length) {
        e.preventDefault();
        items[selectedIndex].click();
      }
    }
  });

  document.addEventListener("click", function (e) {
    if (!input.contains(e.target) && !results.contains(e.target)) {
      results.classList.add("hidden");
      selectedIndex = -1;
    }
  });
})();
```

- [ ] **Step 2: Verify it lands in the embedded assets**

Run:
```bash
cargo build
```

`RustEmbed` rebuilds the embed automatically when files in `static/` change.

- [ ] **Step 3: Integration test that search.js is in output**

Extend the existing `build_writes_index_and_static_assets` test (or add a new one):

```rust
#[test]
fn build_writes_search_js() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("_site");
    let seed = seed_dir();

    Command::cargo_bin("cook")
        .unwrap()
        .args([
            "build",
            out.to_str().unwrap(),
            "--base-path",
            seed.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(out.join("static/js/search.js").is_file(), "search.js should exist");
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --test build`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(build): add client-side search script"
```

---

## Task 16: Verify static mode strips dynamic UI strings

A safety-net test that proves the gating in templates is correctly disabled in static mode.

**Files:**
- Modify: `tests/build.rs`

- [ ] **Step 1: Write assertion**

Append to `tests/build.rs`:

```rust
#[test]
fn static_output_omits_dynamic_ui() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("_site");
    let seed = seed_dir();

    Command::cargo_bin("cook")
        .unwrap()
        .args([
            "build",
            out.to_str().unwrap(),
            "--base-path",
            seed.to_str().unwrap(),
        ])
        .assert()
        .success();

    let index = std::fs::read_to_string(out.join("index.html")).unwrap();

    // Nav links to dynamic-only pages should be gone.
    assert!(!index.contains("/shopping-list\""), "shopping-list nav present");
    assert!(!index.contains("/pantry\""), "pantry nav present");
    assert!(!index.contains("/preferences\""), "preferences nav present");

    // The dynamic search fetch should be gone; the static search.js link should be present.
    assert!(!index.contains("/api/search"), "api search reference remains");
    assert!(index.contains("/static/js/search.js"), "static search.js missing");
}
```

- [ ] **Step 2: Run test**

Run: `cargo test --test build static_output_omits_dynamic_ui`
Expected: PASS. If any assertion fails, return to Task 4/5/6 and find the un-gated element.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "test(build): verify static output omits dynamic UI"
```

---

## Task 17: Manual verification

Not a code task, but documents the manual smoke test for verification-before-completion.

- [ ] **Step 1: Run a clean build against `./seed`**

```bash
cargo run -- build --base-path ./seed /tmp/cook-static
```

Expected output (counts will vary):
```
Building static site from /…/seed into /tmp/cook-static
Wrote N recipes, N images, N static assets
```

- [ ] **Step 2: Browse via local web server**

```bash
python3 -m http.server -d /tmp/cook-static 8765
```

Open `http://localhost:8765/` in a browser. Confirm:
- Recipe listing renders
- Clicking a recipe loads its page
- Search box filters as you type
- No "Shopping List", "Pantry", or "Edit" controls visible
- Images render (if seed contains any)

- [ ] **Step 3: Browse via `file://`**

```bash
open /tmp/cook-static/index.html
```

Confirm: links navigate correctly within the static site, no broken relative paths.

- [ ] **Step 4: Verify formatting and lint**

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

All three must pass with no warnings.

- [ ] **Step 5: Final commit if any formatting changes were applied**

```bash
git add -A
git commit -m "chore: fmt"  # only if needed
```

---

## Self-Review

This plan was reviewed against the spec at `docs/superpowers/specs/2026-05-15-static-site-build-design.md`. Spec coverage:

- CLI `cook build [OUTPUT_DIR] [--base-path] [--base-url]` — Task 1, 2, 11.
- Output layout (`index.html`, `directory/<path>.html`, `recipe/<path>.html`, `menu/<path>.html`, `static/`, `api/static/`) — Tasks 11, 12, 13, 10.
- Module `src/build/` (renderer, writer, links, index) — Tasks 8, 9, 10, 11, 12, 14.
- Reuse Askama templates via `static_mode` flag — Tasks 3, 4, 5, 6.
- Refactor shared builders out of `ui.rs` — Task 7.
- Search index + `search.js` — Tasks 14, 15.
- Recipe images copied — Task 13.
- Relative URL prefix per page; `--base-url` override — Tasks 8, 11.
- Lenient parsing (warn + skip on bad recipes) — Task 12 (`walk_recipes` warns and continues).
- Smoke tests + manual verification — Tasks 11, 12, 13, 14, 15, 16, 17.

No placeholders, TBDs, or "implement later" steps remain. Method signatures (`render_index`, `render_directory`, `render_recipe`, `build_recipes_template`, `build_recipe_template`, `relative_prefix`, `write_html`, `copy_static_assets`, `copy_image`) are referenced consistently across tasks.
