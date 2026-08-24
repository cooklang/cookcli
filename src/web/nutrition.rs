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

//! Rendering support for non-standard, list-valued recipe metadata (e.g. a
//! `nutrition:` YAML array). Each such key becomes one "family" line under the
//! tags on the recipe page; see `docs/metadata.md`.

use crate::web::templates::{MetaListFamily, MetaListItem};

/// Tabler Icons (MIT license, https://tabler.io/icons) picked by keyword found
/// in a `nutrition:` entry's unit text.
const ICON_FLAME: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2c1 3-2 4-2 7a2 2 0 0 0 4 0c1 1 2 2.5 2 4.5A6 6 0 0 1 4 13.5C4 9 8 7 8 3c1.5 1 2 2.5 1.5 4C10 5 11 3.5 12 2Z"/></svg>"##;
const ICON_MEAT: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 3h9l3 5-8 13L4 8Z"/><path d="M4 8h16"/><path d="M9.5 3 12 8l2.5-5"/></svg>"##;
const ICON_DROPLET: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3c2.5 2 5 5.5 5 9a5 5 0 0 1-10 0c0-1.5.7-2.7 1.5-3.8C9 9 9.5 8 9.5 6.5 10.5 8 11 5.5 12 3Z"/></svg>"##;
const ICON_CANDY: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 3"/></svg>"##;
const ICON_WHEAT: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 4c4 0 4 4 8 4s4-4 8-4M4 4v16M20 4v16M4 12c4 0 4 4 8 4s4-4 8-4M4 20h16"/></svg>"##;

/// Picks an icon for one `nutrition:` entry based on keywords in its text
/// (e.g. `"258 kcal"`, `"4.2 g of proteins"`). Returns `None` for anything
/// unrecognized, which renders as a plain bullet.
fn nutrition_icon(text: &str) -> Option<&'static str> {
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

/// Turns a raw entry value like `"258%kcal"` into display text `"258 kcal"`.
/// Entries without the `%` separator (used by non-nutrition families) are
/// left untouched.
fn format_list_entry(raw: &str) -> String {
    match raw.split_once('%') {
        Some((value, unit)) => format!("{} {}", value.trim(), unit.trim()),
        None => raw.trim().to_string(),
    }
}

fn capitalize(label: &str) -> String {
    let label = label.replace(['_', '-'], " ");
    let mut chars = label.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => label,
    }
}

/// Builds the "one line per family" metadata view from every non-standard,
/// list-valued key in a recipe's metadata (`tags` is always excluded — it has
/// its own row). Icons are only attached to the `nutrition` family.
pub fn build_custom_list_families<'a>(
    map_filtered: impl Iterator<Item = (&'a serde_yaml::Value, &'a serde_yaml::Value)>,
) -> Vec<MetaListFamily> {
    let mut families = Vec::new();
    for (key, value) in map_filtered {
        let Some(key_str) = key.as_str() else {
            continue;
        };
        if key_str.eq_ignore_ascii_case("tags") || key_str.eq_ignore_ascii_case("tag") {
            continue;
        }
        let Some(seq) = value.as_sequence() else {
            continue;
        };
        let is_nutrition = key_str.eq_ignore_ascii_case("nutrition");
        let items: Vec<MetaListItem> = seq
            .iter()
            .filter_map(|v| v.as_str())
            .map(|raw| {
                let text = format_list_entry(raw);
                let icon = if is_nutrition {
                    nutrition_icon(&text).map(str::to_string)
                } else {
                    None
                };
                MetaListItem { icon, text }
            })
            .collect();
        if !items.is_empty() {
            families.push(MetaListFamily {
                label: capitalize(key_str),
                items,
            });
        }
    }
    families
}

/// Extracts the calorie entry (`"258%kcal"` -> `"258 kcal"`) from a recipe's
/// `nutrition:` list, for the compact badge shown on the recipe list page.
/// Uses the lightweight [`cooklang_find::Metadata`] (frontmatter only, no
/// full recipe parse) so listing a directory stays cheap.
pub fn extract_nutrition_kcal(metadata: &cooklang_find::Metadata) -> Option<String> {
    let seq = metadata.get("nutrition")?.as_sequence()?;
    seq.iter().find_map(|v| {
        let raw = v.as_str()?;
        if raw.to_lowercase().contains("kcal") {
            Some(format_list_entry(raw))
        } else {
            None
        }
    })
}
