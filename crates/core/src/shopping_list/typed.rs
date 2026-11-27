//! Typed lines to free-hand items, for the shopping list's add field.
//!
//! The parsing is `cooklang-ingredient-parser`'s, shared with the Cooklang
//! apps, so a line typed here is stored exactly as the iOS app would store it.
//! No model is passed: CookCLI ships as one binary without ONNX Runtime.

use super::store::quantity_from_text;
use super::FreeHandItem;
use cooklang::aisle::AisleConf;
use cooklang::Quantity;
use cooklang_ingredient_parser::{
    edit_text, parse_ingredient, parse_ingredient_list, ParsedIngredient,
};
use serde::Serialize;

/// One typed line, read: what would be written and how the page shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TypedItem {
    /// The free-hand line to write: the cleaned name (the aisle's spelling
    /// when it matched) and the stored quantity (`1.5%kg`, `3`).
    pub item: FreeHandItem,
    /// The name of the row it lands on in the aisle list: the aisle's common
    /// name when it matched, otherwise the written name.
    pub row: String,
    /// The recognised amount as the chip shows it ("1.5 kg", "x3").
    pub chip: Option<String>,
    /// The aisle category the name matched.
    pub category: Option<String>,
    /// The line that reads back as this item ("Sugar 1.5 kg").
    pub edit_text: String,
    /// Set by [`edited_item`] when the old item's quantity was kept because
    /// the edit field could not show it. Such an edit is never folded into
    /// another line.
    #[serde(skip)]
    pub keeps_quantity: bool,
}

fn typed_item(parsed: ParsedIngredient) -> Option<TypedItem> {
    if parsed.name.is_empty() {
        return None;
    }
    let quantity = parsed
        .amount
        .as_ref()
        .and_then(|amount| amount.to_quantity());
    let chip = quantity
        .as_ref()
        .and(parsed.amount.as_ref())
        .map(|amount| amount.chip_text());
    let row = parsed
        .aisle_match
        .as_ref()
        .map_or_else(|| parsed.name.clone(), |m| m.common_name.clone());
    let category = parsed.aisle_match.map(|m| m.category);
    let edit_text = edit_text(&parsed.name, quantity.as_ref());
    let item = FreeHandItem {
        name: parsed.name,
        quantity: quantity.as_ref().map(Quantity::to_cooklang),
    };
    Some(TypedItem {
        edit_text,
        item,
        row,
        chip,
        category,
        keeps_quantity: false,
    })
}

impl FreeHandItem {
    /// The line an edit field shows for this item, which reads back as the
    /// same item: `Milk 2 l`, or the name alone when the field cannot show
    /// the quantity (`2%bunches`, `a pinch`).
    pub fn edit_text(&self) -> String {
        let quantity = self.quantity.as_deref().and_then(quantity_from_text);
        edit_text(&self.name, quantity.as_ref())
    }
}

/// One typed line; `None` when it has no name (blank, or only braces).
pub fn read_typed_line(line: &str, aisle: Option<&AisleConf>) -> Option<TypedItem> {
    typed_item(parse_ingredient(line, aisle, None))
}

/// Pasted text: one item per kept line (bullets, checkboxes, Markdown
/// headings and blank lines dropped), lines without a name skipped.
pub fn read_typed_text(text: &str, aisle: Option<&AisleConf>) -> Vec<TypedItem> {
    parse_ingredient_list(text, aisle, None)
        .into_iter()
        .filter_map(typed_item)
        .collect()
}

/// The item an edit of `old` makes from `line`. An item whose quantity the
/// field could not show (`2%bunches`, `a pinch`: its edit text is the name
/// alone) keeps that quantity unless the edited line has an amount; the
/// result then has `keeps_quantity` set.
pub fn edited_item(old: &FreeHandItem, line: &str, aisle: Option<&AisleConf>) -> Option<TypedItem> {
    let mut typed = read_typed_line(line, aisle)?;
    let shown_without_quantity = old.edit_text() == old.name;
    if typed.item.quantity.is_none() && shown_without_quantity {
        typed.item.quantity = old.quantity.clone();
        typed.edit_text = typed.item.edit_text();
        typed.keeps_quantity = old.quantity.is_some();
    }
    Some(typed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aisle() -> AisleConf<'static> {
        cooklang::aisle::parse("[baking]\nsugar | caster sugar\n\n[veg]\ntomatoes\n").unwrap()
    }

    fn item(name: &str, quantity: Option<&str>) -> FreeHandItem {
        FreeHandItem {
            name: name.into(),
            quantity: quantity.map(Into::into),
        }
    }

    #[test]
    fn a_line_with_an_amount() {
        let typed = read_typed_line("Sugar 1.5kg", Some(&aisle())).unwrap();
        assert_eq!(typed.item, item("sugar", Some("1.5%kg")));
        assert_eq!(typed.row, "sugar");
        assert_eq!(typed.chip.as_deref(), Some("1.5 kg"));
        assert_eq!(typed.category.as_deref(), Some("baking"));
        assert_eq!(typed.edit_text, "sugar 1.5 kg");
    }

    #[test]
    fn an_alias_lands_on_the_common_name_row() {
        let typed = read_typed_line("Caster sugar 200 g", Some(&aisle())).unwrap();
        assert_eq!(typed.item.name, "caster sugar");
        assert_eq!(typed.row, "sugar");
    }

    #[test]
    fn a_plural_takes_the_aisle_spelling() {
        let typed = read_typed_line("Tomato 3", Some(&aisle())).unwrap();
        assert_eq!(typed.item, item("tomatoes", Some("3")));
        assert_eq!(typed.chip.as_deref(), Some("x3"));
    }

    #[test]
    fn no_aisle_no_amount() {
        let typed = read_typed_line("Paper towels", None).unwrap();
        assert_eq!(typed.item, item("Paper towels", None));
        assert_eq!(typed.row, "Paper towels");
        assert_eq!(typed.chip, None);
        assert_eq!(typed.category, None);
    }

    #[test]
    fn a_line_without_a_name_is_none() {
        assert_eq!(read_typed_line("   ", None), None);
        assert_eq!(read_typed_line("{}", None), None);
    }

    #[test]
    fn pasted_text() {
        let typed = read_typed_text("## Dairy\n- Milk 2 l\n\n* [ ] 3 lemons\n- {}\n", None);
        let items: Vec<_> = typed.into_iter().map(|t| t.item).collect();
        assert_eq!(
            items,
            [item("Milk", Some("2%l")), item("lemons", Some("3"))]
        );
    }

    #[test]
    fn an_edit_keeps_a_quantity_the_field_could_not_show() {
        let old = item("parsley", Some("2%bunches"));
        assert_eq!(
            edited_item(&old, "flat parsley", None).unwrap().item,
            item("flat parsley", Some("2%bunches"))
        );
        assert_eq!(
            edited_item(&old, "flat parsley 3", None).unwrap().item,
            item("flat parsley", Some("3"))
        );
    }

    #[test]
    fn an_edit_can_drop_a_shown_amount() {
        let old = item("Milk", Some("2%l"));
        assert_eq!(
            edited_item(&old, "Milk", None).unwrap().item,
            item("Milk", None)
        );
    }

    #[test]
    fn a_line_is_cleaned() {
        let typed = read_typed_line("Tomatoes {ripe} 2", None).unwrap();
        assert_eq!(typed.item, item("Tomatoes ripe", Some("2")));
    }

    #[test]
    fn an_edit_reports_a_kept_quantity() {
        let old = item("parsley", Some("2%bunches"));
        assert!(
            edited_item(&old, "flat parsley", None)
                .unwrap()
                .keeps_quantity
        );
        assert!(
            !edited_item(&old, "flat parsley 3", None)
                .unwrap()
                .keeps_quantity
        );
        let plain = item("bread", None);
        assert!(
            !edited_item(&plain, "rye bread", None)
                .unwrap()
                .keeps_quantity
        );
    }
}
