//! Menu discovery shared by the web server and the static site builder.

use crate::web::templates::TodaysMenu;

/// Find the first menu with a section matching today's date, using
/// cooklang-find's `list_menus_for_date`. A section header containing the date
/// anywhere (e.g. `= Day 1 (2026-06-24)` or `= 2026-06-24 Dinner`) counts as a
/// match. Returns the menu name, path, and a human-friendly date for display.
pub fn find_todays_menu(base_path: &camino::Utf8Path) -> Option<TodaysMenu> {
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

    Some(TodaysMenu {
        menu_name,
        menu_path,
        date_display: today_display,
    })
}

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
    fn find_todays_menu_matches_bare_date_header() {
        // The library matches the date as a substring, so a header without
        // parentheses (e.g. "= 2026-06-24 Dinner") also counts as today.
        let temp = TempDir::new().unwrap();
        let dir = camino::Utf8Path::from_path(temp.path()).unwrap();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let content = format!("= {today} Dinner\n\nBreakfast:\n- @eggs{{}}\n");
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
