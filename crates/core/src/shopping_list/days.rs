//! Shopping for some days of a menu.
//!
//! A menu's days are its sections, dated the way the editor toolbar and meal
//! plans write them: `== Wednesday (2026-10-07) ==` or `= 2026-10-07 Dinner`. Narrowing a menu to a
//! [`DayRange`] keeps what those sections ask for — the recipes they reference
//! and the ingredients written into them — and leaves out the rest.

use std::collections::HashSet;
use std::sync::LazyLock;

use chrono::NaiveDate;
use cooklang::{
    ingredient_list::GroupedIngredient, Content, Converter, GroupedQuantity, Item, Recipe,
};
use regex::Regex;

/// The days of a menu to shop for: sections dated from `from` to `to`, both
/// included. Either end may be left open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DayRange {
    /// The first day, or `None` for every day up to `to`.
    pub from: Option<NaiveDate>,
    /// The last day, or `None` for every day from `from` on.
    pub to: Option<NaiveDate>,
}

impl DayRange {
    /// Whether `date` falls in the range.
    pub fn contains(&self, date: NaiveDate) -> bool {
        self.from.is_none_or(|from| from <= date) && self.to.is_none_or(|to| date <= to)
    }
}

impl std::fmt::Display for DayRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.from, self.to) {
            (Some(from), Some(to)) if from == to => write!(f, "{from}"),
            (Some(from), Some(to)) => write!(f, "{from} to {to}"),
            (Some(from), None) => write!(f, "from {from}"),
            (None, Some(to)) => write!(f, "up to {to}"),
            (None, None) => write!(f, "any day"),
        }
    }
}

/// A `YYYY-MM-DD` anywhere in a section name, not inside a longer number.
static SECTION_DATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:^|[^\d-])(\d{4}-\d{2}-\d{2})(?:[^\d-]|$)").unwrap());

/// The day a section is for, and the rest of its name after the date: the
/// first `YYYY-MM-DD` in it, as in `Wednesday (2026-10-07)` or
/// `2026-10-07 Dinner` — the rule meal plans and cooklang-find's
/// `list_menus_for_date` follow.
pub fn section_date(name: &str) -> Option<(NaiveDate, &str)> {
    let found = SECTION_DATE_RE.captures(name)?.get(1)?;
    let date = NaiveDate::parse_from_str(found.as_str(), "%Y-%m-%d").ok()?;
    Some((date, &name[found.end()..]))
}

/// What narrowing a recipe to some days left of it.
pub(super) enum Kept {
    /// No section of the recipe is dated, so it has no days to choose from.
    Undated,
    /// The ingredients, by index, that the dated sections in range use. Empty
    /// when no day falls in the range.
    Ingredients(HashSet<usize>),
}

/// The ingredients `recipe`'s sections dated within `days` use.
pub(super) fn kept_ingredients(recipe: &Recipe, days: &DayRange) -> Kept {
    let mut dated = false;
    let mut kept = HashSet::new();
    for section in &recipe.sections {
        let Some((date, _)) = section.name.as_deref().and_then(section_date) else {
            continue;
        };
        dated = true;
        if !days.contains(date) {
            continue;
        }
        for content in &section.content {
            if let Content::Step(step) = content {
                for item in &step.items {
                    if let Item::Ingredient { index } = item {
                        kept.insert(*index);
                    }
                }
            }
        }
    }
    if dated {
        Kept::Ingredients(kept)
    } else {
        Kept::Undated
    }
}

/// What a group of ingredients amounts to on the days kept: `None` when the
/// chosen days use none of it, so it is left off the list.
///
/// [`Recipe::group_ingredients`] adds each ingredient together with the `&`
/// references to it, wherever they are in the recipe. Here a group counts only
/// the members the chosen days use: a definition on Monday referenced from
/// Tuesday adds Tuesday's amount alone when only Tuesday is chosen.
pub(super) fn kept_quantity(
    recipe: &Recipe,
    entry: GroupedIngredient<'_>,
    kept: &HashSet<usize>,
    converter: &Converter,
) -> Option<GroupedQuantity> {
    let members: Vec<usize> = std::iter::once(entry.index)
        .chain(entry.ingredient.relation.referenced_from().iter().copied())
        .collect();
    if !members.iter().any(|index| kept.contains(index)) {
        return None;
    }
    if members.iter().all(|index| kept.contains(index)) {
        return Some(entry.quantity);
    }
    let mut quantity = GroupedQuantity::default();
    for index in members.iter().filter(|index| kept.contains(index)) {
        if let Some(q) = &recipe.ingredients[*index].quantity {
            quantity.add(q, converter);
        }
    }
    let _ = quantity.fit(converter);
    Some(quantity)
}
