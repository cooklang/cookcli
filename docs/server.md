# Server Command

Start a local web server to browse and view your recipe collection.

<img width="600" alt="recipes" src="screenshots/recipe-list.png" />
<img width="600" alt="recipe" src="screenshots/recipe-detail.png" />
<img width="600" alt="shopping list" src="screenshots/shopping-list.png" />
<img width="600" alt="pantry" src="screenshots/pantry.png" />

## Usage

```
cook server [OPTIONS] [BASE_PATH]
```

## Arguments

| Argument | Description |
|----------|-------------|
| `[BASE_PATH]` | Root directory containing recipe files (default: current directory) |

## Options

| Option | Description |
|--------|-------------|
| `--host [<ADDRESS>]` | Allow connections from external hosts (default: localhost only). Optionally bind to a specific address. |
| `-p, --port <PORT>` | Port number (default: 9080) |
| `--open` | Automatically open the web interface in your default browser |
| `--cors-origin <ORIGIN>` | Origin allowed to make cross-origin browser requests. Repeatable. `*` for any origin (default). |
| `--cors-allow-credentials` | Allow cross-origin requests to carry cookies and credentials. Requires an explicit `--cors-origin`. |
| `--no-csrf-check` | Disable same-origin enforcement on requests that modify recipes. |
| `--max-lsp-sessions <N>` | Language server sessions to run at once (default: 8). `0` disables the editor's language server. |

## Examples

```bash
# Start on localhost:9080
cook server

# Serve recipes from a specific directory
cook server ~/my-recipes

# Custom port with auto-open
cook server --port 8080 --open

# Allow access from other devices on the network
cook server --host

# Let a frontend at localhost:3000 use the full API, including writes
cook server --cors-origin http://localhost:3000

# Behind a reverse proxy, name the public origin so the UI can still write
cook server --cors-origin https://cook.example.com
```

## Notes

- By default, only accepts connections from localhost
- Use `--host` on trusted networks only — recipes become accessible to anyone on the network
- Cross-origin browser requests can read (`GET`) from any origin by default, but one that would modify recipes is refused with `403`. Naming origins with `--cors-origin` lets those origins write too, so a page you have not listed cannot change your recipes. Requests with no `Origin` header — `curl`, scripts, anything that is not a browser — are unaffected. See [the API reference](api.md).
- Behind a reverse proxy that rewrites `Host`, pass `--cors-origin` with the public origin (for example `--cors-origin https://cook.example.com`). The same-origin check reads the real `Host` header and ignores `X-Forwarded-Host`, which any client can set freely.
- `--no-csrf-check` turns that same-origin enforcement off entirely, for both the API and the web UI's new-recipe form. Its former spelling, `--no-cors`, still works.
- The built-in editor gets its diagnostics and completions from a `cook lsp` subprocess, one per open edit tab, and the endpoint that starts them has no authentication. `--max-lsp-sessions` caps how many run at once (8 by default) so that a client which is not that editor cannot spawn them without bound; beyond the cap the websocket handshake is refused with `503` and the editor retries. Under `--host`, consider `--max-lsp-sessions 0`, which serves the recipes but never starts a subprocess for a remote client.
- The web interface supports recipe browsing, scaling, search, and shopping list management
- The UI language is negotiated per request from the browser's `Accept-Language` header — each visitor sees the interface in their own language (supported: `en-US`, `de-DE`, `nl-NL`, `fr-FR`, `es-ES`, `eu-ES`, `sv-SE`). For static sites, see the `--lang` flag of [`cook build web`](build.md#localization).
- Mobile-friendly responsive layout

## Web feeds

The server publishes an Atom feed at `/atom.xml` and an RSS 2.0 feed at `/rss.xml`, with one item per recipe and menu, newest first. They are built from the recipe files on each request, so they are always up to date. Every page advertises them with `<link rel="alternate">` tags, so a feed reader finds them from the site's address alone.

Items use the same metadata as the static site's feeds (`title`, `date`, `description`, `author`, `tags`); see [Web feeds](build.md#web-feeds). The feed title and language follow the request's `Accept-Language` header.

Feed links are absolute. They are built from the request's `Host` header and `--url-prefix`. Behind a TLS-terminating reverse proxy, send `X-Forwarded-Proto: https` to get `https://` links. As with the same-origin check, `X-Forwarded-Host` is ignored, so the proxy must pass the public `Host` through.
