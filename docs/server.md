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
| `--no-csrf-check` | Disable same-origin enforcement on requests that modify recipes and on the editor's language server connection. |
| `--max-lsp-sessions <N>` | Language server sessions to run at once (default: 8). `0` disables the editor's language server. |

## Environment

| Variable | Description |
|----------|-------------|
| `COOK_CORS_ORIGIN` | Origins allowed to make cross-origin browser requests, separated by commas. Same values as `--cors-origin`, which overrides it. For containers, where passing a flag means restating the image's whole command. An empty value means "unset". |
| `COOK_CONFIG_DIR` | Global configuration directory, holding `aisle.conf`, `pantry.conf`, the cook.md session and the sync database. See [the README](../README.md#cook_config_dir). |

No other option can be set this way. `--no-csrf-check` in particular has to be passed on the command line.

## Examples

```bash
# Start on localhost:9080
cook server

# Serve recipes from a specific directory
cook server ~/my-recipes

# Custom port with auto-open
cook server --port 8080 --open

# Allow access from other devices on the network, which reach it by IP address
# (http://192.168.1.20:9080) and can both read and save
cook server --host

# Only for devices that open the web UI by host name instead: name that origin,
# or its pages can read but not save
cook server --host --cors-origin http://nas.local:9080

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
- Without more flags, the web UI can only modify recipes when it is opened at `localhost` or an IP address, such as `http://127.0.0.1:9080` or `http://192.168.1.20:9080`. Opened at any other host name — `http://nas.local:9080`, or a reverse proxy's `https://cook.example.com` even when the proxy passes `Host` through — its writes are refused until that origin is named with `--cors-origin`. Otherwise any website could point a domain of its own at your server (DNS rebinding) and pass for the web UI. The `403` and the server's log name the exact flag to add; only add origins you recognise.
- The recipe editor talks to its language server over a websocket, which browsers exempt from CORS, so the server checks that connection's `Origin` itself, by the same rule: only its own page at `localhost` or an IP address, or a `--cors-origin`, may open it. Under any other host name the editor's completions and diagnostics stop until you name that origin. Whatever a client asks for, the language server only ever sees the directory being served.
- `--no-csrf-check` turns that same-origin enforcement off entirely, for the API, the web UI's new-recipe form and the editor's language server. Its former spelling, `--no-cors`, still works.
- The built-in editor gets its diagnostics and completions from a `cook lsp` subprocess, one per open edit tab, and the endpoint that starts them has no authentication. `--max-lsp-sessions` caps how many run at once (8 by default) so that a client which is not that editor cannot spawn them without bound; beyond the cap the websocket handshake is refused with `503` and the editor retries. Under `--host`, consider `--max-lsp-sessions 0`, which serves the recipes but never starts a subprocess for a remote client.
- The web interface supports recipe browsing, scaling, search, editing, and shopping list management
- The UI language is negotiated per request from the browser's `Accept-Language` header — each visitor sees the interface in their own language (supported: `en-US`, `de-DE`, `nl-NL`, `fr-FR`, `es-ES`, `eu-ES`, `sv-SE`, `it-IT`). For static sites, see the `--lang` flag of [`cook build web`](build.md#localization).
- Mobile-friendly responsive layout

## Containers

The published image runs `cook server /recipes --host`, so nothing extra is needed to open the web UI at `http://localhost:9080`, or at the host's address on the network — both read and save. Reaching it by host name instead, directly or through a reverse proxy, means naming that origin, and `COOK_CORS_ORIGIN` does it without restating the image's command:

```yaml
services:
  cookcli:
    image: ghcr.io/cooklang/cookcli:latest
    ports:
      - "9080:9080"
    volumes:
      - ./recipes:/recipes
    environment:
      COOK_CORS_ORIGIN: https://cook.example.com
```

Name the origin the browser shows, so `https://` when the proxy terminates TLS, and separate several with commas. A container that calls the API from another container sends no `Origin` header and needs none of this.

## Web feeds

The server publishes an Atom feed at `/atom.xml` and an RSS 2.0 feed at `/rss.xml`, with one item per recipe and menu, newest first. They are built from the recipe files on each request, so they are always up to date. Every page advertises them with `<link rel="alternate">` tags, so a feed reader finds them from the site's address alone.

Items use the same metadata as the static site's feeds (`title`, `date`, `description`, `author`, `tags`); see [Web feeds](build.md#web-feeds). The feed title and language follow the request's `Accept-Language` header.

Feed links are absolute. They are built from the request's `Host` header and `--url-prefix`. Behind a TLS-terminating reverse proxy, send `X-Forwarded-Proto: https` to get `https://` links. As with the same-origin check, `X-Forwarded-Host` is ignored, so the proxy must pass the public `Host` through.

## Title pictures

The recipe editor's **Picture** button adds, replaces or removes a recipe's title picture — the `Recipe.jpg` next to `Recipe.cook` that the recipe page and the recipe list show — without touching the server's files directly. Choose a file or drop one on the dialog.

- JPEG, PNG and WebP are accepted. The browser scales a photo down before sending it, so a phone photo of several megabytes goes up as a few hundred kilobytes; the server takes at most 10 MB.
- Every picture is saved as `Recipe.jpg`: turned upright from the photo's orientation data, scaled down to 2048 px on its longer edge, laid over white where it is transparent, and always re-encoded on the server — never stored as sent — so a malformed or doctored file cannot reach the recipe folder.
- Re-encoding drops the photo's metadata, including its GPS location.
- An older `Recipe.jpeg`, `Recipe.png` or `Recipe.webp` is removed when a new picture is saved. Step pictures (`Recipe.1.jpg`) are never touched.
- HEIC and AVIF photos cannot be read by the server. An iPhone's own browser converts a HEIC photo to JPEG as it uploads it, and so does Safari on a Mac; from another browser, export the photo as JPEG first, or set the iPhone camera to **Most Compatible** (Settings › Camera › Formats).
- A recipe whose metadata names a picture (`image:` in its frontmatter) shows that one instead. The dialog says so; remove the line to use an uploaded picture.

The dialog uses `/api/recipe_image/{*path}`; see [the API reference](api.md).
