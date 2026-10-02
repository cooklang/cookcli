//! Shopping for some days of a menu.
//!
//! A menu's days are its sections, dated the way the editor toolbar and meal
//! plans write them: `== Wednesday (2026-10-07) ==`. Narrowing a menu to a
//! [`DayRange`] keeps what those sections ask for — the recipes they reference
//! and the ingredients written into them — and leaves out the rest.

use std::collections::HashSet;

use chrono::NaiveDate;
use cooklang::{
    ingredient_list::IngredientList, Content, Converter, GroupedQuantity, Item, Recipe,
};

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

/// The date a section's name gives it: the first `(YYYY-MM-DD)` in it, as in
/// `Wednesday (2026-10-07)` — the same rule the menu API and the planner
/// follow. A bare date without parentheses does not count.
pub fn section_date(name: &str) -> Option<NaiveDate> {
    name.match_indices('(').find_map(|(open, _)| {
        let rest = &name[open + 1..];
        let date = rest.get(..10)?;
        if !rest.get(10..)?.starts_with(')') {
            return None;
        }
        NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()
    })
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
        let Some(date) = section.name.as_deref().and_then(section_date) else {
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

/// [`IngredientList::add_recipe`] for only the ingredients in `kept`.
///
/// `add_recipe` lists each ingredient together with the `&` references to it,
/// wherever they are in the recipe. Here a group counts only the members the
/// chosen days use: a definition on Monday referenced from Tuesday adds
/// Tuesday's amount alone when only Tuesday is chosen. Everything else —
/// which references are handed back to be expanded, which ingredients are
/// listed — is decided exactly as `add_recipe` decides it.
pub(super) fn add_kept(
    list: &mut IngredientList,
    recipe: &Recipe,
    kept: &HashSet<usize>,
    converter: &Converter,
    list_references: bool,
) -> Vec<usize> {
    let mut references = Vec::new();
    for entry in recipe.group_ingredients(converter) {
        let members: Vec<usize> = std::iter::once(entry.index)
            .chain(entry.ingredient.relation.referenced_from().iter().copied())
            .collect();
        if !members.iter().any(|index| kept.contains(index)) {
            continue;
        }

        if entry.ingredient.reference.is_some() {
            references.push(entry.index);
            if !list_references {
                continue;
            }
        }
        if !entry.ingredient.modifiers().should_be_listed() {
            continue;
        }

        let quantity = if members.iter().all(|index| kept.contains(index)) {
            entry.quantity
        } else {
            let mut quantity = GroupedQuantity::default();
            for index in members.iter().filter(|index| kept.contains(index)) {
                if let Some(q) = &recipe.ingredients[*index].quantity {
                    quantity.add(q, converter);
                }
            }
            let _ = quantity.fit(converter);
            quantity
        };
        list.add_ingredient(
            entry.ingredient.display_name().into_owned(),
            &quantity,
            converter,
        );
    }
    references
}
