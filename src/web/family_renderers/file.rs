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

//! Specific renderer for the `file:`/`meta:` family (`created-by`,
//! `created-at`, `modified-by`, `modified-at`).

use super::{format_mapping_entry_default, FamilyRenderer};
use fluent_templates::Loader;
use unic_langid::LanguageIdentifier;

/// Tabler Icons (MIT license, https://tabler.io/icons).
const ICON_USER: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="8" r="3.5"/><path d="M5 20c0-3.5 3-6 7-6s7 2.5 7 6"/></svg>"##;
const ICON_CALENDAR: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="5" width="16" height="15" rx="2"/><path d="M4 10h16M8 3v4M16 3v4"/></svg>"##;
const ICON_PENCIL: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15.5 4.5 19 8l-10 10H5v-4Z"/></svg>"##;
const ICON_HISTORY: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 9a8 8 0 1 1 1.5 6.5"/><path d="M4 4v5h5"/><path d="M12 8v4l3 2"/></svg>"##;

/// Specific renderer for the `file`/`meta` family.
pub(super) struct FileRender;

impl FamilyRenderer for FileRender {
    fn mapping_icon(&self, field: &str) -> Option<&'static str> {
        match field.to_lowercase().replace('_', "-").as_str() {
            "created-by" => Some(ICON_USER),
            "created-at" => Some(ICON_CALENDAR),
            "modified-by" => Some(ICON_PENCIL),
            "modified-at" => Some(ICON_HISTORY),
            _ => None,
        }
    }

    /// `("created-by", "Yannick")` -> `"Created by: Yannick"` (en) /
    /// `"Créé par : Yannick"` (fr, with the French space before `:`) /
    /// `"Created by: Yannick"` (en), via Fluent. Any other field falls back
    /// to the raw field name, same as [`super::generic::GenericRenderer`].
    fn format_mapping_entry(&self, field: &str, raw: &str, lang: &LanguageIdentifier) -> String {
        let key = match field.to_lowercase().replace('_', "-").as_str() {
            "created-by" => "meta-created-by",
            "created-at" => "meta-created-at",
            "modified-by" => "meta-modified-by",
            "modified-at" => "meta-modified-at",
            _ => return format_mapping_entry_default(field, raw),
        };
        let label = crate::web::i18n::LOCALES.lookup(lang, key);
        format!("{label}{}{}", colon_separator(lang), raw.trim())
    }
}

/// French typography puts a (narrow no-break) space before `:`; other
/// supported locales just follow it with a space.
fn colon_separator(lang: &LanguageIdentifier) -> &'static str {
    if lang.language == "fr" {
        "\u{202f}: "
    } else {
        ": "
    }
}
