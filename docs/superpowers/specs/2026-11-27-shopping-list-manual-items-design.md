# Type items into the web shopping list

**Date:** 2026-11-27
**Status:** Approved in conversation (scope: iOS parity without Undo; parsing in a shared crate; always-visible add field)
**Design:** Figma "Cooklang iOS Final", section "Shopping List States" (node `6598:34624`), as inspiration
**Depends on:** `cooklang-ingredient-parser` (`cooklang-rs/docs/superpowers/specs/2026-11-27-ingredient-parser-design.md`)
**Mirrors:** the iOS feature (`mobile-app-ios/docs/superpowers/specs/2026-11-26-shopping-list-manual-items-design.md`)

## Problem

The web shopping list can only be filled from recipes and menus. Milk, bin bags or a lemon that no recipe calls for need a second list somewhere else. The iOS app now lets people type these in, and writes them to the same `.shopping-list` CookCLI serves.

## What already works

- Free-hand items are top-level lines of `.shopping-list` (`milk{2%l}`). `ShoppingListStore` reads (`load_free_hand`), removes (`remove_free_hand`) and preserves them, and `add_all` writes them when a menu's days are added.
- `GET /api/shopping_list/extra_items` and `POST /api/shopping_list/remove_extra_item` exist.
- The page merges free-hand items into the aisle list (`POST /api/shopping_list` takes them as `ListRequest::Item`) and lists them in an "Extra items" box in the sidebar, each with Remove.

Missing: adding by typing, the amount chip, pasting a list, editing.

## Behaviour

### Adding one item

- An add field sits in the main column above the list, below the title row. It is always visible to someone who may edit lists (`viewer.can_edit_lists()`), including on the empty list. It sits outside `#shopping-list-header`, which is hidden on an empty list. Placeholder: "Add an item, e.g. Milk 2 l".
- As the user types, a chip beside the field shows the recognised amount ("1.5 kg", "x3"). When the name matches the aisle configuration, the chip also names the category ("1.5 kg · Baking", or just "Baking"). There is no chip when there is nothing to show. The chip comes from `POST /api/shopping_list/recognize`, sent 150 ms after the last keystroke; an answer for older text than the field now holds is ignored.
- Enter or the send button adds the item. Send is disabled while the field is blank. After an add the field is cleared and keeps focus.
- The new row on the aisle list is scrolled into view and highlighted for 2 seconds; the highlight then fades over 0.5 seconds. There is no notice for a single add.
- The field carries `data-unsaved`, so leaving the page with text typed asks first (`static/js/unsaved.js`). This replaces the iOS Discard prompt.

### Pasting a list

- Pasting text with more than one non-blank line into the field opens a preview card directly below it instead of filling the field. Each line becomes a row, parsed by the same rules (`parse_ingredient_list`: bullets, checkboxes and blank lines dropped).
- Each row has a remove button, the line's text in an editable input, and its chip. Editing a row's text updates its chip (one `recognize` call for the row, same debounce). Removing a row strikes it through and turns its button into restore.
- "Add items" adds every row not removed, in one request, closes the card, scrolls to the first new row and highlights all new rows at once with the same timing. It is disabled when every row is removed. "Cancel" closes the card and drops the pasted lines; the field keeps what it had before the paste.
- The card carries `data-unsaved` while open.

### The "Other items" box

- The sidebar's "Extra items" box is renamed "Other items", the iOS title. It lists every top-level free-hand line, whoever wrote it (typed here, typed on iOS, or the ingredients of a menu's days).
- Each row shows the name and its amount, and, to someone who may edit lists, Edit and Remove.
- **Edit** puts `edit_text(name, quantity)` into the add field ("Milk 2 l", "Parsley, 2 bunches", or the name alone for a text quantity) and turns the field into edit mode: the send button becomes a check mark, a Cancel button appears, Esc cancels. Saving sends `POST /api/shopping_list/edit_extra_item` and returns the field to adding. The edited row is highlighted like a new one.
- **Remove** stays as it is.

### Where items appear

As today: a free-hand item is combined with recipe ingredients of the same name on the aisle list, in the aisle its name has in `aisle.conf`, or "Other". A typed name that matches an aisle name or alias, ignoring case and simple English plurals, is written in the aisle's spelling, so "Tomato" joins the `tomatoes` row.

### Ticks

Adding to or editing an item unticks the row it lands on: there is more to buy. The row's name is the aisle common name when the item matched the aisle configuration, otherwise the written name. The untick is written under `AppState::checked_log_lock`, like `uncheck_shopping_item`.

## Design

### `cookcli-core`

- Depends on `cooklang-ingredient-parser` (a path dependency on `../cooklang-rs/ingredient-parser` during development; the published version before merge).
- `ShoppingListStore` gains:
  - `add_typed(&self, items: Vec<FreeHandItem>) -> Result<Vec<FreeHandItem>, CoreError>`: for each item, if a top-level free-hand line has the same name (ignoring case) and `add_quantities` can add the two quantities, the line's quantity becomes the sum; otherwise the item becomes a new line at the end. One read and one write for the whole batch. Returns the lines as written. `add_all` is unchanged (adding a menu's days uses it).
  - `replace_free_hand(&self, old: &FreeHandItem, new: FreeHandItem) -> Result<bool, CoreError>`: replaces the first line equal to `old` in place, then folds it into an earlier or later line with the same name if `add_quantities` allows it. `false` if `old` is no longer there. Writes nothing when `new == old`.
- Turning raw lines into `FreeHandItem`s lives in a small core function, `typed_items(lines, aisle) -> Vec<FreeHandItem>`: `parse_ingredient` per line, the name already cleaned, a line whose name is empty skipped, quantity from `Amount::to_quantity`.
- **Edit of a quantity the field cannot show.** When `edit_text` gives the name alone (a text quantity such as `a pinch`), and the edited line has no amount, the old quantity is kept.

### API

All three are under `/api/shopping_list`, which `auth::middleware::WRITES` already gives `Capability::EditLists`.

- `POST /api/shopping_list/recognize`, body `{ "lines": ["Sugar 1.5kg", ...] }` (at most 200 lines, each at most 500 characters), returns one entry per line: `{ "name", "quantity", "chip", "category" }`, the last three nullable. It changes nothing, so it is added to `OPEN_WRITES` next to `/api/shopping_list`, and needs no `Viewer`. It reads the aisle configuration the way `read_aisle_file` does.
- `POST /api/shopping_list/add_items`, body `{ "lines": [...] }` (same limits). Returns `{ "added": [{ "name", "quantity", "row" }] }`, `row` being the name the aisle list shows (for highlighting). `400` when no line has a name.
- `POST /api/shopping_list/edit_extra_item`, body `{ "name", "quantity", "line" }`: the item as `extra_items` returned it and the new text. `404` when the item is gone (changed elsewhere), `400` when the new line has no name. Returns the item as written, with `row`.
- The two writes take `Extension<Viewer>`, call `activity::record` once the write succeeded, quoting the items with `activity::quoted`, and report errors through `handlers::common::json_error`. `get_extra_items` and `remove_extra_item` move to `json_error` too, as they put `e.to_string()` in the body by hand today.
- The file watcher already pushes `.shopping-list` changes to open pages, so other tabs update as they do today.
- `src/web/api_docs.rs` describes the three routes; `docs/api.md` is regenerated with `UPDATE_API_DOCS=1 cargo test --test api_docs_md_test`.

### Page (`templates/shopping_list.html`)

- The add field, chip, send/save and Cancel buttons, and the paste card are rendered only when `viewer.can_edit_lists()`; with `--recipes-only` they are never shown.
- The paste is caught in the field's `paste` event: when the clipboard text has more than one non-blank line, the default is prevented and the card opens.
- Highlights are kept as a set of row names with an expiry time and re-applied after each render of the list, since a refresh can redraw the rows during the 2.5 seconds. Rows are found by `data-ingredient-name`, compared ignoring case. The highlight uses existing tokens (`bg-accent-soft` or equivalent) with a CSS transition; `prefers-reduced-motion` drops the fade.
- DOM building follows the page's existing helpers (`escHtml`, `setHtml`); no `cond && node` in `append`.
- New strings in every locale under `locales/*/shopping.ftl`: the placeholder, send, save, cancel, "Add items", remove and restore labels for paste rows, "Other items" (replacing `shopping-extra-items`), "Edit", and their accessibility labels.

## Error handling

- A failed write answers through `json_error`, and the page shows it in the existing error banner; the field keeps its text.
- A failed `recognize` hides the chip and does not block adding.
- `edit_extra_item` answering `404` refreshes the list and shows a short message that the item had changed.

## Testing

- **Core:** `typed_items` (cleaning, aisle spelling, empty names skipped); `add_typed` (new line, same name and unit sums, different unit is a second line, case-insensitive name, one write for a batch, existing recipe lines and menu-day lines untouched); `replace_free_hand` (in place, fold into a matching line, missing `old`, unchanged text writes nothing, text quantity kept). A hostile name written by `add_typed` loads back through `load_free_hand` and through `add_extra_item`, so neither the list load nor the generate request fails.
- **Server:** the three routes in an integration test through `common::with_isolated_config`; the auth tests list `add_items` and `edit_extra_item` as `EditLists` writes and `recognize` as an open write; activity lines for add and edit.
- **Playwright** `tests/e2e/shopping-list-manual-items.spec.ts`: type "Sugar 1.5kg" and see the chip, add it and find a highlighted "Sugar" row with 1.5 kg; paste three lines, remove one, add two; edit an item from "Other items"; remove it. Restores `seed/.shopping-list` and `seed/.shopping-checked` afterwards. Run the whole Playwright project, as the box rename touches selectors other specs may use.
- `tests/locales_test.rs` covers the new keys.

## Known risks

- **Older iOS builds rewrite a list holding a free-hand line.** iOS 0.14.1 and earlier treat any top-level line that is not a `./` reference as their old format and rewrite the whole file on launch. CookCLI could already write such lines (adding a menu's days); the add field makes them common. Rollout is a release-notes decision.
- **The desktop editor drops free-hand lines** when it rewrites `.shopping-list` (`cook-md/editor`, `shopping-list-types.ts`).
- **iOS parses without the comma rule and plural matching** until it moves to the shared crate. Its written lines stay compatible: stored quantities are formatted identically.

## Out of scope

- Undo.
- The CLI's `--extra` flag using the parser.
- Autocomplete, choosing an aisle by hand, editing from the aisle list.
