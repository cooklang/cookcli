# Build Command

Generate a self-contained static website from your recipe collection. The output mirrors `cook server`'s browsing experience but ships as plain HTML, CSS, and JS — no Rust process needed at runtime, so it can be hosted on GitHub Pages, Netlify, S3, or opened directly via `file://`.

## Usage

```
cook build [OPTIONS] [OUTPUT_DIR]
```

## Arguments

| Argument | Description |
|----------|-------------|
| `[OUTPUT_DIR]` | Directory to write the generated site into (default: `./_site`). Created if missing; existing files are overwritten as needed. |

## Options

| Option | Description |
|--------|-------------|
| `--base-path <PATH>` | Root directory containing recipe files (default: current directory) |
| `--base-url <URL>` | Absolute URL prefix for hosting under a subpath (e.g. `/recipes/`). When unset, links are page-relative and the site works under any prefix, including `file://`. |

## Examples

```bash
# Build into ./_site from the current directory
cook build

# Build a specific recipe collection into a custom output directory
cook build dist --base-path ~/my-recipes

# Build for hosting under /recipes/ on your domain
cook build --base-url /recipes/
```

## What gets generated

| Output | Contents |
|--------|----------|
| `index.html` | Root recipe listing |
| `directory/<path>.html` | One listing page per subdirectory |
| `recipe/<path>.html` | One page per `.cook` recipe (URL uses the file stem, not the title metadata) |
| `menu/<path>.html` | One page per `.menu` file |
| `api/static/<path>` | Images alongside recipes (`.jpg`, `.jpeg`, `.png`, `.gif`, `.webp`, `.avif`) |
| `static/css/`, `static/js/` | Compiled CSS, fonts, icons, and the client-side search script |
| `static/search-index.json` | Search index consumed by `static/js/search.js` |

## What's excluded

The static site is read-only. The following dynamic features from `cook server` are intentionally omitted:

- Shopping list and pantry pages
- Preferences and sync
- Recipe editor and "New recipe" button
- Recipe scaling controls (output is always 1×)
- "Add to shopping list" buttons
- Server-side search API (`/api/search`) — replaced by a client-side index

The keyboard-shortcuts modal also hides entries for the removed features so the help is accurate for what's actually available.

## Hosting

Because internal links default to page-relative paths, no configuration is needed for most hosts:

```bash
# GitHub Pages: push _site/ to gh-pages
cook build && git -C _site init && git -C _site add . && \
  git -C _site commit -m "site" && \
  git -C _site push -f git@github.com:user/repo gh-pages

# Netlify drop: drag and drop _site/ into the Netlify UI

# Static S3 bucket
aws s3 sync _site/ s3://my-recipes-bucket --delete

# Just open it locally
open _site/index.html
```

Use `--base-url` only if your host serves the site under a fixed subpath and you cannot rely on relative URLs.

## Notes

- The generated site has no server dependency — it works fully offline via `file://`.
- Search runs entirely in the browser by loading `static/search-index.json`.
- Re-run `cook build` after editing recipes; the command is idempotent.
- For a live editing experience, use `cook server` instead.
