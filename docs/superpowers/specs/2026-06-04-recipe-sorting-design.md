# Recipe Sorting — Design Spec

**Date:** 2026-06-04  
**Status:** Approved

## Overview

Add client-side sorting to the recipes list page. Users can sort by name, last modified date, or creation date. The sort controls must scale gracefully to future sort options without consuming excessive space.

## Scope

- Recipes list page (`/` and `/directory/*`)
- Sorting applies to recipe cards only; directories always stay pinned at the top
- No changes to the recipe detail page, API, or search

## Data Layer

**`RecipeItem`** (`src/server/templates.rs`) gains two new fields:

```rust
pub modified_at: Option<i64>,  // Unix timestamp (seconds), None for directories
pub created_at: Option<i64>,   // Unix timestamp (seconds), None for directories or if unsupported
```

**`build_recipes_template`** (`src/server/builders.rs`): after building each non-directory item, call `std::fs::metadata(path)` on the recipe file path and extract:
- `.modified()` → `SystemTime` → seconds since Unix epoch → `i64`
- `.created()` → same, but wrapped in an extra `Option` because `created()` is not available on all Linux filesystems

Both are silently `None` on any error. Directories get `None` for both fields.

## Template

**`templates/recipes.html`**:

1. Embed timestamps on each recipe card `<a>` tag:
   ```html
   {% if let Some(ts) = item.modified_at %}data-modified="{{ ts }}"{% endif %}
   {% if let Some(ts) = item.created_at %}data-created="{{ ts }}"{% endif %}
   ```
   Directory cards get no date attributes.

2. Add a sort control row between the title/new-recipe row and the grid:
   ```
   Sort by: [dropdown ▾]  [↑↓ toggle button]
   ```
   The dropdown contains options: Name, Modified, Created.  
   The Created option is hidden by JS on page load if no card has a `data-created` attribute.  
   The control row is hidden entirely (`display:none`) when there is only one item or the list is empty.

## JavaScript

Inline `<script>` at the bottom of `recipes.html` (inside `{% block scripts %}`).

**State:** two variables — `sortField` (string: `"name"` | `"modified"` | `"created"`) and `sortDir` (string: `"asc"` | `"desc"`). Default: `name` / `asc`.

**On load:**
- If no card has `data-created`, hide the Created `<option>` from the dropdown.
- If total item count ≤ 1, hide the sort controls entirely.

**Sort function:**
1. Collect all children of `#recipes-grid` (add `id="recipes-grid"` to the grid `<div>` in the template).
2. Partition into directories (no `data-modified`) and recipes.
3. Sort the recipe partition:
   - `name`: compare `textContent` of the card's `<h3>` (case-insensitive)
   - `modified` / `created`: compare numeric value of the corresponding `data-*` attribute; cards missing the attribute sort to the end
4. Re-append: directories first (preserving their original order), then sorted recipes.
5. Update the toggle button icon (↑ for asc, ↓ for desc).

**Direction toggle:** clicking the ↑↓ button flips `sortDir` and re-runs the sort function.

**Dropdown change:** sets `sortField` and re-runs the sort function.

## Styling

The sort controls use existing Tailwind utility classes consistent with the rest of the page. The direction toggle is a small icon button matching the style of other utility buttons in the nav (e.g., the theme toggle).

## Not in scope

- Persisting the selected sort across page loads (no localStorage)
- Sorting inside subdirectories independently (same controls apply to all levels)
- Server-side sort (URL params)
- Sorting the search results dropdown
