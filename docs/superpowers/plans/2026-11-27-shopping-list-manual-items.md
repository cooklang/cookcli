# Typed Items on the Web Shopping List — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let someone type "Sugar 1.5kg" (or paste a list) into the web shopping list, see the amount recognised as they type, and edit or remove those items from the sidebar.

**Architecture:** Parsing comes from the published `cooklang-ingredient-parser` 0.20.0 crate. `cookcli-core` turns typed lines into `FreeHandItem`s and stores them (merging same name + unit) in `.shopping-list` as top-level free-hand lines, the form the iOS app also writes. Three new `/api/shopping_list` routes (`recognize`, `add_items`, `edit_extra_item`) sit under the existing `EditLists` capability. The page gets an always-visible add field, a paste preview card and Edit buttons in the "Other items" box, in a new `static/js/shopping-manual-items.js`.

**Tech Stack:** Rust (axum, askama, serde), `cooklang-ingredient-parser`, vanilla JS, Tailwind CSS, Fluent, Playwright.

**Spec:** `docs/superpowers/specs/2026-11-27-shopping-list-manual-items-design.md`

**Branch:** `feat/shopping-list-manual-items` (the spec is committed on it). The checkout is shared with other sessions: run `git branch --show-current` in the same command as every `git commit`.

---

## File structure

```
crates/core/Cargo.toml                         + cooklang-ingredient-parser
crates/core/src/shopping_list/typed.rs         NEW: typed lines → TypedItem (name, quantity, row, chip, category, edit text)
crates/core/src/shopping_list/mod.rs           mod typed; re-exports
crates/core/src/shopping_list/store.rs         + add_typed, replace_free_hand
crates/core/src/shopping_list/store/tests.rs   + tests for both
src/server/handlers/shopping_list.rs           + recognize, add_items, edit_extra_item; extra_items gains edit_text; json_error
src/server/handlers/mod.rs                     re-exports
src/server/mod.rs                              routes
src/server/auth/middleware.rs                  auth test lists the new routes
src/web/api_docs.rs, docs/api.md               API docs
tests/shopping_list_typed_items_test.rs        NEW: server integration tests
locales/*/shopping.ftl                         7 new messages, "Other items"
static/css/components.css                      .row-flashable / .row-flash
static/js/shopping-manual-items.js             NEW: add field, chip, paste card, edit, highlight
templates/shopping_list.html                   field markup, script include, Edit button, afterRender hook
tests/e2e/shopping-list-manual-items.spec.ts   NEW: Playwright
```

---

### Task 1: Depend on the parser and read typed lines in core

**Files:**
- Modify: `crates/core/Cargo.toml`
- Create: `crates/core/src/shopping_list/typed.rs`
- Modify: `crates/core/src/shopping_list/mod.rs:46-52`

- [ ] **Step 1: Add the dependency**

In `crates/core/Cargo.toml`, in `[dependencies]` directly under the `cooklang = …` line, add:

```toml
cooklang-ingredient-parser = "0.20.0"
```

Run: `cargo build -p cookcli-core`
Expected: builds (it downloads the crate from crates.io).

- [ ] **Step 2: Write `typed.rs` with tests**

Create `crates/core/src/shopping_list/typed.rs`:

```rust
//! Typed lines to free-hand items, for the shopping list's add field.
//!
//! The parsing is `cooklang-ingredient-parser`'s, shared with the Cooklang
//! apps, so a line typed here is stored exactly as the iOS app would store it.
//! No model is passed: CookCLI ships as one binary without ONNX Runtime.

use super::FreeHandItem;
use cooklang::aisle::AisleConf;
use cooklang_ingredient_parser::{edit_text, parse_ingredient, parse_ingredient_list, ParsedIngredient};
use serde::Serialize;

/// One typed line, read: what would be written and how the page shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TypedItem {
    /// The free-hand line to write: the cleaned name (the aisle's spelling
    /// when it matched) and the stored quantity (`1.5%kg`, `3`).
    pub item: FreeHandItem,
    /// The name of the row it lands on in the aisle list: the aisle's common
    /// name when it matched, otherwise the written name.
    pub row: String,
    /// The recognised amount as the chip shows it ("1.5 kg", "x3").
    pub chip: Option<String>,
    /// The aisle category the name matched.
    pub category: Option<String>,
    /// The line that reads back as this item ("Sugar 1.5 kg").
    pub edit_text: String,
}

fn typed_item(parsed: ParsedIngredient) -> Option<TypedItem> {
    if parsed.name.is_empty() {
        return None;
    }
    let quantity = parsed.amount.as_ref().and_then(|amount| amount.to_quantity());
    let chip = quantity
        .as_ref()
        .and(parsed.amount.as_ref())
        .map(|amount| amount.chip_text());
    let row = parsed
        .aisle_match
        .as_ref()
        .map_or_else(|| parsed.name.clone(), |m| m.common_name.clone());
    let category = parsed.aisle_match.map(|m| m.category);
    let item = FreeHandItem { name: parsed.name, quantity };
    Some(TypedItem {
        edit_text: edit_text(&item.name, item.quantity.as_deref()),
        item,
        row,
        chip,
        category,
    })
}

/// One typed line; `None` when it has no name (blank, or only braces).
pub fn read_typed_line(line: &str, aisle: Option<&AisleConf>) -> Option<TypedItem> {
    typed_item(parse_ingredient(line, aisle, None))
}

/// Pasted text: one item per kept line (bullets, checkboxes, Markdown
/// headings and blank lines dropped), lines without a name skipped.
pub fn read_typed_text(text: &str, aisle: Option<&AisleConf>) -> Vec<TypedItem> {
    parse_ingredient_list(text, aisle, None)
        .into_iter()
        .filter_map(typed_item)
        .collect()
}

/// The item an edit of `old` makes from `line`. An item whose quantity the
/// field could not show (`2%bunches`, `a pinch`: its edit text is the name
/// alone) keeps that quantity unless the edited line has an amount.
pub fn edited_item(old: &FreeHandItem, line: &str, aisle: Option<&AisleConf>) -> Option<TypedItem> {
    let mut typed = read_typed_line(line, aisle)?;
    let shown_without_quantity = edit_text(&old.name, old.quantity.as_deref()) == old.name;
    if typed.item.quantity.is_none() && shown_without_quantity {
        typed.item.quantity = old.quantity.clone();
        typed.edit_text = edit_text(&typed.item.name, typed.item.quantity.as_deref());
    }
    Some(typed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aisle() -> AisleConf<'static> {
        cooklang::aisle::parse("[baking]\nsugar | caster sugar\n\n[veg]\ntomatoes\n").unwrap()
    }

    fn item(name: &str, quantity: Option<&str>) -> FreeHandItem {
        FreeHandItem { name: name.into(), quantity: quantity.map(Into::into) }
    }

    #[test]
    fn a_line_with_an_amount() {
        let typed = read_typed_line("Sugar 1.5kg", Some(&aisle())).unwrap();
        assert_eq!(typed.item, item("sugar", Some("1.5%kg")));
        assert_eq!(typed.row, "sugar");
        assert_eq!(typed.chip.as_deref(), Some("1.5 kg"));
        assert_eq!(typed.category.as_deref(), Some("baking"));
        assert_eq!(typed.edit_text, "sugar 1.5 kg");
    }

    #[test]
    fn an_alias_lands_on_the_common_name_row() {
        let typed = read_typed_line("Caster sugar 200 g", Some(&aisle())).unwrap();
        assert_eq!(typed.item.name, "caster sugar");
        assert_eq!(typed.row, "sugar");
    }

    #[test]
    fn a_plural_takes_the_aisle_spelling() {
        let typed = read_typed_line("Tomato 3", Some(&aisle())).unwrap();
        assert_eq!(typed.item, item("tomatoes", Some("3")));
        assert_eq!(typed.chip.as_deref(), Some("x3"));
    }

    #[test]
    fn no_aisle_no_amount() {
        let typed = read_typed_line("Paper towels", None).unwrap();
        assert_eq!(typed.item, item("Paper towels", None));
        assert_eq!(typed.row, "Paper towels");
        assert_eq!(typed.chip, None);
        assert_eq!(typed.category, None);
    }

    #[test]
    fn a_line_without_a_name_is_none() {
        assert_eq!(read_typed_line("   ", None), None);
        assert_eq!(read_typed_line("{}", None), None);
    }

    #[test]
    fn pasted_text() {
        let typed = read_typed_text("## Dairy\n- Milk 2 l\n\n* [ ] 3 lemons\n- {}\n", None);
        let items: Vec<_> = typed.into_iter().map(|t| t.item).collect();
        assert_eq!(items, [item("Milk", Some("2%l")), item("lemons", Some("3"))]);
    }

    #[test]
    fn an_edit_keeps_a_quantity_the_field_could_not_show() {
        let old = item("parsley", Some("2%bunches"));
        assert_eq!(edited_item(&old, "flat parsley", None).unwrap().item, item("flat parsley", Some("2%bunches")));
        assert_eq!(edited_item(&old, "flat parsley 3", None).unwrap().item, item("flat parsley", Some("3")));
    }

    #[test]
    fn an_edit_can_drop_a_shown_amount() {
        let old = item("Milk", Some("2%l"));
        assert_eq!(edited_item(&old, "Milk", None).unwrap().item, item("Milk", None));
    }
}
```

- [ ] **Step 3: Register the module**

In `crates/core/src/shopping_list/mod.rs`, after `mod store;` (line 47) add `mod typed;`, and after the `pub use store::{…};` block add:

```rust
pub use typed::{edited_item, read_typed_line, read_typed_text, TypedItem};
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p cookcli-core typed`
Expected: 8 passed. If the aisle constant fails to parse, check `cooklang::aisle::parse` syntax in its tests; the `|` alias form is what `seed/config/aisle.conf` uses.

- [ ] **Step 5: Commit**

```bash
[ "$(git branch --show-current)" = feat/shopping-list-manual-items ] && git add crates/core Cargo.lock && git commit -m "feat(core): read typed shopping list lines with the ingredient parser"
```

---

### Task 2: Store typed items, merging same name and unit

**Files:**
- Modify: `crates/core/src/shopping_list/store.rs` (after `add_all`, ~line 352)
- Modify: `crates/core/src/shopping_list/store/tests.rs` (append)

- [ ] **Step 1: Write the failing tests**

Append to `crates/core/src/shopping_list/store/tests.rs`:

```rust
// -- Typed items ---------------------------------------------------------------

fn free(name: &str, quantity: Option<&str>) -> FreeHandItem {
    FreeHandItem {
        name: name.to_string(),
        quantity: quantity.map(str::to_string),
    }
}

#[test]
fn typed_items_are_written_as_free_hand_lines() {
    let dir = temp();
    let store = store(&dir);
    store.add(entry("Soup.cook", 1.0)).unwrap();
    let written = store
        .add_typed(vec![free("sugar", Some("1.5%kg")), free("bread", None)])
        .unwrap();
    assert_eq!(written, [free("sugar", Some("1.5%kg")), free("bread", None)]);
    assert_eq!(list_file(&dir), "./Soup\nsugar{1.5%kg}\nbread\n");
}

#[test]
fn the_same_name_and_unit_add_up_ignoring_case() {
    let dir = temp();
    let store = store(&dir);
    store.add_typed(vec![free("Sugar", Some("1.5%kg"))]).unwrap();
    let written = store.add_typed(vec![free("sugar", Some("1%kg"))]).unwrap();
    assert_eq!(written, [free("Sugar", Some("2.5%kg"))]);
    assert_eq!(list_file(&dir), "Sugar{2.5%kg}\n");
}

#[test]
fn a_different_unit_is_a_second_line() {
    let dir = temp();
    let store = store(&dir);
    store.add_typed(vec![free("milk", Some("2%l"))]).unwrap();
    store.add_typed(vec![free("milk", Some("500%ml")), free("milk", Some("2"))]).unwrap();
    assert_eq!(list_file(&dir), "milk{2%l}\nmilk{500%ml}\nmilk{2}\n");
}

#[test]
fn two_lines_without_an_amount_are_one() {
    let dir = temp();
    let store = store(&dir);
    store.add_typed(vec![free("bread", None), free("Bread", None)]).unwrap();
    assert_eq!(list_file(&dir), "bread\n");
}

#[test]
fn a_typed_item_merges_into_a_menu_day_line_too() {
    let dir = temp();
    let store = store(&dir);
    store.add_all(Vec::new(), vec![free("almonds", Some("50%g"))]).unwrap();
    store.add_typed(vec![free("almonds", Some("100%g"))]).unwrap();
    assert_eq!(list_file(&dir), "almonds{150%g}\n");
}
```

Check the expected text against how the format writes a recipe entry: if `list_file` after `store.add(entry("Soup.cook", 1.0))` reads differently in the existing tests (look at `an_added_recipe_comes_back_with_its_path_name_and_scale` and the file contents other tests assert), use that exact form for the first line in `typed_items_are_written_as_free_hand_lines`.

- [ ] **Step 2: Run to see them fail**

Run: `cargo test -p cookcli-core typed_items_are_written`
Expected: FAIL to compile, `no method named add_typed`.

- [ ] **Step 3: Implement `add_typed`**

In `crates/core/src/shopping_list/store.rs`, add at the top with the other imports:

```rust
use cooklang_ingredient_parser::add_quantities;
```

and after `add_all` add:

```rust
    /// Add typed items: each joins a top-level free-hand line with the same
    /// name (ignoring case) whose quantity it can be added to — the same
    /// unit, or both without one — and otherwise becomes a new line at the
    /// end. One read and one write for the batch. Returns each item as the
    /// line it ended up in, in the order given.
    ///
    /// The merging is what the iOS app does, so the two agree on the file.
    pub fn add_typed(&self, items: Vec<FreeHandItem>) -> Result<Vec<FreeHandItem>, CoreError> {
        self.migrate_if_needed()?;
        let mut list = self.load_list()?;
        let mut written = Vec::with_capacity(items.len());
        for item in items {
            let merged = list.items.iter_mut().find_map(|line| match line {
                ShoppingListItem::Ingredient(line)
                    if line.name.to_lowercase() == item.name.to_lowercase() =>
                {
                    add_quantities(line.quantity.as_deref(), item.quantity.as_deref())
                        .ok()
                        .map(|sum| {
                            line.quantity = sum;
                            FreeHandItem { name: line.name.clone(), quantity: line.quantity.clone() }
                        })
                }
                _ => None,
            });
            match merged {
                Some(line) => written.push(line),
                None => {
                    list.items.push(ShoppingListItem::Ingredient(IngredientItem {
                        name: item.name.clone(),
                        quantity: item.quantity.clone(),
                        optional: false,
                    }));
                    written.push(item);
                }
            }
        }
        self.save_list(&list)?;
        Ok(written)
    }
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p cookcli-core store::tests`
Expected: all pass, the five new ones included.

- [ ] **Step 5: Commit**

```bash
[ "$(git branch --show-current)" = feat/shopping-list-manual-items ] && git add crates/core && git commit -m "feat(core): store typed shopping list items, adding up the same name and unit"
```

---

### Task 3: Replace a free-hand line (edit)

**Files:**
- Modify: `crates/core/src/shopping_list/store.rs` (after `remove_free_hand`)
- Modify: `crates/core/src/shopping_list/store/tests.rs` (append)

- [ ] **Step 1: Write the failing tests**

Append to `store/tests.rs`:

```rust
#[test]
fn an_edit_replaces_the_line_in_place() {
    let dir = temp();
    let store = store(&dir);
    store.add_typed(vec![free("milk", Some("2%l")), free("bread", None)]).unwrap();
    let found = store
        .replace_free_hand(&free("milk", Some("2%l")), free("oat milk", Some("1%l")))
        .unwrap();
    assert!(found);
    assert_eq!(list_file(&dir), "oat milk{1%l}\nbread\n");
}

#[test]
fn an_edit_that_matches_another_line_folds_into_it() {
    let dir = temp();
    let store = store(&dir);
    store.add_typed(vec![free("sugar", Some("1%kg")), free("flour", Some("2%kg"))]).unwrap();
    store
        .replace_free_hand(&free("flour", Some("2%kg")), free("Sugar", Some("2%kg")))
        .unwrap();
    assert_eq!(list_file(&dir), "sugar{3%kg}\n");
}

#[test]
fn an_edit_of_a_line_that_is_gone_changes_nothing() {
    let dir = temp();
    let store = store(&dir);
    store.add_typed(vec![free("bread", None)]).unwrap();
    let found = store
        .replace_free_hand(&free("milk", Some("2%l")), free("milk", Some("3%l")))
        .unwrap();
    assert!(!found);
    assert_eq!(list_file(&dir), "bread\n");
}

#[test]
fn an_unchanged_edit_writes_nothing() {
    let dir = temp();
    let store = store(&dir);
    store.add_typed(vec![free("bread", None)]).unwrap();
    let path = base(&dir).join(".shopping-list");
    let before = std::fs::metadata(path.as_std_path()).unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    assert!(store.replace_free_hand(&free("bread", None), free("bread", None)).unwrap());
    let after = std::fs::metadata(path.as_std_path()).unwrap().modified().unwrap();
    assert_eq!(before, after);
}
```

- [ ] **Step 2: Run to see them fail**

Run: `cargo test -p cookcli-core an_edit`
Expected: FAIL to compile, `no method named replace_free_hand`.

- [ ] **Step 3: Implement**

After `remove_free_hand` in `store.rs`:

```rust
    /// Replace the first free-hand line equal to `old` with `new`, in place,
    /// then fold it into another top-level free-hand line with the same name
    /// (ignoring case) if their quantities add up. `Ok(false)` when `old` is
    /// no longer on the list (changed elsewhere). Writes nothing when `new`
    /// equals `old`.
    pub fn replace_free_hand(&self, old: &FreeHandItem, new: FreeHandItem) -> Result<bool, CoreError> {
        self.migrate_if_needed()?;
        let mut list = self.load_list()?;
        let Some(pos) = list.items.iter().position(|line| {
            matches!(line, ShoppingListItem::Ingredient(line)
                if line.name == old.name && line.quantity == old.quantity)
        }) else {
            return Ok(false);
        };
        if &new == old {
            return Ok(true);
        }
        let other = list.items.iter().enumerate().position(|(i, line)| {
            i != pos
                && matches!(line, ShoppingListItem::Ingredient(line)
                    if line.name.to_lowercase() == new.name.to_lowercase()
                        && add_quantities(line.quantity.as_deref(), new.quantity.as_deref()).is_ok())
        });
        match other {
            Some(other) => {
                if let ShoppingListItem::Ingredient(line) = &mut list.items[other] {
                    line.quantity = add_quantities(line.quantity.as_deref(), new.quantity.as_deref())
                        .unwrap_or_else(|_| line.quantity.clone());
                }
                list.items.remove(pos);
            }
            None => {
                list.items[pos] = ShoppingListItem::Ingredient(IngredientItem {
                    name: new.name,
                    quantity: new.quantity,
                    optional: false,
                });
            }
        }
        self.save_list(&list)?;
        Ok(true)
    }
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p cookcli-core store::tests`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
[ "$(git branch --show-current)" = feat/shopping-list-manual-items ] && git add crates/core && git commit -m "feat(core): edit a free-hand shopping list line in place"
```

---

### Task 4: Server routes

**Files:**
- Modify: `src/server/handlers/shopping_list.rs` (after `remove_extra_item`, ~line 750; `get_extra_items` ~712)
- Modify: `src/server/handlers/mod.rs:30-31`
- Modify: `src/server/mod.rs` (routes, after `/shopping_list/remove_extra_item`)
- Modify: `src/server/auth/middleware.rs` (test `list_writes_need_edit_lists`)
- Create: `tests/shopping_list_typed_items_test.rs`

- [ ] **Step 1: Write the failing integration test**

Create `tests/shopping_list_typed_items_test.rs`:

```rust
//! Typing items into the web shopping list: `recognize`, `add_items` and
//! `edit_extra_item` under `/api/shopping_list`.

#![cfg(feature = "server")]

use reqwest::StatusCode;
use serde_json::{json, Value};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

#[path = "common/mod.rs"]
mod common;

struct ServerGuard {
    child: Child,
    port: u16,
    dir: TempDir,
}

impl ServerGuard {
    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", self.port, path)
    }

    fn recipes(&self) -> PathBuf {
        self.dir.path().join("recipes")
    }

    fn list_file(&self) -> String {
        std::fs::read_to_string(self.recipes().join(".shopping-list")).unwrap_or_default()
    }

    async fn post(&self, path: &str, body: Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(self.url(path))
            .json(&body)
            .send()
            .await
            .expect("request")
    }

    async fn get_json(&self, path: &str) -> Value {
        reqwest::Client::new()
            .get(self.url(path))
            .send()
            .await
            .expect("request")
            .json()
            .await
            .expect("json")
    }
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    listener.local_addr().expect("local addr").port()
}

async fn start_server() -> ServerGuard {
    for _ in 0..5 {
        if let Some(server) = try_start_server().await {
            return server;
        }
    }
    panic!("could not start cook server on a free port after 5 attempts");
}

async fn try_start_server() -> Option<ServerGuard> {
    let dir = TempDir::new().expect("temp dir");
    let recipes = dir.path().join("recipes");
    std::fs::create_dir_all(recipes.join("config")).unwrap();
    std::fs::write(recipes.join("Cake.cook"), "Mix @sugar{200%g} and @flour{300%g}.\n").unwrap();
    std::fs::write(
        recipes.join("config/aisle.conf"),
        "[baking]\nsugar | caster sugar\nflour\n\n[veg]\ntomatoes\n",
    )
    .unwrap();

    let port = free_port();
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin("cook"));
    cmd.arg("server").arg(&recipes).arg("--port").arg(port.to_string());
    let child = common::with_isolated_config(&mut cmd, dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn cook server");

    let mut guard = ServerGuard { child, port, dir };
    let client = reqwest::Client::new();
    let url = guard.url("/api/recipes");
    for _ in 0..200 {
        if guard.child.try_wait().expect("poll server").is_some() {
            return None;
        }
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                return Some(guard);
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("cook server on port {port} never became ready");
}

#[tokio::test]
async fn recognize_reads_lines_without_writing() {
    let server = start_server().await;
    let resp = server
        .post("/api/shopping_list/recognize", json!({ "lines": ["Sugar 1.5kg", "Tomato 3", "  "] }))
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(
        body,
        json!([
            { "name": "sugar", "quantity": "1.5%kg", "chip": "1.5 kg", "category": "baking", "edit_text": "sugar 1.5 kg" },
            { "name": "tomatoes", "quantity": "3", "chip": "x3", "category": "veg", "edit_text": "tomatoes 3" },
            { "name": "", "quantity": null, "chip": null, "category": null, "edit_text": "" }
        ])
    );
    assert_eq!(server.list_file(), "");
}

#[tokio::test]
async fn recognize_splits_pasted_text() {
    let server = start_server().await;
    let body: Value = server
        .post("/api/shopping_list/recognize", json!({ "text": "## Shop\n- Milk 2 l\n\n* [ ] bread\n" }))
        .await
        .json()
        .await
        .unwrap();
    let names: Vec<&str> = body.as_array().unwrap().iter().map(|e| e["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["Milk", "bread"]);
}

#[tokio::test]
async fn add_items_writes_merges_and_answers_rows() {
    let server = start_server().await;
    let resp = server
        .post(
            "/api/shopping_list/add_items",
            json!({ "lines": ["Sugar 1.5kg", "Caster sugar 1 kg", "Tomato 2", "Paper towels"] }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    let rows: Vec<&str> = body["added"].as_array().unwrap().iter().map(|e| e["row"].as_str().unwrap()).collect();
    assert_eq!(rows, ["sugar", "sugar", "tomatoes", "Paper towels"]);
    assert_eq!(
        server.list_file(),
        "sugar{1.5%kg}\ncaster sugar{1%kg}\ntomatoes{2}\nPaper towels\n"
    );

    server.post("/api/shopping_list/add_items", json!({ "lines": ["sugar 1 kg"] })).await;
    assert!(server.list_file().starts_with("sugar{2.5%kg}\n"), "{}", server.list_file());
}

#[tokio::test]
async fn add_items_refuses_nothing_to_add_and_too_much() {
    let server = start_server().await;
    let resp = server.post("/api/shopping_list/add_items", json!({ "lines": ["  ", "{}"] })).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let many: Vec<String> = (0..201).map(|i| format!("item {i}")).collect();
    let resp = server.post("/api/shopping_list/add_items", json!({ "lines": many })).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let long = "x".repeat(501);
    let resp = server.post("/api/shopping_list/recognize", json!({ "lines": [long] })).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(server.list_file(), "");
}

#[tokio::test]
async fn adding_unticks_the_row() {
    let server = start_server().await;
    server.post("/api/shopping_list/add_items", json!({ "lines": ["sugar 1 kg"] })).await;
    server.post("/api/shopping_list/check", json!({ "name": "sugar" })).await;
    let checked = server.get_json("/api/shopping_list/checked").await;
    assert!(checked.as_array().unwrap().iter().any(|n| n == "sugar"), "{checked}");

    server.post("/api/shopping_list/add_items", json!({ "lines": ["Sugar 500 g"] })).await;
    let checked = server.get_json("/api/shopping_list/checked").await;
    assert!(!checked.as_array().unwrap().iter().any(|n| n == "sugar"), "{checked}");
}

#[tokio::test]
async fn extra_items_carry_their_edit_text() {
    let server = start_server().await;
    std::fs::write(server.recipes().join(".shopping-list"), "milk{2%l}\nparsley{2%bunches}\n").unwrap();
    let body = server.get_json("/api/shopping_list/extra_items").await;
    assert_eq!(
        body,
        json!([
            { "name": "milk", "quantity": "2%l", "edit_text": "milk 2 l" },
            { "name": "parsley", "quantity": "2%bunches", "edit_text": "parsley" }
        ])
    );
}

#[tokio::test]
async fn edit_replaces_keeps_an_unshown_quantity_and_reports_a_gone_item() {
    let server = start_server().await;
    std::fs::write(server.recipes().join(".shopping-list"), "milk{2%l}\nparsley{2%bunches}\n").unwrap();

    let resp = server
        .post(
            "/api/shopping_list/edit_extra_item",
            json!({ "name": "milk", "quantity": "2%l", "line": "oat milk 1 l" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body, json!({ "name": "oat milk", "quantity": "1%l", "row": "oat milk" }));

    server
        .post(
            "/api/shopping_list/edit_extra_item",
            json!({ "name": "parsley", "quantity": "2%bunches", "line": "flat parsley" }),
        )
        .await;
    assert_eq!(server.list_file(), "oat milk{1%l}\nflat parsley{2%bunches}\n");

    let resp = server
        .post(
            "/api/shopping_list/edit_extra_item",
            json!({ "name": "milk", "quantity": "2%l", "line": "milk 3 l" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let resp = server
        .post(
            "/api/shopping_list/edit_extra_item",
            json!({ "name": "oat milk", "quantity": "1%l", "line": "  " }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
```

If `/api/shopping_list/checked` answers a different JSON shape than an array of names, read `get_checked_items` and adjust only the two `checked` assertions.

- [ ] **Step 2: Run to see it fail**

Run: `cargo test --test shopping_list_typed_items_test`
Expected: FAIL (routes answer 404/405; `extra_items` has no `edit_text`).

- [ ] **Step 3: Implement the handlers**

In `src/server/handlers/shopping_list.rs`:

(a) Extend the `cookcli_core::shopping_list` import with `edited_item, read_typed_line, read_typed_text, TypedItem`, and change `use serde::Deserialize;` to `use serde::{Deserialize, Serialize};`.

(b) Replace `get_extra_items` and the error mapping in `remove_extra_item` so they use `json_error` (never `e.to_string()` by hand), and add the edit text:

```rust
/// A free-hand item as `extra_items` answers it: the stored line and the
/// text the add field shows to edit it.
#[derive(Debug, Serialize)]
pub struct ExtraItem {
    #[serde(flatten)]
    item: FreeHandItem,
    edit_text: String,
}

/// The free-hand items on the list: lines no recipe put there, such as items
/// typed into the add field or the ingredients written straight into a menu
/// whose days were added.
pub async fn get_extra_items(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ExtraItem>>, (StatusCode, Json<serde_json::Value>)> {
    let store = ShoppingListStore::new(&state.base_path);
    let items = store.load_free_hand().map_err(|e| {
        tracing::error!("Failed to load shopping list: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, json_error(e))
    })?;
    Ok(Json(
        items
            .into_iter()
            .map(|item| ExtraItem {
                edit_text: cooklang_ingredient_parser::edit_text(&item.name, item.quantity.as_deref()),
                item,
            })
            .collect(),
    ))
}
```

`cooklang_ingredient_parser` is a dependency of `cookcli-core`, not of the `cookcli` crate. Rather than adding it to the root `Cargo.toml`, re-export it from core: in `crates/core/src/lib.rs` next to `pub use cooklang;` add `pub use cooklang_ingredient_parser;`, and here write `cookcli_core::cooklang_ingredient_parser::edit_text`.

In `remove_extra_item` change the `map_err` body to:

```rust
        tracing::error!("Failed to remove from shopping list: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, json_error(e))
```

(c) Add after `remove_extra_item`:

```rust
/// The most lines one request may carry, and the longest line.
const MAX_TYPED_LINES: usize = 200;
const MAX_TYPED_LINE_CHARS: usize = 500;

type JsonError = (StatusCode, Json<serde_json::Value>);

fn bad_request(message: &str) -> JsonError {
    (StatusCode::BAD_REQUEST, json_error(message))
}

fn check_lines(lines: &[String]) -> Result<(), JsonError> {
    if lines.len() > MAX_TYPED_LINES {
        return Err(bad_request("too many lines"));
    }
    if lines.iter().any(|line| line.chars().count() > MAX_TYPED_LINE_CHARS) {
        return Err(bad_request("a line is too long"));
    }
    Ok(())
}

/// What `recognize` and `add_items` take: typed lines, or pasted text
/// (`recognize` only) to be split into lines.
#[derive(Debug, Deserialize)]
pub struct TypedLinesRequest {
    #[serde(default)]
    lines: Option<Vec<String>>,
    #[serde(default)]
    text: Option<String>,
}

/// One line as `recognize` reads it. `name` is `""` for a line with no name.
#[derive(Debug, Serialize)]
pub struct RecognizedLine {
    name: String,
    quantity: Option<String>,
    chip: Option<String>,
    category: Option<String>,
    edit_text: String,
}

impl From<Option<TypedItem>> for RecognizedLine {
    fn from(typed: Option<TypedItem>) -> Self {
        match typed {
            Some(t) => Self {
                name: t.item.name,
                quantity: t.item.quantity,
                chip: t.chip,
                category: t.category,
                edit_text: t.edit_text,
            },
            None => Self { name: String::new(), quantity: None, chip: None, category: None, edit_text: String::new() },
        }
    }
}

/// Read typed lines as the add field would store them, without writing
/// anything: the amount chip and the paste preview.
pub async fn recognize_typed_lines(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<TypedLinesRequest>,
) -> Result<Json<Vec<RecognizedLine>>, JsonError> {
    let aisle_content = read_aisle_file(&state);
    let aisle = cooklang::aisle::parse_lenient(&aisle_content).output().cloned();
    let aisle = aisle.as_ref();
    match (payload.lines, payload.text) {
        (Some(lines), None) => {
            check_lines(&lines)?;
            Ok(Json(lines.iter().map(|line| read_typed_line(line, aisle).into()).collect()))
        }
        (None, Some(text)) => {
            if text.chars().count() > MAX_TYPED_LINES * MAX_TYPED_LINE_CHARS {
                return Err(bad_request("the text is too long"));
            }
            let typed = read_typed_text(&text, aisle);
            if typed.len() > MAX_TYPED_LINES {
                return Err(bad_request("too many lines"));
            }
            Ok(Json(typed.into_iter().map(|t| Some(t).into()).collect()))
        }
        _ => Err(bad_request("send either lines or text")),
    }
}

/// A typed item as written, with the row it landed on.
#[derive(Debug, Serialize)]
pub struct WrittenItem {
    name: String,
    quantity: Option<String>,
    row: String,
}

/// Untick `rows` that are ticked: there is more to buy. Under the checked
/// log's lock, as `uncheck_shopping_item`.
async fn untick_rows(state: &AppState, store: &ShoppingListStore, rows: &[String]) {
    let _guard = state.checked_log_lock.lock().await;
    let checked = match store.checked_set() {
        Ok(checked) => checked,
        Err(e) => {
            tracing::warn!("Could not read the ticks to untick added items: {:?}", e);
            return;
        }
    };
    for row in rows {
        if checked.contains(&row.to_lowercase()) {
            if let Err(e) = store.uncheck(row) {
                tracing::warn!("Could not untick {row}: {:?}", e);
            }
        }
    }
}

/// Add typed lines to the list as free-hand items.
pub async fn add_typed_items(
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
    Json(payload): Json<TypedLinesRequest>,
) -> Result<Json<serde_json::Value>, JsonError> {
    let lines = payload.lines.ok_or_else(|| bad_request("send lines"))?;
    check_lines(&lines)?;
    let aisle_content = read_aisle_file(&state);
    let aisle = cooklang::aisle::parse_lenient(&aisle_content).output().cloned();
    let typed: Vec<TypedItem> = lines.iter().filter_map(|line| read_typed_line(line, aisle.as_ref())).collect();
    if typed.is_empty() {
        return Err(bad_request("nothing to add"));
    }
    let store = ShoppingListStore::new(&state.base_path);
    let written = store
        .add_typed(typed.iter().map(|t| t.item.clone()).collect())
        .map_err(|e| {
            tracing::error!("Failed to add to shopping list: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, json_error(e))
        })?;
    let rows: Vec<String> = typed.iter().map(|t| t.row.clone()).collect();
    untick_rows(&state, &store, &rows).await;
    let names: Vec<String> = typed.iter().map(|t| activity::quoted(&t.item.spec()).to_string()).collect();
    activity::record(&viewer, format_args!("added {} to the shopping list", names.join(", ")));
    let added: Vec<WrittenItem> = written
        .into_iter()
        .zip(rows)
        .map(|(item, row)| WrittenItem { name: item.name, quantity: item.quantity, row })
        .collect();
    Ok(Json(serde_json::json!({ "added": added })))
}

#[derive(Debug, Deserialize)]
pub struct EditExtraItemRequest {
    name: String,
    #[serde(default)]
    quantity: Option<String>,
    line: String,
}

/// Replace one free-hand item with what an edited line says.
pub async fn edit_extra_item(
    State(state): State<Arc<AppState>>,
    Extension(viewer): Extension<Viewer>,
    Json(payload): Json<EditExtraItemRequest>,
) -> Result<Json<WrittenItem>, JsonError> {
    check_lines(std::slice::from_ref(&payload.line))?;
    let old = FreeHandItem { name: payload.name, quantity: payload.quantity };
    let aisle_content = read_aisle_file(&state);
    let aisle = cooklang::aisle::parse_lenient(&aisle_content).output().cloned();
    let typed = edited_item(&old, &payload.line, aisle.as_ref()).ok_or_else(|| bad_request("the item needs a name"))?;
    let store = ShoppingListStore::new(&state.base_path);
    let found = store.replace_free_hand(&old, typed.item.clone()).map_err(|e| {
        tracing::error!("Failed to edit shopping list item: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, json_error(e))
    })?;
    if !found {
        return Err((StatusCode::NOT_FOUND, json_error("the item is no longer on the list")));
    }
    untick_rows(&state, &store, std::slice::from_ref(&typed.row)).await;
    activity::record(
        &viewer,
        format_args!(
            "changed {} to {} on the shopping list",
            activity::quoted(&old.spec()),
            activity::quoted(&typed.item.spec())
        ),
    );
    compact_after_remove(&state, &store).await;
    Ok(Json(WrittenItem { name: typed.item.name, quantity: typed.item.quantity, row: typed.row }))
}
```

Notes for the implementer:
- `activity::quoted(..)` returns `impl Display`; `.to_string()` on it is fine for building the joined list.
- `parse_lenient(..).output().cloned()` mirrors `aggregate_current_ingredient_names` (~line 964); the `AisleConf` borrows `aisle_content`, so keep `aisle_content` alive in the same scope.
- If `store.checked_set()` returns names in another case than lowercase, check `crates/core/src/shopping_list/store.rs` `checked_set` and the test `a_checked_ingredient_is_remembered_lowercased`, and compare accordingly.
- `compact_after_remove` takes `&AppState` and `&ShoppingListStore` (line ~695); it drops ticks for names no longer on the list, which an edit can cause.

(d) In `src/server/handlers/mod.rs`, add `add_typed_items, edit_extra_item, recognize_typed_lines` to the `pub use shopping_list::{…}` list (lines 30-31).

(e) In `src/server/mod.rs`, after the `/shopping_list/remove_extra_item` route add:

```rust
        .route(
            "/shopping_list/recognize",
            post(handlers::recognize_typed_lines),
        )
        .route("/shopping_list/add_items", post(handlers::add_typed_items))
        .route(
            "/shopping_list/edit_extra_item",
            post(handlers::edit_extra_item),
        )
```

(f) In `src/server/auth/middleware.rs`, test `list_writes_need_edit_lists`, add to the array:

```rust
            (Method::POST, "/api/shopping_list/recognize"),
            (Method::POST, "/api/shopping_list/add_items"),
            (Method::POST, "/api/shopping_list/edit_extra_item"),
            (Method::POST, "/api/shopping_list/remove_extra_item"),
```

No change to `WRITES` or `OPEN_WRITES`: the `/api/shopping_list` prefix already needs `EditLists`.

- [ ] **Step 4: Run the tests**

Run: `cargo test --test shopping_list_typed_items_test && cargo test --test plan_shopping_test && cargo test --bin cook middleware`
Expected: all pass. (`plan_shopping_test` reads `extra_items`; the added `edit_text` field must not break it. If its assertions compare whole objects, update them to include `edit_text`.) If `--bin cook middleware` matches nothing, run `cargo test middleware` instead.

- [ ] **Step 5: Commit**

```bash
[ "$(git branch --show-current)" = feat/shopping-list-manual-items ] && git add src crates tests Cargo.lock && git commit -m "feat(server): add, recognise and edit typed shopping list items"
```

---

### Task 5: API docs

**Files:**
- Modify: `src/web/api_docs.rs` (shopping list section, after the `remove_extra_item` entry ~line 1398; the `extra_items` entry ~1358)
- Regenerate: `docs/api.md`

- [ ] **Step 1: Update `extra_items`**

In the `GET /api/shopping_list/extra_items` entry, add to the description: `` `edit_text` is the line the add field shows to edit the item ("almonds 100 g"); it is the name alone when the quantity is not one the field reads back (`2%bunches`, `a pinch`). `` and change its response example to:

```json
[
  { "name": "almonds", "quantity": "100%g", "edit_text": "almonds 100 g" },
  { "name": "paper towels", "edit_text": "paper towels" }
]
```

(`quantity` is omitted when absent: check how `FreeHandItem` serializes `None` — it has `skip_serializing_if = "Option::is_none"` — and keep the example consistent with that.)

- [ ] **Step 2: Add the three new entries after `remove_extra_item`**

```rust
            ep(
                "POST",
                "/api/shopping_list/recognize",
                "Read typed lines without adding them",
                "Reads each line as the add field would store it, using the aisle file, and \
                 changes nothing. Send `lines` (one entry per line, in order; a line with no \
                 name answers `name: \"\"`) or `text` (pasted text, split into lines with \
                 bullets, checkboxes, Markdown headings and blank lines dropped; one entry per \
                 kept line). At most 200 lines of 500 characters. `quantity` is the stored \
                 form, `chip` the amount as the page shows it, `category` the aisle the name \
                 matched, `edit_text` the line that reads back as the item.",
            )
            .params(vec![
                param("lines", "body", "string[]", false, "Lines to read."),
                param("text", "body", "string", false, "Pasted text to split and read, instead of `lines`."),
            ])
            .request(
                r#"
{ "lines": ["Sugar 1.5kg", "Tomato 3"] }
"#,
            )
            .response(
                r#"
[
  { "name": "sugar", "quantity": "1.5%kg", "chip": "1.5 kg", "category": "baking", "edit_text": "sugar 1.5 kg" },
  { "name": "tomatoes", "quantity": "3", "chip": "x3", "category": "veg", "edit_text": "tomatoes 3" }
]
"#,
            ),
            ep(
                "POST",
                "/api/shopping_list/add_items",
                "Add typed lines as free-hand items",
                "Reads each line as `recognize` does and adds it as a free-hand line of \
                 `.shopping-list`. An item joins a free-hand line with the same name (ignoring \
                 case) and unit, adding up the amounts; otherwise it is a new line. A ticked \
                 row an item lands on is unticked. `row` is the name of the row the item lands \
                 on in the generated list. `400` when no line has a name, or for more than 200 \
                 lines or a line over 500 characters.",
            )
            .params(vec![param("lines", "body", "string[]", true, "Typed lines, such as `Milk 2 l`.")])
            .request(
                r#"
{ "lines": ["Sugar 1.5kg", "Paper towels"] }
"#,
            )
            .response(
                r#"
{ "added": [
  { "name": "sugar", "quantity": "1.5%kg", "row": "sugar" },
  { "name": "Paper towels", "quantity": null, "row": "Paper towels" }
] }
"#,
            ),
            ep(
                "POST",
                "/api/shopping_list/edit_extra_item",
                "Change one free-hand item",
                "Replaces the first free-hand item with this name and quantity by what `line` \
                 says, in place, folding it into another line with the same name and unit. An \
                 item whose `edit_text` is its name alone keeps its quantity unless `line` has \
                 an amount. `404` when the item is no longer on the list, `400` when `line` has \
                 no name.",
            )
            .params(vec![
                param("name", "body", "string", true, "The item's name, as `extra_items` returned it."),
                param("quantity", "body", "string", false, "Its quantity as `extra_items` returned it; omit for none."),
                param("line", "body", "string", true, "The edited line, such as `oat milk 1 l`."),
            ])
            .request(
                r#"
{ "name": "milk", "quantity": "2%l", "line": "oat milk 1 l" }
"#,
            )
            .response(
                r#"
{ "name": "oat milk", "quantity": "1%l", "row": "oat milk" }
"#,
            ),
```

Make the `add_items` response example match what the handler really sends for `quantity: None` (`WrittenItem` has no `skip_serializing_if`, so `null`).

- [ ] **Step 3: Regenerate and check**

Run: `UPDATE_API_DOCS=1 cargo test --test api_docs_md_test && cargo test --test api_docs_md_test`
Expected: the first run rewrites `docs/api.md`, the second passes.

- [ ] **Step 4: Commit**

```bash
[ "$(git branch --show-current)" = feat/shopping-list-manual-items ] && git add src/web/api_docs.rs docs/api.md && git commit -m "docs(api): typed shopping list items"
```

---

### Task 6: Translations

**Files:**
- Modify: `locales/*/shopping.ftl` (10 files)

- [ ] **Step 1: Add the messages**

In each `locales/<lang>/shopping.ftl`, change the `shopping-extra-items` line and add the seven new messages right after it. Exact text per locale:

`en-US`:
```
shopping-extra-items = Other items
shopping-add-placeholder = Add an item, e.g. Milk 2 l
shopping-add-label = Add an item to the shopping list
shopping-paste-title = Pasted items
shopping-paste-add = Add items
shopping-paste-remove = Leave this line out
shopping-paste-restore = Put this line back
shopping-item-changed = This item changed elsewhere, so the list was reloaded. Try the edit again.
```

`de-DE`:
```
shopping-extra-items = Weitere Artikel
shopping-add-placeholder = Artikel hinzufügen, z. B. Milch 2 l
shopping-add-label = Artikel zur Einkaufsliste hinzufügen
shopping-paste-title = Eingefügte Artikel
shopping-paste-add = Artikel hinzufügen
shopping-paste-remove = Diese Zeile weglassen
shopping-paste-restore = Diese Zeile wieder aufnehmen
shopping-item-changed = Dieser Artikel wurde anderswo geändert, die Liste wurde neu geladen. Bitte erneut bearbeiten.
```

`el-GR`:
```
shopping-extra-items = Άλλα είδη
shopping-add-placeholder = Προσθέστε ένα είδος, π.χ. Γάλα 2 l
shopping-add-label = Προσθήκη είδους στη λίστα αγορών
shopping-paste-title = Είδη από επικόλληση
shopping-paste-add = Προσθήκη ειδών
shopping-paste-remove = Παράλειψη αυτής της γραμμής
shopping-paste-restore = Επαναφορά αυτής της γραμμής
shopping-item-changed = Αυτό το είδος άλλαξε αλλού, γι' αυτό η λίστα ανανεώθηκε. Δοκιμάστε ξανά.
```

`es-ES`:
```
shopping-extra-items = Otros artículos
shopping-add-placeholder = Añade un artículo, p. ej. Leche 2 l
shopping-add-label = Añadir un artículo a la lista de la compra
shopping-paste-title = Artículos pegados
shopping-paste-add = Añadir artículos
shopping-paste-remove = Omitir esta línea
shopping-paste-restore = Recuperar esta línea
shopping-item-changed = Este artículo cambió en otro lugar y la lista se ha recargado. Vuelve a intentarlo.
```

`eu-ES`:
```
shopping-extra-items = Beste elementu batzuk
shopping-add-placeholder = Gehitu elementu bat, adib. Esnea 2 l
shopping-add-label = Gehitu elementu bat erosketa-zerrendara
shopping-paste-title = Itsatsitako elementuak
shopping-paste-add = Gehitu elementuak
shopping-paste-remove = Utzi lerro hau kanpoan
shopping-paste-restore = Berreskuratu lerro hau
shopping-item-changed = Elementu hau beste nonbait aldatu da, eta zerrenda berriro kargatu da. Saiatu berriro.
```

`fr-FR`:
```
shopping-extra-items = Autres articles
shopping-add-placeholder = Ajouter un article, p. ex. Lait 2 l
shopping-add-label = Ajouter un article à la liste de courses
shopping-paste-title = Articles collés
shopping-paste-add = Ajouter les articles
shopping-paste-remove = Ignorer cette ligne
shopping-paste-restore = Rétablir cette ligne
shopping-item-changed = Cet article a été modifié ailleurs ; la liste a été rechargée. Réessayez.
```

`it-IT`:
```
shopping-extra-items = Altri articoli
shopping-add-placeholder = Aggiungi un articolo, ad es. Latte 2 l
shopping-add-label = Aggiungi un articolo alla lista della spesa
shopping-paste-title = Articoli incollati
shopping-paste-add = Aggiungi articoli
shopping-paste-remove = Escludi questa riga
shopping-paste-restore = Ripristina questa riga
shopping-item-changed = Questo articolo è stato modificato altrove e la lista è stata ricaricata. Riprova.
```

`ja-JP` (its `shopping-extra-items` is on line 22):
```
shopping-extra-items = その他の品目
shopping-add-placeholder = 品目を追加（例: 牛乳 2 l）
shopping-add-label = 買い物リストに品目を追加
shopping-paste-title = 貼り付けた品目
shopping-paste-add = 品目を追加
shopping-paste-remove = この行を除外
shopping-paste-restore = この行を戻す
shopping-item-changed = この品目は別の場所で変更されたため、リストを再読み込みしました。もう一度編集してください。
```

`nl-NL`:
```
shopping-extra-items = Overige artikelen
shopping-add-placeholder = Voeg een artikel toe, bijv. Melk 2 l
shopping-add-label = Een artikel aan de boodschappenlijst toevoegen
shopping-paste-title = Geplakte artikelen
shopping-paste-add = Artikelen toevoegen
shopping-paste-remove = Deze regel weglaten
shopping-paste-restore = Deze regel terugzetten
shopping-item-changed = Dit artikel is elders gewijzigd, de lijst is opnieuw geladen. Probeer het nog eens.
```

`sv-SE`:
```
shopping-extra-items = Övriga varor
shopping-add-placeholder = Lägg till en vara, t.ex. Mjölk 2 l
shopping-add-label = Lägg till en vara på inköpslistan
shopping-paste-title = Inklistrade varor
shopping-paste-add = Lägg till varor
shopping-paste-remove = Hoppa över den här raden
shopping-paste-restore = Ta tillbaka den här raden
shopping-item-changed = Varan har ändrats någon annanstans, så listan har laddats om. Försök igen.
```

- [ ] **Step 2: Run the locale test**

Run: `cargo test --test locales_test`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
[ "$(git branch --show-current)" = feat/shopping-list-manual-items ] && git add locales && git commit -m "feat(web): strings for typing items into the shopping list"
```

---

### Task 7: The page — add field, paste card, edit, highlight

**Files:**
- Modify: `static/css/components.css` (inside `@layer components`, at the end of the layer)
- Create: `static/js/shopping-manual-items.js`
- Modify: `templates/shopping_list.html`

- [ ] **Step 1: CSS**

Append inside the `@layer components { … }` block of `static/css/components.css`:

```css
    /* A row just added from the add field: tinted for 2 s, then fading. */
    .row-flashable {
        transition: background-color 0.5s ease-out;
    }
    .row-flash {
        background-color: var(--color-accent-soft);
    }
    @media (prefers-reduced-motion: reduce) {
        .row-flashable {
            transition: none;
        }
    }
```

Run: `npm run build-css`
Expected: builds; `grep -c row-flash static/css/output.css` prints at least 1.

- [ ] **Step 2: Write `static/js/shopping-manual-items.js`**

```js
// The shopping list's add field: type "Milk 2 l" or paste a list, edit an
// item from "Other items", and see new rows highlighted. Loaded by
// templates/shopping_list.html, which calls `CookManualItems.init(config)`
// once and `CookManualItems.afterRender()` after every render of the list.
//
// config: {
//   prefix,                      // URL prefix of the server
//   strings: { add, save, remove, restore, itemChanged },
//   refresh(),                   // the page's requestRefresh('list'), a promise
//   showError(message), hideError(),
// }
(function () {
    'use strict';

    const HIGHLIGHT_MS = 2000;
    const RECOGNIZE_DELAY_MS = 150;

    let config = null;
    // `{ name, quantity }` of the item the field is editing, or null.
    let editing = null;
    // Lowercase row name -> time (ms) its highlight ends.
    const highlights = new Map();
    // Lowercase row name to scroll to on the next render.
    let pendingScroll = null;

    function el(id) {
        return document.getElementById(id);
    }

    async function post(path, body) {
        const response = await fetch(config.prefix + '/api/shopping_list/' + path, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(body),
        });
        const data = await response.json().catch(() => ({}));
        return { response, data };
    }

    function chipText(entry) {
        if (!entry) return '';
        return [entry.chip, entry.category].filter(Boolean).join(' · ');
    }

    function setChip(chip, entry) {
        const text = chipText(entry);
        chip.textContent = text;
        chip.classList.toggle('hidden', text === '');
    }

    // Ask the server how `input`'s line reads, 150 ms after the last change,
    // and show it in `chip`. An answer for older text is dropped.
    function recognizeInto(input, chip) {
        const state = input._recognize || (input._recognize = { timer: null, seq: 0 });
        clearTimeout(state.timer);
        const seq = ++state.seq;
        const line = input.value;
        if (line.trim() === '') {
            setChip(chip, null);
            return;
        }
        state.timer = setTimeout(async () => {
            let entry = null;
            try {
                const { response, data } = await post('recognize', { lines: [line] });
                if (response.ok && Array.isArray(data)) entry = data[0];
            } catch (_) {
                // No chip; adding still works.
            }
            if (seq === state.seq) setChip(chip, entry);
        }, RECOGNIZE_DELAY_MS);
    }

    function updateSubmit() {
        el('manual-add-submit').disabled = el('manual-add-input').value.trim() === '';
    }

    function flash(rows) {
        const until = Date.now() + HIGHLIGHT_MS;
        rows.filter(Boolean).forEach(row => highlights.set(row.toLowerCase(), until));
        if (rows[0]) pendingScroll = rows[0].toLowerCase();
    }

    async function addLines(lines) {
        let result;
        try {
            result = await post('add_items', { lines });
        } catch (error) {
            config.showError('Failed to add items');
            return false;
        }
        const { response, data } = result;
        if (!response.ok) {
            config.showError(data.error || 'Failed to add items');
            return false;
        }
        config.hideError();
        flash((data.added || []).map(item => item.row));
        await config.refresh();
        return true;
    }

    async function saveEdit(line) {
        let result;
        try {
            result = await post('edit_extra_item', { name: editing.name, quantity: editing.quantity, line });
        } catch (error) {
            config.showError('Failed to save the item');
            return false;
        }
        const { response, data } = result;
        if (response.status === 404) {
            config.showError(config.strings.itemChanged);
            await config.refresh();
            return true;
        }
        if (!response.ok) {
            config.showError(data.error || 'Failed to save the item');
            return false;
        }
        config.hideError();
        flash([data.row]);
        await config.refresh();
        return true;
    }

    function clearField() {
        const input = el('manual-add-input');
        input.value = '';
        if (input._recognize) {
            clearTimeout(input._recognize.timer);
            input._recognize.seq++;
        }
        setChip(el('manual-add-chip'), null);
        updateSubmit();
    }

    function stopEditing() {
        editing = null;
        el('manual-add-submit').textContent = config.strings.add;
        el('manual-add-cancel').classList.add('hidden');
        clearField();
    }

    function startEditing(item) {
        closePaste();
        editing = { name: item.name, quantity: item.quantity };
        const input = el('manual-add-input');
        input.value = item.editText;
        el('manual-add-submit').textContent = config.strings.save;
        el('manual-add-cancel').classList.remove('hidden');
        updateSubmit();
        recognizeInto(input, el('manual-add-chip'));
        input.focus();
        input.select();
    }

    async function submit(event) {
        event.preventDefault();
        const input = el('manual-add-input');
        const line = input.value.trim();
        if (line === '') return;
        el('manual-add-submit').disabled = true;
        const ok = editing ? await saveEdit(line) : await addLines([line]);
        if (ok) {
            if (editing) stopEditing();
            else clearField();
        } else {
            updateSubmit();
        }
        input.focus();
    }

    // -- Paste preview ------------------------------------------------------

    function pastedLines(text) {
        return text.split(/\r\n|\r|\n| | /).filter(line => line.trim() !== '');
    }

    function updatePasteAdd() {
        const kept = el('manual-paste-rows').querySelectorAll('li:not([data-removed])');
        el('manual-paste-add').disabled = kept.length === 0;
    }

    function pasteRow(entry) {
        const li = document.createElement('li');
        li.className = 'flex items-center gap-2';

        const toggle = document.createElement('button');
        toggle.type = 'button';
        toggle.className = 'icon-btn';
        toggle.textContent = '×';
        toggle.setAttribute('aria-label', config.strings.remove);

        const input = document.createElement('input');
        input.type = 'text';
        input.className = 'search-input flex-1';
        input.value = entry.edit_text;
        input.maxLength = 500;
        input.setAttribute('aria-label', entry.edit_text);

        const chip = document.createElement('span');
        chip.className = 'tag';
        setChip(chip, entry);

        input.addEventListener('input', () => recognizeInto(input, chip));
        toggle.addEventListener('click', () => {
            const removed = li.toggleAttribute('data-removed');
            input.disabled = removed;
            input.classList.toggle('line-through', removed);
            input.classList.toggle('text-faint', removed);
            toggle.textContent = removed ? '↺' : '×';
            toggle.setAttribute('aria-label', removed ? config.strings.restore : config.strings.remove);
            updatePasteAdd();
        });

        li.append(toggle, input, chip);
        return li;
    }

    function closePaste() {
        el('manual-paste').classList.add('hidden');
        el('manual-paste-rows').replaceChildren();
    }

    async function openPaste(text) {
        let entries = [];
        try {
            const { response, data } = await post('recognize', { text });
            if (response.ok && Array.isArray(data)) entries = data;
        } catch (_) {
            // Fall through to the field below.
        }
        if (entries.length === 0) {
            const input = el('manual-add-input');
            input.value = text.replace(/\s+/g, ' ').trim();
            recognizeInto(input, el('manual-add-chip'));
            updateSubmit();
            return;
        }
        el('manual-paste-rows').replaceChildren(...entries.map(pasteRow));
        el('manual-paste').classList.remove('hidden');
        updatePasteAdd();
    }

    async function addPasted() {
        const lines = Array.from(el('manual-paste-rows').querySelectorAll('li:not([data-removed]) input'))
            .map(input => input.value.trim())
            .filter(line => line !== '');
        if (lines.length === 0) return;
        el('manual-paste-add').disabled = true;
        if (await addLines(lines)) {
            closePaste();
            clearField();
        } else {
            updatePasteAdd();
        }
    }

    function onPaste(event) {
        if (editing) return;
        const data = event.clipboardData || window.clipboardData;
        const text = data ? data.getData('text') : '';
        if (pastedLines(text).length < 2) return;
        event.preventDefault();
        openPaste(text);
    }

    // -- After each render of the list --------------------------------------

    function afterRender() {
        const now = Date.now();
        let scrollTarget = null;
        document.querySelectorAll('#list-content [data-ingredient-name]').forEach(box => {
            const key = box.dataset.ingredientName.toLowerCase();
            const until = highlights.get(key);
            const row = box.closest('li');
            if (!row || !until) return;
            if (until <= now) {
                highlights.delete(key);
                return;
            }
            row.classList.add('row-flashable', 'row-flash');
            setTimeout(() => row.classList.remove('row-flash'), until - now);
            if (key === pendingScroll) scrollTarget = row;
        });
        if (scrollTarget) {
            const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
            scrollTarget.scrollIntoView({ block: 'nearest', behavior: reduce ? 'auto' : 'smooth' });
            pendingScroll = null;
        }
    }

    function init(options) {
        if (!el('manual-add-form')) return null;
        config = options;
        const input = el('manual-add-input');
        el('manual-add-form').addEventListener('submit', submit);
        input.addEventListener('input', () => {
            updateSubmit();
            recognizeInto(input, el('manual-add-chip'));
        });
        input.addEventListener('paste', onPaste);
        input.addEventListener('keydown', event => {
            if (event.key !== 'Escape') return;
            if (editing) stopEditing();
            else if (!el('manual-paste').classList.contains('hidden')) closePaste();
        });
        el('manual-add-cancel').addEventListener('click', () => {
            stopEditing();
            input.focus();
        });
        el('manual-paste-cancel').addEventListener('click', () => {
            closePaste();
            input.focus();
        });
        el('manual-paste-add').addEventListener('click', addPasted);
        updateSubmit();
        return api;
    }

    const api = { init, afterRender, edit: startEditing };
    window.CookManualItems = api;
})();
```

- [ ] **Step 3: Template — markup**

In `templates/shopping_list.html`, between the closing `</div>` of `#shopping-list-header` and `<div id="shopping-list-results">` (around line 103), insert:

```html
        {% if viewer.can_edit_lists() %}
        <div id="manual-add" class="mb-6" data-unsaved>
            <form id="manual-add-form" class="flex items-center gap-2" autocomplete="off">
                <label for="manual-add-input" class="sr-only">{{ tr.t("shopping-add-label") }}</label>
                <input id="manual-add-input" name="item" type="text" maxlength="500" enterkeyhint="done"
                       class="search-input flex-1" placeholder="{{ tr.t("shopping-add-placeholder") }}">
                <span id="manual-add-chip" class="tag hidden" aria-live="polite"></span>
                <button id="manual-add-cancel" type="button" class="btn hidden">{{ tr.t("action-cancel") }}</button>
                <button id="manual-add-submit" type="submit" class="btn btn-primary" disabled>{{ tr.t("action-add") }}</button>
            </form>
            <div id="manual-paste" class="hidden card p-4 mt-3">
                <h2 class="font-semibold text-title mb-3 text-text">{{ tr.t("shopping-paste-title") }}</h2>
                <ul id="manual-paste-rows" class="space-y-2"></ul>
                <div class="flex justify-end gap-2 mt-4">
                    <button id="manual-paste-cancel" type="button" class="btn">{{ tr.t("action-cancel") }}</button>
                    <button id="manual-paste-add" type="button" class="btn btn-primary">{{ tr.t("shopping-paste-add") }}</button>
                </div>
            </div>
        </div>
        {% endif %}
```

Check that `search-input`, `tag`, `icon-btn` and `sr-only` exist (`grep -n "search-input\|\.tag\b\|icon-btn" static/css/components.css`; `sr-only` is a Tailwind utility). If `.search-input` carries a search icon or padding meant for the search box only, use the class the pantry page's text inputs use instead (`grep -n "<input" templates/pantry.html`).

- [ ] **Step 4: Template — script include and wiring**

Directly before the page's inline `<script>` (the one starting `let shoppingList = [];`, ~line 110) add:

```html
<script src="{{ prefix }}/static/js/shopping-manual-items.js"></script>
```

At the end of the inline script (after the existing event-stream setup), add:

```js
// The add field (static/js/shopping-manual-items.js); absent for someone who
// may not edit lists.
const manualItems = window.CookManualItems
    ? window.CookManualItems.init({
        prefix: '{{ prefix }}',
        strings: {
            add: {{ tr.t("action-add")|json|safe }},
            save: {{ tr.t("action-save")|json|safe }},
            remove: {{ tr.t("shopping-paste-remove")|json|safe }},
            restore: {{ tr.t("shopping-paste-restore")|json|safe }},
            itemChanged: {{ tr.t("shopping-item-changed")|json|safe }},
        },
        refresh: () => requestRefresh('list'),
        showError,
        hideError,
    })
    : null;
```

At the very end of `displayShoppingList` (after `applyCheckedStates();`) add:

```js
    if (manualItems) manualItems.afterRender();
```

`manualItems` is a top-level `const` declared later in the same script; `displayShoppingList` only runs after the script has finished loading (from `requestRefresh`), so it is initialised by then. If a render can happen synchronously during script evaluation (check where the first `requestRefresh` is called), move the `const manualItems = …` block above that call.

- [ ] **Step 5: Template — Edit button in "Other items"**

In `extraItemsHtml()` replace the `const remove = …;` statement and the `return` of the row with:

```js
        const data = `data-name="${escHtml(item.name)}" data-quantity="${escHtml(item.quantity == null ? '' : item.quantity)}" data-has-quantity="${item.quantity == null ? '' : '1'}"`;
        const actions = window.__CAN_EDIT_LISTS__
            ? `<span class="flex items-center gap-3">
                   <button type="button" data-action="edit-extra" ${data} data-edit-text="${escHtml(item.edit_text || item.name)}" class="text-accent-text hover:underline font-medium text-sm">{{ tr.t("action-edit") }}</button>
                   <button type="button" data-action="remove-extra" ${data} class="text-danger hover:underline font-medium text-sm">{{ tr.t("shopping-remove") }}</button>
               </span>`
            : '';
        return `<li class="flex items-center justify-between gap-2 text-sm text-text"><span>${escHtml(item.name)}<span class="text-muted">${amount}</span></span>${actions}</li>`;
```

and next to the existing `remove-extra` click listener add:

```js
document.addEventListener('click', (event) => {
    const btn = event.target.closest('[data-action="edit-extra"]');
    if (!btn || !manualItems) return;
    manualItems.edit({
        name: btn.dataset.name,
        quantity: btn.dataset.hasQuantity ? btn.dataset.quantity : null,
        editText: btn.dataset.editText,
    });
});
```

Check `text-accent-text` exists as a utility (`grep -n "accent-text" static/css/input.css`); it is used for headings on this page already.

- [ ] **Step 6: Build and look at it**

Run: `npm run build-css && cargo build && ./target/debug/cook server ./seed --port 9081` (in the background), open `http://localhost:9081/shopping-list` with Playwright or a browser, then stop the server. Check by hand:
- typing `Sugar 1.5kg` shows the chip "1.5 kg · packaged goods, pasta and sauces" (the seed aisle's category for sugar);
- Enter adds it, the field clears and keeps focus, the `sugar` row is tinted and fades;
- pasting three lines opens the card; removing one strikes it through; "Add items" adds two;
- Edit in "Other items" fills the field with `sugar 1.5 kg`, the button reads Save, Esc cancels;
- leaving the page with text typed asks first.

Afterwards restore the seed: `git checkout -- seed/ && rm -f seed/.shopping-list seed/.shopping-checked` only if `git status` shows those as changed or new by this step (check first, do not delete files that existed before).

- [ ] **Step 7: Commit**

```bash
[ "$(git branch --show-current)" = feat/shopping-list-manual-items ] && git add static/css/components.css static/js/shopping-manual-items.js templates/shopping_list.html && git commit -m "feat(web): type, paste and edit items on the shopping list"
```

---

### Task 8: Playwright

**Files:**
- Create: `tests/e2e/shopping-list-manual-items.spec.ts`

- [ ] **Step 1: Write the spec**

```ts
import { test, expect, Page } from '@playwright/test';

// Typing items into the shopping list. Shares the seed's `.shopping-list`
// with the other shopping list specs: every item here starts with "E2E ",
// and only those are taken off afterwards.

async function removeOurs(request: import('@playwright/test').APIRequestContext) {
  const extra = await (await request.get('/api/shopping_list/extra_items')).json();
  for (const item of extra) {
    if (String(item.name).startsWith('E2E ')) {
      await request.post('/api/shopping_list/remove_extra_item', {
        data: { name: item.name, quantity: item.quantity ?? null },
      });
    }
  }
}

async function paste(page: Page, text: string) {
  await page.locator('#manual-add-input').evaluate((input, value) => {
    const data = new DataTransfer();
    data.setData('text/plain', value);
    input.dispatchEvent(new ClipboardEvent('paste', { clipboardData: data, bubbles: true, cancelable: true }));
  }, text);
}

function row(page: Page, name: string) {
  return page.locator(`#list-content li:has([data-ingredient-name="${name}"])`);
}

test.describe('Typed items on the shopping list', () => {
  test.beforeEach(async ({ request }) => removeOurs(request));
  test.afterEach(async ({ request }) => removeOurs(request));

  test('typing shows the amount and Enter adds the item', async ({ page }) => {
    await page.goto('/shopping-list');
    const input = page.locator('#manual-add-input');
    await input.fill('E2E oat drink 1.5l');
    await expect(page.locator('#manual-add-chip')).toHaveText('1.5 l');
    await input.press('Enter');
    await expect(input).toHaveValue('');
    await expect(input).toBeFocused();
    await expect(row(page, 'E2E oat drink')).toContainText('1.5');
    await expect(row(page, 'E2E oat drink')).toHaveClass(/row-flash/);
    await expect(page.locator('#extra-items')).toContainText('E2E oat drink');
  });

  test('pasting a list previews it, and removed lines stay out', async ({ page, request }) => {
    await page.goto('/shopping-list');
    await paste(page, '- E2E rye bread\n- E2E lemons 3\n- E2E basil');
    const rows = page.locator('#manual-paste-rows li');
    await expect(rows).toHaveCount(3);
    await rows.nth(2).getByRole('button').click();
    await page.locator('#manual-paste-add').click();
    await expect(page.locator('#manual-paste')).toBeHidden();
    await expect(row(page, 'E2E rye bread')).toBeVisible();
    await expect(row(page, 'E2E lemons')).toBeVisible();
    const extra = await (await request.get('/api/shopping_list/extra_items')).json();
    const names = extra.map((item: { name: string }) => item.name);
    expect(names).toContain('E2E lemons');
    expect(names).not.toContain('E2E basil');
  });

  test('an item is edited from Other items', async ({ page, request }) => {
    await request.post('/api/shopping_list/add_items', { data: { lines: ['E2E rye flour 1 kg'] } });
    await page.goto('/shopping-list');
    await page.locator('[data-action="edit-extra"][data-name="E2E rye flour"]').click();
    const input = page.locator('#manual-add-input');
    await expect(input).toHaveValue('E2E rye flour 1 kg');
    await input.fill('E2E rye flour 2 kg');
    await input.press('Enter');
    await expect(page.locator('#manual-add-cancel')).toBeHidden();
    await expect.poll(async () => {
      const extra = await (await request.get('/api/shopping_list/extra_items')).json();
      return extra.find((item: { name: string }) => item.name === 'E2E rye flour')?.quantity;
    }).toBe('2%kg');
  });

  test('an item is removed from Other items', async ({ page, request }) => {
    await request.post('/api/shopping_list/add_items', { data: { lines: ['E2E capers'] } });
    await page.goto('/shopping-list');
    await page.locator('[data-action="remove-extra"][data-name="E2E capers"]').click();
    await expect(page.locator('#extra-items')).not.toContainText('E2E capers');
  });
});
```

Check the page URL the other shopping list specs use (`grep -rn "goto(" tests/e2e/shopping-list.spec.ts | head -3`) and use the same.

- [ ] **Step 2: Run it, then the whole project**

Run: `npx playwright test tests/e2e/shopping-list-manual-items.spec.ts --project=chromium --workers=1`
Expected: 4 passed.

Run: `npx playwright test --project=chromium`
Expected: all pass. The "Other items" rename and the new field can affect specs that select by the "Extra items" text or count inputs on the shopping list page; fix those selectors if they fail. Retry a shopping-list or pantry failure with `--workers=1` before blaming the change (they share `./seed`).

Afterwards: `git status` — restore `seed/config/aisle.conf` and `seed/config/pantry.conf` if a spec modified them (`git checkout -- seed/config/`), and make sure no `E2E ` lines remain in `seed/.shopping-list`.

- [ ] **Step 3: Commit**

```bash
[ "$(git branch --show-current)" = feat/shopping-list-manual-items ] && git add tests/e2e && git commit -m "test(e2e): type, paste, edit and remove shopping list items"
```

---

### Task 9: Final checks

- [ ] **Step 1: Format, lint, test**

Run:
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --no-fail-fast
cargo build --no-default-features --features server,lsp
```
Expected: no formatting changes left, no clippy warnings, all tests pass, the Docker feature set builds. Fix anything in files this branch changed.

- [ ] **Step 2: Docs**

In `docs/server.md`, in the bullet list where line ~100 says "The web interface supports recipe browsing, scaling, search, editing, and shopping list management", add a bullet after it:

```markdown
- Items no recipe calls for can be typed into the shopping list ("Milk 2 l", "2l milk"), or pasted as a list. The amount is recognised as you type, the name takes the aisle file's spelling, and the item is stored as a free-hand line of `.shopping-list` that the Cooklang apps read too. Edit or remove them under **Other items**
```

- [ ] **Step 3: Commit**

```bash
[ "$(git branch --show-current)" = feat/shopping-list-manual-items ] && git add -A docs src crates tests templates static locales && git commit -m "docs: typing items into the shopping list"
```

(Stage only what changed; check `git status` first and leave out anything unrelated, such as `seed/` changes.)
