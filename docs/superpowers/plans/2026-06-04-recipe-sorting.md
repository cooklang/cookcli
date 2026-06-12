# Recipe Sorting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add client-side sorting (by name, last modified, creation date) to the recipes list page via a field dropdown + direction toggle.

**Architecture:** Rust populates `modified_at`/`created_at` Unix timestamps on each `RecipeItem`; the Askama template embeds them as `data-*` attributes on recipe cards; inline JavaScript handles DOM reordering without any page reload.

**Tech Stack:** Rust (Askama templates, std::fs::metadata), Tailwind CSS, vanilla JavaScript

---

## File Map

| File | Change |
|------|--------|
| `src/server/templates.rs` | Add `modified_at: Option<i64>` and `created_at: Option<i64>` to `RecipeItem` |
| `src/server/builders.rs` | Populate those fields from `std::fs::metadata` when building recipe items |
| `templates/recipes.html` | Add `data-modified`/`data-created`/`data-type` attributes to cards; add sort control UI; add sort JS |

---

## Task 1: Add timestamp fields to `RecipeItem`

**Files:**
- Modify: `src/server/templates.rs:469-479`

- [ ] **Step 1: Add the two new fields to `RecipeItem`**

Open `src/server/templates.rs`. Find `RecipeItem` (around line 469) and add `modified_at` and `created_at`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeItem {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub count: Option<usize>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub image_path: Option<String>,
    pub is_menu: bool,
    pub modified_at: Option<i64>,
    pub created_at: Option<i64>,
}
```

- [ ] **Step 2: Verify it compiles (errors expected in builders.rs — that's fine)**

```bash
cd /Users/romain/Projects/cooklang/cookcli && cargo build 2>&1 | grep "error"
```

Expected: errors about missing fields in `RecipeItem { ... }` struct literals in `builders.rs`. That confirms the field was added and is now required.

---

## Task 2: Populate timestamps in `build_recipes_template`

**Files:**
- Modify: `src/server/builders.rs:64-108`

- [ ] **Step 1: Extract timestamps alongside tags/image in the recipe branch**

In `src/server/builders.rs`, find the block starting at line 64:

```rust
// Extract tags, image, and is_menu if this is a recipe
let (tags, image_path, is_menu) = if let Some(ref recipe) = child.recipe {
```

Replace it with:

```rust
// Extract tags, image, is_menu, and file timestamps if this is a recipe
let (tags, image_path, is_menu, modified_at, created_at) = if let Some(ref recipe) = child.recipe {
    let img_path = recipe.title_image().clone().and_then(|img| {
        if img.starts_with("http://") || img.starts_with("https://") {
            Some(img)
        } else {
            // Make path relative to base and accessible via /api/static
            let img_path = camino::Utf8Path::new(&img);
            if let Ok(relative) = img_path.strip_prefix(base_path) {
                Some(format!("{url_prefix}/api/static/{relative}"))
            } else if !img_path.is_absolute() {
                Some(format!("{url_prefix}/api/static/{img_path}"))
            } else {
                img_path
                    .file_name()
                    .map(|name| format!("{url_prefix}/api/static/{name}"))
            }
        }
    });

    let (modified_at, created_at) = recipe.path().map(|p| {
        let meta = std::fs::metadata(p).ok();
        let modified = meta.as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64);
        let created = meta.as_ref()
            .and_then(|m| m.created().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64);
        (modified, created)
    }).unwrap_or((None, None));

    (recipe.tags(), img_path, recipe.is_menu(), modified_at, created_at)
} else {
    (Vec::new(), None, false, None, None)
};
```

- [ ] **Step 2: Pass the new fields into the `RecipeItem` push**

Still in `builders.rs`, find the `items.push(RecipeItem { ... })` call (around line 88) and add the two new fields:

```rust
items.push(RecipeItem {
    name: name.to_string(),
    path: item_path,
    is_directory: is_dir,
    count: if is_dir {
        count_recipes_tree(child)
    } else {
        None
    },
    description: None,
    tags,
    image_path,
    is_menu,
    modified_at,
    created_at,
});
```

- [ ] **Step 3: Verify clean compile**

```bash
cd /Users/romain/Projects/cooklang/cookcli && cargo build 2>&1 | grep "error"
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
cd /Users/romain/Projects/cooklang/cookcli
git add src/server/templates.rs src/server/builders.rs
git commit -m "feat: add modified_at and created_at timestamps to RecipeItem"
```

---

## Task 3: Embed data attributes and add sort controls to template

**Files:**
- Modify: `templates/recipes.html`

- [ ] **Step 1: Add `id` and `data-type` attributes to the grid and cards**

In `templates/recipes.html`, find the grid opening tag (line 58):

```html
<div class="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
```

Replace with:

```html
<div id="recipes-grid" class="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
```

Find the directory card `<a>` tag (line 61):

```html
<a href="{{ prefix }}/directory/{{ item.path }}{% if static_mode %}.html{% endif %}" class="bg-white rounded-2xl shadow-lg overflow-hidden hover:shadow-xl transition-all hover:scale-105 recipe-card flex flex-col">
```

Add `data-type="directory"`:

```html
<a href="{{ prefix }}/directory/{{ item.path }}{% if static_mode %}.html{% endif %}" data-type="directory" class="bg-white rounded-2xl shadow-lg overflow-hidden hover:shadow-xl transition-all hover:scale-105 recipe-card flex flex-col">
```

Find the recipe card `<a>` tag (line 75):

```html
<a href="{{ prefix }}/{% if static_mode %}{% if item.is_menu %}menu{% else %}recipe{% endif %}{% else %}recipe{% endif %}/{{ item.path }}{% if static_mode %}.html{% endif %}" class="bg-white rounded-2xl shadow-lg overflow-hidden hover:shadow-xl transition-all hover:scale-105 recipe-card flex flex-col">
```

Add `data-type="recipe"` and the timestamp attributes:

```html
<a href="{{ prefix }}/{% if static_mode %}{% if item.is_menu %}menu{% else %}recipe{% endif %}{% else %}recipe{% endif %}/{{ item.path }}{% if static_mode %}.html{% endif %}"
   data-type="recipe"
   {% if let Some(ts) = item.modified_at %}data-modified="{{ ts }}"{% endif %}
   {% if let Some(ts) = item.created_at %}data-created="{{ ts }}"{% endif %}
   class="bg-white rounded-2xl shadow-lg overflow-hidden hover:shadow-xl transition-all hover:scale-105 recipe-card flex flex-col">
```

- [ ] **Step 2: Add the sort controls row**

Find the closing `</div>` of the title row (line 33, after the `{% endif %}` that closes the `{% if !static_mode %}` new-recipe button block):

```html
    </div>

    {% match todays_menu %}
```

Insert the sort controls between the title row and the today's menu block:

```html
    </div>

    <div id="sort-controls" class="hidden items-center gap-2 mb-6">
        <span class="text-sm text-gray-500 dark:text-gray-400">Sort by:</span>
        <select id="sort-field" class="text-sm border border-gray-300 dark:border-gray-600 rounded-lg px-2 py-1.5 bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-200 focus:outline-none focus:ring-2 focus:ring-purple-300">
            <option value="name">Name</option>
            <option value="modified">Modified</option>
            <option value="created" id="sort-created-option">Created</option>
        </select>
        <button id="sort-dir" class="px-2 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-600 transition-colors" title="Toggle sort direction">↑</button>
    </div>

    {% match todays_menu %}
```

- [ ] **Step 3: Verify template renders (start dev server and check the page)**

```bash
cd /Users/romain/Projects/cooklang/cookcli && cargo run -- server ./seed
```

Open http://localhost:9080 — the recipes grid should still render correctly with no visible change yet (sort controls are hidden by default until JS runs).

- [ ] **Step 4: Commit**

```bash
cd /Users/romain/Projects/cooklang/cookcli
git add templates/recipes.html
git commit -m "feat: add data attributes and sort controls markup to recipes template"
```

---

## Task 4: Add client-side sort JavaScript

**Files:**
- Modify: `templates/recipes.html` — `{% block scripts %}` at the bottom

- [ ] **Step 1: Add the sort script inside `{% block scripts %}`**

At the bottom of `templates/recipes.html`, inside the existing `{% block scripts %}{% endblock %}` block, add:

```html
{% block scripts %}
<script>
(function () {
    const grid = document.getElementById('recipes-grid');
    const controls = document.getElementById('sort-controls');
    const fieldSelect = document.getElementById('sort-field');
    const dirBtn = document.getElementById('sort-dir');
    const createdOption = document.getElementById('sort-created-option');

    if (!grid) return;

    let sortField = 'name';
    let sortDir = 'asc';

    // Hide Created option if no recipe has a creation date
    if (!grid.querySelector('[data-created]') && createdOption) {
        createdOption.remove();
    }

    // Show controls only when there are at least 2 recipe cards
    const recipeCount = grid.querySelectorAll('[data-type="recipe"]').length;
    if (recipeCount >= 2) {
        controls.classList.remove('hidden');
        controls.classList.add('flex');
    }

    function getValue(el) {
        if (sortField === 'name') {
            return (el.querySelector('h3')?.textContent ?? '').trim().toLowerCase();
        }
        const attr = sortField === 'modified' ? 'data-modified' : 'data-created';
        const raw = el.getAttribute(attr);
        return raw !== null ? parseInt(raw, 10) : null;
    }

    function applySort() {
        const all = Array.from(grid.children);
        const dirs = all.filter(el => el.dataset.type === 'directory');
        const recipes = all.filter(el => el.dataset.type === 'recipe');
        const others = all.filter(el => !el.dataset.type); // empty-state div

        recipes.sort((a, b) => {
            const av = getValue(a);
            const bv = getValue(b);
            if (av === null && bv === null) return 0;
            if (av === null) return 1;
            if (bv === null) return -1;
            const cmp = typeof av === 'string' ? av.localeCompare(bv) : av - bv;
            return sortDir === 'asc' ? cmp : -cmp;
        });

        dirs.forEach(el => grid.appendChild(el));
        recipes.forEach(el => grid.appendChild(el));
        others.forEach(el => grid.appendChild(el));

        dirBtn.textContent = sortDir === 'asc' ? '↑' : '↓';
    }

    fieldSelect.addEventListener('change', function () {
        sortField = this.value;
        // Default to desc for date sorts (newest first), asc for name
        sortDir = sortField === 'name' ? 'asc' : 'desc';
        applySort();
    });

    dirBtn.addEventListener('click', function () {
        sortDir = sortDir === 'asc' ? 'desc' : 'asc';
        applySort();
    });
})();
</script>
{% endblock %}
```

- [ ] **Step 2: Test name sort**

With the dev server running (`cargo run -- server ./seed`), open http://localhost:9080.
- Sort controls should be visible if there are ≥ 2 recipes.
- Select "Name" — recipes should be in A→Z order.
- Click ↑ to flip to ↓ — recipes should reverse to Z→A.

- [ ] **Step 3: Test modified sort**

- Select "Modified" in the dropdown — recipes should reorder by last-modified date, newest first (↓).
- Click the direction button to verify oldest-first order.
- Right-click a recipe card, inspect element — verify `data-modified` contains a plausible Unix timestamp (e.g. ~1700000000).

- [ ] **Step 4: Test created sort (if available)**

- If on macOS, the "Created" option should appear in the dropdown. Select it — recipes reorder by creation date.
- If on a Linux filesystem that doesn't support birth time, the option should not appear.

- [ ] **Step 5: Test directory pinning**

If any subdirectories exist in the seed recipes, verify they always appear before recipe cards regardless of the selected sort.

- [ ] **Step 6: Commit**

```bash
cd /Users/romain/Projects/cooklang/cookcli
git add templates/recipes.html
git commit -m "feat: add client-side recipe sorting by name, modified date, and created date"
```

---

## Self-Review Checklist

- [x] **Spec coverage:** All spec sections covered — data fields (Task 1+2), template attributes (Task 3), sort controls UI (Task 3), JS logic with Created visibility, directory pinning, direction toggle (Task 4).
- [x] **No placeholders:** All steps contain complete code.
- [x] **Type consistency:** `modified_at: Option<i64>` / `created_at: Option<i64>` defined in Task 1, populated in Task 2, read as `data-modified`/`data-created` string attributes in Task 4 — consistent throughout.
- [x] **Static site:** Both `ui.rs` and `build/renderer.rs` call `build_recipes_template`, so the static builder gets timestamps for free.
- [x] **Empty state:** The empty-state `<div>` has no `data-type` attribute and is handled by the `others` bucket in `applySort()`, so it stays at the end without breaking the sort.
