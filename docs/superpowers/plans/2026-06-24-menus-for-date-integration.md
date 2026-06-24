# Menus-for-date Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace cookcli's custom "today's menu" scan with `cooklang_find::list_menus_for_date` from cooklang-find 0.6.0.

**Architecture:** Bump the `cooklang-find` dependency to 0.6, then rewrite `find_todays_menu` in `src/server/handlers/menus.rs` to delegate the directory scan to the library, keeping chrono in cookcli for computing/displaying "today". Drop the now-unused `tree` parameter and update its single caller.

**Tech Stack:** Rust, cooklang-find 0.6, chrono, axum, tempfile (dev).

---

## Spec

See `docs/superpowers/specs/2026-06-24-menus-for-date-integration-design.md`.

## File Structure

- **Modify:** `Cargo.toml` — bump `cooklang-find` to `0.6`.
- **Modify:** `src/server/handlers/menus.rs` — rewrite `find_todays_menu`; add a `#[cfg(test)]` module (none exists today).
- **Modify:** `src/server/builders.rs:142` — update the call site to drop the `tree` argument.

`collect_menus`, `extract_date`, `extract_time`, `extract_meal_type`, `is_meal_header`, and the `get_menu`/`list_menus` handlers are unchanged. The `RecipeTree` import in `menus.rs` stays (used by `collect_menus`).

---

## Task 1: Bump cooklang-find to 0.6

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Update the dependency version**

In `Cargo.toml`, change the `cooklang-find` line (currently `cooklang-find = { version = "0.5.0" }`) to:

```toml
cooklang-find = { version = "0.6" }
```

- [ ] **Step 2: Update the lockfile and verify the build still compiles**

Run: `cargo build`
Expected: Compiles successfully. `Cargo.lock` now resolves `cooklang-find` to `0.6.0`. (The 0.6 release is backward compatible with the native API cookcli already uses: `search`, `build_tree`, `get_recipe`, `RecipeEntry`, `RecipeTree`.)

- [ ] **Step 3: Confirm the resolved version**

Run: `grep -A2 'name = "cooklang-find"' Cargo.lock`
Expected: shows `version = "0.6.0"`.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: bump cooklang-find to 0.6"
```

---

## Task 2: Rewrite `find_todays_menu` to use `list_menus_for_date`

**Files:**
- Modify: `src/server/handlers/menus.rs` (rewrite `find_todays_menu` at lines 411-453; add a test module at end of file)
- Modify: `src/server/builders.rs:142` (drop the `tree` argument)
- Test: `#[cfg(test)] mod tests` in `src/server/handlers/menus.rs`

- [ ] **Step 1: Write the failing tests**

Append this test module at the very end of `src/server/handlers/menus.rs` (after the `LineItem` enum). It calls `find_todays_menu` with the new single-argument signature, so it will not compile until Step 3 changes the signature.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn find_todays_menu_matches_section_with_today() {
        let temp = TempDir::new().unwrap();
        let dir = camino::Utf8Path::from_path(temp.path()).unwrap();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let content = format!("= Day 1 ({today})\n\nBreakfast:\n- @eggs{{}}\n");
        fs::write(dir.join("week.menu"), content).unwrap();

        let result = find_todays_menu(dir);

        assert!(result.is_some());
        assert_eq!(result.unwrap().menu_path, "week");
    }

    #[test]
    fn find_todays_menu_returns_none_when_no_section_matches_today() {
        let temp = TempDir::new().unwrap();
        let dir = camino::Utf8Path::from_path(temp.path()).unwrap();
        let content = "= Day 1 (1999-01-01)\n\nBreakfast:\n- @eggs{}\n";
        fs::write(dir.join("week.menu"), content).unwrap();

        let result = find_todays_menu(dir);

        assert!(result.is_none());
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test find_todays_menu`
Expected: FAIL to compile — `find_todays_menu` takes 2 arguments but 1 was supplied (the signature still requires `tree`).

- [ ] **Step 3: Rewrite `find_todays_menu`**

Replace the entire existing `find_todays_menu` function (lines 411-453, the doc comment plus the body) with:

```rust
/// Find the menu whose section matches today's date, using cooklang-find's
/// `list_menus_for_date`. Returns the first match with menu name, path, and a
/// human-friendly date for display.
pub fn find_todays_menu(
    base_path: &camino::Utf8Path,
) -> Option<crate::server::templates::TodaysMenu> {
    let now = chrono::Local::now();
    let today = now.format("%Y-%m-%d").to_string();
    let today_display = now.format("%A, %B %-d").to_string();

    let menus = cooklang_find::list_menus_for_date(&[base_path], &today).unwrap_or_default();
    let entry = menus.first()?;

    let full_path = entry.path()?;
    let relative = full_path
        .strip_prefix(base_path)
        .unwrap_or(full_path.as_ref());
    let menu_name = entry.name().clone().unwrap_or_else(|| relative.to_string());
    let menu_path = relative
        .as_str()
        .trim_end_matches(".cook")
        .trim_end_matches(".menu")
        .to_string();

    Some(crate::server::templates::TodaysMenu {
        menu_name,
        menu_path,
        date_display: today_display,
    })
}
```

- [ ] **Step 4: Update the caller in `src/server/builders.rs`**

At `src/server/builders.rs:142`, change:

```rust
        crate::server::handlers::find_todays_menu(base_path, &tree)
```

to:

```rust
        crate::server::handlers::find_todays_menu(base_path)
```

(`tree` remains used earlier in the function to build the recipe list, so it does not become unused.)

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test find_todays_menu`
Expected: PASS (2 tests).

- [ ] **Step 6: Verify the whole crate builds cleanly with no warnings**

Run: `cargo build`
Expected: builds with no errors and no warnings. (In particular, confirm `find_todays_menu`'s old per-menu parsing locals are gone and no `unused` warnings appear.)

- [ ] **Step 7: Run the full test suite**

Run: `cargo test`
Expected: PASS (existing tests plus the 2 new ones).

- [ ] **Step 8: Commit**

```bash
git add src/server/handlers/menus.rs src/server/builders.rs
git commit -m "feat: use list_menus_for_date for the web UI today's menu"
```

---

## Self-Review Notes

- **Spec coverage:** dependency bump (Task 1); `find_todays_menu` rewrite keeping chrono for `today`/`today_display` (Task 2 Step 3); `tree` param removal + caller update (Task 2 Steps 3-4); helpers/handlers left unchanged (not touched); tests for match + no-match (Task 2 Step 1). All covered.
- **Type consistency:** `find_todays_menu(base_path: &camino::Utf8Path) -> Option<TodaysMenu>` is used identically in the test, the implementation, and the caller. `entry.path()` returns `Option<&Utf8PathBuf>` (handled via `?` then `.as_ref()`); `entry.name()` returns `&Option<String>` (handled via `.clone()`). `list_menus_for_date(&[base_path], &today)` matches the library signature `<P: AsRef<Utf8Path>>(base_dirs: &[P], date: &str)` since `&Utf8Path: AsRef<Utf8Path>`.
- **No placeholders:** every step contains the exact code/command.
- **Spec note correction:** the spec mentioned "existing tests for extract_date remain unchanged"; in fact `menus.rs` currently has no test module, so Task 2 Step 1 creates one. No behavior impact.
