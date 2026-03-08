# Today's Menu Banner on Recipes Page

## Problem

Users with menu files that contain dated sections (e.g. `==Monday (2026-03-07)==`) have no way to see at a glance what's planned for today when they open the recipes page.

## Solution

A hero banner at the top of the root recipes page that shows today's menu when a matching section exists in any menu file.

## Behavior

- On the **root recipes page only** (`/`), scan all `.menu` files in the collection.
- For each menu, parse its sections and check if any section header contains today's date in `YYYY-MM-DD` format (e.g. `==Day 3 (2026-03-07)==`).
- If a match is found, render a banner above the recipe grid showing:
  - Today's formatted date (e.g. "Saturday, March 7")
  - The menu name (e.g. "2 Day Plan")
  - Meal types for today listed as pills (e.g. "Breakfast", "Lunch", "Dinner")
  - A link to the full menu page (`/recipe/{menu_path}`)
- If no menu has today's date, the banner is not rendered — page looks unchanged.
- If multiple menus match, show the first one found.

## Backend Changes

### New struct: `TodaysMenu`

```rust
pub struct TodaysMenu {
    pub menu_name: String,     // Display name of the menu
    pub menu_path: String,     // URL path for navigation
    pub date_display: String,  // "Saturday, March 7"
    pub meals: Vec<String>,    // ["Breakfast", "Lunch", "Dinner"]
}
```

### Modified: `RecipesTemplate`

Add field: `pub todays_menu: Option<TodaysMenu>`

### Modified: `recipes_handler`

When `path` is `None` (root page):
1. Build full recipe tree from base path
2. Collect all menu entries using existing `collect_menus` logic
3. For each menu, parse it and check section dates against today
4. If a match is found, extract meal type names from that section
5. Build `TodaysMenu` struct and pass to template

Reuse existing date extraction (`extract_date`) and meal parsing from `handlers/menus.rs`.

## Template Changes

In `recipes.html`, before the recipe grid, conditionally render:

```html
{% match todays_menu %}
{% when Some with (menu) %}
<div class="banner with purple gradient">
  <date>{{ menu.date_display }}</date>
  <menu name>{{ menu.menu_name }}</menu>
  <meal pills>{{ menu.meals }}</meal>
  <link to="/recipe/{{ menu.menu_path }}">View</link>
</div>
{% when None %}
{% endmatch %}
```

## Styling

- Purple-to-pink gradient background (consistent with existing menu badge aesthetic)
- White text, rounded corners
- Meal types as semi-transparent white pill badges
- Responsive — stacks vertically on mobile
