# Build Command

The `cook build` command groups artifact-generation subcommands. Today it offers `web` for static-site generation; future targets (e.g. cookbooks) will live alongside it.

## `cook build web`

Generate a self-contained static website from your recipe collection. The output mirrors `cook server`'s browsing experience but ships as plain HTML, CSS, and JS — no Rust process needed at runtime, so it can be hosted on GitHub Pages, Netlify, S3, or opened directly via `file://`.

## Usage

```
cook build web [OPTIONS] [OUTPUT_DIR]
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
| `--lang <LANG>` | UI language for the generated site (default: system locale, falling back to `en-US`). See [Localization](#localization). |
| `--sitemap <URL>` | Full base URL of the deployed site (e.g. `https://recipes.example.com`). When set, writes a `sitemap.xml` at the output root listing every page with absolute URLs. |
| `--repo-url <URL>` | URL of the recipe repository. When set, the footer's "Built with CookCLI" line gains a "View source" link pointing here. |
| `--compress` | Also write gzip-compressed copies (`.gz`) of generated text assets for hosts that serve precompressed files (e.g. GitLab Pages). Images are skipped. |

## Examples

```bash
# Build into ./_site from the current directory
cook build web

# Build a specific recipe collection into a custom output directory
cook build web dist --base-path ~/my-recipes

# Build for hosting under /recipes/ on your domain
cook build web --base-url /recipes/

# Link back to the recipe repository from the footer
cook build web --repo-url https://github.com/user/my-recipes

# Render the site in French
cook build web --lang fr-FR

# Write .gz siblings for precompressed hosting (e.g. GitLab Pages)
cook build web --compress
```

## Localization

`cook server` and `cook build web` are both localized, but they pick the language differently:

- **`cook server`** negotiates the language per request from the browser's `Accept-Language` header, so each visitor sees the UI in their own language automatically.
- **`cook build web`** produces static HTML, so there is no request to negotiate against — the whole site is rendered in a single language chosen at build time. It defaults to your system locale (falling back to `en-US`) and can be set explicitly with `--lang`:

  ```bash
  cook build web --lang fr-FR
  ```

Supported languages: `en-US`, `de-DE`, `nl-NL`, `fr-FR`, `es-ES`, `eu-ES`, `sv-SE`. Bare language codes work too (`--lang fr`).

Note that only the UI chrome (navigation, headings, labels) is translated — your recipe content is rendered as written.

## What gets generated

| Output | Contents |
|--------|----------|
| `index.html` | Root recipe listing |
| `directory/<path>.html` | One listing page per subdirectory |
| `recipe/<path>.html` | One page per `.cook` recipe (URL uses the file stem, not the title metadata) |
| `recipe/<path>.cook` | Raw `.cook` source for each recipe — exposed as a download link on the recipe page |
| `menu/<path>.html` | One page per `.menu` file |
| `api/static/<path>` | Images alongside recipes (`.jpg`, `.jpeg`, `.png`, `.gif`, `.webp`, `.avif`) |
| `static/css/`, `static/js/` | Compiled CSS, fonts, icons, and the client-side search script |
| `static/search-index.js` | Search index consumed by `static/js/search.js`, as a script assigning `window.__SEARCH_INDEX__` |

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
cook build web && git -C _site init && git -C _site add . && \
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

- The generated site has no server dependency: it works fully offline via `file://`.
  Not with `--base-url` though. That flag makes every asset reference absolute, so from
  disk the page gets no stylesheet, no icons and no search. Use it only for HTTP hosting.
- Search runs entirely in the browser by loading `static/search-index.js`. It is a script
  rather than JSON so that search also works over `file://`, where browsers block
  `fetch()` between local files.
- Re-run `cook build web` after editing recipes; the command is idempotent.
- For a live editing experience, use `cook server` instead.
