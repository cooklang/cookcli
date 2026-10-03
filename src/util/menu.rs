//! Glue between `cooklang_find::Menu` and CookCLI's three menu consumers: the
//! JSON API, the HTML page (also the static export) and the shopping list.
//!
//! It lives in `util` rather than next to the server handlers because
//! `crate::server` is behind the `server` feature while `crate::web` is not.

use anyhow::{anyhow, Result};
use camino::Utf8Path;
use cooklang_find::{Menu, MenuItem, RecipeEntry};

/// Load a menu with every reference's `scale` resolved.
///
/// `menu_path` is the menu as named relative to `base_path`. Reference paths
/// from `Menu` are the authored ones, so a `@../Shared/Sauce` in `Plans/Week`
/// is still `../Shared/Sauce`; `cooklang-find` resolves paths against the
/// library root only. They are rewritten here to be root-relative, which is
/// what the rest of CookCLI expects (and what `resolve_scales` needs to find
/// the referenced recipe's `servings`/`yield`).
#[cfg_attr(not(feature = "server"), allow(dead_code))]
pub fn load_menu(
    entry: &RecipeEntry,
    menu_path: &Utf8Path,
    base_path: &Utf8Path,
    scale: f64,
) -> Result<Menu> {
    let mut menu = parse_menu(entry, menu_path)?;
    menu.resolve_scales(&[base_path], scale);
    Ok(menu)
}

/// [`load_menu`] without resolving scales, for the page, which needs the
/// menu's own `servings` to pick the scale first.
pub fn parse_menu(entry: &RecipeEntry, menu_path: &Utf8Path) -> Result<Menu> {
    let mut menu = entry
        .menu()
        .ok_or_else(|| anyhow!("not a menu file"))?
        .map_err(|e| anyhow!("Failed to read menu: {e}"))?;

    let menu_dir = menu_path.parent().unwrap_or(Utf8Path::new(""));
    for item in menu_items_mut(&mut menu) {
        if let MenuItem::RecipeReference { path, unit, .. } = item {
            if path.starts_with("../") {
                match cookcli_core::resolve_reference(menu_dir, path) {
                    Some(resolved) => *path = resolved.to_string(),
                    // Climbs out of the recipe directory. The authored path is
                    // kept so callers can tell (`is_outside_reference`), and
                    // the unit is dropped so `resolve_scales` does not join
                    // the path onto the base dir and read a file outside it:
                    // the target is then used as a raw multiplier.
                    None => *unit = None,
                }
            }
        }
    }

    Ok(menu)
}

/// Put back the meals a plan names but leaves empty (`Dinner: \\` over a bare
/// `- `), which `cooklang_find::Menu` drops because it holds no items.
///
/// FALLBACK: the planner on the menu page lays out the meals a plan names,
/// even on days where they are still empty ("Nothing planned"), so it needs
/// those headers. The `Menu` model has nowhere to keep a meal without items,
/// so the authored `content` is scanned once more, following the same line
/// rules as `cooklang-find`'s parser, only to find them. The API and the
/// shopping list want no empty meals and do not call this. If the scan does
/// not line up with the parsed sections, the menu is left as parsed.
pub fn keep_empty_meals(menu: &mut Menu, content: &str) {
    use cooklang_find::MenuMeal;
    use regex::Regex;
    use std::sync::LazyLock;

    static HEADER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^([^@:]+?)\s*(?:\((\d{1,2}:\d{2})\))?\s*:(?:\s|$)").unwrap());
    static BLOCK_COMMENT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)\[-.*?-\]").unwrap());

    // What one meal looks like to the scan: its heading, if it has one, and
    // whether any item followed.
    struct Seen {
        header: Option<(String, Option<String>)>,
        has_items: bool,
    }
    struct SeenSection {
        named: bool,
        meals: Vec<Seen>,
    }

    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    let mut lines = content.split_inclusive('\n');
    let mut body = content;
    if lines.next().is_some_and(|first| first.trim() == "---") {
        let mut offset = content.split_inclusive('\n').next().map_or(0, str::len);
        for line in lines {
            offset += line.len();
            if line.trim() == "---" {
                body = &content[offset..];
                break;
            }
        }
    }
    let body = BLOCK_COMMENT.replace_all(body, "");

    let mut sections: Vec<SeenSection> = Vec::new();
    let mut section = SeenSection {
        named: false,
        meals: Vec::new(),
    };
    let mut meal = Seen {
        header: None,
        has_items: false,
    };
    fn flush_meal(section: &mut SeenSection, meal: &mut Seen) {
        let done = std::mem::replace(
            meal,
            Seen {
                header: None,
                has_items: false,
            },
        );
        if done.has_items || done.header.is_some() {
            section.meals.push(done);
        }
    }
    for raw in body.lines() {
        let trimmed = raw.trim();
        let is_rule = trimmed.len() >= 3 && trimmed.bytes().all(|b| b == b'-');
        if trimmed.is_empty() || is_rule || trimmed.starts_with(">>") {
            continue;
        }
        if trimmed.starts_with('=') {
            flush_meal(&mut section, &mut meal);
            let name = trimmed.trim_matches('=').trim();
            let done = std::mem::replace(
                &mut section,
                SeenSection {
                    named: !name.is_empty(),
                    meals: Vec::new(),
                },
            );
            if done.named || done.meals.iter().any(|m| m.has_items) {
                sections.push(done);
            }
            continue;
        }
        let line = trimmed.strip_suffix('\\').unwrap_or(trimmed).trim_end();
        let line = match line.strip_prefix('-') {
            Some(rest) if rest.is_empty() || rest.starts_with(char::is_whitespace) => {
                rest.trim_start()
            }
            _ => line,
        };
        let mut rest = line;
        if !line.starts_with("--") {
            if let Some(caps) = HEADER.captures(line) {
                let after = &line[caps[0].len()..];
                let after_trim = after.trim_start();
                let meal_type = caps[1].trim();
                if !meal_type.is_empty()
                    && (after_trim.is_empty()
                        || after_trim.starts_with('@')
                        || after_trim.starts_with("--"))
                {
                    flush_meal(&mut section, &mut meal);
                    meal.header = Some((
                        meal_type.to_string(),
                        caps.get(2).map(|m| m.as_str().to_string()),
                    ));
                    rest = after;
                }
            }
        }
        let rest = rest.trim();
        let empty_note = rest.strip_prefix("--").is_some_and(|n| n.trim().is_empty());
        if !rest.is_empty() && !empty_note {
            meal.has_items = true;
        }
    }
    flush_meal(&mut section, &mut meal);
    if section.named || section.meals.iter().any(|m| m.has_items) {
        sections.push(section);
    }

    let filled = |section: &SeenSection| section.meals.iter().filter(|m| m.has_items).count();
    let aligned = sections.len() == menu.sections.len()
        && sections
            .iter()
            .zip(&menu.sections)
            .all(|(seen, parsed)| filled(seen) == parsed.meals.len());
    if !aligned {
        return;
    }
    for (seen, parsed) in sections.iter().zip(&mut menu.sections) {
        let mut parsed_meals = std::mem::take(&mut parsed.meals).into_iter();
        for meal in &seen.meals {
            if meal.has_items {
                parsed.meals.extend(parsed_meals.next());
            } else if let Some((meal_type, time)) = &meal.header {
                parsed.meals.push(MenuMeal {
                    meal_type: Some(meal_type.clone()),
                    time: time.clone(),
                    items: Vec::new(),
                });
            }
        }
    }
}

/// True for a reference [`load_menu`] could not keep inside the recipe
/// directory (`@../../etc/passwd{}`).
#[cfg_attr(not(feature = "server"), allow(dead_code))]
pub fn is_outside_reference(path: &str) -> bool {
    path.starts_with("../")
}

fn menu_items_mut(menu: &mut Menu) -> impl Iterator<Item = &mut MenuItem> {
    menu.sections
        .iter_mut()
        .flat_map(|section| &mut section.meals)
        .flat_map(|meal| &mut meal.items)
}

/// Quantity and unit of a loose ingredient, scaled by `scale` and formatted
/// the way the menu API and page have always shown them.
///
/// `cooklang-find` hands back the authored text (`1 1/2`, `1-2`, `some`,
/// `=100`), and scaling that is cooklang's job (fractions, ranges, fixed
/// `=` quantities, text values). So the authored pieces are put back into a
/// one-ingredient recipe, parsed and scaled by `cooklang`, and formatted as
/// before. If the fragment somehow does not parse the authored text is
/// returned unscaled.
pub fn scaled_quantity(
    quantity: Option<&str>,
    unit: Option<&str>,
    scale: f64,
) -> (Option<String>, Option<String>) {
    if quantity.is_none() && unit.is_none() {
        return (None, None);
    }
    let fragment = match unit {
        Some(unit) => format!("@x{{{}%{}}}", quantity.unwrap_or(""), unit),
        None => format!("@x{{{}}}", quantity.unwrap_or("")),
    };
    let scaled = crate::util::PARSER
        .parse(&fragment)
        .into_output()
        .and_then(|mut recipe| {
            recipe.scale(scale, crate::util::PARSER.converter());
            let ingredient = recipe.ingredients.into_iter().next()?;
            let quantity = ingredient.quantity?;
            Some((
                crate::util::format::number::format_quantity(quantity.value()),
                quantity.unit().as_ref().map(|u| u.to_string()),
            ))
        });
    scaled.unwrap_or_else(|| (quantity.map(str::to_string), unit.map(str::to_string)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_numbers_scale() {
        assert_eq!(
            scaled_quantity(Some("2"), Some("cup"), 3.0),
            (Some("6".to_string()), Some("cup".to_string()))
        );
    }

    #[test]
    fn fractions_and_mixed_numbers_scale() {
        assert_eq!(
            scaled_quantity(Some("1/2"), Some("tbsp"), 3.0).0.as_deref(),
            Some("1.5")
        );
        assert_eq!(
            scaled_quantity(Some("1 1/2"), Some("cup"), 2.0)
                .0
                .as_deref(),
            Some("3")
        );
    }

    #[test]
    fn fixed_and_text_quantities_do_not_scale() {
        assert_eq!(
            scaled_quantity(Some("=100"), Some("g"), 3.0).0.as_deref(),
            Some("100")
        );
        assert_eq!(
            scaled_quantity(Some("some"), None, 3.0).0.as_deref(),
            Some("some")
        );
    }

    #[test]
    fn empty_meals_come_back_in_place() {
        let text = "---\ntitle: P\n---\n== Mon (2026-10-05) ==\n\nBreakfast: \\\n- \n\n\
                    Dinner (19:00): \\\n- @soup{}\n\nLunch:\n\n== Empty ==\n";
        let mut menu = Menu::parse(text, "P");
        assert_eq!(menu.sections[0].meals.len(), 1);
        keep_empty_meals(&mut menu, text);
        let meals: Vec<_> = menu.sections[0]
            .meals
            .iter()
            .map(|m| (m.meal_type.as_deref(), m.time.as_deref(), m.items.len()))
            .collect();
        assert_eq!(
            meals,
            vec![
                (Some("Breakfast"), None, 0),
                (Some("Dinner"), Some("19:00"), 1),
                (Some("Lunch"), None, 0),
            ]
        );
        assert_eq!(menu.sections.len(), 2);
    }

    #[test]
    fn nothing_authored_is_nothing() {
        assert_eq!(scaled_quantity(None, None, 3.0), (None, None));
    }
}
