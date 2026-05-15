# Static Site Build Command — Design

## Summary

A new top-level CLI command `cook build` that generates a self-contained static website from a Cooklang recipe collection. The output mirrors the existing web server UI (directory browsing, recipe pages, menus, search) but omits all dynamic, user-state features (shopping list, pantry, edit/new, preferences, sync, reload, scaling).

The static site can be hosted on any static-file host (GitHub Pages, Netlify, S3, etc.) or browsed directly from disk via `file://`.

## Goals

- Render the recipe collection as static HTML browsable without a server.
- Maximize reuse of the existing Askama templates and rendering code; do not duplicate the recipe rendering pipeline.
- Self-contained output: the chosen output directory is uploadable as-is.
- Works on `file://` and arbitrary host subpaths.

## Non-goals (v1)

- Recipe scaling in the static output. Quantities render as written.
- Shopping list, pantry, edit, new, preferences, sync, reload.
- Incremental / watch-mode builds.
- Themes or template customization.
- Pretty (extensionless) URLs.

## CLI

```
cook build [OUTPUT_DIR]
  [--base-path <DIR>]        # source recipes, defaults to current working dir / Context base_path
  [--base-url <PREFIX>]      # absolute URL prefix (e.g. /recipes/) for subpath hosting
```

- `OUTPUT_DIR` defaults to `./_site`.
- The output directory is created if missing. Existing contents are **not** wiped; the build writes files over existing ones. (User controls cleanup.)
- Aliases: none in v1.

## Output Layout

```
_site/
  index.html                          # root recipe listing
  directory/<path>.html               # one per non-empty subdirectory
  recipe/<path>.html                  # one per .cook file
  menu/<path>.html                    # one per menu
  static/
    css/output.css
    js/search.js
    search-index.json
    (other embedded static assets)
  api/static/<recipe-image-path>      # recipe images, mirrors source layout
```

`recipe/<path>.html` keeps the URL scheme close to the running server (`/recipe/...`), which means the existing `prefix`-based URL helpers in templates continue to work with a per-page relative prefix.

## Architecture

### New module: `src/build/`

Mirrors the layout of `src/server/`:

- `mod.rs` — `BuildArgs` (clap), `run(ctx, args) -> Result<()>`, top-level orchestrator.
- `renderer.rs` — wraps existing Askama template structs (`RecipesTemplate`, `RecipeTemplate`, `MenuTemplate`, etc.) with `static_mode: true` and computed per-page `prefix`.
- `writer.rs` — handles directory creation, file writes, copying embedded static assets, copying recipe images.
- `index.rs` — walks the recipe tree to build `search-index.json`.
- `links.rs` — computes per-page relative `prefix` strings; helper to map an output path to its `../`-walking root prefix.

### Template changes

Add `static_mode: bool` to every Askama template struct in `src/server/templates.rs`. Default remains `false`; the server continues to pass `false`, the build command passes `true`.

In templates (`templates/*.html`):
- Gate the following behind `{% if !static_mode %}`:
  - Shopping-list nav link, add-to-shopping-list buttons, "in pantry" badges
  - Edit / New / Delete controls
  - Preferences nav link
  - Sync UI (login/logout/status)
  - Reload button
  - Recipe scaling input
- Search box stays visible. Its JS handler points to `static/search.js` (client-side filter over `search-index.json`) in static mode, or `/api/search` in server mode.
- URL helpers that produce `/recipe/<path>` etc. honor `static_mode` by appending `.html`.

### Link strategy

Existing templates already generate URLs using a `prefix` value. The build renderer computes `prefix` per page so all internal links are relative and work on `file://` and any host subpath:

- `_site/index.html` → `prefix = "."`
- `_site/directory/Breakfast.html` → `prefix = ".."`
- `_site/recipe/Breakfast/Pancakes.html` → `prefix = "../.."`

When `--base-url` is provided, the build uses it as an absolute prefix instead of computing relative paths. This is useful for known subpath hosting (e.g. `/recipes/`).

### Search

Build-time generation of `static/search-index.json`. Each entry:

```json
{
  "title": "Pancakes",
  "path": "recipe/Breakfast/Pancakes.html",
  "tags": ["breakfast", "quick"],
  "ingredients": ["flour", "milk", "egg"]
}
```

A small `static/js/search.js` (new, shipped alongside other embedded static assets) reads the JSON, filters as the user types, and renders results into the existing search dropdown markup. No external libraries.

### Assets

- Embedded static assets (CSS, JS, images under `static/`) are written verbatim to `_site/static/`. Uses the existing `RustEmbed` `StaticFiles` source.
- Recipe images are discovered the same way the server does (matched by stem alongside `.cook` files). They are copied into `_site/api/static/<same-relative-path>` so the templates' existing image URL logic continues to resolve correctly when given the per-page `prefix`.

## Data Flow (Build Pipeline)

A single pass:

1. **Resolve paths**: `source = args.base_path || ctx.base_path`, `output = args.output_dir || "./_site"`. Make `output` absolute. Create if missing. Bail if `source` is not a directory.
2. **Build recipe tree**: `cooklang_find::build_tree(source)`.
3. **Build search index** by walking the tree.
4. **Render pages**:
   - `index.html` from the root tree.
   - One `directory/<path>.html` per non-empty subdirectory.
   - One `recipe/<path>.html` per `.cook` file.
   - One `menu/<path>.html` per menu file.
5. **Copy assets**:
   - Embedded `StaticFiles` → `_site/static/`.
   - Discovered recipe images → `_site/api/static/<path>`.
   - Write `search-index.json` and `search.js` to `_site/static/`.
6. **Print summary**: counts of pages and assets written, output path.

## Error Handling

- Fatal (return `Err`, exit non-zero):
  - Source directory does not exist or is a file.
  - Output directory cannot be created or written to.
  - Failure to write a critical static asset.
- Non-fatal (log via `tracing::warn!`, skip, continue):
  - A `.cook` file fails to parse.
  - A menu fails to resolve.
  - An image referenced by a recipe is missing.

Matches the server's lenient philosophy: one bad recipe should not break the whole build.

## Testing

### Automated (`cargo test`)

A smoke test that runs `cook build` against `./seed` recipes into a `tempfile::TempDir`:

- Assert these files exist: `index.html`, at least one `recipe/*.html`, `static/css/output.css`, `static/js/search.js`, `static/search-index.json`.
- Parse one recipe HTML and assert it contains the recipe title.
- Assert no dynamic-mode strings appear (e.g., "Add to shopping list", "Edit recipe").
- Parse `search-index.json` and assert it has entries with the expected schema.

### Manual

Documented as part of the command's help text or CLAUDE.md:

```bash
cook build ./seed _site
python3 -m http.server -d _site 8000
# browse http://localhost:8000

# or directly via file://
open _site/index.html
```

### Regression safety

The existing Playwright suite covers the dynamic server. The `static_mode` flag defaults to `false`, so the server's behavior is unchanged.

## Open Questions / Future Work

- Theme/customization: deferred. v1 ships with the existing Tailwind look.
- Watch / incremental builds: deferred. Users can rerun `cook build`.
- Pretty URLs: deferred. `.html` extensions are universal.
- Pre-rendered scaling variants: deferred. Scaling is dropped in v1 entirely.
- Sitemap/RSS feed: deferred.
- Adding a download link to source `.cook` file from each recipe page: deferred.
