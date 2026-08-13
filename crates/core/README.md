# cookcli-core

The logic behind CookCLI's commands, as a library.

`cook recipe`, `cook shopping-list`, `cook search`, `cook doctor`, `cook pantry`
and `cook report` are all thin argument-parsing and output-formatting shells
over this crate. It exists so that the other things that want to do those jobs
— the Cooklang editor through its NAPI-RS addon, CookBot — can call the same
code instead of maintaining a parallel reimplementation that drifts.

## The shape

Almost everything public has the same signature:

```text
fn(&Context, Request) -> Result<Outcome<T>, CoreError>
```

- **`Context`** is the configuration bundle: a base path plus the aisle and
  pantry configuration. [`Context::new`] touches nothing;
  [`Context::discover`] is the opt-in that reproduces CookCLI's search order
  (`<base>/config/<name>`, then the platform configuration directory). A caller
  that already knows its configuration never has the user's `~/.config` read
  behind its back.
- **`Request`** is a plain struct with public fields, one per command. Adding
  an option to a command is adding a field, not a new function.
- **`Outcome<T>`** is the result *plus* the diagnostics raised on the way —
  parse warnings, an aisle file that would not parse, a missing configuration.
  The CLI used to log those and drop them; here they come back to the caller,
  each attributed to the file it came from, so an editor can put a squiggle
  under the right line.
- **`CoreError`** is the failure case: no result could be produced at all. It
  renders as a single lowercase line, library-style, with the long-form parse
  report kept in a field for callers that want to print it verbatim.

## Paths or text

Recipes come in as a `RecipeSource` and configuration as a `ConfigSource`, and
both have an in-memory variant:

```text
RecipeSource::Path(path)              ConfigSource::Path(path)
RecipeSource::Content { text, name }  ConfigSource::Inline(text)
                                      ConfigSource::None
```

That is the case a path-only API cannot serve: an editor rendering the buffer
the user is typing into, before it has ever been saved.

## Example

```rust
use cookcli_core::{recipe, Context, CoreError, RecipeSource};

/// Read an unsaved buffer at double scale, and report what the parser thought
/// of it.
fn preview(text: &str) -> Result<String, CoreError> {
    // `new`, not `discover`: nothing here reads aisle or pantry configuration,
    // so there is no reason to go looking for any.
    let ctx = Context::new("/recipes".into());

    let outcome = recipe::read(
        &ctx,
        recipe::ReadRequest {
            source: RecipeSource::Content {
                text: text.to_string(),
                name: "unsaved buffer".to_string(),
            },
            scale: 2.0,
        },
    )?;

    for diagnostic in &outcome.diagnostics {
        eprintln!("{:?}: {}", diagnostic.severity, diagnostic.message);
    }

    Ok(outcome.value.title)
}

let buffer = "---\ntitle: Leek Soup\n---\n\nSlice the @leek{2}.\n";
assert_eq!(preview(buffer).unwrap(), "Leek Soup");
```

## Consumer coverage

The editor's NAPI addon (`packages/cooklang-native`) is the first consumer, and
this crate's API was checked against its real exported surface rather than
designed against one imagined caller. Where each in-scope export lands:

| Editor export | `cookcli-core` |
| --- | --- |
| `parse(input)` | `parse_recipe(text, name, scale)` — the `Outcome` carries the warnings, and errors arrive as `CoreError::Parse` with the same diagnostics attached, so the addon's `{ recipe, errors, warnings }` shape is reconstructible. See the two notes below. |
| `generateShoppingList(recipesJson, aisleConf?, pantryConf?)` | **Not expressible today.** `shopping_list::generate` takes `ScaledRecipe { name, scale }`, resolved against `Context::base_path` through `cooklang-find`; there is no way to hand it recipe *text*. The aisle and pantry halves are fine (`ConfigSource::Inline`) — it is the recipes that cannot come from a buffer. See "Known gap" below. |
| `findRecipe(baseDir, name)` | `find::get_recipe(base_path, name)`, then `RecipeEntry::content()`. `RecipeEntry` is re-exported, so a consumer needs no `cooklang-find` dependency of its own. |
| `renderReport(recipe, template, configJson)` | `report::render(&ctx, RenderRequest { source, template, scale, datastore, base_path })`. Takes the template as text rather than a path, so a buffer works; the aisle and pantry paths in the addon's config map onto `Context::aisle` / `Context::pantry`. |
| `parseShoppingList` / `writeShoppingList` / `parseChecked` / `writeCheckEntry` / `checkedSet` / `compactChecked` | Pure text transforms over `cooklang::shopping_list::{parse, write, parse_checked, write_check_entry, checked_set, compact_checked_log}`, reachable through this crate's `cooklang` re-export. `shopping_list::ShoppingListStore` is the file-backed superset — the `.shopping-list` / `.shopping-checked` pair beside a recipe collection — which is what CookCLI and the web server use. |

Out of scope, and deliberately absent: `parseMenu` (menu handling has not been
extracted), `startSync` / `stopSync` / `getSyncStatus` /
`onSyncStatusChanged` (the sync client is its own crate and pulls in tokio and
reqwest, which this crate has no business depending on), and `LspServer`
(`cooklang-language-server`).

### Known gap

`generateShoppingList` cannot be expressed. Closing it means letting
`ScaledRecipe` name a `RecipeSource` rather than a `String`, so that
`generate` and `extract_ingredients` can aggregate recipe text that was never
written to disk. Until then, an editor building a shopping list from unsaved
buffers still has to reimplement the aggregation — which is the exact
duplication this crate exists to remove.

### Two notes on `parse`

- **The parser configuration differs.** `PARSER` here is
  `Extensions::empty()`, matching CookCLI. The addon's `parse` uses
  `Extensions::all()`. Adopting this crate therefore changes what the editor
  accepts; that is a decision to make deliberately, not an oversight.
- **There is no "parse without scaling".** `parse_recipe(text, name, 1.0)`
  still calls `Recipe::scale`, which re-fits units — `1500 ml` comes back as
  `1.5 l`. The unscaled path exists internally but is not public.

## License

MIT. See `LICENSE`.

[`Context::new`]: https://docs.rs/cookcli-core/latest/cookcli_core/context/struct.Context.html#method.new
[`Context::discover`]: https://docs.rs/cookcli-core/latest/cookcli_core/context/struct.Context.html#method.discover
