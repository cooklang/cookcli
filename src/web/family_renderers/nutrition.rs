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

//! Specific renderer for the `nutrition:` family.

use super::FamilyRenderer;
use fluent_templates::Loader;
use unic_langid::LanguageIdentifier;

/// Tabler Icons (MIT license, https://tabler.io/icons).
const ICON_FLAME: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2c1 3-2 4-2 7a2 2 0 0 0 4 0c1 1 2 2.5 2 4.5A6 6 0 0 1 4 13.5C4 9 8 7 8 3c1.5 1 2 2.5 1.5 4C10 5 11 3.5 12 2Z"/></svg>"##;
const ICON_MEAT: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 3h9l3 5-8 13L4 8Z"/><path d="M4 8h16"/><path d="M9.5 3 12 8l2.5-5"/></svg>"##;
const ICON_DROPLET: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3c2.5 2 5 5.5 5 9a5 5 0 0 1-10 0c0-1.5.7-2.7 1.5-3.8C9 9 9.5 8 9.5 6.5 10.5 8 11 5.5 12 3Z"/></svg>"##;
const ICON_DROPLET_FILLED: &str = r##"<svg viewBox="0 0 24 24" fill="currentColor" stroke="none"><path d="M12 3c2.5 2 5 5.5 5 9a5 5 0 0 1-10 0c0-1.5.7-2.7 1.5-3.8C9 9 9.5 8 9.5 6.5 10.5 8 11 5.5 12 3Z"/></svg>"##;
const ICON_CANDY: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 3"/></svg>"##;
const ICON_WHEAT: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 4c4 0 4 4 8 4s4-4 8-4M4 4v16M20 4v16M4 12c4 0 4 4 8 4s4-4 8-4M4 20h16"/></svg>"##;
const ICON_BREAD: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 12c0-4.5 3.5-8 8-8s8 3.5 8 8v5a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2v-5Z"/><path d="M9 12h6"/></svg>"##;
const ICON_SALT: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 3h6l1 4H8l1-4Z"/><path d="M8 7h8l1 12a2 2 0 0 1-2 2H9a2 2 0 0 1-2-2L8 7Z"/><circle cx="10.5" cy="12.5" r=".6" fill="currentColor" stroke="none"/><circle cx="13.5" cy="12.5" r=".6" fill="currentColor" stroke="none"/><circle cx="12" cy="15.5" r=".6" fill="currentColor" stroke="none"/></svg>"##;

/// Specific renderer for the `nutrition` family. Used both for the recipe
/// page's family line ([`super::build_custom_list_families`]) and, directly,
/// for the recipe list page's kcal badge ([`super::extract_nutrition_kcal`]).
pub(super) struct NutritionRenderer;

impl FamilyRenderer for NutritionRenderer {
    fn list_icon(&self, text: &str) -> Option<&'static str> {
        // Legacy list form: entries are free text written by the recipe
        // author (not translated), so match on keywords in the text itself.
        let lower = text.to_lowercase();
        if lower.contains("kcal") || lower.contains("cal") || lower.contains("energy") {
            Some(ICON_FLAME)
        } else if lower.contains("protein") {
            Some(ICON_MEAT)
        } else if lower.contains("lipid") || lower.contains("fat") {
            Some(ICON_DROPLET)
        } else if lower.contains("sugar") {
            Some(ICON_CANDY)
        } else if lower.contains("fiber") || lower.contains("fibre") {
            Some(ICON_WHEAT)
        } else {
            None
        }
    }

    fn mapping_icon(&self, field: &str) -> Option<&'static str> {
        // Mapping form: the field name stays in English regardless of the
        // UI locale, so match on it directly instead of the translated text.
        match field.to_lowercase().as_str() {
            "kcal" | "cal" | "calories" | "energy" => Some(ICON_FLAME),
            "proteins" | "protein" => Some(ICON_MEAT),
            "lipids" | "lipid" | "fat" | "fats" => Some(ICON_DROPLET),
            "saturated-fat" | "saturated_fat" => Some(ICON_DROPLET_FILLED),
            "sugars" | "sugar" => Some(ICON_CANDY),
            "fibers" | "fiber" | "fibre" | "fibres" => Some(ICON_WHEAT),
            "carbohydrates" | "carbohydrate" | "carbs" => Some(ICON_BREAD),
            "salt" | "sodium" => Some(ICON_SALT),
            _ => None,
        }
    }

    /// A bare number (e.g. `proteins: 9.8`) gets its unit inferred from the
    /// field name; the legacy `"value%unit"` form (e.g. `"45.3%g"`) still
    /// works too. The field name then becomes a localized label composed via
    /// [`format_with_label`]: `("proteins", "9.8")` -> `"9.8 g fat"` (en) /
    /// `"9.8 g lipides"` (fr). `("kcal", "234")` -> `"234 kcal"` (no repeated
    /// unit when it already matches the field name — `kcal` needs no
    /// translation).
    fn format_mapping_entry(&self, field: &str, raw: &str, lang: &LanguageIdentifier) -> String {
        let (amount, mut unit) = match raw.split_once('%') {
            Some((a, u)) => (a.trim(), u.trim().to_string()),
            None => (raw.trim(), String::new()),
        };
        if unit.is_empty() {
            if let Some(default_unit) = default_nutrition_unit(field) {
                unit = default_unit.to_string();
            }
        }
        if unit.is_empty() || unit.eq_ignore_ascii_case(field) {
            format!("{amount} {unit}").trim().to_string()
        } else {
            let label = translated_nutrient_label(field, lang).unwrap_or_else(|| field.to_string());
            format_with_label(amount, &unit, &label)
        }
    }

    fn note(&self, lang: &LanguageIdentifier) -> Option<String> {
        Some(crate::web::i18n::LOCALES.lookup(lang, "nutrition-per-serving"))
    }
}

/// The unit implied by a nutrient field name when the recipe author wrote a
/// bare number (e.g. `kcal: 234`, `proteins: 9.8`) instead of the
/// `"value%unit"` form. Returns `None` for unknown fields, which are then
/// shown with no unit at all rather than a guessed one.
fn default_nutrition_unit(field: &str) -> Option<&'static str> {
    match field.to_lowercase().as_str() {
        "kcal" | "cal" | "calories" | "energy" => Some("kcal"),
        "proteins" | "protein" | "lipids" | "lipid" | "fat" | "fats" | "saturated-fat"
        | "saturated_fat" | "carbohydrates" | "carbohydrate" | "carbs" | "sugars" | "sugar"
        | "fibers" | "fiber" | "fibre" | "fibres" | "salt" | "sodium" => Some("g"),
        _ => None,
    }
}

/// Translates a known nutrient field name via the app's Fluent locales.
/// Returns `None` for anything else (including `kcal`, which needs no
/// translation), so the caller can fall back to the raw field name as
/// authored.
fn translated_nutrient_label(field: &str, lang: &LanguageIdentifier) -> Option<String> {
    let key = match field.to_lowercase().as_str() {
        "proteins" | "protein" => "nutrition-proteins",
        "lipids" | "lipid" | "fat" | "fats" => "nutrition-lipids",
        "saturated-fat" | "saturated_fat" => "nutrition-saturated-fat",
        "carbohydrates" | "carbohydrate" | "carbs" => "nutrition-carbohydrates",
        "sugars" | "sugar" => "nutrition-sugars",
        "fibers" | "fiber" | "fibre" | "fibres" => "nutrition-fibers",
        "salt" | "sodium" => "nutrition-salt",
        _ => return None,
    };
    Some(crate::web::i18n::LOCALES.lookup(lang, key))
}

/// Composes `"{amount} {unit} {label}"`, e.g. `"73.3 g fat"` / `"73,3 g
/// lipides"` — just the translated nutrient name after the amount and unit,
/// no preposition (only `label` is localized; word order is the same in
/// every supported locale).
fn format_with_label(amount: &str, unit: &str, label: &str) -> String {
    format!("{amount} {unit} {label}")
}
