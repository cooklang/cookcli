# Server Command

Start a local web server to browse and view your recipe collection.

<img width="600" alt="recipes" src="screenshots/recipe-list.png" />
<img width="600" alt="recipe" src="screenshots/recipe-detail.png" />
<img width="600" alt="shopping list" src="screenshots/shopping-list.png" />
<img width="600" alt="pantry" src="screenshots/pantry.png" />

## Usage

```
cook server [OPTIONS] [BASE_PATH]
cook server user <add|passwd|remove|list> [NAME] [--users-file <PATH>]
cook server hash-password
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
| `--users-file <PATH>` | Users who may sign in to make changes. Defaults to `COOK_USERS_FILE`, then `users.toml` in the configuration directory. See [Signing in to make changes](#signing-in-to-make-changes). |

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

# Require sign-in before anyone can change recipes
cook server user add alice
cook server --host
```

## Notes

- By default, only accepts connections from localhost
- Use `--host` on trusted networks only — recipes become accessible to anyone on the network, and without [users](#signing-in-to-make-changes) anyone there can change them
- Cross-origin browser requests can read (`GET`) from any origin by default, but one that would modify recipes is refused with `403`. Naming origins with `--cors-origin` lets those origins write too, so a page you have not listed cannot change your recipes. Requests with no `Origin` header — `curl`, scripts, anything that is not a browser — are unaffected. See [the API reference](api.md).
- Behind a reverse proxy that rewrites `Host`, pass `--cors-origin` with the public origin (for example `--cors-origin https://cook.example.com`). The same-origin check reads the real `Host` header and ignores `X-Forwarded-Host`, which any client can set freely.
- Without more flags, the web UI can only modify recipes when it is opened at `localhost` or an IP address, such as `http://127.0.0.1:9080` or `http://192.168.1.20:9080`. Opened at any other host name — `http://nas.local:9080`, or a reverse proxy's `https://cook.example.com` even when the proxy passes `Host` through — its writes are refused until that origin is named with `--cors-origin`. Otherwise any website could point a domain of its own at your server (DNS rebinding) and pass for the web UI. The `403` and the server's log name the exact flag to add; only add origins you recognise.
- `--no-csrf-check` turns that same-origin enforcement off entirely, for both the API and the web UI's new-recipe form. Its former spelling, `--no-cors`, still works.
- The built-in editor gets its diagnostics and completions from a `cook lsp` subprocess, one per open edit tab, and the endpoint that starts them has no authentication unless [sign-in](#signing-in-to-make-changes) is on. `--max-lsp-sessions` caps how many run at once (8 by default) so that a client which is not that editor cannot spawn them without bound; beyond the cap the websocket handshake is refused with `503` and the editor retries. Under `--host`, consider `--max-lsp-sessions 0`, which serves the recipes but never starts a subprocess for a remote client.
- The web interface supports recipe browsing, scaling, search, editing, and shopping list management
- The UI language is negotiated per request from the browser's `Accept-Language` header — each visitor sees the interface in their own language (supported: `en-US`, `de-DE`, `nl-NL`, `fr-FR`, `es-ES`, `eu-ES`, `sv-SE`). For static sites, see the `--lang` flag of [`cook build web`](build.md#localization).
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

## Signing in to make changes

Out of the box the server is open: anyone who can reach it can change your
recipes. Add a user, and it asks for sign-in before any change:

```bash
cook server user add alice   # asks for alice's password, twice
cook server
```

Once the server has users:

- Anyone can still browse recipes and menus, search, and look at the shopping
  list and the pantry.
- Creating, editing and deleting recipes, and changing the pantry or the
  shopping list, need a signed-in user. Guests see a **Sign in** link at the
  top of every page instead of the controls that change things.
- The editor's language server (`/api/ws/lsp`) and the CookCloud sync
  controls are for signed-in users only.
- An API request that would change something, sent without a session, gets
  `401`. [The API reference](api.md) shows how a script signs in.

Users live on the server only: nobody can sign up or change a password from
the browser.

### Managing users

| Command | What it does |
|---------|--------------|
| `cook server user add <name>` | Add a user, asking for their password. Creates the users file if needed. |
| `cook server user passwd <name>` | Change a user's password. |
| `cook server user remove <name>` | Remove a user. |
| `cook server user list` | List the users. |
| `cook server hash-password` | Print a password hash, for editing the users file by hand. |

A username may use letters, digits and `_ . @ -`. The password prompt does not
echo what you type. When standard input is not a terminal, the password is
read from its first line instead, for scripts:

```bash
printf '%s\n' "$PASSWORD" | cook server user add alice
```

`user` has to come straight after `server`: in
`cook server --port 8080 user add alice`, `user` is read as the recipe
directory. Pass `--users-file` after the subcommand when you need it.

A running server picks up changes to the users file on its own. A new user can
sign in right away, and removing a user or changing their password signs them
out everywhere. The one exception is the first user: sign-in turns on when the
server *starts* with a users file, so restart it once after creating one.

### The users file

`cook server` reads `users.toml` from the global configuration directory (see
[Configuration](../README.md#️-configuration)), unless `--users-file <PATH>`
or the `COOK_USERS_FILE` environment variable names another file. The flag
wins, and an empty variable counts as unset. The `user` commands edit the same
file and keep any comments in it.

```toml
# Who may change recipes on this server.
[users]
alice = "$argon2id$v=19$m=19456,t=2,p=1$…"
```

- Sign-in is on whenever this file exists when the server starts. Delete it
  and restart to open the server to everyone again.
- An empty `[users]` table keeps sign-in on with nobody able to sign in, which
  makes the site read-only.
- The server will not start with a users file it cannot use — one it cannot
  read, a malformed entry, a file named with `--users-file` that does not
  exist — rather than fall back to letting everyone in. While it runs, an edit
  that breaks the file is ignored: the server logs an error and keeps the
  users it had.
- The server refuses a users file inside the recipe directory, because it
  publishes that directory's files at `/api/static/`.

### Sessions and HTTPS

Signing in sets a `cook_session` cookie that lasts 30 days. The server signs it
with a key it keeps in `auth-secret`, next to `users.toml` in the
configuration directory. Delete that file and restart to sign everyone out.
Signing out clears the cookie in that browser only. If the configuration
directory cannot be written, the server uses a temporary key, and everyone has
to sign in again after a restart.

Over plain HTTP, passwords and cookies cross the network in the clear. Before
exposing the server beyond a network you trust, put it behind a reverse proxy
that serves HTTPS, and have the proxy send `X-Forwarded-Proto: https` so the
cookie is marked `Secure`.

In a container, give the server a configuration directory it can write to;
see [Sign-in in a container](../README.md#sign-in-in-a-container).

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
