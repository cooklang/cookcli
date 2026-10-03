//! Menu discovery shared by the web server and the static site builder.

use crate::web::templates::TodaysMenu;
use regex::Regex;
use std::sync::LazyLock;

#[cfg(feature = "server")]
static DATE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\((\d{4}-\d{2}-\d{2})\)").unwrap());
static TIME_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\((\d{2}:\d{2})\)").unwrap());
static MEAL_HEADER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*\(\d{2}:\d{2}\)\s*").unwrap());

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

/// Extract a date in YYYY-MM-DD format from a section name.
/// Matches patterns like "Day 1 (2026-03-04)".
#[cfg(feature = "server")]
pub fn extract_date(name: &str) -> Option<String> {
    DATE_RE.captures(name).map(|caps| caps[1].to_string())
}

/// Extract a time in HH:MM format from a meal type header.
/// Matches patterns like "Breakfast (08:30):".
pub fn extract_time(header: &str) -> Option<String> {
    TIME_RE.captures(header).map(|caps| caps[1].to_string())
}

/// Extract the meal type name from a header string.
/// Strips the trailing colon and any time in parentheses.
/// "Breakfast (08:30):" -> "Breakfast"
/// "Dinner:" -> "Dinner"
pub fn extract_meal_type(header: &str) -> String {
    // Remove trailing colon (and whitespace around it)
    let stripped = header.trim().trim_end_matches(':').trim();
    // Remove time in parentheses
    MEAL_HEADER_RE.replace_all(stripped, "").trim().to_string()
}

/// Check if a text line is a meal type header (ends with ":" possibly with whitespace).
pub fn is_meal_header(text: &str) -> bool {
    let trimmed = text.trim();
    // Must end with ':'
    if !trimmed.ends_with(':') {
        return false;
    }
    // Must have some content before the colon
    let before_colon = trimmed.trim_end_matches(':').trim();
    !before_colon.is_empty()
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
