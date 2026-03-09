# Today's Menu Banner Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Show a hero banner on the root recipes page when a menu file has a section dated today.

**Architecture:** Add a `find_todays_menu` function in `handlers/menus.rs` that scans all menus for today's date in section headers. Pass the result as `Option<TodaysMenu>` to `RecipesTemplate`. Render conditionally in `recipes.html`.

**Tech Stack:** Rust (Axum, Askama, chrono), Tailwind CSS, Fluent i18n

---

### Task 1: Add `TodaysMenu` struct and update `RecipesTemplate`

**Files:**
- Modify: `src/server/templates.rs:47-55`

**Step 1: Add `TodaysMenu` struct above `RecipesTemplate`**

After line 46 (after `ErrorTemplate`), add:

```rust
pub struct TodaysMenu {
    pub menu_name: String,
    pub menu_path: String,
    pub date_display: String,
    pub meals: Vec<String>,
}
```

**Step 2: Add `todays_menu` field to `RecipesTemplate`**

Add to `RecipesTemplate` struct (after `items` field, before `tr`):

```rust
pub todays_menu: Option<TodaysMenu>,
```

**Step 3: Verify it compiles**

Run: `cargo build 2>&1 | head -30`
Expected: Compile errors in `ui.rs` where `RecipesTemplate` is constructed (missing field). This is expected — we'll fix it in Task 3.

**Step 4: Commit**

```bash
git add src/server/templates.rs
git commit -m "feat: add TodaysMenu struct to templates"
```

---

### Task 2: Add `find_todays_menu` function in menus handler

**Files:**
- Modify: `src/server/handlers/menus.rs`
- Modify: `src/server/handlers/mod.rs:9`

**Step 1: Make `collect_menus` and `extract_date` public**

In `src/server/handlers/menus.rs`:
- Change `fn collect_menus(` to `pub fn collect_menus(`  (line 30)
- Change `fn extract_date(` to `pub fn extract_date(`  (line 129)
- Change `fn extract_meal_type(` to `pub fn extract_meal_type(`  (line 143)
- Change `fn is_meal_header(` to `pub fn is_meal_header(`  (line 151)

**Step 2: Add re-exports in `handlers/mod.rs`**

Change line 9 from:
```rust
pub use menus::{get_menu, list_menus};
```
to:
```rust
pub use menus::{collect_menus, extract_date, extract_meal_type, get_menu, is_meal_header, list_menus};
```

**Step 3: Add `find_todays_menu` function**

At the end of `src/server/handlers/menus.rs` (before the `LineItem` enum), add:

```rust
/// Scan all menus for a section whose date matches today.
/// Returns the first match with menu name, path, formatted date, and meal types.
pub fn find_todays_menu(
    base_path: &camino::Utf8Path,
    tree: &RecipeTree,
) -> Option<crate::server::templates::TodaysMenu> {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let today_display = chrono::Local::now().format("%A, %B %-d").to_string();

    let mut menus = Vec::new();
    collect_menus(tree, base_path, &mut menus);

    for menu_item in &menus {
        let recipe_path = camino::Utf8PathBuf::from(&menu_item.path);
        let entry = match cooklang_find::get_recipe(vec![base_path], &recipe_path) {
            Ok(e) => e,
            Err(_) => continue,
        };

        let recipe = match crate::util::parse_recipe_from_entry(&entry, 1.0) {
            Ok(r) => r,
            Err(_) => continue,
        };

        for section in &recipe.sections {
            let date = section.name.as_deref().and_then(extract_date);
            if date.as_deref() == Some(today.as_str()) {
                // Found today's section — extract meal types
                let mut meals = Vec::new();
                for content in &section.content {
                    use cooklang::Content;
                    if let Content::Step(step) = content {
                        for item in &step.items {
                            use cooklang::Item;
                            if let Item::Text { value } = item {
                                for line in value.lines() {
                                    if is_meal_header(line) {
                                        meals.push(extract_meal_type(line));
                                    }
                                }
                            }
                        }
                    }
                }

                return Some(crate::server::templates::TodaysMenu {
                    menu_name: menu_item.name.clone(),
                    menu_path: menu_item.path.clone(),
                    date_display: today_display,
                    meals,
                });
            }
        }
    }

    None
}
```

**Step 4: Add re-export for `find_todays_menu`**

In `src/server/handlers/mod.rs`, update the menus re-export line to also include `find_todays_menu`:

```rust
pub use menus::{collect_menus, extract_date, extract_meal_type, find_todays_menu, get_menu, is_meal_header, list_menus};
```

**Step 5: Add `use chrono;` if not already imported**

Check top of `menus.rs` — chrono is not currently imported. No explicit import needed since we use the fully qualified `chrono::Local`.

**Step 6: Verify it compiles**

Run: `cargo build 2>&1 | head -30`
Expected: Still fails on `RecipesTemplate` missing field (from Task 1). The new function itself should have no errors.

**Step 7: Commit**

```bash
git add src/server/handlers/menus.rs src/server/handlers/mod.rs
git commit -m "feat: add find_todays_menu function"
```

---

### Task 3: Wire up `recipes_handler` to pass today's menu to template

**Files:**
- Modify: `src/server/ui.rs:51-167`

**Step 1: Call `find_todays_menu` when on root page**

In `recipes_handler` function, after the `items.sort_by` block (around line 133) and before building `breadcrumbs`, add:

```rust
let todays_menu = if path.is_none() {
    // Only check for today's menu on the root page
    // We already have `tree` for the current path, but for today's menu
    // we need the full tree from base path
    crate::server::handlers::find_todays_menu(base, &tree)
} else {
    None
};
```

Note: When `path.is_none()`, `search_path == base`, so `tree` is already the full tree. No extra tree build needed.

**Step 2: Pass `todays_menu` to `RecipesTemplate`**

Update the `RecipesTemplate` construction (around line 158) to include the new field:

```rust
let template = RecipesTemplate {
    active: "recipes".to_string(),
    current_name,
    breadcrumbs,
    items,
    todays_menu,
    tr: Tr::new(lang),
};
```

**Step 3: Verify it compiles**

Run: `cargo build 2>&1 | head -30`
Expected: Compiles successfully.

**Step 4: Commit**

```bash
git add src/server/ui.rs
git commit -m "feat: pass today's menu data to recipes template"
```

---

### Task 4: Add i18n translation keys

**Files:**
- Modify: `locales/en-US/recipes.ftl`
- Modify: `locales/de-DE/recipes.ftl`
- Modify: `locales/es-ES/recipes.ftl`
- Modify: `locales/fr-FR/recipes.ftl`
- Modify: `locales/nl-NL/recipes.ftl`

**Step 1: Add keys to `locales/en-US/recipes.ftl`**

Append at the end:

```ftl
# Today's Menu Banner
todays-menu-title = Today's Menu
todays-menu-from = From
todays-menu-view = View Menu
```

**Step 2: Add keys to all other locale files**

For `de-DE/recipes.ftl`:
```ftl
# Today's Menu Banner
todays-menu-title = Heutiges Menü
todays-menu-from = Aus
todays-menu-view = Menü ansehen
```

For `es-ES/recipes.ftl`:
```ftl
# Today's Menu Banner
todays-menu-title = Menú de Hoy
todays-menu-from = De
todays-menu-view = Ver Menú
```

For `fr-FR/recipes.ftl`:
```ftl
# Today's Menu Banner
todays-menu-title = Menu du Jour
todays-menu-from = De
todays-menu-view = Voir le Menu
```

For `nl-NL/recipes.ftl`:
```ftl
# Today's Menu Banner
todays-menu-title = Menu van Vandaag
todays-menu-from = Van
todays-menu-view = Menu bekijken
```

**Step 3: Commit**

```bash
git add locales/
git commit -m "feat: add i18n keys for today's menu banner"
```

---

### Task 5: Add banner template to `recipes.html`

**Files:**
- Modify: `templates/recipes.html`

**Step 1: Add banner markup**

In `templates/recipes.html`, after the header `<div>` with the title and "New Recipe" button (after line 31, before the grid `<div>`), add:

```html
    {% match todays_menu %}
    {% when Some with (menu) %}
    <div class="mb-8 bg-gradient-to-r from-purple-600 to-pink-600 rounded-2xl shadow-lg p-6 text-white">
        <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
            <div class="flex-1">
                <div class="flex items-center gap-3 mb-2">
                    <span class="text-2xl">📋</span>
                    <h2 class="text-xl font-bold">{{ tr.t("todays-menu-title") }}</h2>
                    <span class="text-sm opacity-80">{{ menu.date_display }}</span>
                </div>
                <p class="text-sm opacity-90 mb-3">{{ tr.t("todays-menu-from") }}: {{ menu.menu_name }}</p>
                {% if !menu.meals.is_empty() %}
                <div class="flex flex-wrap gap-2">
                    {% for meal in menu.meals %}
                    <span class="px-3 py-1 bg-white/20 rounded-full text-sm font-medium">{{ meal }}</span>
                    {% endfor %}
                </div>
                {% endif %}
            </div>
            <a href="/recipe/{{ menu.menu_path }}" class="inline-flex items-center gap-2 px-5 py-2.5 bg-white text-purple-700 rounded-xl font-semibold hover:bg-purple-50 transition-colors shadow-md self-start sm:self-center">
                {{ tr.t("todays-menu-view") }}
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"></path>
                </svg>
            </a>
        </div>
    </div>
    {% when None %}
    {% endmatch %}
```

**Step 2: Verify it compiles**

Run: `cargo build 2>&1 | head -30`
Expected: Compiles successfully.

**Step 3: Commit**

```bash
git add templates/recipes.html
git commit -m "feat: add today's menu hero banner to recipes page"
```

---

### Task 6: Manual test with a dated menu file

**Step 1: Create a test menu file with today's date**

Create or modify a `.menu` file in the seed directory to include today's date. For example, edit `seed/2 Day Plan.menu` to change `==Day 1==` to `==Day 1 (YYYY-MM-DD)==` where YYYY-MM-DD is today's actual date.

Alternatively create a new test file `seed/Weekly Plan.menu`:

```
---
servings: 2
---

==Today (YYYY-MM-DD)==

Breakfast:
- @oats{1%cup} with @milk{1/2%cup}

Lunch:
- @./lamb-chops{}

Dinner:
- @./Neapolitan Pizza{}
```

Replace `YYYY-MM-DD` with the actual current date.

**Step 2: Start the dev server**

Run: `cargo run -- server ./seed`

**Step 3: Open browser and verify**

Navigate to `http://localhost:9080/`. Verify:
- Purple gradient banner appears at the top
- Shows today's date formatted nicely
- Shows menu name
- Shows meal type pills (Breakfast, Lunch, Dinner)
- "View Menu" button navigates to the full menu page
- Banner does NOT appear on subdirectory pages (e.g., `/directory/Breakfast`)

**Step 4: Test without matching date**

Remove or change the date in the menu file so it doesn't match today. Refresh the page. Verify the banner does not appear.

**Step 5: Clean up test file if needed**

Revert the seed menu file to its original state if modified.

**Step 6: Commit final state**

```bash
git add -A
git commit -m "feat: today's menu banner on recipes page"
```

---

### Task 7: Run pre-PR checks

**Step 1: Format code**

Run: `cargo fmt`
Expected: No changes (or auto-fixes formatting).

**Step 2: Run clippy**

Run: `cargo clippy 2>&1 | tail -20`
Expected: No warnings related to our changes.

**Step 3: Run tests**

Run: `cargo test`
Expected: All tests pass.

**Step 4: Fix any issues and commit**

If any checks fail, fix the issues and commit:

```bash
git add -A
git commit -m "fix: address clippy/fmt issues in today's menu banner"
```
