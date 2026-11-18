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

use super::{
    format_mapping_entry_default, list_family, scalar_to_string, FamilyRenderer, PanelContext,
};
use crate::web::templates::{FamilyPanel, MetaListFamily, MetaListItem};
use fluent_templates::Loader;
use unic_langid::LanguageIdentifier;

/// Tabler Icons (MIT license, https://tabler.io/icons).
const ICON_USER: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="8" r="3.5"/><path d="M5 20c0-3.5 3-6 7-6s7 2.5 7 6"/></svg>"##;
const ICON_CALENDAR: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="5" width="16" height="15" rx="2"/><path d="M4 10h16M8 3v4M16 3v4"/></svg>"##;
const ICON_PENCIL: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15.5 4.5 19 8l-10 10H5v-4Z"/></svg>"##;
const ICON_HISTORY: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 9a8 8 0 1 1 1.5 6.5"/><path d="M4 4v5h5"/><path d="M12 8v4l3 2"/></svg>"##;
const ICON_GLOBE: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3c3 3 3 15 0 18M12 3c-3 3-3 15 0 18"/></svg>"##;

/// The keys of `file:` the panel groups; any other key keeps its own chip.
const KNOWN: [&str; 5] = [
    "created-at",
    "created-by",
    "modified-at",
    "modified-by",
    "source-locale",
];

/// A key as the recipe may spell it: lower case, `-` for `_`.
fn norm(key: &str) -> String {
    key.to_lowercase().replace('_', "-")
}

/// One event of the file's life as one line: `Created on 2026-08-20 · Yannick`.
/// With no date, just who: `Created by: Yannick`. `None` when it has neither.
fn event(
    lang: &LanguageIdentifier,
    on_key: &str,
    by_key: &str,
    date: Option<String>,
    author: Option<String>,
) -> Option<String> {
    let l = |key: &str| crate::web::i18n::LOCALES.lookup(lang, key);
    match (date, author) {
        (Some(date), Some(author)) => Some(format!("{} {date} \u{b7} {author}", l(on_key))),
        (Some(date), None) => Some(format!("{} {date}", l(on_key))),
        (None, Some(author)) => Some(format!("{}{}{author}", l(by_key), colon_separator(lang))),
        (None, None) => None,
    }
}

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

    /// The panel groups the keys by event instead of showing one chip per key:
    /// when it was created and by whom, when it was last modified and by whom,
    /// and the language it was translated from. A key that is absent leaves its
    /// chip out; an unknown key keeps its own chip.
    fn panel(
        &self,
        family: &str,
        value: &serde_yaml::Value,
        ctx: &PanelContext<'_>,
    ) -> Option<FamilyPanel> {
        let Some(mapping) = value.as_mapping() else {
            return list_family(self, family, value, ctx.lang).map(FamilyPanel::File);
        };
        let lang = ctx.lang;
        let get = |key: &str| {
            mapping
                .iter()
                .find(|(k, _)| k.as_str().is_some_and(|k| norm(k) == key))
                .and_then(|(_, v)| scalar_to_string(v))
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };

        let mut items = Vec::new();
        let mut push = |icon: &str, text: Option<String>| {
            if let Some(text) = text {
                items.push(MetaListItem {
                    icon: Some(icon.to_string()),
                    text,
                    url: None,
                });
            }
        };
        push(
            ICON_CALENDAR,
            event(
                lang,
                "meta-created-at",
                "meta-created-by",
                get("created-at"),
                get("created-by"),
            ),
        );
        push(
            ICON_HISTORY,
            event(
                lang,
                "meta-modified-at",
                "meta-modified-by",
                get("modified-at"),
                get("modified-by"),
            ),
        );
        push(
            ICON_GLOBE,
            get("source-locale").map(|code| {
                format!(
                    "{} {code}",
                    crate::web::i18n::LOCALES.lookup(lang, "meta-translated-from")
                )
            }),
        );
        for (key, v) in mapping {
            let Some(field) = key.as_str().filter(|f| !f.starts_with('.')) else {
                continue;
            };
            if KNOWN.contains(&norm(field).as_str()) {
                continue;
            }
            if let Some(raw) = scalar_to_string(v) {
                items.push(MetaListItem {
                    icon: None,
                    text: format_mapping_entry_default(field, &raw),
                    url: None,
                });
            }
        }

        (!items.is_empty()).then(|| {
            FamilyPanel::File(MetaListFamily {
                label: crate::web::i18n::LOCALES.lookup(lang, "meta-file-title"),
                note: None,
                items,
            })
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn panel_of(yaml: &str) -> Option<MetaListFamily> {
        let value: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        let lang = LanguageIdentifier::default();
        let ctx = PanelContext {
            lang: &lang,
            servings: None,
            weight: None,
            standard: crate::util::format::reference::standard("eu").unwrap(),
            essential: false,
        };
        match FileRender.panel("file", &value, &ctx)? {
            FamilyPanel::File(family) => Some(family),
            _ => None,
        }
    }

    #[test]
    fn dates_and_authors_are_grouped_by_event() {
        let family = panel_of(
            "created-at: 2026-08-20\ncreated-by: Yannick\nmodified-at: 2026-08-24\nmodified-by: Yannick",
        )
        .unwrap();
        assert_eq!(family.items.len(), 2);
        assert!(
            family.items[0].text.contains("2026-08-20") && family.items[0].text.contains("Yannick")
        );
        assert!(family.items[1].text.contains("2026-08-24"));
    }

    #[test]
    fn an_absent_key_leaves_its_chip_out() {
        let family = panel_of("modified-at: 2026-08-24\nmodified-by: Yannick").unwrap();
        assert_eq!(family.items.len(), 1);
    }

    #[test]
    fn the_source_locale_and_unknown_keys_each_get_a_chip() {
        let family = panel_of("source-locale: en\nreviewed: yes\n.hidden: x").unwrap();
        assert_eq!(family.items.len(), 2);
        assert!(family.items[0].text.ends_with(" en"));
        assert!(family.items[1].text.contains("reviewed"));
    }

    #[test]
    fn nothing_to_show_means_no_panel() {
        assert!(panel_of("created-at: ''").is_none());
    }
}
