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

use super::{list_family, FamilyRenderer, PanelContext};
use crate::util::format::nutrition::find;
use crate::web::templates::FamilyPanel;
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
/// A 24x24 stroked Tabler-style icon around `$body`.
macro_rules! tabler {
    ($body:expr) => {
        concat!(
            r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"##,
            $body,
            "</svg>"
        )
    };
}

const ICON_HEART: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19.5 12.572 12 20l-7.5-7.428a5 5 0 1 1 7.5-6.566 5 5 0 1 1 7.5 6.572"/></svg>"##;
const ICON_BONE: &str = tabler!(
    r##"<path d="M7.5 16.5 16.5 7.5"/><circle cx="5.5" cy="15" r="2"/><circle cx="9" cy="18.5" r="2"/><circle cx="15" cy="5.5" r="2"/><circle cx="18.5" cy="9" r="2"/>"##
);
const ICON_BOLT: &str = tabler!(r##"<path d="M13 3 5 13h6l-1 8 8-10h-6Z"/>"##);
const ICON_PULSE: &str = tabler!(r##"<path d="M3 12h4l3-8 4 16 3-8h4"/>"##);
const ICON_BANANA: &str = tabler!(r##"<path d="M5 6c0 8 5 13 15 13-3-3-4-4-6-7-2-2-3-4-3-8Z"/>"##);
const ICON_CUBE: &str = tabler!(
    r##"<path d="m12 3 8 4.5v9L12 21l-8-4.5v-9Z"/><path d="m4 7.5 8 4.5 8-4.5M12 12v9"/>"##
);
const ICON_WAVES: &str =
    tabler!(r##"<path d="M3 9c3-3 6 3 9 0s6 3 9 0M3 15c3-3 6 3 9 0s6 3 9 0"/>"##);
const ICON_MAGNET: &str = tabler!(
    r##"<path d="M4 4h4v9a4 4 0 0 0 8 0V4h4v9a8 8 0 0 1-16 0Z"/><path d="M4 8h4M16 8h4"/>"##
);
const ICON_SHIELD: &str = tabler!(r##"<path d="m12 3 7 3v6c0 4-3 7-7 9-4-2-7-5-7-9V6Z"/>"##);
const ICON_POT: &str = tabler!(
    r##"<path d="M4 10h16v7a3 3 0 0 1-3 3H7a3 3 0 0 1-3-3Z"/><path d="M2 12h2M20 12h2M8 7h8"/>"##
);
const ICON_COG: &str = tabler!(
    r##"<circle cx="12" cy="12" r="3"/><circle cx="12" cy="12" r="7" stroke-dasharray="3.3 2.2"/>"##
);
const ICON_STAR: &str = tabler!(
    r##"<path d="m12 3 2.7 5.6 6.1.9-4.4 4.3 1 6.1-5.4-2.9-5.4 2.9 1-6.1-4.4-4.3 6.1-.9Z"/>"##
);
const ICON_TOOTH: &str = tabler!(
    r##"<path d="M12 6c-2-2-7-2-7 2 0 3 1.5 5 2 8 .3 2 1.5 4 2.5 4s1-3 2.5-3 1.5 3 2.5 3 2.2-2 2.5-4c.5-3 2-5 2-8 0-4-5-4-7-2Z"/>"##
);
const ICON_GLOSS: &str =
    tabler!(r##"<circle cx="12" cy="12" r="9"/><path d="M7.5 12A4.5 4.5 0 0 1 12 7.5"/>"##);
const ICON_NUT: &str =
    tabler!(r##"<path d="m12 3 7.8 4.5v9L12 21l-7.8-4.5v-9Z"/><circle cx="12" cy="12" r="3"/>"##);
const ICON_BUTTERFLY: &str = tabler!(
    r##"<path d="M12 8C10 4 5 4 4.5 8.5S7 17 9 17s3-3 3-9Zm0 0c2-4 7-4 7.5.5S17 17 15 17s-3-3-3-9Z"/>"##
);
const ICON_EYE: &str = tabler!(
    r##"<path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12Z"/><circle cx="12" cy="12" r="3"/>"##
);
const ICON_SUN: &str = tabler!(
    r##"<circle cx="12" cy="12" r="4"/><path d="M12 3v2M12 19v2M3 12h2M19 12h2M5.6 5.6 7 7M17 17l1.4 1.4M18.4 5.6 17 7M7 17l-1.4 1.4"/>"##
);
const ICON_LEAF: &str =
    tabler!(r##"<path d="M5 19C5 10 10 5 20 4c0 10-5 15-14 15Z"/><path d="M5 19 14 10"/>"##);
const ICON_BROCCOLI: &str = tabler!(
    r##"<path d="M12 21v-6"/><path d="M9 15a4 4 0 1 1 1-7.7 3.5 3.5 0 0 1 4 0A4 4 0 1 1 15 15Z"/>"##
);
const ICON_CITRUS: &str = tabler!(
    r##"<circle cx="12" cy="12" r="9"/><path d="M12 3v18M4.2 7.5l15.6 9M19.8 7.5l-15.6 9"/>"##
);
const ICON_GRAIN: &str = tabler!(
    r##"<path d="M12 21V8"/><path d="M12 8c-3 0-4-2-4-4 3 0 4 2 4 4Zm0 0c3 0 4-2 4-4-3 0-4 2-4 4Zm0 6c-3 0-4-2-4-4m4 4c3 0 4-2 4-4"/>"##
);
const ICON_EGG: &str = tabler!(r##"<path d="M12 3c4 0 7 6 7 10a7 7 0 0 1-14 0c0-4 3-10 7-10Z"/>"##);
const ICON_FISH: &str = tabler!(
    r##"<path d="M3 12c3-5 8-6 13-3l5-3v12l-5-3c-5 3-10 2-13-3Z"/><circle cx="8" cy="11.5" r=".6" fill="currentColor" stroke="none"/>"##
);
const ICON_AVOCADO: &str = tabler!(
    r##"<path d="M12 3c2 0 3 2 3 4 3 1 5 4 5 7a8 8 0 0 1-16 0c0-3 2-6 5-7 0-2 1-4 3-4Z"/><circle cx="12" cy="15" r="2.5"/>"##
);
const ICON_SMILE: &str = tabler!(
    r##"<circle cx="12" cy="12" r="9"/><path d="M8 14s1.5 2 4 2 4-2 4-2"/><path d="M9 9.5h.01M15 9.5h.01"/>"##
);
const ICON_HAIR: &str = tabler!(
    r##"<path d="M8 3c-2 4 2 6 0 9s2 6 0 9M12 3c-2 4 2 6 0 9s2 6 0 9M16 3c-2 4 2 6 0 9s2 6 0 9"/>"##
);
const ICON_SPROUT: &str = tabler!(
    r##"<path d="M12 21v-8"/><path d="M12 13c-4 0-6-2-6-6 4 0 6 2 6 6Zm0 0c0-3 2-5 6-5 0 4-2 6-6 6Z"/>"##
);
const ICON_MOLECULE: &str = tabler!(
    r##"<circle cx="6" cy="6" r="2.5"/><circle cx="18" cy="8" r="2.5"/><circle cx="12" cy="18" r="2.5"/><path d="m8 7.5 8 .5M7.2 8.2l3.5 7.6M16.8 10.2l-3.5 5.6"/>"##
);
const ICON_BULB: &str = tabler!(
    r##"<path d="M3 12h1M12 3v1M20 12h1M5.6 5.6l.7.7M18.4 5.6l-.7.7M9 16a5 5 0 1 1 6 0 3.5 3.5 0 0 0-1 3 2 2 0 0 1-4 0 3.5 3.5 0 0 0-1-3M9.7 17h4.6"/>"##
);
const ICON_FLASK: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 3h6"/><path d="M10 9h4"/><path d="M10 3v6l-4 11a.7.7 0 0 0 .5 1h11a.7.7 0 0 0 .5-1l-4-11V3"/></svg>"##;

/// Specific renderer for the `nutrition` family. Used both for the recipe
/// page's family line ([`super::build_family_panels`]) and, directly,
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
        // UI locale, so match on the nutrient it names, not on the text.
        match find(field)?.name {
            "calories" => Some(ICON_FLAME),
            "proteinContent" => Some(ICON_MEAT),
            "fatContent" => Some(ICON_DROPLET),
            "saturatedFatContent" => Some(ICON_DROPLET_FILLED),
            "unsaturatedFatContent"
            | "monounsaturatedFatContent"
            | "polyunsaturatedFatContent"
            | "transFatContent" => Some(ICON_DROPLET),
            "starchContent" => Some(ICON_BREAD),
            "sugarContent" => Some(ICON_CANDY),
            "fiberContent" => Some(ICON_WHEAT),
            "carbohydrateContent" => Some(ICON_BREAD),
            "salt" => Some(ICON_SALT),
            "sodiumContent" => Some(ICON_CUBE),
            "cholesterolContent" => Some(ICON_HEART),
            "addedSugars" => Some(ICON_CANDY),
            "omega3" | "omega6" => Some(ICON_DROPLET),
            "polyols" => Some(ICON_FLASK),
            "calcium" => Some(ICON_BONE),
            "phosphorus" => Some(ICON_BOLT),
            "magnesium" => Some(ICON_PULSE),
            "potassium" => Some(ICON_BANANA),
            "chloride" => Some(ICON_WAVES),
            "iron" => Some(ICON_MAGNET),
            "zinc" => Some(ICON_SHIELD),
            "copper" => Some(ICON_POT),
            "manganese" => Some(ICON_COG),
            "selenium" => Some(ICON_STAR),
            "fluoride" => Some(ICON_TOOTH),
            "chromium" => Some(ICON_GLOSS),
            "molybdenum" => Some(ICON_NUT),
            "iodine" => Some(ICON_BUTTERFLY),
            "vitaminA" => Some(ICON_EYE),
            "vitaminD" => Some(ICON_SUN),
            "vitaminE" => Some(ICON_LEAF),
            "vitaminK" => Some(ICON_BROCCOLI),
            "vitaminC" => Some(ICON_CITRUS),
            "thiamin" => Some(ICON_GRAIN),
            "riboflavin" => Some(ICON_EGG),
            "niacin" => Some(ICON_FISH),
            "pantothenicAcid" => Some(ICON_AVOCADO),
            "vitaminB6" => Some(ICON_SMILE),
            "biotin" => Some(ICON_HAIR),
            "folate" => Some(ICON_SPROUT),
            "vitaminB12" => Some(ICON_MOLECULE),
            "choline" => Some(ICON_BULB),
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
        if unit.is_empty()
            || unit.eq_ignore_ascii_case(field)
            || find(field).is_some_and(|n| n.energy)
        {
            format!("{amount} {unit}").trim().to_string()
        } else {
            let label = translated_nutrient_label(field, lang).unwrap_or_else(|| field.to_string());
            format_with_label(amount, &unit, &label)
        }
    }

    fn note(&self, lang: &LanguageIdentifier) -> Option<String> {
        Some(crate::web::i18n::LOCALES.lookup(lang, "nutrition-per-serving"))
    }

    /// A mapping of numbers becomes the nutrition table; the list form and
    /// anything else keeps the generic line of chips.
    fn panel(
        &self,
        family: &str,
        value: &serde_yaml::Value,
        ctx: &PanelContext<'_>,
    ) -> Option<FamilyPanel> {
        super::panel::build_nutrition_panel(
            Some(value),
            ctx.weight,
            ctx.servings,
            ctx.lang,
            ctx.standard,
        )
        .map(|mut panel| {
            panel.essential = ctx.essential;
            if ctx.essential {
                // the choice is the essential table, not the standard that still rates the cards
                panel
                    .picks
                    .iter_mut()
                    .for_each(|p| p.selected = p.id == "essential");
            }
            FamilyPanel::Nutrition(Box::new(panel))
        })
        .or_else(|| list_family(self, family, value, ctx.lang).map(FamilyPanel::Generic))
    }
}

/// The unit implied by a nutrient field name when the recipe author wrote a
/// bare number (e.g. `kcal: 234`, `proteins: 9.8`) instead of the
/// `"value%unit"` form. Returns `None` for unknown fields, which are then
/// shown with no unit at all rather than a guessed one.
fn default_nutrition_unit(field: &str) -> Option<&'static str> {
    find(field).map(|n| n.default_unit)
}

/// Translates a known nutrient field name via the app's Fluent locales.
/// Returns `None` for anything else (including `kcal`, which needs no
/// translation), so the caller can fall back to the raw field name as
/// authored.
pub(super) fn translated_nutrient_label(field: &str, lang: &LanguageIdentifier) -> Option<String> {
    let key = nutrient_label_key(find(field)?.name)?;
    Some(crate::web::i18n::LOCALES.lookup(lang, key))
}

/// The message that names a nutrient, by its canonical name.
pub(super) fn nutrient_label_key(name: &str) -> Option<&'static str> {
    NUTRIENT_LABELS
        .iter()
        .find(|(canon, _)| *canon == name)
        .map(|(_, key)| *key)
}

/// The message that names each nutrient (`calories` needs none: it is "energy"/`kcal`).
const NUTRIENT_LABELS: &[(&str, &str)] = &[
    ("proteinContent", "nutrition-proteins"),
    ("fatContent", "nutrition-lipids"),
    ("saturatedFatContent", "nutrition-saturated-fat"),
    ("unsaturatedFatContent", "nutrition-unsaturated-fat"),
    ("monounsaturatedFatContent", "nutrition-monounsaturated-fat"),
    ("polyunsaturatedFatContent", "nutrition-polyunsaturated-fat"),
    ("transFatContent", "nutrition-trans-fat"),
    ("starchContent", "nutrition-starch"),
    ("carbohydrateContent", "nutrition-carbohydrates"),
    ("sugarContent", "nutrition-sugars"),
    ("fiberContent", "nutrition-fibers"),
    ("salt", "nutrition-salt"),
    ("cholesterolContent", "nutrition-cholesterol"),
    ("sodiumContent", "nutrition-sodium"),
    ("addedSugars", "nutrition-added-sugars"),
    ("polyols", "nutrition-polyols"),
    ("omega3", "nutrition-omega-3"),
    ("omega6", "nutrition-omega-6"),
    ("calcium", "nutrition-calcium"),
    ("phosphorus", "nutrition-phosphorus"),
    ("magnesium", "nutrition-magnesium"),
    ("potassium", "nutrition-potassium"),
    ("chloride", "nutrition-chloride"),
    ("iron", "nutrition-iron"),
    ("zinc", "nutrition-zinc"),
    ("copper", "nutrition-copper"),
    ("manganese", "nutrition-manganese"),
    ("fluoride", "nutrition-fluoride"),
    ("selenium", "nutrition-selenium"),
    ("chromium", "nutrition-chromium"),
    ("molybdenum", "nutrition-molybdenum"),
    ("iodine", "nutrition-iodine"),
    ("vitaminA", "nutrition-vitamin-a"),
    ("vitaminD", "nutrition-vitamin-d"),
    ("vitaminE", "nutrition-vitamin-e"),
    ("vitaminK", "nutrition-vitamin-k"),
    ("vitaminC", "nutrition-vitamin-c"),
    ("thiamin", "nutrition-vitamin-b1"),
    ("riboflavin", "nutrition-vitamin-b2"),
    ("niacin", "nutrition-vitamin-b3"),
    ("pantothenicAcid", "nutrition-vitamin-b5"),
    ("vitaminB6", "nutrition-vitamin-b6"),
    ("biotin", "nutrition-vitamin-b8"),
    ("folate", "nutrition-vitamin-b9"),
    ("vitaminB12", "nutrition-vitamin-b12"),
    ("choline", "nutrition-choline"),
];

/// The name of a family of a standard's tree (`minerals`, `fatSolubleVitamins`...), translated.
pub(super) fn translated_group_label(group: &str, lang: &LanguageIdentifier) -> String {
    let key = match group {
        "macronutrients" => "nutrition-group-macronutrients",
        "other" => "nutrition-group-other",
        "minerals" => "nutrition-group-minerals",
        "macroMinerals" => "nutrition-group-macro-minerals",
        "traceElements" => "nutrition-group-trace-elements",
        "vitamins" => "nutrition-group-vitamins",
        "fatSolubleVitamins" => "nutrition-group-fat-soluble",
        "waterSolubleVitamins" => "nutrition-group-water-soluble",
        other => return other.to_string(),
    };
    crate::web::i18n::LOCALES.lookup(lang, key)
}

/// Composes `"{amount} {unit} {label}"`, e.g. `"73.3 g fat"` / `"73,3 g
/// lipides"` — just the translated nutrient name after the amount and unit,
/// no preposition (only `label` is localized; word order is the same in
/// every supported locale).
fn format_with_label(amount: &str, unit: &str, label: &str) -> String {
    format!("{amount} {unit} {label}")
}
