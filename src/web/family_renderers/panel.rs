// MIT License
//
// Copyright (c) 2024 cooklang
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! The nutrition panel of a recipe page and the nutrition summary of a recipe
//! card.
//!
//! The recipe's `nutrition:` mapping holds the values **per serving**. The
//! optional `weight:` key holds the weight of the whole recipe as written
//! (`weight: 900g`, or a bare number of grams) and enables the per-100 g
//! column and the colour of the small cards. Per serving and per 100 g never
//! change with the scale; the amounts shown for "the servings chosen" do.

use super::nutrition::{translated_nutrient_label, NutritionRenderer};
use super::FamilyRenderer;
use crate::util::format::nutrition::{amount_and_unit, find, Nutrient};
use crate::web::templates::{
    LegendItem, ListNutrition, MacroLine, NutrientCard, NutritionPanel, NutritionRow, RingArc,
};
use fluent_templates::Loader;
use unic_langid::LanguageIdentifier;

/// The recipe's weight in grams from `weight:` — a bare number, `"900g"`,
/// `"900 g"` or `"0.9kg"`.
pub(super) fn parse_weight_grams(value: &serde_yaml::Value) -> Option<f64> {
    let grams = match value {
        serde_yaml::Value::Number(n) => n.as_f64()?,
        serde_yaml::Value::String(s) => {
            let s = s.trim().to_lowercase().replace(',', ".");
            let split = s
                .find(|c: char| !(c.is_ascii_digit() || c == '.'))
                .unwrap_or(s.len());
            let (number, unit) = s.split_at(split);
            let number: f64 = number.parse().ok()?;
            match unit.trim() {
                "" | "g" | "gr" => number,
                "kg" => number * 1000.0,
                _ => return None,
            }
        }
        _ => return None,
    };
    (grams.is_finite() && grams > 0.0).then_some(grams)
}

/// An amount of an unknown nutrient: one decimal, without a trailing `.0`.
fn fmt_unknown(value: f64) -> String {
    let text = format!("{:.1}", (value * 10.0).round() / 10.0);
    text.strip_suffix(".0").unwrap_or(&text).to_string()
}

/// `value` with the rounding of its nutrient, or generically when unknown.
fn fmt_amount(nutrient: Option<&Nutrient>, value: f64) -> String {
    nutrient.map_or_else(|| fmt_unknown(value), |n| n.format_amount(value))
}

/// How a nutrient reads against its amount per 100 g: `low`, `mid` and `high`
/// for the nutrients to limit (the thresholds of the UK traffic-light scheme),
/// `good` for fibre at 3 g per 100 g or more (the "source of fibre" claim),
/// `neutral` where no threshold exists.
fn level_for(name: &str, per_100g: f64) -> &'static str {
    let (low, high) = match name {
        "fatContent" => (3.0, 17.5),
        "saturatedFatContent" => (1.5, 5.0),
        "sugarContent" => (5.0, 22.5),
        "salt" => (0.3, 1.5),
        "fiberContent" => return if per_100g >= 3.0 { "good" } else { "neutral" },
        _ => return "neutral",
    };
    if per_100g <= low {
        "low"
    } else if per_100g <= high {
        "mid"
    } else {
        "high"
    }
}

/// Each macro's share of the energy (4 kcal/g for carbohydrates and
/// proteins, 9 kcal/g for fat), as percentages that add up to 100, in the
/// order carbohydrates, fat, proteins.
fn energy_shares(carbs: f64, fat: f64, proteins: f64) -> Option<[f64; 3]> {
    let energy = [carbs * 4.0, fat * 9.0, proteins * 4.0];
    let total: f64 = energy.iter().sum();
    (total > 0.0 && total.is_finite()).then(|| energy.map(|e| e / total * 100.0))
}

/// Whole percentages of `shares` that add up to 100 (largest remainder).
fn percents(shares: [f64; 3]) -> [u32; 3] {
    let mut out = shares.map(|s| s.floor() as u32);
    let mut missing = 100u32.saturating_sub(out.iter().sum());
    let mut order = [0usize, 1, 2];
    order.sort_by(|&a, &b| {
        let (ra, rb) = (shares[a].fract(), shares[b].fract());
        rb.partial_cmp(&ra).unwrap_or(std::cmp::Ordering::Equal)
    });
    for i in order {
        if missing == 0 {
            break;
        }
        out[i] += 1;
        missing -= 1;
    }
    out
}

/// The donut's arcs for `shares` (carbohydrates, fat, proteins), laid end to
/// end from the top on a circle whose circumference is 100.
fn ring_arcs(shares: [f64; 3]) -> Vec<RingArc> {
    let mut before = 0.0;
    ["carb", "fat", "prot"]
        .into_iter()
        .zip(shares)
        .map(|(kind, share)| {
            let arc = RingArc {
                kind: kind.to_string(),
                dasharray: format!("{share:.2} {:.2}", 100.0 - share),
                dashoffset: format!("{:.2}", 25.0 - before),
            };
            before += share;
            arc
        })
        .collect()
}

/// One nutrient the recipe gives, as written.
struct Entry<'a> {
    field: &'a str,
    nutrient: Option<&'static Nutrient>,
    per_serving: f64,
    unit: String,
}

impl Entry<'_> {
    fn text(&self, value: f64) -> String {
        format!("{} {}", fmt_amount(self.nutrient, value), self.unit)
            .trim()
            .to_string()
    }
}

/// The numeric entries of a `nutrition:` mapping, hidden (`.`-prefixed) ones
/// left out.
fn entries(mapping: &serde_yaml::Mapping) -> Vec<Entry<'_>> {
    mapping
        .iter()
        .filter_map(|(key, value)| {
            let field = key.as_str().filter(|f| !f.starts_with('.'))?;
            let (per_serving, written_unit) = amount_and_unit(value)?;
            let nutrient = find(field);
            let unit = if written_unit.is_empty() {
                nutrient.map_or("", |n| n.default_unit).to_string()
            } else {
                written_unit
            };
            Some(Entry {
                field,
                nutrient,
                per_serving,
                unit,
            })
        })
        .collect()
}

/// The per-serving amount of the nutrient called `name`.
fn amount_of(entries: &[Entry<'_>], name: &str) -> Option<f64> {
    entries
        .iter()
        .find(|e| e.nutrient.is_some_and(|n| n.name == name))
        .map(|e| e.per_serving)
}

/// The three macros as (kind, nutrient name) in the order they are listed.
const MACROS: [(&str, &str); 3] = [
    ("fat", "fatContent"),
    ("carb", "carbohydrateContent"),
    ("prot", "proteinContent"),
];

/// The energy of one serving in kcal: the `kcal` value, or worked out from
/// the macros when they are all given.
fn energy_per_serving(entries: &[Entry<'_>]) -> Option<f64> {
    amount_of(entries, "calories").or_else(|| {
        let carbs = amount_of(entries, "carbohydrateContent")?;
        let fat = amount_of(entries, "fatContent")?;
        let proteins = amount_of(entries, "proteinContent")?;
        Some(carbs * 4.0 + fat * 9.0 + proteins * 4.0)
    })
}

/// The donut's arcs when the carbohydrates, fat and proteins are all given.
fn ring_of(entries: &[Entry<'_>]) -> Option<(Vec<RingArc>, [u32; 3])> {
    let shares = energy_shares(
        amount_of(entries, "carbohydrateContent")?,
        amount_of(entries, "fatContent")?,
        amount_of(entries, "proteinContent")?,
    )?;
    Some((ring_arcs(shares), percents(shares)))
}

/// The percentage of `MACROS[i]` out of the order carbohydrates, fat,
/// proteins that `percents` uses.
fn macro_percent(percents: &[u32; 3], kind: &str) -> u32 {
    match kind {
        "carb" => percents[0],
        "fat" => percents[1],
        _ => percents[2],
    }
}

/// The slice of the donut a macro is: `fat`, `carb` or `prot`.
fn macro_kind(name: &str) -> Option<&'static str> {
    match name {
        "fatContent" => Some("fat"),
        "carbohydrateContent" => Some("carb"),
        "proteinContent" => Some("prot"),
        _ => None,
    }
}

/// Which slice of the donut a nutrient belongs to, for the highlight on click:
/// a macro is its own slice, sub-nutrients belong to their parent's, the energy
/// belongs to all of them and the rest to none.
fn group_name(nutrient: Option<&Nutrient>) -> String {
    match nutrient {
        Some(n) if n.energy => "all",
        Some(n) => macro_kind(n.name)
            .or_else(|| n.parent.and_then(macro_kind))
            .unwrap_or("other"),
        None => "other",
    }
    .to_string()
}

/// Capitalises the first letter of a translated label.
fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Builds the panel from `nutrition:` (a mapping of numeric values per
/// serving). `base` is the servings the recipe was written for and `chosen`
/// the servings the page is scaled to. Returns `None` when `nutrition:` is
/// not a mapping or holds no numeric value, so the list form keeps its
/// plain family line.
pub fn build_nutrition_panel(
    nutrition: Option<&serde_yaml::Value>,
    weight: Option<&serde_yaml::Value>,
    servings: Option<(f64, f64)>,
    lang: &LanguageIdentifier,
) -> Option<NutritionPanel> {
    let mapping = nutrition?.as_mapping()?;
    // The weight is that of the whole recipe: it only means something next to
    // the number of servings it makes.
    let weight = servings.and(weight).and_then(parse_weight_grams);
    let per_serving_only = servings.is_none();
    let (base, chosen) = servings.unwrap_or((1.0, 1.0));
    let renderer = NutritionRenderer;
    let entries = entries(mapping);
    if entries.is_empty() {
        return None;
    }
    let l = |key: &str| crate::web::i18n::LOCALES.lookup(lang, key);
    let label_of = |entry: &Entry<'_>| {
        if entry.nutrient.is_some_and(|n| n.energy) {
            l("nutrition-energy")
        } else {
            translated_nutrient_label(entry.field, lang).unwrap_or_else(|| entry.field.to_string())
        }
    };

    // The full table: known nutrients in the table's order, then any other
    // numeric field as written.
    let mut table: Vec<(usize, NutritionRow)> = entries
        .iter()
        .map(|entry| {
            // Per 100 g: the recipe holds `base` servings of `per_serving` each.
            let per_100g = weight.map(|w| entry.text(entry.per_serving * base / w * 100.0));
            let sub = entry.nutrient.is_some_and(|n| n.parent.is_some());
            // A sub-nutrient reads "of which saturated fat": its label stays lower case.
            let label = if sub {
                label_of(entry)
            } else {
                capitalize(&label_of(entry))
            };
            (
                entry.nutrient.map_or(usize::MAX, Nutrient::index),
                NutritionRow {
                    icon: renderer.mapping_icon(entry.field).map(str::to_string),
                    label,
                    sub,
                    group: group_name(entry.nutrient),
                    per_serving: entry.text(entry.per_serving),
                    per_100g,
                    total: entry.text(entry.per_serving * chosen),
                },
            )
        })
        .collect();
    table.sort_by_key(|(order, _)| *order);
    let rows = table.into_iter().map(|(_, row)| row).collect();

    // The donut and the three macros, for the servings chosen.
    let ring = ring_of(&entries);
    let macros = MACROS
        .iter()
        .filter_map(|(kind, name)| {
            let entry = entries
                .iter()
                .find(|e| e.nutrient.is_some_and(|n| n.name == *name))?;
            Some(MacroLine {
                kind: kind.to_string(),
                label: capitalize(&label_of(entry)),
                amount: entry.text(entry.per_serving * chosen),
                pct: ring.as_ref().map(|(_, p)| macro_percent(p, kind)),
            })
        })
        .collect();

    // The small cards: what to watch, coloured by the amount per 100 g.
    let cards: Vec<NutrientCard> = [
        "saturatedFatContent",
        "unsaturatedFatContent",
        "monounsaturatedFatContent",
        "polyunsaturatedFatContent",
        "transFatContent",
        "sugarContent",
        "starchContent",
        "fiberContent",
        "salt",
    ]
    .iter()
    .filter_map(|name| {
        let entry = entries
            .iter()
            .find(|e| e.nutrient.is_some_and(|n| n.name == *name))?;
        let level = match weight {
            Some(w) if entry.unit == "g" => level_for(name, entry.per_serving * base / w * 100.0),
            _ => "neutral",
        };
        // The word says the health rating, like the colour: a low sugar and a
        // rich fibre are both "good".
        let word = match level {
            "low" | "good" => Some(l("nutrition-rating-good")),
            "mid" => Some(l("nutrition-rating-medium")),
            "high" => Some(l("nutrition-rating-bad")),
            _ => None,
        };
        Some(NutrientCard {
            icon: renderer.mapping_icon(entry.field).map(str::to_string),
            label: capitalize(&label_of(entry)),
            value: entry.text(entry.per_serving * chosen),
            level: level.to_string(),
            word,
            group: group_name(entry.nutrient),
        })
    })
    .collect();

    // The legend lists the health ratings the cards actually use, once each,
    // and only when the weight made them mean something. A low sugar and a rich
    // fibre are both "good for health", so they share the green entry.
    let legend: Vec<LegendItem> = if weight.is_some() {
        [
            ("low", &["low", "good"][..], "nutrition-legend-good"),
            ("mid", &["mid"][..], "nutrition-legend-medium"),
            ("high", &["high"][..], "nutrition-legend-bad"),
            ("neutral", &["neutral"][..], "nutrition-legend-unrated"),
        ]
        .iter()
        .filter(|(_, levels, _)| {
            cards
                .iter()
                .any(|c: &NutrientCard| levels.contains(&c.level.as_str()))
        })
        .map(|(swatch, _, key)| LegendItem {
            level: swatch.to_string(),
            word: l(key),
        })
        .collect()
    } else {
        Vec::new()
    };

    let energy = energy_per_serving(&entries);
    Some(NutritionPanel {
        title: l("nutrition-title"),
        per_serving_label: l("nutrition-per-serving"),
        per_100g_label: l("nutrition-per-100g"),
        whole_recipe_label: l("nutrition-whole-recipe"),
        of_which: l("nutrition-of-which"),
        legend_label: l("nutrition-legend"),
        legend,
        for_label: l("nutrition-for"),
        full_table_label: l("nutrition-full-table"),
        servings: fmt_unknown(chosen),
        has_100g: weight.is_some(),
        per_serving_only,
        kcal_total: energy.map(|e| format!("{:.0}", e * chosen)),
        kcal_per_serving: energy.map(|e| format!("{e:.0}")),
        ring: ring.map(|(arcs, _)| arcs),
        macros,
        cards,
        rows,
    })
}

/// What a recipe card shows: the donut with the three macros, per serving.
/// `None` unless the recipe gives carbohydrates, fat and proteins, so the
/// card keeps the plain kcal badge.
pub fn build_list_nutrition(
    metadata: &cooklang_find::Metadata,
    lang: &LanguageIdentifier,
) -> Option<ListNutrition> {
    let mapping = metadata.get("nutrition")?.as_mapping()?;
    let entries = entries(mapping);
    let (arcs, shares) = ring_of(&entries)?;
    let energy = energy_per_serving(&entries)?;
    let macros = MACROS
        .iter()
        .filter_map(|(kind, name)| {
            let entry = entries
                .iter()
                .find(|e| e.nutrient.is_some_and(|n| n.name == *name))?;
            let label = translated_nutrient_label(entry.field, lang)
                .unwrap_or_else(|| entry.field.to_string());
            Some(MacroLine {
                kind: kind.to_string(),
                label: capitalize(&label),
                amount: entry.text(entry.per_serving),
                pct: Some(macro_percent(&shares, kind)),
            })
        })
        .collect();
    Some(ListNutrition {
        kcal: format!("{energy:.0}"),
        arcs,
        macros,
        per_serving: crate::web::i18n::LOCALES.lookup(lang, "nutrition-per-serving"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yaml(s: &str) -> serde_yaml::Value {
        serde_yaml::from_str(s).unwrap()
    }

    #[test]
    fn weight_forms() {
        assert_eq!(parse_weight_grams(&yaml("900")), Some(900.0));
        assert_eq!(parse_weight_grams(&yaml("\"900g\"")), Some(900.0));
        assert_eq!(parse_weight_grams(&yaml("\"0.9 kg\"")), Some(900.0));
        assert_eq!(parse_weight_grams(&yaml("\"2 cups\"")), None);
        assert_eq!(parse_weight_grams(&yaml("0")), None);
    }

    #[test]
    fn total_follows_servings_and_100g_does_not() {
        let nutrition = yaml("kcal: 200\nproteins: 5");
        let weight = yaml("800g");
        let lang = LanguageIdentifier::default();
        // Written for 4 servings of 200 kcal, weighing 800 g in all.
        let base = build_nutrition_panel(Some(&nutrition), Some(&weight), Some((4.0, 4.0)), &lang)
            .unwrap();
        let scaled =
            build_nutrition_panel(Some(&nutrition), Some(&weight), Some((4.0, 6.0)), &lang)
                .unwrap();
        assert_eq!(base.rows[0].per_serving, "200 kcal");
        assert_eq!(base.rows[0].total, "800 kcal");
        assert_eq!(scaled.rows[0].total, "1200 kcal");
        // 4 x 200 kcal over 800 g = 100 kcal per 100 g, at any scale.
        assert_eq!(base.rows[0].per_100g.as_deref(), Some("100 kcal"));
        assert_eq!(scaled.rows[0].per_100g, base.rows[0].per_100g);
        assert_eq!(scaled.kcal_total.as_deref(), Some("1200"));
        assert_eq!(scaled.kcal_per_serving.as_deref(), Some("200"));
    }

    #[test]
    fn rows_follow_the_table_order_and_decimals() {
        // Written out of order, with a sub-nutrient and a salt value.
        let nutrition = yaml("salt: 0.05\nsaturated-fat: 7.6\nlipids: 12.1\nkcal: 110.6");
        let lang = LanguageIdentifier::default();
        let panel = build_nutrition_panel(Some(&nutrition), None, Some((1.0, 1.0)), &lang).unwrap();
        let amounts: Vec<&str> = panel.rows.iter().map(|r| r.per_serving.as_str()).collect();
        assert_eq!(amounts, ["111 kcal", "12.1 g", "7.6 g", "0.05 g"]);
        assert!(panel.rows[2].sub && !panel.rows[1].sub);
    }

    #[test]
    fn the_ring_shares_add_up_to_one_hundred() {
        // Butter-like: nearly all fat.
        let shares = energy_shares(0.5, 12.1, 0.5).unwrap();
        assert!((shares.iter().sum::<f64>() - 100.0).abs() < 1e-9);
        assert_eq!(percents(shares), [2, 96, 2]);
        let arcs = ring_arcs(shares);
        assert_eq!(
            arcs.iter().map(|a| a.kind.as_str()).collect::<Vec<_>>(),
            ["carb", "fat", "prot"]
        );
        assert_eq!(arcs[0].dashoffset, "25.00");
    }

    #[test]
    fn levels_follow_the_amount_per_100g() {
        assert_eq!(level_for("sugarContent", 0.6), "low");
        assert_eq!(level_for("sugarContent", 10.0), "mid");
        assert_eq!(level_for("saturatedFatContent", 42.2), "high");
        assert_eq!(level_for("fiberContent", 3.0), "good");
        assert_eq!(level_for("fiberContent", 0.6), "neutral");
        assert_eq!(level_for("proteinContent", 30.0), "neutral");
    }

    #[test]
    fn the_legend_lists_the_colours_in_use() {
        let nutrition = yaml("saturatedFatContent: 7\nsugarContent: 0.5\nstarchContent: 3");
        let weight = yaml("100g");
        let lang = LanguageIdentifier::default();
        let panel = build_nutrition_panel(Some(&nutrition), Some(&weight), Some((1.0, 1.0)), &lang)
            .unwrap();
        let levels: Vec<&str> = panel.legend.iter().map(|i| i.level.as_str()).collect();
        // saturated fat 7 g per 100 g is high, sugars 0.5 g low, starch has no threshold.
        assert_eq!(levels, ["low", "high", "neutral"]);
        // Without a weight every card is grey and nothing needs explaining.
        let none = build_nutrition_panel(Some(&nutrition), None, Some((1.0, 1.0)), &lang).unwrap();
        assert!(none.legend.is_empty());
    }

    #[test]
    fn low_and_rich_share_one_green_entry() {
        // sugars 0.5 g per 100 g is low, fibre 8 g per 100 g is rich: both green.
        let nutrition = yaml(
            "sugarContent: 0.5
fiberContent: 8",
        );
        let weight = yaml("100g");
        let lang = LanguageIdentifier::default();
        let panel = build_nutrition_panel(Some(&nutrition), Some(&weight), Some((1.0, 1.0)), &lang)
            .unwrap();
        assert_eq!(panel.legend.len(), 1);
        assert_eq!(panel.legend[0].level, "low");
    }

    #[test]
    fn cards_are_neutral_without_a_weight() {
        let nutrition = yaml("sugars: 40");
        let lang = LanguageIdentifier::default();
        let panel = build_nutrition_panel(Some(&nutrition), None, Some((1.0, 1.0)), &lang).unwrap();
        assert_eq!(panel.cards[0].level, "neutral");
        assert!(panel.cards[0].word.is_none());
    }

    #[test]
    fn without_a_whole_number_of_servings_everything_is_per_serving() {
        // `servings: 10-12`: the weight of the whole recipe cannot give an amount per 100 g.
        let nutrition = yaml("kcal: 79\nproteins: 4.7");
        let weight = yaml("325g");
        let lang = LanguageIdentifier::default();
        let panel = build_nutrition_panel(Some(&nutrition), Some(&weight), None, &lang).unwrap();
        assert!(panel.per_serving_only && !panel.has_100g);
        assert!(panel.rows[0].per_100g.is_none());
        assert_eq!(panel.kcal_total.as_deref(), Some("79"));
        assert_eq!(panel.cards.len(), 0);
    }

    #[test]
    fn sub_nutrients_follow_their_macro_for_the_highlight() {
        let nutrition = yaml(
            "calories: 100\nfatContent: 5\nsaturatedFatContent: 2\nsugarContent: 1\nsalt: 0.1",
        );
        let lang = LanguageIdentifier::default();
        let panel = build_nutrition_panel(Some(&nutrition), None, Some((1.0, 1.0)), &lang).unwrap();
        let group = |label_part: &str| {
            panel
                .rows
                .iter()
                .find(|r| r.label.to_lowercase().contains(label_part))
                .map(|r| r.group.clone())
        };
        assert_eq!(group("energy").as_deref(), Some("all"));
        assert_eq!(group("fat").as_deref(), Some("fat"));
        assert_eq!(
            panel
                .cards
                .iter()
                .find(|c| c.label.to_lowercase().contains("saturated"))
                .map(|c| c.group.as_str()),
            Some("fat")
        );
        assert_eq!(
            panel
                .cards
                .iter()
                .find(|c| c.label.to_lowercase().contains("sugar"))
                .map(|c| c.group.as_str()),
            Some("carb")
        );
        assert_eq!(
            panel
                .cards
                .iter()
                .find(|c| c.label.to_lowercase().contains("salt"))
                .map(|c| c.group.as_str()),
            Some("other")
        );
    }

    #[test]
    fn every_sub_nutrient_gets_a_card_under_its_macro() {
        let nutrition = yaml(
            "fatContent: 8.1\nsaturatedFatContent: 0.9\nmonounsaturatedFatContent: 3.4\npolyunsaturatedFatContent: 3.0\n\
             transFatContent: 0.1\ncarbohydrateContent: 60.5\nsugarContent: 42.5\nstarchContent: 15.2\nfiberContent: 6.7\nsalt: 0.21",
        );
        let lang = LanguageIdentifier::default();
        let panel =
            build_nutrition_panel(Some(&nutrition), None, Some((12.0, 12.0)), &lang).unwrap();
        let groups: Vec<&str> = panel.cards.iter().map(|c| c.group.as_str()).collect();
        assert_eq!(
            groups,
            ["fat", "fat", "fat", "fat", "carb", "carb", "other", "other"]
        );
        // The table lists them in the table's order, the sub-nutrients indented.
        assert_eq!(panel.rows.iter().filter(|r| r.sub).count(), 6);
    }

    #[test]
    fn no_weight_means_no_100g_column() {
        let nutrition = yaml("kcal: 200");
        let lang = LanguageIdentifier::default();
        let panel = build_nutrition_panel(Some(&nutrition), None, Some((4.0, 4.0)), &lang).unwrap();
        assert!(!panel.has_100g);
        assert!(panel.rows[0].per_100g.is_none());
        assert!(panel.ring.is_none());
    }

    #[test]
    fn list_form_has_no_panel() {
        let nutrition = yaml("[\"258%kcal\"]");
        let lang = LanguageIdentifier::default();
        assert!(build_nutrition_panel(Some(&nutrition), None, Some((1.0, 1.0)), &lang).is_none());
    }
}
