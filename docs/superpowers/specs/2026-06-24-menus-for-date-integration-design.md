# Design: Use `cooklang_find::list_menus_for_date` for the web UI's today's-menu

**Date:** 2026-06-24
**Status:** Approved

## Overview

The cookcli web server shows "today's menu" on its home page. The detection
currently lives in `find_todays_menu` (`src/server/handlers/menus.rs`), which
builds the full menu list and then, for each menu, loads and fully parses the
file, walks its sections, and regex-extracts a `(YYYY-MM-DD)` date to find one
matching today.

`cooklang-find` 0.6.0 adds `list_menus_for_date(base_dirs, date)`, which scans
`.menu` files for a section header containing a date string and returns the
matching entries. This change replaces the custom scan in `find_todays_menu`
with that library call.

Scope is limited to the *implementation* of `find_todays_menu`. Behavior is
preserved: the web UI still surfaces today's menu. No template/UI changes, no
new endpoint, today only (not tomorrow).

## Changes

### 1. Dependency bump

`Cargo.toml`: `cooklang-find = { version = "0.6" }` (currently `"0.5.0"`).
Version 0.6.0 is published on crates.io.

### 2. `find_todays_menu`

Keep using `chrono` to compute:
- `today` — `chrono::Local::now().format("%Y-%m-%d")` (the match key).
- `today_display` — `format("%A, %B %-d")` (for `TodaysMenu.date_display`).

Replace the `collect_menus` + per-menu `get_recipe` / `parse_recipe_from_entry`
/ section-loop / `extract_date` block with:

```rust
let menus = cooklang_find::list_menus_for_date(&[base_path], &today).unwrap_or_default();
let entry = menus.first()?;
```

Build `TodaysMenu` from the first matching entry:
- `menu_name` — `entry.name()` if present, else the relative path string.
- `menu_path` — `entry.path()` stripped of the `base_path` prefix, with a
  trailing `.menu`/`.cook` removed (matching current behavior).
- `date_display` — `today_display`.

Signature stays `fn find_todays_menu(base_path: &Utf8Path, tree: &RecipeTree) -> Option<TodaysMenu>`.
The `tree` parameter is no longer needed by the body; remove it and update the
single caller (`src/server/builders.rs:111`) — OR keep it to minimize the call
site change. Decision: **remove the now-unused `tree` parameter** and update the
caller, since `list_menus_for_date` does its own directory scan and leaving an
unused parameter is misleading. The caller still has `base_path` available.

### What stays unchanged

- `collect_menus` — still used by the `list_menus` handler.
- `extract_date`, `extract_time`, `extract_meal_type`, `is_meal_header` — still
  used by `get_menu` for per-section date/meal display.
- `chrono` dependency — still needed here.

## Semantic difference (accepted)

The old code required the date inside parentheses (`(2026-06-24)`); the library
matches the date as a substring anywhere in a section header. For menus written
as "Day 1 (2026-06-24)" the two are equivalent. The library is slightly more
lenient (it would also match a bare `= 2026-06-24` header). This is acceptable
and arguably an improvement.

If multiple menus match today, the old code returned the first found during a
recursive tree walk; the new code returns `menus.first()`. Ordering may differ
slightly (glob order vs tree-walk order). Only one today's-menu is shown, and no
ordering guarantee was documented, so this is acceptable.

## Testing

Add unit tests in `src/server/handlers/menus.rs`:

- **Match:** create a temp dir with a `.menu` file whose section header contains
  today's date (computed via `chrono::Local::now()`), assert `find_todays_menu`
  returns `Some` with the expected `menu_path`/`menu_name`.
- **No match:** a `.menu` file with a clearly non-today date → `None`.

Existing tests for `extract_date` / `extract_meal_type` / `is_meal_header`
remain unchanged.

## Out of scope

- Tomorrow's menu (today only).
- Any template/HTML or frontend changes.
- A new `/api/menus?date=...` endpoint.
- Changing the menu detail view (`get_menu`) date handling.
