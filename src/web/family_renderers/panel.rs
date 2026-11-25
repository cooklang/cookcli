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
//! column and the per-100 g ratings. Per serving and per 100 g never change
//! with the scale; the amounts shown for "the servings chosen" do.
//!
//! **Nothing here decides how nutrition is read.** The shape of the table (what
//! nests in what, where the fibre sits), the reference intakes behind the
//! percentages, how a nutrient is rated, what counts as well present and the
//! substances the donut is made of all come from the active
//! [standard](cooklang_format::reference::Standard), a file of
//! `crates/format/data/standards/`. This module lays out what that file says.

use super::nutrition::{
    decimal_separator, localized_number, nutrient_label_key, translated_group_label,
    translated_nutrient_label, NutritionRenderer,
};
use super::FamilyRenderer;
use crate::util::format::nutrition::{amount_and_unit, find, to_micrograms, Nutrient};
use crate::util::format::reference::{Node, Rating, Standard};
use crate::web::templates::{
    LegendItem, ListNutrition, MacroLine, NutrientCard, NutritionPanel, NutritionPick,
    NutritionRow, RingArc, WellPresent,
};
use fluent_templates::Loader;
use std::collections::{BTreeMap, HashMap};
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

/// The slice of the donut a substance is: `fat`, `carb` or `prot`. These are the
/// names of the colours of `components.css`; which substances the energy is made
/// of is the standard's `energy_sources`.
fn kind_of(name: &str) -> Option<&'static str> {
    match name {
        "fatContent" => Some("fat"),
        "carbohydrateContent" => Some("carb"),
        "proteinContent" => Some("prot"),
        _ => None,
    }
}

/// Energy in kcal per gram of what a slice is made of: the standard's own factors
/// (Annex XIV of the EU regulation), or the general Atwater factors where a
/// standard gives none (the FDA pages read state no value).
fn kcal_per_gram(standard: &Standard, kind: &str) -> f64 {
    let (key, atwater) = match kind {
        "fat" => ("fat", 9.0),
        "carb" => ("carbohydrate", 4.0),
        _ => ("protein", 4.0),
    };
    standard
        .energy_factors
        .get(key)
        .map_or(atwater, |factor| factor.kcal)
}

/// Each slice's share of the energy, as percentages that add up to 100, in the
/// order carbohydrates, fat, proteins.
fn energy_shares(carbs: f64, fat: f64, proteins: f64, kcal: (f64, f64, f64)) -> Option<[f64; 3]> {
    let energy = [carbs * kcal.0, fat * kcal.1, proteins * kcal.2];
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

/// One nutrient the recipe gives, in the unit of the nutrient (as written for a key the table
/// does not know).
struct Entry<'a> {
    field: &'a str,
    nutrient: Option<&'static Nutrient>,
    per_serving: f64,
    unit: String,
}

impl Entry<'_> {
    /// `value` as shown, with `separator` between the units and the decimals.
    fn text(&self, value: f64, separator: &str) -> String {
        let number = fmt_amount(self.nutrient, value).replace('.', separator);
        format!("{number} {}", unit_label(&self.unit))
            .trim()
            .to_string()
    }

    /// The canonical name of the nutrient, when the table knows it.
    fn canon(&self) -> Option<&'static str> {
        self.nutrient.map(|n| n.name)
    }
}

/// The numeric entries of a `nutrition:` mapping, hidden (`.`-prefixed) ones
/// left out.
fn entries(mapping: &serde_yaml::Mapping) -> Vec<Entry<'_>> {
    mapping
        .iter()
        .filter_map(|(key, value)| {
            let field = key.as_str().filter(|f| !f.starts_with('.'))?;
            let nutrient = find(field);
            let (per_serving, unit) = match nutrient {
                Some(n) => (n.amount(value)?, n.default_unit.to_string()),
                None => amount_and_unit(value)?,
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

/// What a recipe gives once the standard has read it.
struct Prepared<'a> {
    entries: Vec<Entry<'a>>,
}

/// The entries of `mapping`, read the way the standard reads a label: salt and sodium are one
/// quantity, so the one the norm declares is there even when the recipe gave the other; and a
/// norm that gives carbohydrates without the fibre gets them without it.
fn prepare<'a>(mapping: &'a serde_yaml::Mapping, standard: &Standard) -> Prepared<'a> {
    let mut entries = entries(mapping);
    let position =
        |entries: &[Entry<'_>], canon: &str| entries.iter().position(|e| e.canon() == Some(canon));
    let factor = standard.conventions.salt_per_sodium;
    let (salt, sodium) = (
        position(&entries, "salt"),
        position(&entries, "sodiumContent"),
    );
    if standard.conventions.declares == "sodium" {
        if let (Some(i), None) = (salt, sodium) {
            if let Some(grams) = to_micrograms(entries[i].per_serving, &entries[i].unit) {
                entries.push(Entry {
                    field: "sodiumContent",
                    nutrient: find("sodiumContent"),
                    per_serving: grams / 1e6 / factor * 1000.0,
                    unit: "mg".to_string(),
                });
            }
        }
    } else if let (None, Some(i)) = (salt, sodium) {
        if let Some(grams) = to_micrograms(entries[i].per_serving, &entries[i].unit) {
            entries.push(Entry {
                field: "salt",
                nutrient: find("salt"),
                per_serving: grams / 1e6 * factor,
                unit: "g".to_string(),
            });
        }
    }
    if !standard.conventions.fibre_in_carbohydrate {
        if let (Some(c), Some(f)) = (
            position(&entries, "carbohydrateContent"),
            position(&entries, "fiberContent"),
        ) {
            if entries[c].unit == "g" && entries[f].unit == "g" {
                entries[c].per_serving = (entries[c].per_serving - entries[f].per_serving).max(0.0);
            }
        }
    }
    Prepared { entries }
}

/// The per-serving amount of the nutrient called `name`.
fn amount_of(entries: &[Entry<'_>], name: &str) -> Option<f64> {
    entries
        .iter()
        .find(|e| e.canon() == Some(name))
        .map(|e| e.per_serving)
}

/// The slices of the donut the standard names, in its order, as (kind, nutrient name).
fn slices(standard: &Standard) -> Vec<(&'static str, &str)> {
    standard
        .energy_sources
        .iter()
        .filter_map(|name| Some((kind_of(name)?, name.as_str())))
        .collect()
}

/// The energy of one serving in kcal: the `kcal` value, or worked out from
/// the macros when they are all given.
fn energy_per_serving(entries: &[Entry<'_>], standard: &Standard) -> Option<f64> {
    amount_of(entries, "calories").or_else(|| {
        let carbs = amount_of(entries, "carbohydrateContent")?;
        let fat = amount_of(entries, "fatContent")?;
        let proteins = amount_of(entries, "proteinContent")?;
        Some(
            carbs * kcal_per_gram(standard, "carb")
                + fat * kcal_per_gram(standard, "fat")
                + proteins * kcal_per_gram(standard, "prot"),
        )
    })
}

/// The donut's arcs when the carbohydrates, fat and proteins are all given.
fn ring_of(entries: &[Entry<'_>], standard: &Standard) -> Option<(Vec<RingArc>, [u32; 3])> {
    let shares = energy_shares(
        amount_of(entries, "carbohydrateContent")?,
        amount_of(entries, "fatContent")?,
        amount_of(entries, "proteinContent")?,
        (
            kcal_per_gram(standard, "carb"),
            kcal_per_gram(standard, "fat"),
            kcal_per_gram(standard, "prot"),
        ),
    )?;
    Some((ring_arcs(shares), percents(shares)))
}

/// The percentage of the slice `kind` out of the order carbohydrates, fat,
/// proteins that `percents` uses.
fn macro_percent(percents: &[u32; 3], kind: &str) -> u32 {
    match kind {
        "carb" => percents[0],
        "fat" => percents[1],
        _ => percents[2],
    }
}

/// The slice of the donut a nutrient belongs to in the standard's tree (a macro is its
/// own, a sub-nutrient is its macro's), or `None`.
fn slice_of(standard: &Standard, canon: &str) -> Option<&'static str> {
    let mut name = Some(canon);
    let mut found = None;
    while let Some(current) = name {
        if standard.energy_sources.iter().any(|s| s == current) {
            found = kind_of(current);
        }
        name = standard.parent_of(current);
    }
    found
}

/// Capitalises the first letter of a translated label.
fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// `text` with the unit `ug` written as `µg`.
fn unit_label(unit: &str) -> &str {
    if matches!(unit, "ug" | "mcg" | "μg") {
        "µg"
    } else {
        unit
    }
}

/// A sum of masses, in the unit the terms share, or in the one that reads best.
fn format_mass(micrograms: f64, unit: Option<&str>, separator: &str) -> String {
    let unit = unit
        .filter(|u| to_micrograms(1.0, u).is_some())
        .unwrap_or(if micrograms >= 1e6 {
            "g"
        } else if micrograms >= 1e3 {
            "mg"
        } else {
            "µg"
        });
    let value = micrograms / to_micrograms(1.0, unit).unwrap_or(1.0);
    let decimals = match unit {
        "g" => {
            if value >= 10.0 {
                1
            } else {
                2
            }
        }
        "mg" => usize::from(value < 100.0),
        _ => 0,
    };
    let number = format!("{value:.decimals$}").replace('.', separator);
    format!("{number} {}", unit_label(unit))
}

/// What the table and the cards are made from.
struct Layout<'a> {
    standard: &'a Standard,
    entries: &'a [Entry<'a>],
    /// What stands before the decimals in the language of the page.
    separator: String,
    weight: Option<f64>,
    base: f64,
    chosen: f64,
    per_serving_only: bool,
    lang: &'a LanguageIdentifier,
}

impl Layout<'_> {
    fn l(&self, key: &str) -> String {
        crate::web::i18n::LOCALES.lookup(self.lang, key)
    }

    fn label_of(&self, entry: &Entry<'_>) -> String {
        if entry.nutrient.is_some_and(|n| n.energy) {
            self.l("nutrition-energy")
        } else {
            translated_nutrient_label(entry.field, self.lang)
                .unwrap_or_else(|| entry.field.to_string())
        }
    }

    /// An amount of what `entry` gives, as shown.
    fn text(&self, entry: &Entry<'_>, value: f64) -> String {
        entry.text(value, &self.separator)
    }

    /// The amount per 100 g of what one serving holds.
    fn per_100g(&self, per_serving: f64) -> Option<f64> {
        self.weight.map(|w| per_serving * self.base / w * 100.0)
    }

    /// Whether a nutrient of the tree has a row: the recipe gave it.
    fn has(&self, canon: &str) -> bool {
        self.entries.iter().any(|e| e.canon() == Some(canon))
    }

    /// Whether something under `name` in the tree has a row.
    fn below(&self, name: &str) -> bool {
        self.standard.tree.iter().any(|c| {
            c.parent.as_deref() == Some(name)
                && (if c.group {
                    self.below(&c.name)
                } else {
                    self.has(&c.name) || self.below(&c.name)
                })
        })
    }

    fn shown(&self, node: &Node) -> bool {
        if node.group {
            self.below(&node.name)
        } else {
            self.has(&node.name)
        }
    }

    /// The nodes above `node` that the table shows, nearest first.
    fn ancestors(&self, node: &Node) -> Vec<&Node> {
        let mut out = Vec::new();
        let mut parent = node.parent.as_deref().and_then(|p| self.standard.node(p));
        while let Some(p) = parent {
            if self.shown(p) {
                out.push(p);
            }
            parent = p.parent.as_deref().and_then(|p| self.standard.node(p));
        }
        out
    }
}

/// What a family adds up: the three columns in micrograms, and the unit its terms share.
type Sum = ([Option<f64>; 3], Option<String>);

/// A row with the numbers the subtotals add up, kept next to it.
struct Built {
    row: NutritionRow,
    /// The masses of the three columns in micrograms; `None` for energy and for a family.
    mass: [Option<f64>; 3],
    unit: String,
}

/// The table, laid out by the tree of the standard: every nutrient the recipe gave that the tree
/// knows sits where the tree puts it, under the families that hold it; what the tree does not
/// know follows in a family of its own.
fn table_rows(layout: &Layout<'_>) -> Vec<NutritionRow> {
    let standard = layout.standard;
    let renderer = NutritionRenderer;
    let by_canon: HashMap<&str, &Entry<'_>> = layout
        .entries
        .iter()
        .rev()
        .filter_map(|e| Some((e.canon()?, e)))
        .collect();
    let mut built: Vec<Built> = Vec::new();
    let mut placed: Vec<&str> = Vec::new();
    for node in &standard.tree {
        if !layout.shown(node) {
            continue;
        }
        let ancestors = layout.ancestors(node);
        let depth = ancestors.len();
        let parent = ancestors.first().copied();
        let part = parent.is_some_and(|p| !p.group) && node.relation.as_deref() != Some("listed");
        let sub = !node.group
            && (depth >= 2 || (depth == 1 && parent.is_some_and(|p| p.name != "macronutrients")));
        let family = ancestors.last().copied().unwrap_or(node);
        let family_kind = if family.group {
            match family.name.as_str() {
                "macronutrients" => "macro",
                "minerals" => "min",
                "vitamins" => "vit",
                _ => "other",
            }
        } else {
            ""
        };
        let mac = slice_of(standard, &node.name).unwrap_or("");
        let group = if !mac.is_empty() {
            mac
        } else if node.name == "calories" || node.name == "macronutrients" {
            "all"
        } else {
            "other"
        };
        let mut row = NutritionRow {
            icon: None,
            label: String::new(),
            sub,
            part,
            fam: node.group,
            family: family_kind.to_string(),
            mac: mac.to_string(),
            depth: depth.min(255) as u8,
            group: group.to_string(),
            key: node.name.clone(),
            per_serving: String::new(),
            per_100g: None,
            total: String::new(),
            pct: None,
            foldable: false,
            simple: 0,
        };
        let mut mass = [None; 3];
        let mut unit = String::new();
        if node.group {
            row.label = capitalize(&translated_group_label(&node.name, layout.lang));
        } else if let Some(entry) = by_canon.get(node.name.as_str()) {
            placed.push(&node.name);
            let label = layout.label_of(entry);
            row.label = if part { label } else { capitalize(&label) };
            row.icon = renderer.mapping_icon(entry.field).map(str::to_string);
            row.simple = SIMPLE_VIEW
                .iter()
                .find(|(name, _)| *name == node.name)
                .map_or(0, |(_, of_which)| if *of_which { 2 } else { 1 });
            row.per_serving = layout.text(entry, entry.per_serving);
            row.per_100g = layout
                .per_100g(entry.per_serving)
                .map(|v| layout.text(entry, v));
            row.total = layout.text(entry, entry.per_serving * layout.chosen);
            row.pct = if node.name == "calories" {
                (entry.unit == "kcal")
                    .then(|| (entry.per_serving / standard.energy_kcal * 100.0).round() as u32)
            } else {
                standard
                    .share_of(&node.name, entry.per_serving, &entry.unit)
                    .map(|share| (share * 100.0).round() as u32)
            };
            mass = [
                to_micrograms(entry.per_serving, &entry.unit),
                layout
                    .per_100g(entry.per_serving)
                    .and_then(|v| to_micrograms(v, &entry.unit)),
                to_micrograms(entry.per_serving * layout.chosen, &entry.unit),
            ];
            unit = entry.unit.clone();
        }
        built.push(Built { row, mass, unit });
    }

    // What the tree does not know: unknown fields, and nutrients this norm has no place for.
    let loose: Vec<&Entry<'_>> = layout
        .entries
        .iter()
        .filter(|e| !e.canon().is_some_and(|c| placed.contains(&c)))
        .collect();
    if !loose.is_empty() {
        let new_row = |label: String, depth: u8, fam: bool| NutritionRow {
            icon: None,
            label,
            sub: !fam,
            part: false,
            fam,
            family: "other".to_string(),
            mac: String::new(),
            depth,
            group: "other".to_string(),
            key: if fam {
                "otherData".to_string()
            } else {
                String::new()
            },
            per_serving: String::new(),
            per_100g: None,
            total: String::new(),
            pct: None,
            foldable: false,
            simple: 0,
        };
        built.push(Built {
            row: new_row(layout.l("nutrition-group-other-data"), 0, true),
            mass: [None; 3],
            unit: String::new(),
        });
        for entry in loose {
            // a nutrient is named like any other row, an unknown key stays as written
            let label = layout.label_of(entry);
            let label = match entry.nutrient {
                Some(_) => capitalize(&label),
                None => label,
            };
            let mut row = new_row(label, 1, false);
            row.icon = renderer.mapping_icon(entry.field).map(str::to_string);
            row.per_serving = layout.text(entry, entry.per_serving);
            row.per_100g = layout
                .per_100g(entry.per_serving)
                .map(|v| layout.text(entry, v));
            row.total = layout.text(entry, entry.per_serving * layout.chosen);
            built.push(Built {
                row,
                mass: [None; 3],
                unit: String::new(),
            });
        }
    }

    // A family shows the sum of what sits directly under it (a sub-family counts as its own sum),
    // in mass. Adding milligrams of different minerals or vitamins says nothing about nutrition,
    // so only the other families get a subtotal.
    let mut sums: Vec<Option<Sum>> = vec![None; built.len()];
    for i in (0..built.len()).rev() {
        if !built[i].row.fam {
            continue;
        }
        let depth = built[i].row.depth;
        let mut total = [Some(0.0); 3];
        let mut units: Vec<String> = Vec::new();
        for j in i + 1..built.len() {
            if built[j].row.depth <= depth {
                break;
            }
            if built[j].row.depth != depth + 1 {
                continue;
            }
            let (mass, unit) = match &sums[j] {
                Some((mass, unit)) if built[j].row.fam => (*mass, unit.clone().unwrap_or_default()),
                _ => (built[j].mass, built[j].unit.clone()),
            };
            units.push(unit);
            for (slot, value) in total.iter_mut().zip(mass) {
                if let (Some(sum), Some(value)) = (slot.as_mut(), value) {
                    *sum += value;
                }
            }
        }
        let common = units
            .first()
            .filter(|first| units.iter().all(|u| u == *first) && !first.is_empty())
            .cloned();
        sums[i] = Some((total, common));
    }
    for (i, item) in built.iter_mut().enumerate() {
        if !item.row.fam || matches!(item.row.family.as_str(), "min" | "vit") {
            continue;
        }
        if let Some((mass, unit)) = &sums[i] {
            let text = |value: Option<f64>| {
                value
                    .filter(|v| *v > 0.0)
                    .map(|v| format_mass(v, unit.as_deref(), &layout.separator))
                    .unwrap_or_default()
            };
            item.row.per_serving = text(mass[0]);
            item.row.per_100g = layout.weight.map(|_| text(mass[1]));
            item.row.total = text(mass[2]);
        }
    }

    // A row with something under it can be folded.
    let depths: Vec<u8> = built.iter().map(|b| b.row.depth).collect();
    for (i, item) in built.iter_mut().enumerate() {
        item.row.foldable = depths.get(i + 1).is_some_and(|d| *d > item.row.depth);
    }
    let _ = layout.per_serving_only;
    built.into_iter().map(|b| b.row).collect()
}

/// The essential table, the one that came before the standards: these rows, flat, in the
/// order of the tree, with an "of which" line for the saturated fat and the sugars. It is a way of
/// looking that belongs to no standard, so it is the one place that names nutrients for a layout.
const SIMPLE_VIEW: &[(&str, bool)] = &[
    ("calories", false),
    ("fatContent", false),
    ("saturatedFatContent", true),
    ("carbohydrateContent", false),
    ("sugarContent", true),
    ("fiberContent", false),
    ("proteinContent", false),
    ("salt", false),
];

/// The small cards: the nutrients the standard singles out, coloured by its own rating.
fn cards_of(layout: &Layout<'_>, renderer: &NutritionRenderer) -> Vec<NutrientCard> {
    let standard = layout.standard;
    standard
        .cards
        .iter()
        .filter_map(|canon| {
            let entry = layout
                .entries
                .iter()
                .find(|e| e.canon() == Some(canon.as_str()))?;
            let level = if to_micrograms(1.0, &entry.unit).is_some() {
                standard.rate(
                    canon,
                    layout.per_100g(entry.per_serving),
                    entry.per_serving,
                    &entry.unit,
                )
            } else {
                "neutral"
            };
            // The word says the rating, like the colour: a low sugar and a rich fibre are both "good".
            let word = match level {
                "low" | "good" => Some(layout.l("nutrition-rating-good")),
                "mid" => Some(layout.l("nutrition-rating-medium")),
                "high" => Some(layout.l("nutrition-rating-bad")),
                _ => None,
            };
            Some(NutrientCard {
                icon: renderer.mapping_icon(entry.field).map(str::to_string),
                label: capitalize(&layout.label_of(entry)),
                value: layout.text(entry, entry.per_serving * layout.chosen),
                level: level.to_string(),
                word,
                group: slice_of(standard, canon).unwrap_or("other").to_string(),
                claim: None,
                key: canon.clone(),
            })
        })
        .collect()
}

/// The vitamins and minerals the standard's rule calls well present, the most present first.
fn well_present(layout: &Layout<'_>, renderer: &NutritionRenderer) -> Option<WellPresent> {
    let standard = layout.standard;
    let mut found: Vec<(f64, NutrientCard)> = layout
        .entries
        .iter()
        .filter_map(|entry| {
            let canon = entry.canon()?;
            let presence = standard.presence(
                canon,
                layout.per_100g(entry.per_serving),
                entry.per_serving,
                &entry.unit,
            )?;
            let share = standard.share_of(canon, entry.per_serving, &entry.unit)?;
            Some((
                share,
                NutrientCard {
                    icon: renderer.mapping_icon(entry.field).map(str::to_string),
                    label: capitalize(&layout.label_of(entry)),
                    value: layout.text(entry, entry.per_serving * layout.chosen),
                    level: "good".to_string(),
                    word: None,
                    group: "other".to_string(),
                    key: canon.to_string(),
                    claim: Some(layout.l(if presence == "high" {
                        "nutrition-well-high"
                    } else {
                        "nutrition-well-source"
                    })),
                },
            ))
        })
        .collect();
    if found.is_empty() {
        return None;
    }
    found.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let rule = &standard.significant_amount;
    let args = HashMap::from([
        (
            std::borrow::Cow::Borrowed("source"),
            fluent::FluentValue::from(rule.source_percent.unwrap_or(0.0)),
        ),
        (
            std::borrow::Cow::Borrowed("high"),
            fluent::FluentValue::from(rule.high_percent),
        ),
    ]);
    let key = match (rule.basis.as_str(), rule.source_percent) {
        ("per-serving", None) => "nutrition-well-rule-serving-high",
        ("per-serving", Some(_)) => "nutrition-well-rule-serving",
        _ => "nutrition-well-rule-100g",
    };
    Some(WellPresent {
        title: layout.l("nutrition-well-present"),
        rule: crate::web::i18n::LOCALES.lookup_with_args(layout.lang, key, &args),
        tiles: found.into_iter().map(|(_, card)| card).collect(),
    })
}

/// One explanation bubble: what a nutrient is, what it is for, where to find it, where that text
/// comes from, then the reference intake and the rating of the standard in use and the recipe's figure.
#[derive(serde::Serialize)]
struct Tip {
    title: String,
    what: Option<String>,
    #[serde(rename = "use")]
    usage: Option<String>,
    #[serde(rename = "where")]
    whereabouts: Option<String>,
    source: String,
    reference: String,
    rating: Option<String>,
    recipe: String,
}

/// A message with its arguments.
fn say(
    lang: &LanguageIdentifier,
    key: &str,
    args: &[(&'static str, fluent::FluentValue<'static>)],
) -> String {
    let args: HashMap<std::borrow::Cow<'static, str>, fluent::FluentValue<'static>> = args
        .iter()
        .map(|(name, value)| (std::borrow::Cow::Borrowed(*name), value.clone()))
        .collect();
    crate::web::i18n::LOCALES.lookup_with_args(lang, key, &args)
}

/// The text of a message, when the locale (or English) has it.
fn try_say(lang: &LanguageIdentifier, key: &str) -> Option<String> {
    crate::web::i18n::LOCALES.try_lookup(lang, key)
}

/// An amount of a nutrient the way a person reads it: `800 mg`, `2.5 µg`.
fn amount_text(amount: f64, unit: &str, separator: &str) -> String {
    let number = fmt_unknown(amount).replace('.', separator);
    format!("{number} {}", unit_label(unit))
}

/// How the standard in use rates a nutrient, in a sentence; `None` when it does not.
fn rating_line(layout: &Layout<'_>, canon: &str) -> Option<String> {
    use fluent::FluentValue as V;
    let (standard, lang) = (layout.standard, layout.lang);
    let factor = standard.conventions.salt_per_sodium;
    // salt and sodium are one quantity: what the norm says of one holds for the other
    let twin = match canon {
        "salt" => Some("sodiumContent"),
        "sodiumContent" => Some("salt"),
        _ => None,
    };
    let names = [Some(canon), twin];
    let threshold = |v: f64| V::from(localized_number(v, &layout.separator));
    if let Rating::TrafficLight { nutrients, .. } = &standard.rating {
        if let Some(t) = nutrients.get(canon) {
            return Some(say(
                lang,
                "nutrition-help-rating-traffic",
                &[
                    ("low", threshold(t.low)),
                    ("high", threshold(t.high)),
                    ("unit", V::from(t.unit.clone())),
                ],
            ));
        }
    }
    if matches!(
        standard.rating,
        Rating::Claims {} | Rating::TrafficLight { .. }
    ) {
        let mut parts = Vec::new();
        for claim in standard.claims.iter().filter(|c| c.per == "100g") {
            if !names.contains(&Some(claim.nutrient.as_str())) {
                continue;
            }
            let Some(name) = try_say(lang, &format!("nutrition-claim-{}", claim.claim)) else {
                continue;
            };
            // a claim written for sodium reads as salt on the salt card
            let as_salt = canon == "salt" && claim.nutrient == "sodiumContent";
            let convert = |v: f64| {
                if as_salt {
                    to_micrograms(v, &claim.unit).map_or(v, |ug| ug / 1e6 * factor)
                } else {
                    v
                }
            };
            let unit = if as_salt { "g" } else { claim.unit.as_str() };
            if let (true, Some(max)) = (claim.claim.starts_with("low-"), claim.max) {
                parts.push(say(
                    lang,
                    "nutrition-help-rating-claim-max",
                    &[
                        ("name", V::from(name)),
                        ("amount", threshold(convert(max))),
                        ("unit", V::from(unit.to_string())),
                    ],
                ));
            } else if let Some(min) = claim.min {
                parts.push(say(
                    lang,
                    "nutrition-help-rating-claim-min",
                    &[
                        ("name", V::from(name)),
                        ("amount", threshold(convert(min))),
                        ("unit", V::from(unit.to_string())),
                    ],
                ));
            }
        }
        return (!parts.is_empty()).then(|| parts.join(" "));
    }
    if let Rating::PercentDv {
        low_percent,
        high_percent,
    } = &standard.rating
    {
        let (low, high) = (V::from(*low_percent), V::from(*high_percent));
        for name in names.into_iter().flatten() {
            if standard.direction.limit.iter().any(|n| n == name) {
                return Some(say(
                    lang,
                    "nutrition-help-rating-dv-limit",
                    &[("low", low), ("high", high)],
                ));
            }
            if standard.direction.encourage.iter().any(|n| n == name) {
                return Some(say(
                    lang,
                    "nutrition-help-rating-dv-encourage",
                    &[("high", high)],
                ));
            }
        }
        if names
            .into_iter()
            .flatten()
            .all(|n| standard.intake(n).is_none())
        {
            return Some(l_none(lang));
        }
    }
    None
}

fn l_none(lang: &LanguageIdentifier) -> String {
    crate::web::i18n::LOCALES.lookup(lang, "nutrition-help-rating-dv-none")
}

/// The bubbles, as JSON for the page's script.
fn tips_json(layout: &Layout<'_>) -> String {
    use fluent::FluentValue as V;
    let (standard, lang) = (layout.standard, layout.lang);
    let standard_name = layout.l(&format!("standard-{}", standard.id));
    let reference_amount =
        |amount: f64, unit: &str| V::from(amount_text(amount, unit, &layout.separator));
    let mut tips: BTreeMap<String, Tip> = BTreeMap::new();
    for entry in layout.entries {
        let Some(canon) = entry.canon() else { continue };
        let slug = if canon == "calories" {
            Some("energy")
        } else {
            nutrient_label_key(canon).and_then(|key| key.strip_prefix("nutrition-"))
        };
        let part =
            |what: &str| slug.and_then(|s| try_say(lang, &format!("nutrition-help-{s}-{what}")));
        let source = match slug.and_then(|s| try_say(lang, &format!("nutrition-help-{s}-source"))) {
            Some(source) => say(
                lang,
                "nutrition-help-source",
                &[("source", V::from(source))],
            ),
            None => layout.l("nutrition-help-source-todo"),
        };
        let reference = match (canon, standard.intake(canon)) {
            ("calories", _) => say(
                lang,
                "nutrition-help-reference",
                &[
                    ("standard", V::from(standard_name.clone())),
                    ("amount", reference_amount(standard.energy_kcal, "kcal")),
                ],
            ),
            (_, Some(intake)) => say(
                lang,
                "nutrition-help-reference",
                &[
                    ("standard", V::from(standard_name.clone())),
                    ("amount", reference_amount(intake.amount, &intake.unit)),
                ],
            ),
            _ => say(
                lang,
                "nutrition-help-reference-none",
                &[("standard", V::from(standard_name.clone()))],
            ),
        };
        let pct = if canon == "calories" {
            (entry.unit == "kcal").then(|| entry.per_serving / standard.energy_kcal * 100.0)
        } else {
            standard
                .share_of(canon, entry.per_serving, &entry.unit)
                .map(|s| s * 100.0)
        };
        let amount = layout.text(entry, entry.per_serving);
        let recipe = match pct {
            Some(p) => say(
                lang,
                "nutrition-help-recipe-pct",
                &[("amount", V::from(amount)), ("pct", V::from(p.round()))],
            ),
            None => say(
                lang,
                "nutrition-help-recipe",
                &[("amount", V::from(amount))],
            ),
        };
        tips.insert(
            canon.to_string(),
            Tip {
                title: capitalize(&layout.label_of(entry)),
                what: part("what"),
                usage: part("use"),
                whereabouts: part("where"),
                source,
                reference,
                rating: rating_line(layout, canon),
                recipe,
            },
        );
    }
    let labels = serde_json::json!({
        "what": layout.l("nutrition-help-what"),
        "use": layout.l("nutrition-help-use"),
        "where": layout.l("nutrition-help-where"),
    });
    // `<` is escaped so no text can close the script element it is written in
    serde_json::json!({ "labels": labels, "tips": tips })
        .to_string()
        .replace('<', "\\u003c")
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
    standard: &Standard,
) -> Option<NutritionPanel> {
    let mapping = nutrition?.as_mapping()?;
    // The weight is that of the whole recipe: it only means something next to
    // the number of servings it makes.
    let weight = servings.and(weight).and_then(parse_weight_grams);
    let per_serving_only = servings.is_none();
    let (base, chosen) = servings.unwrap_or((1.0, 1.0));
    let renderer = NutritionRenderer;
    let prepared = prepare(mapping, standard);
    if prepared.entries.is_empty() {
        return None;
    }
    let layout = Layout {
        standard,
        entries: &prepared.entries,
        separator: decimal_separator(lang),
        weight,
        base,
        chosen,
        per_serving_only,
        lang,
    };
    let l = |key: &str| layout.l(key);

    let rows = table_rows(&layout);

    // The donut and its slices, for the servings chosen.
    let ring = ring_of(&prepared.entries, standard);
    let macros = slices(standard)
        .into_iter()
        .filter_map(|(kind, name)| {
            let entry = prepared.entries.iter().find(|e| e.canon() == Some(name))?;
            Some(MacroLine {
                kind: kind.to_string(),
                label: capitalize(&layout.label_of(entry)),
                amount: layout.text(entry, entry.per_serving * chosen),
                pct: ring.as_ref().map(|(_, p)| macro_percent(p, kind)),
            })
        })
        .collect();

    let cards = cards_of(&layout, &renderer);

    // The legend lists the ratings the standard can give, once each, and greys those no card has.
    // A low sugar and a rich fibre are both "good for health", so they share the green entry. The
    // EU only gives a green (a claim earned) or nothing: no amber and no red to explain.
    let legend: Vec<LegendItem> = if cards.is_empty() {
        Vec::new()
    } else {
        let claims_only = matches!(standard.rating, Rating::Claims {});
        [
            ("low", &["low", "good"][..], "nutrition-legend-good"),
            ("mid", &["mid"][..], "nutrition-legend-medium"),
            ("high", &["high"][..], "nutrition-legend-bad"),
            ("neutral", &["neutral"][..], "nutrition-legend-unrated"),
        ]
        .iter()
        .filter(|(swatch, _, _)| !claims_only || matches!(*swatch, "low" | "neutral"))
        .map(|(swatch, levels, key)| LegendItem {
            level: swatch.to_string(),
            word: l(key),
            used: cards
                .iter()
                .any(|c: &NutrientCard| levels.contains(&c.level.as_str())),
        })
        .collect()
    };
    // What the legend title says depends on how the standard rates.
    let legend_label = match standard.rating {
        Rating::Claims {} => l("nutrition-legend-claims"),
        Rating::PercentDv { .. } => l("nutrition-legend-dv"),
        _ => l("nutrition-legend"),
    };

    let energy = energy_per_serving(&prepared.entries, standard);
    Some(NutritionPanel {
        title: l("nutrition-title"),
        per_serving_label: l("nutrition-per-serving"),
        per_100g_label: l("nutrition-per-100g"),
        whole_recipe_label: l("nutrition-whole-recipe"),
        of_which: l("nutrition-of-which"),
        legend_label,
        legend,
        for_label: l("nutrition-for"),
        full_table_label: l("nutrition-more-info"),
        less_label: l("nutrition-less-info"),
        collapse_all_label: l("nutrition-collapse-all"),
        expand_all_label: l("nutrition-expand-all"),
        fold_label: l("nutrition-fold"),
        picks: crate::util::format::reference::standards()
            .values()
            .map(|s| NutritionPick {
                id: s.id.clone(),
                name: l(&format!("standard-{}", s.id)),
                selected: s.id == standard.id,
            })
            .chain(std::iter::once(NutritionPick {
                id: crate::web::language::ESSENTIAL.to_string(),
                name: l("standard-essential"),
                selected: false,
            }))
            .collect(),
        essential: false,
        pick_label: l("nutrition-pick-label"),
        well_present: well_present(&layout, &renderer),
        tips_json: tips_json(&layout),
        servings: fmt_unknown(chosen).replace('.', &layout.separator),
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
    standard: &Standard,
) -> Option<ListNutrition> {
    let mapping = metadata.get("nutrition")?.as_mapping()?;
    let prepared = prepare(mapping, standard);
    let (arcs, shares) = ring_of(&prepared.entries, standard)?;
    let energy = energy_per_serving(&prepared.entries, standard)?;
    let separator = decimal_separator(lang);
    let macros = slices(standard)
        .into_iter()
        .filter_map(|(kind, name)| {
            let entry = prepared.entries.iter().find(|e| e.canon() == Some(name))?;
            let label = translated_nutrient_label(entry.field, lang)
                .unwrap_or_else(|| entry.field.to_string());
            Some(MacroLine {
                kind: kind.to_string(),
                label: capitalize(&label),
                amount: entry.text(entry.per_serving, &separator),
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
    use crate::util::format::reference::standard;

    fn yaml(s: &str) -> serde_yaml::Value {
        serde_yaml::from_str(s).unwrap()
    }

    fn eu() -> &'static Standard {
        standard("eu").unwrap()
    }

    fn panel(
        nutrition: &str,
        weight: Option<&str>,
        servings: Option<(f64, f64)>,
        which: &str,
    ) -> NutritionPanel {
        let nutrition = yaml(nutrition);
        let weight = weight.map(yaml);
        build_nutrition_panel(
            Some(&nutrition),
            weight.as_ref(),
            servings,
            &LanguageIdentifier::default(),
            standard(which).unwrap(),
        )
        .unwrap()
    }

    fn row<'a>(panel: &'a NutritionPanel, key: &str) -> &'a NutritionRow {
        panel
            .rows
            .iter()
            .find(|r| r.key == key)
            .unwrap_or_else(|| panic!("no row {key}"))
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
        // Written for 4 servings of 200 kcal, weighing 800 g in all.
        let base = panel(
            "kcal: 200\nproteins: 5",
            Some("800g"),
            Some((4.0, 4.0)),
            "eu",
        );
        let scaled = panel(
            "kcal: 200\nproteins: 5",
            Some("800g"),
            Some((4.0, 6.0)),
            "eu",
        );
        let energy = row(&base, "calories");
        assert_eq!(energy.per_serving, "200 kcal");
        assert_eq!(energy.total, "800 kcal");
        assert_eq!(row(&scaled, "calories").total, "1200 kcal");
        // 4 x 200 kcal over 800 g = 100 kcal per 100 g, at any scale.
        assert_eq!(energy.per_100g.as_deref(), Some("100 kcal"));
        assert_eq!(row(&scaled, "calories").per_100g, energy.per_100g);
        assert_eq!(scaled.kcal_total.as_deref(), Some("1200"));
        assert_eq!(scaled.kcal_per_serving.as_deref(), Some("200"));
    }

    #[test]
    fn rows_follow_the_tree_of_the_standard_and_the_decimals_of_the_table() {
        // Written out of order, with a sub-nutrient and a salt value.
        let p = panel(
            "salt: 0.05\nsaturated-fat: 7.6\nlipids: 12.1\nkcal: 110.6",
            None,
            Some((1.0, 1.0)),
            "eu",
        );
        let keys: Vec<&str> = p.rows.iter().map(|r| r.key.as_str()).collect();
        // energy, the macronutrients family and what is under it, then the other components
        assert_eq!(
            keys,
            [
                "calories",
                "macronutrients",
                "fatContent",
                "saturatedFatContent",
                "other",
                "salt"
            ]
        );
        let amounts: Vec<&str> = p
            .rows
            .iter()
            .filter(|r| !r.fam)
            .map(|r| r.per_serving.as_str())
            .collect();
        assert_eq!(amounts, ["111 kcal", "12.1 g", "7.6 g", "0.05 g"]);
        assert!(row(&p, "saturatedFatContent").sub && !row(&p, "fatContent").sub);
        assert!(row(&p, "saturatedFatContent").part && !row(&p, "fatContent").part);
    }

    #[test]
    fn the_fibre_sits_where_the_standard_puts_it() {
        let recipe = "carbohydrateContent: 58\nsugarContent: 7.5\nfiberContent: 14.5\nproteinContent: 38\nfatContent: 36";
        let in_eu = panel(recipe, None, Some((1.0, 1.0)), "eu");
        let in_us = panel(recipe, None, Some((1.0, 1.0)), "us");
        // next to the carbohydrates in the EU, a part of them in the US
        assert_eq!(
            row(&in_eu, "fiberContent").depth,
            row(&in_eu, "carbohydrateContent").depth
        );
        assert!(!row(&in_eu, "fiberContent").part);
        assert_eq!(
            row(&in_us, "fiberContent").depth,
            row(&in_us, "carbohydrateContent").depth + 1
        );
        assert!(row(&in_us, "fiberContent").part);
        // and it follows the carbohydrates when a slice of the donut is picked
        assert_eq!(row(&in_us, "fiberContent").group, "carb");
        assert_eq!(row(&in_eu, "fiberContent").group, "other");
    }

    #[test]
    fn carbohydrates_are_shown_without_the_fibre_where_the_norm_gives_them_so() {
        let recipe = "carbohydrateContent: 58\nfiberContent: 14.5";
        let eu = panel(recipe, None, Some((1.0, 1.0)), "eu");
        let us = panel(recipe, None, Some((1.0, 1.0)), "us");
        assert_eq!(row(&eu, "carbohydrateContent").per_serving, "43.5 g");
        assert_eq!(row(&us, "carbohydrateContent").per_serving, "58.0 g");
    }

    #[test]
    fn the_percentages_use_the_reference_intakes_of_the_standard() {
        let recipe = "calcium: 142\nvitaminD: 12.5\ncholesterolContent: 78";
        let eu = panel(recipe, None, Some((1.0, 1.0)), "eu");
        let us = panel(recipe, None, Some((1.0, 1.0)), "us");
        // 142 mg of 800 mg in the EU, of 1300 mg in the US
        assert_eq!(row(&eu, "calcium").pct, Some(18));
        assert_eq!(row(&us, "calcium").pct, Some(11));
        assert_eq!(row(&eu, "vitaminD").pct, Some(250));
        assert_eq!(row(&us, "vitaminD").pct, Some(63));
        // the EU has no reference for cholesterol, the US has
        assert_eq!(row(&eu, "cholesterolContent").pct, None);
        assert_eq!(row(&us, "cholesterolContent").pct, Some(26));
    }

    #[test]
    fn the_label_gives_salt_or_sodium_and_the_other_follows_from_the_factor() {
        // a recipe that gives salt, read by a norm that declares sodium: 1,4 g of salt is 560 mg
        let us = panel("salt: 1.4", None, Some((1.0, 1.0)), "us");
        assert_eq!(row(&us, "sodiumContent").per_serving, "560 mg");
        assert_eq!(row(&us, "salt").per_serving, "1.40 g");
        // and the other way round
        let eu = panel("sodiumContent: 560", None, Some((1.0, 1.0)), "eu");
        assert_eq!(row(&eu, "salt").per_serving, "1.40 g");
    }

    #[test]
    fn a_family_adds_up_what_is_directly_under_it() {
        let p = panel(
            "fatContent: 36\ncarbohydrateContent: 58\nproteinContent: 38\ncalcium: 142\nvitaminD: 12.5\nsalt: 1.4",
            None,
            Some((1.0, 1.0)),
            "us",
        );
        // macronutrients: fat + carbohydrates + proteins, in grams
        assert_eq!(row(&p, "macronutrients").per_serving, "132.0 g");
        // the other components: the salt alone, in its own unit
        assert_eq!(row(&p, "other").per_serving, "1.40 g");
        // minerals and vitamins add up to nothing worth saying
        assert_eq!(row(&p, "minerals").per_serving, "");
        assert_eq!(row(&p, "vitamins").per_serving, "");
        // only a row with something under it can be folded
        assert!(row(&p, "macronutrients").foldable && !row(&p, "carbohydrateContent").foldable);
    }

    #[test]
    fn what_the_tree_does_not_know_follows_in_a_family_of_its_own() {
        let p = panel("kcal: 100\nmystery: 3", None, Some((1.0, 1.0)), "eu");
        let last = p.rows.last().unwrap();
        assert_eq!(last.label, "mystery");
        assert!(last.sub && last.depth == 1);
        assert!(p.rows[p.rows.len() - 2].fam);
    }

    #[test]
    fn bare_numbers_are_in_the_unit_of_their_key() {
        let recipe = "calories: 185\nfatContent: 1.54\nsaturatedFatContent: 0.205\nunsaturatedFatContent: 0.722\nmonounsaturatedFatContent: 0.269\npolyunsaturatedFatContent: 0.453";
        let p = panel(recipe, None, Some((1.0, 1.0)), "eu");
        assert_eq!(row(&p, "calories").per_serving, "185 kcal");
        assert_eq!(row(&p, "fatContent").per_serving, "1.5 g");
        assert_eq!(row(&p, "saturatedFatContent").per_serving, "0.2 g");
        assert_eq!(row(&p, "polyunsaturatedFatContent").per_serving, "0.5 g");
    }

    #[test]
    fn values_written_with_a_unit_are_converted_before_any_figure() {
        // as an importer writes them, and sodium given in grams
        let recipe =
            "calories: 1255 kJ\nfat: 20000 mg\ncarbohydrates: 30 g\nprotein: 10 g\nsodium: 0.4 g";
        let p = panel(recipe, Some("200g"), Some((2.0, 2.0)), "eu");
        assert_eq!(row(&p, "calories").per_serving, "300 kcal");
        assert_eq!(row(&p, "fatContent").per_serving, "20.0 g");
        assert_eq!(row(&p, "fatContent").total, "40.0 g");
        assert_eq!(row(&p, "fatContent").per_100g.as_deref(), Some("20.0 g"));
        assert_eq!(row(&p, "sodiumContent").per_serving, "400 mg");
        // the salt the EU declares follows from the converted sodium
        assert_eq!(row(&p, "salt").per_serving, "1.00 g");
        assert!(p.ring.is_some());
    }

    #[test]
    fn a_nutrient_outside_the_tree_is_named_like_the_others() {
        // the US tree has no fluoride
        let p = panel("fluoride: 0.1", None, Some((1.0, 1.0)), "us");
        assert_eq!(p.rows.last().unwrap().label, "Fluoride");
    }

    #[test]
    fn decimals_follow_the_language() {
        let nutrition = yaml("fat: 12.5\ncarbs: 30\nproteins: 4.2\nsalt: 1.4");
        let lang: LanguageIdentifier = "fr-FR".parse().unwrap();
        let p =
            build_nutrition_panel(Some(&nutrition), None, Some((1.0, 1.0)), &lang, eu()).unwrap();
        assert_eq!(row(&p, "fatContent").per_serving, "12,5 g");
        assert_eq!(row(&p, "macronutrients").per_serving, "46,7 g");
        assert!(p.macros.iter().any(|m| m.amount == "12,5 g"));
        assert!(p.tips_json.contains("1,40 g"), "{}", p.tips_json);
        // the donut is drawn with numbers a browser reads
        assert!(p.ring.unwrap().iter().all(|a| !a.dasharray.contains(',')));
    }

    #[test]
    fn the_ring_shares_add_up_to_one_hundred() {
        // Butter-like: nearly all fat.
        let shares = energy_shares(0.5, 12.1, 0.5, (4.0, 9.0, 4.0)).unwrap();
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
    fn the_energy_factors_come_from_the_standard() {
        // the EU gives 9, 4 and 4 kcal per gram; the US file gives none: the Atwater factors stand in
        assert_eq!(kcal_per_gram(eu(), "fat"), 9.0);
        assert_eq!(kcal_per_gram(eu(), "carb"), 4.0);
        assert!(standard("us").unwrap().energy_factors.is_empty());
        assert_eq!(kcal_per_gram(standard("us").unwrap(), "fat"), 9.0);
    }

    #[test]
    fn cards_are_the_standards_and_are_coloured_by_its_rating() {
        let recipe = "saturatedFatContent: 6.2\nsugarContent: 7.5\nfiberContent: 14.5\nsalt: 1.4";
        let weight = Some("720g");
        let servings = Some((2.0, 2.0));
        let level = |p: &NutritionPanel, label: &str| {
            p.cards
                .iter()
                .find(|c| c.label.to_lowercase().contains(label))
                .map(|c| c.level.clone())
        };
        let uk = panel(recipe, weight, servings, "uk");
        assert_eq!(level(&uk, "saturated").as_deref(), Some("mid"));
        assert_eq!(level(&uk, "sugar").as_deref(), Some("low"));
        let eu = panel(recipe, weight, servings, "eu");
        assert_eq!(level(&eu, "saturated").as_deref(), Some("neutral"));
        assert_eq!(level(&eu, "sugar").as_deref(), Some("good"));
        let us = panel(recipe, weight, servings, "us");
        assert_eq!(level(&us, "saturated").as_deref(), Some("high"));
        // the US card is the sodium, not the salt
        assert!(level(&us, "sodium").is_some() && level(&us, "salt").is_none());
        assert!(level(&eu, "salt").is_some() && level(&eu, "sodium").is_none());
    }

    #[test]
    fn the_legend_lists_what_the_standard_can_give_and_greys_what_no_card_has() {
        let recipe = "saturatedFatContent: 7\nsugarContent: 0.5\nstarchContent: 3";
        let uk = panel(recipe, Some("100g"), Some((1.0, 1.0)), "uk");
        let used = |p: &NutritionPanel| -> Vec<(String, bool)> {
            p.legend.iter().map(|i| (i.level.clone(), i.used)).collect()
        };
        // the UK can give all four; saturated fat 7 g per 100 g is high, sugars 0.5 g low, starch has no threshold
        assert_eq!(
            used(&uk),
            [
                ("low".into(), true),
                ("mid".into(), false),
                ("high".into(), true),
                ("neutral".into(), true)
            ]
        );
        // Without a weight every per-100 g rating is neutral: the legend is still there, only the blue is in use.
        let none = panel(recipe, None, Some((1.0, 1.0)), "uk");
        assert_eq!(none.legend.len(), 4);
        assert_eq!(none.legend.iter().filter(|i| i.used).count(), 1);
        // the EU gives a green for a claim earned and nothing else: no amber, no red
        let eu = panel(recipe, Some("100g"), Some((1.0, 1.0)), "eu");
        let levels: Vec<&str> = eu.legend.iter().map(|i| i.level.as_str()).collect();
        assert_eq!(levels, ["low", "neutral"]);
        // each standard says in its own words how it rates
        let us = panel(recipe, Some("100g"), Some((1.0, 1.0)), "us");
        assert_ne!(eu.legend_label, uk.legend_label);
        assert_ne!(us.legend_label, uk.legend_label);
    }

    #[test]
    fn low_and_rich_share_one_green_entry() {
        // sugars 0.5 g per 100 g is low, fibre 8 g per 100 g is rich: both green, one entry.
        let p = panel(
            "sugarContent: 0.5\nfiberContent: 8",
            Some("100g"),
            Some((1.0, 1.0)),
            "uk",
        );
        assert_eq!(p.legend.iter().filter(|i| i.level == "low").count(), 1);
        assert!(p.legend.iter().find(|i| i.level == "low").unwrap().used);
    }

    #[test]
    fn well_present_follows_the_rule_of_the_standard() {
        // vitamin D 12,5 µg and vitamin E 6,8 mg per serving of 360 g
        let recipe = "vitaminD: 12.5\nvitaminE: 6.8\ncalcium: 142\nsodiumContent: 560";
        let eu = panel(recipe, Some("720g"), Some((2.0, 2.0)), "eu");
        let wp = eu.well_present.expect("a vitamin is well present");
        let labels: Vec<&str> = wp.tiles.iter().map(|t| t.label.as_str()).collect();
        assert_eq!(labels, ["Vitamin D", "Vitamin E"]);
        // no weight: the EU's per-100 g rule gives nothing, the US's per-serving rule still does
        assert!(panel(recipe, None, Some((1.0, 1.0)), "eu")
            .well_present
            .is_none());
        let us = panel(recipe, None, Some((1.0, 1.0)), "us")
            .well_present
            .unwrap();
        assert!(us.tiles.iter().any(|t| t.label == "Vitamin D"));
        // what is to limit is never well present
        assert!(us
            .tiles
            .iter()
            .all(|t| !t.label.to_lowercase().contains("sodium")));
    }

    #[test]
    fn cards_are_neutral_without_a_weight_under_a_per_100g_rating() {
        let p = panel("sugars: 40", None, Some((1.0, 1.0)), "uk");
        assert_eq!(p.cards[0].level, "neutral");
        assert!(p.cards[0].word.is_none());
    }

    #[test]
    fn without_a_whole_number_of_servings_everything_is_per_serving() {
        // `servings: 10-12`: the weight of the whole recipe cannot give an amount per 100 g.
        let p = panel("kcal: 79\nproteins: 4.7", Some("325g"), None, "eu");
        assert!(p.per_serving_only && !p.has_100g);
        assert!(row(&p, "calories").per_100g.is_none());
        assert_eq!(p.kcal_total.as_deref(), Some("79"));
        assert_eq!(p.cards.len(), 0);
    }

    #[test]
    fn sub_nutrients_follow_their_macro_for_the_highlight() {
        let p = panel(
            "calories: 100\nfatContent: 5\nsaturatedFatContent: 2\nsugarContent: 1\nsalt: 0.1",
            None,
            Some((1.0, 1.0)),
            "eu",
        );
        assert_eq!(row(&p, "calories").group, "all");
        assert_eq!(row(&p, "fatContent").group, "fat");
        assert_eq!(row(&p, "saturatedFatContent").group, "fat");
        assert_eq!(row(&p, "sugarContent").group, "carb");
        let card_group = |label: &str| {
            p.cards
                .iter()
                .find(|c| c.label.to_lowercase().contains(label))
                .map(|c| c.group.as_str())
        };
        assert_eq!(card_group("saturated"), Some("fat"));
        assert_eq!(card_group("sugar"), Some("carb"));
        assert_eq!(card_group("salt"), Some("other"));
    }

    #[test]
    fn a_deeper_nutrient_is_indented_further_and_stays_in_its_slice() {
        let p = panel(
            "fatContent: 8\nunsaturatedFatContent: 5\nmonounsaturatedFatContent: 3\npolyunsaturatedFatContent: 2",
            None,
            Some((1.0, 1.0)),
            "eu",
        );
        let depths: Vec<u8> = p.rows.iter().filter(|r| !r.fam).map(|r| r.depth).collect();
        assert_eq!(depths, [1, 2, 3, 3]);
        assert!(p.rows.iter().filter(|r| !r.fam).all(|r| r.group == "fat"));
        // The cards follow: all of them under the fat slice of the donut.
        assert!(p.cards.iter().all(|c| c.group == "fat"));
    }

    #[test]
    fn no_weight_means_no_100g_column() {
        let p = panel("kcal: 200", None, Some((4.0, 4.0)), "eu");
        assert!(!p.has_100g);
        assert!(row(&p, "calories").per_100g.is_none());
        assert!(p.ring.is_none());
    }

    #[test]
    fn list_form_has_no_panel() {
        let nutrition = yaml("[\"258%kcal\"]");
        let lang = LanguageIdentifier::default();
        assert!(
            build_nutrition_panel(Some(&nutrition), None, Some((1.0, 1.0)), &lang, eu()).is_none()
        );
    }

    #[test]
    fn a_bubble_says_what_a_nutrient_is_and_where_the_text_comes_from() {
        let recipe = "saturatedFatContent: 6.2
vitaminD: 12.5
choline: 300
salt: 1.4
sodiumContent: 560";
        let tips = |which: &str| -> serde_json::Value {
            let p = panel(recipe, Some("720g"), Some((2.0, 2.0)), which);
            serde_json::from_str(&p.tips_json).unwrap()
        };
        let eu = tips("eu");
        let sat = &eu["tips"]["saturatedFatContent"];
        assert!(sat["what"].as_str().unwrap().contains("animal"));
        assert!(sat["source"].as_str().unwrap().contains("WHO"));
        // 6,2 g of 20 g is 31 % of the EU's reference intake
        assert!(sat["recipe"].as_str().unwrap().contains("31 %"), "{sat}");
        // a nutrient whose text has not been sourced yet says so and invents nothing
        let choline = &eu["tips"]["choline"];
        assert!(choline["what"].is_null());
        assert!(choline["source"]
            .as_str()
            .unwrap()
            .contains("to be documented"));
        // a vitamin whose text comes from the NHS says what it is for, where to find it, and from which page
        let d = &eu["tips"]["vitaminD"];
        assert!(d["use"].as_str().unwrap().contains("calcium"), "{d}");
        assert!(d["where"].as_str().unwrap().contains("Sunlight"), "{d}");
        assert!(d["source"].as_str().unwrap().contains("NHS, Vitamin D"));
        // the EU rates by claims: "low in salt" is 0,3 g of salt, written for the salt card from the sodium claim
        assert!(
            eu["tips"]["salt"]["rating"]
                .as_str()
                .unwrap()
                .contains("0.3"),
            "{}",
            eu["tips"]["salt"]
        );
        // the UK by traffic lights, the US by the daily value, and a sodium to limit says so
        assert!(tips("uk")["tips"]["saturatedFatContent"]["rating"]
            .as_str()
            .unwrap()
            .contains("1.5"));
        assert!(tips("us")["tips"]["sodiumContent"]["rating"]
            .as_str()
            .unwrap()
            .contains("limit"));
        // the sections of a bubble are labelled
        assert_eq!(eu["labels"]["what"], "What it is");
        // nothing in the data can close the script element it is written in
        assert!(!panel(recipe, None, Some((1.0, 1.0)), "eu")
            .tips_json
            .contains('<'));
    }

    #[test]
    fn every_label_the_trees_need_is_translated() {
        let lang = LanguageIdentifier::default();
        for (id, s) in crate::util::format::reference::standards() {
            for node in s.tree.iter().filter(|n| n.name != "calories") {
                let label = if node.group {
                    translated_group_label(&node.name, &lang)
                } else {
                    translated_nutrient_label(&node.name, &lang).unwrap_or_default()
                };
                // a missing message comes back as "Unknown localization key", or empty
                assert!(
                    !label.is_empty() && !label.contains("Unknown localization"),
                    "{id}: {}",
                    node.name
                );
            }
        }
    }
}
