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
- The web interface supports recipe browsing, scaling, search, and shopping list management
- The UI language is negotiated per request from the browser's `Accept-Language` header — each visitor sees the interface in their own language (supported: `en-US`, `de-DE`, `nl-NL`, `fr-FR`, `es-ES`, `eu-ES`, `sv-SE`). For static sites, see the `--lang` flag of [`cook build web`](build.md#localization).
- Mobile-friendly responsive layout

## Custom Metadata Families (e.g. Nutrition)

Beyond the [standard Cooklang metadata keys](https://cooklang.org/docs/spec/#canonical-metadata) (`servings`, `time`, `course`, `author`, ...), any YAML frontmatter key whose value is a **list** or a **mapping** is shown on the recipe page as its own line below the tags, grouped by key — one line per family. This works for any such key, not just `nutrition` — e.g. `allergens:` below renders as its own "Allergens" line automatically, with no code changes required.

```yaml
---
tags:
  - vegan
  - gluten-free
nutrition:
  kcal: 258
  proteins: 4.2
  lipids: 4.8
  sugars: 39.4
  fibers: 5.3
file:
  created-by: Yannick
  created-at: 2026-08-20
  modified-by: Yannick
  modified-at: 2026-08-24
allergens:
  - gluten
  - tree nuts
---
```

### Two forms

- **Mapping** (recommended, shown above): `field: value`. A bare number gets its unit inferred from the field name for `nutrition` (`kcal` → `kcal`, everything else → `g`); `field: "45.3%g"` also works if you want to spell out a different unit.
- **List** (legacy, still supported): `- "258%kcal"`. The `%` is replaced with a space when displayed (`258 kcal`); entries without a `%` are shown as-is. Because list entries are free text, they aren't translated — write them in whichever language you want displayed.

### Hiding a family or a single entry

Prefix a key with `.` to hide it from the recipe page: `.internal-notes:` hides the whole family, and `.lipids:` (inside `nutrition:`) hides just that one entry while the rest of the family still shows. `tags` is never treated as a custom family — it keeps its own dedicated row above, and can't be hidden this way.

### Specific renderers: `nutrition` and `file`/`meta`

Two families get dedicated icons and (for `nutrition`) localized labels, matched on their fields:

- **`nutrition`**: `kcal`/`cal`/`energy` (flame), `proteins` (meat), `lipids`/`fat` (droplet), `saturated-fat` (filled droplet), `carbohydrates`/`carbs` (bread), `sugars` (candy), `fibers`/`fibre` (wheat), `salt`/`sodium` (salt shaker). The nutrient name (everything but `kcal`, which needs none) is translated into the viewer's UI language — see [Localization](build.md#localization) for the supported locales.
- **`file`** (or `meta`, both work): `created-by`/`created-at`/`modified-by`/`modified-at` — person, calendar, pencil, and history icons respectively, with a translated `"Label: value"` line (e.g. `"Modified at: 2026-08-24"`, `"Modifié le : 2026-08-24"` in French, with the French space before `:`).

Every other family (like `allergens` above) falls back to a generic rendering: list entries as-authored, mapping entries as `"field: value"`, no icon. Adding a third specific renderer means adding a case in `src/web/family_renderers/mod.rs`'s `renderer_for` plus a small renderer file next to `nutrition.rs`/`file.rs` — there's no filename-based auto-discovery (Rust has no runtime filesystem scanning for this), so that match statement is always the definitive list of which families get special treatment.

### Recipe list page

Only the calorie entry from `nutrition` (`kcal`) is shown, as a compact badge next to the tags — the other custom families are only shown on the recipe detail page, to keep list cards compact.
