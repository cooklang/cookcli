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

//! Rendering support for non-standard recipe metadata whose value is a YAML
//! list or mapping (e.g. `nutrition:` or `file:`/`meta:`). Each such key
//! becomes one "family" line under the tags on the recipe page; see
//! `docs/server.md`.
//!
//! Every family renders generically by default ([`generic::GenericRenderer`]:
//! `field: value`, no icon). A family opts into a specific rendering by
//! adding a case to [`renderer_for`] plus a [`FamilyRenderer`] impl in its
//! own file — [`nutrition::NutritionRenderer`] for `nutrition:`,
//! [`file::FileRender`] for `file:`/`meta:`. There's no filename-based
//! auto-discovery: Rust has no runtime filesystem scanning for this, so
//! `renderer_for` stays the single explicit, greppable registry — adding a
//! renderer still means adding one match arm here.

pub mod file;
pub mod generic;
pub mod nutrition;

use crate::web::templates::{MetaListFamily, MetaListItem};
use unic_langid::LanguageIdentifier;

/// Converts a YAML scalar to its display string, matching the number
/// formatting used for the recipe's other metadata fields.
fn scalar_to_string(value: &serde_yaml::Value) -> Option<String> {
    if let Some(s) = value.as_str() {
        Some(s.to_string())
    } else if let Some(n) = value.as_i64() {
        Some(n.to_string())
    } else {
        value
            .as_f64()
            .map(crate::util::format::number::format_number)
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

/// Turns a raw list-form entry like `"258%kcal"` into display text `"258
/// kcal"`. Entries without the `%` separator are left untouched. Shared by
/// every family's default list rendering.
fn format_list_entry_default(raw: &str) -> String {
    match raw.split_once('%') {
        Some((value, unit)) => format!("{} {}", value.trim(), unit.trim()),
        None => raw.trim().to_string(),
    }
}

/// Formats a mapping-form `field: value` pair the generic way, e.g.
/// `file: {modified-at: 2026-08-24}` -> `"modified-at: 2026-08-24"`.
fn format_mapping_entry_default(field: &str, raw: &str) -> String {
    match raw.split_once('%') {
        Some((amount, unit)) => format!("{field}: {} {}", amount.trim(), unit.trim()),
        None => format!("{field}: {}", raw.trim()),
    }
}

/// How one metadata family (a non-standard, list- or mapping-valued YAML
/// key) renders its entries. Every method has a generic default; a family
/// overrides only what makes it "specific" — see [`nutrition::NutritionRenderer`]
/// and [`file::FileRender`].
trait FamilyRenderer {
    /// Icon for one list-form entry, given its already-formatted display
    /// text. `None` renders as a plain bullet.
    fn list_icon(&self, _text: &str) -> Option<&'static str> {
        None
    }

    /// Formats one list-form entry's raw text into display text.
    fn format_list_entry(&self, raw: &str) -> String {
        format_list_entry_default(raw)
    }

    /// Icon for one mapping-form entry, given its YAML field name. `None`
    /// renders as a plain bullet.
    fn mapping_icon(&self, _field: &str) -> Option<&'static str> {
        None
    }

    /// Formats one mapping-form field/value pair into display text.
    fn format_mapping_entry(&self, field: &str, raw: &str, _lang: &LanguageIdentifier) -> String {
        format_mapping_entry_default(field, raw)
    }

    /// Small translated qualifier shown under the family's label (e.g.
    /// `nutrition`'s "per serving"). `None` shows no second line.
    fn note(&self, _lang: &LanguageIdentifier) -> Option<String> {
        None
    }
}

/// The renderer "registry": maps a family's YAML key to its
/// [`FamilyRenderer`]. Add a case here (plus an impl in its own file) to
/// give a new family its own icons/formatting instead of the generic
/// fallback.
fn renderer_for(family: &str) -> Box<dyn FamilyRenderer> {
    match family.to_lowercase().as_str() {
        "nutrition" => Box::new(nutrition::NutritionRenderer),
        "file" | "meta" => Box::new(file::FileRender),
        _ => Box::new(generic::GenericRenderer),
    }
}

/// Builds the "one line per family" metadata view from every non-standard
/// key in a recipe's metadata whose value is a YAML **list** (`nutrition:
/// [...]`, the original form) or **mapping** (`nutrition: {kcal: ...}`, or
/// any other family like `file:`). `tags` is always excluded — it has its
/// own row. See [`renderer_for`] for how a family picks its rendering.
pub fn build_custom_list_families<'a>(
    map_filtered: impl Iterator<Item = (&'a serde_yaml::Value, &'a serde_yaml::Value)>,
    lang: &LanguageIdentifier,
) -> Vec<MetaListFamily> {
    let mut families = Vec::new();
    for (key, value) in map_filtered {
        let Some(key_str) = key.as_str() else {
            continue;
        };
        if key_str.eq_ignore_ascii_case("tags") || key_str.eq_ignore_ascii_case("tag") {
            continue;
        }
        // A key prefixed with "." is hidden from the recipe page, e.g.
        // `.file:` hides the whole family, `.lipids:` (inside `nutrition:`)
        // hides just that entry.
        if key_str.starts_with('.') {
            continue;
        }
        let renderer = renderer_for(key_str);

        let items: Vec<MetaListItem> = if let Some(seq) = value.as_sequence() {
            seq.iter()
                .filter_map(|v| v.as_str())
                .map(|raw| {
                    let text = renderer.format_list_entry(raw);
                    let icon = renderer.list_icon(&text).map(str::to_string);
                    MetaListItem { icon, text }
                })
                .collect()
        } else if let Some(mapping) = value.as_mapping() {
            mapping
                .iter()
                .filter_map(|(k, v)| Some((k.as_str()?, scalar_to_string(v)?)))
                .filter(|(field, _)| !field.starts_with('.'))
                .map(|(field, raw)| {
                    let text = renderer.format_mapping_entry(field, &raw, lang);
                    let icon = renderer.mapping_icon(field).map(str::to_string);
                    MetaListItem { icon, text }
                })
                .collect()
        } else {
            Vec::new()
        };

        if !items.is_empty() {
            families.push(MetaListFamily {
                label: capitalize(key_str),
                note: renderer.note(lang),
                items,
            });
        }
    }
    families
}

/// Extracts the calorie entry from a recipe's `nutrition:` metadata, for the
/// compact badge shown on the recipe list page — from either the list form
/// (`"258%kcal"` -> `"258 kcal"`) or the mapping form (`kcal: 258`).
/// Uses the lightweight [`cooklang_find::Metadata`] (frontmatter only, no
/// full recipe parse) so listing a directory stays cheap.
pub fn extract_nutrition_kcal(metadata: &cooklang_find::Metadata) -> Option<String> {
    let nutrition_value = metadata.get("nutrition")?;

    if let Some(seq) = nutrition_value.as_sequence() {
        return seq.iter().find_map(|v| {
            let raw = v.as_str()?;
            if raw.to_lowercase().contains("kcal") {
                Some(format_list_entry_default(raw))
            } else {
                None
            }
        });
    }

    if let Some(mapping) = nutrition_value.as_mapping() {
        for (k, v) in mapping {
            if k.as_str().is_some_and(|f| f.eq_ignore_ascii_case("kcal")) {
                if let Some(raw) = scalar_to_string(v) {
                    // "kcal" needs no translation, so any locale works here.
                    return Some(nutrition::NutritionRenderer.format_mapping_entry(
                        "kcal",
                        &raw,
                        &LanguageIdentifier::default(),
                    ));
                }
            }
        }
    }

    None
}
