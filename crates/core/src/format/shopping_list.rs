// This file includes a substantial portion of code from
// https://github.com/Zheoni/cooklang-chef
//
// The original code is licensed under the MIT License, a copy of which
// is provided below in addition to our project's license.
//
//

// MIT License

// Copyright (c) 2023 Francisco J. Sanchez

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Shopping list output formatters.
//!
//! Each function renders an [`AggregatedList`] into one target format. Unlike
//! the recipe formatters, these build a whole value — a table, a JSON tree, a
//! string — rather than writing into a [`std::io::Write`], because the callers
//! hand the result to `serde` or print it in one go.

use crate::{format::Style, shopping_list::AggregatedList};
use cooklang::quantity::{GroupedQuantity, Quantity, Value};
use serde::Serialize;
use yansi::Paint;

/// Render one quantity the way the human and markdown output show it.
///
/// `"200 g"` with a unit, `"3"` without one.
pub(crate) fn quantity_fmt(qty: &Quantity) -> String {
    if let Some(unit) = qty.unit() {
        format!("{} {}", qty.value(), unit)
    } else {
        format!("{}", qty.value())
    }
}

/// Add a quantity cell to a table row, joining the parts that could not be
/// combined into a single unit with commas.
fn total_quantity_fmt(qty: &GroupedQuantity, row: &mut tabular::Row) {
    let content = qty
        .iter()
        .map(quantity_fmt)
        .reduce(|s, q| format!("{s}, {q}"))
        .unwrap_or_default();
    row.add_ansi_cell(content);
}

/// Render the list as the aligned two-column table `cook shopping-list` prints.
///
/// `plain` drops the aisle category headings and lists every ingredient in the
/// order the recipes introduced them. `style` decides whether the headings
/// carry ANSI colour; `Style::Plain` is `Style::Ansi` with the escapes removed.
pub fn build_human_table(list: AggregatedList, plain: bool, style: Style) -> tabular::Table {
    let mut table = tabular::Table::new("{:<} {:<}");
    if plain {
        for (igr, q) in list.raw_items {
            let mut row = tabular::Row::new().with_cell(igr);
            total_quantity_fmt(&q, &mut row);
            table.add_row(row);
        }
    } else {
        for (cat, items) in list.raw_categories {
            let heading = if style.is_ansi() {
                format!("[{}]", cat.green())
            } else {
                format!("[{cat}]")
            };
            table.add_heading(heading);
            for (igr, q) in items {
                let mut row = tabular::Row::new().with_cell(igr);
                total_quantity_fmt(&q, &mut row);
                table.add_row(row);
            }
        }
    }
    table
}

/// Render the list as Markdown.
///
/// `plain` drops the category headings; `ingredients_only` drops the
/// quantities, leaving a bare checklist of names.
pub fn build_md_value(list: AggregatedList, plain: bool, ingredients_only: bool) -> String {
    let mut output = String::new();

    let format_ingredient = |ingredient: &str, quantity: &GroupedQuantity| {
        if ingredients_only {
            format!("- {ingredient}\n")
        } else {
            let quantity_string = quantity
                .iter()
                .map(quantity_fmt)
                .collect::<Vec<_>>()
                .join(", ");
            format!("- *{quantity_string}* {ingredient}\n")
        }
    };
    if plain {
        // no categories, simple list
        for (ingredient, quantity) in list.raw_items {
            output.push_str(&format_ingredient(&ingredient, &quantity));
        }
    } else {
        for (i, (category, items)) in list.raw_categories.into_iter().enumerate() {
            if i > 0 {
                output.push('\n');
            }
            output.push_str(&format!("# {category}\n"));
            for (ingredient, quantity) in items {
                output.push_str(&format_ingredient(&ingredient, &quantity));
            }
        }
    }
    output
}

/// The JSON shape both the JSON and YAML writers serialise.
#[derive(Serialize)]
struct JsonQuantity {
    value: Value,
    unit: Option<String>,
}

impl From<Quantity> for JsonQuantity {
    fn from(qty: Quantity) -> Self {
        let unit = qty.unit().map(|s| s.to_owned());
        let value = qty.value().clone();
        Self { value, unit }
    }
}

#[derive(Serialize)]
struct JsonIngredient {
    name: String,
    quantity: Vec<JsonQuantity>,
}

impl From<(String, GroupedQuantity)> for JsonIngredient {
    fn from((name, qty): (String, GroupedQuantity)) -> Self {
        JsonIngredient {
            name,
            quantity: qty.into_vec().into_iter().map(JsonQuantity::from).collect(),
        }
    }
}

#[derive(Serialize)]
struct JsonCategory {
    category: String,
    items: Vec<JsonIngredient>,
}

fn json_categories(list: AggregatedList) -> Vec<JsonCategory> {
    list.raw_categories
        .into_iter()
        .map(|(category, items)| JsonCategory {
            category,
            items: items.into_iter().map(JsonIngredient::from).collect(),
        })
        .collect()
}

/// Render the list as JSON: an array of categories, each with its items, or a
/// flat array of items when `plain` is set.
pub fn build_json_value(list: AggregatedList, plain: bool) -> serde_json::Value {
    if plain {
        serde_json::to_value(
            list.raw_items
                .into_iter()
                .map(JsonIngredient::from)
                .collect::<Vec<_>>(),
        )
        .unwrap()
    } else {
        serde_json::to_value(json_categories(list)).unwrap()
    }
}

/// Render the list as YAML: an array of categories, each with its items.
///
/// Note the missing `plain` parameter. Unlike the JSON and Markdown writers,
/// this one always categorises, so `cook shopping-list -f yaml --plain`
/// silently ignores the flag. That is
/// <https://github.com/cooklang/cookcli/issues/419>, preserved here on purpose
/// so the extraction that moved this code changed no behaviour.
pub fn build_yaml_value(list: AggregatedList) -> serde_yaml::Value {
    serde_yaml::to_value(json_categories(list)).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        shopping_list::{generate, GenerateRequest, ScaledRecipe},
        Context,
    };
    use camino::Utf8PathBuf;

    /// Two recipes sharing `tomatoes` and `salt`, plus an ingredient the aisle
    /// configuration does not mention, so every branch below has categories,
    /// an `other` bucket and a merged quantity to render.
    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("a.cook"),
            "Add @tomatoes{3} and @salt{1%tsp} to @water{2%l}.\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("b.cook"),
            "Add @tomatoes{2} and @salt{1%tsp}.\n",
        )
        .unwrap();
        dir
    }

    fn list(dir: &tempfile::TempDir) -> AggregatedList {
        let base = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let ctx = Context::new(base).with_aisle(crate::ConfigSource::Inline(
            "[produce]\ntomatoes\n\n[spices]\nsalt\n".to_string(),
        ));
        generate(
            &ctx,
            GenerateRequest {
                recipes: vec![ScaledRecipe::new("a.cook"), ScaledRecipe::new("b.cook")],
                ignore_references: false,
            },
        )
        .expect("generates")
        .value
    }

    #[test]
    fn the_human_table_groups_by_category_unless_plain() {
        let dir = fixture();
        let categorised = build_human_table(list(&dir), false, Style::Plain).to_string();
        assert!(
            categorised.contains("[produce]") && categorised.contains("[other]"),
            "expected category headings: {categorised}"
        );
        // 3 + 2 merged into one row, not two.
        assert!(
            categorised.contains("tomatoes 5"),
            "quantities must be merged: {categorised}"
        );

        let plain = build_human_table(list(&dir), true, Style::Plain).to_string();
        assert!(
            !plain.contains('['),
            "--plain must drop the headings: {plain}"
        );
        assert!(plain.contains("tomatoes 5"), "{plain}");
    }

    /// `Style` has to be honoured, not merely accepted.
    #[test]
    fn plain_style_is_ansi_with_the_escapes_removed() {
        let dir = fixture();
        let plain = build_human_table(list(&dir), false, Style::Plain).to_string();
        let coloured = build_human_table(list(&dir), false, Style::Ansi).to_string();

        assert!(
            !plain.contains('\u{1b}'),
            "Style::Plain must emit no escape codes: {plain:?}"
        );
        assert_eq!(
            plain,
            anstream::adapter::strip_str(&coloured).to_string(),
            "Plain must be exactly Ansi with the escapes removed"
        );
    }

    #[test]
    fn json_is_categorised_unless_plain() {
        let dir = fixture();

        let categorised = build_json_value(list(&dir), false);
        let categories: Vec<&str> = categorised
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["category"].as_str().unwrap())
            .collect();
        assert_eq!(categories, vec!["produce", "spices", "other"]);
        assert_eq!(categorised[0]["items"][0]["name"], "tomatoes");
        // `value` keeps `cooklang`'s tagged encoding, which the CLI's JSON
        // output has always exposed verbatim.
        assert_eq!(
            categorised[0]["items"][0]["quantity"][0]["value"]["value"]["value"],
            5.0
        );
        assert_eq!(
            categorised[0]["items"][0]["quantity"][0]["unit"],
            serde_json::Value::Null
        );

        let plain = build_json_value(list(&dir), true);
        let names: Vec<&str> = plain
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            vec!["tomatoes", "salt", "water"],
            "--plain lists every ingredient in recipe order, uncategorised"
        );
    }

    /// Records <https://github.com/cooklang/cookcli/issues/419>: the YAML
    /// writer has no `plain` parameter, so the flag cannot reach it.
    #[test]
    fn yaml_always_categorises() {
        let dir = fixture();
        let yaml = serde_yaml::to_string(&build_yaml_value(list(&dir))).unwrap();
        assert!(
            yaml.contains("category: produce"),
            "yaml is always categorised: {yaml}"
        );
    }

    #[test]
    fn markdown_headings_and_quantities_follow_the_flags() {
        let dir = fixture();

        let full = build_md_value(list(&dir), false, false);
        assert!(full.contains("# produce\n- *5* tomatoes\n"), "{full}");

        let plain = build_md_value(list(&dir), true, false);
        assert!(!plain.contains('#'), "--plain drops headings: {plain}");
        assert!(plain.starts_with("- *5* tomatoes\n"), "{plain}");

        let names_only = build_md_value(list(&dir), false, true);
        assert!(
            names_only.contains("# produce\n- tomatoes\n"),
            "{names_only}"
        );
        assert!(
            !names_only.contains('*'),
            "--ingredients-only drops quantities: {names_only}"
        );
    }

    #[test]
    fn quantities_that_do_not_convert_are_listed_side_by_side() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.cook"), "Add @flour{200%g}.\n").unwrap();
        std::fs::write(dir.path().join("b.cook"), "Add @flour{1%cup}.\n").unwrap();

        let base = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let aggregated = generate(
            &Context::new(base),
            GenerateRequest {
                recipes: vec![ScaledRecipe::new("a.cook"), ScaledRecipe::new("b.cook")],
                ignore_references: false,
            },
        )
        .expect("generates")
        .value;

        let table = build_human_table(aggregated, true, Style::Plain).to_string();
        assert!(
            table.contains("flour 1 c, 200 g"),
            "inconvertible units are joined with a comma: {table}"
        );
    }
}
