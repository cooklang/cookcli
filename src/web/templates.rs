use askama::Template;
use fluent_templates::Loader;
use serde::{Deserialize, Serialize};
use unic_langid::LanguageIdentifier;

use crate::web::language::FeatureFlags;
use crate::web::viewer::Viewer;

/// Helper struct for translations in templates
#[derive(Clone, Debug, Serialize)]
pub struct Tr {
    #[serde(skip)]
    lang: LanguageIdentifier,
}

impl Tr {
    pub fn new(lang: LanguageIdentifier) -> Self {
        Self { lang }
    }

    pub fn t(&self, key: &str) -> String {
        crate::web::i18n::LOCALES.lookup(&self.lang, key)
    }

    /// Translate a pluralized message. Passes `$count` as a number so Fluent
    /// plural selectors (`[one]` / `[other]`, etc.) resolve correctly.
    pub fn tn(&self, key: &str, count: &usize) -> String {
        let args = std::collections::HashMap::from([(
            std::borrow::Cow::Borrowed("count"),
            fluent::FluentValue::from(count),
        )]);
        crate::web::i18n::LOCALES.lookup_with_args(&self.lang, key, &args)
    }

    /// Translate a message that shows a scale factor, passed as `$factor` in
    /// the short form the menu badges use: `1.5`, `0.333`.
    pub fn tf(&self, key: &str, factor: &f64) -> String {
        let factor = filters::scale_factor(factor).unwrap_or_else(|_| factor.to_string());
        let args = std::collections::HashMap::from([(
            std::borrow::Cow::Borrowed("factor"),
            fluent::FluentValue::from(factor),
        )]);
        crate::web::i18n::LOCALES.lookup_with_args(&self.lang, key, &args)
    }

    /// Translate a message for a page script to finish: each variable named
    /// in `names` (space-separated) comes out as `{name}`, for the script to
    /// replace with a value it only learns later.
    pub fn tp(&self, key: &str, names: &str) -> String {
        let args = names
            .split_whitespace()
            .map(|name| {
                (
                    std::borrow::Cow::Owned(name.to_string()),
                    fluent::FluentValue::from(format!("{{{name}}}")),
                )
            })
            .collect::<std::collections::HashMap<_, _>>();
        crate::web::i18n::LOCALES.lookup_with_args(&self.lang, key, &args)
    }

    /// Translate a message whose variables are all text, named in `args`.
    pub fn ta(&self, key: &str, args: &[(&str, &str)]) -> String {
        let args = args
            .iter()
            .map(|(name, value)| {
                (
                    std::borrow::Cow::Owned(name.to_string()),
                    fluent::FluentValue::from(value.to_string()),
                )
            })
            .collect::<std::collections::HashMap<_, _>>();
        crate::web::i18n::LOCALES.lookup_with_args(&self.lang, key, &args)
    }

    pub fn lang_string(&self) -> String {
        self.lang.to_string()
    }
}

#[cfg(test)]
mod tr_tests {
    use super::*;
    use unic_langid::langid;

    #[test]
    fn recipes_count_pluralizes() {
        let tr = Tr::new(langid!("en-US"));
        assert_eq!(tr.tn("recipes-count", &1), "1 recipe");
        assert_eq!(tr.tn("recipes-count", &3), "3 recipes");
    }

    #[test]
    fn placeholders_are_left_for_the_script() {
        let tr = Tr::new(langid!("fr-FR"));
        assert_eq!(
            tr.tp("picture-target-step", "step text"),
            "Étape {step} : {text}"
        );
    }
}

#[cfg(test)]
mod scale_factor_tests {
    use super::filters::scale_factor;

    #[test]
    fn drops_trailing_zeros() {
        assert_eq!(scale_factor(&2.0).unwrap(), "2");
        assert_eq!(scale_factor(&0.5).unwrap(), "0.5");
        assert_eq!(scale_factor(&1.25).unwrap(), "1.25");
    }

    #[test]
    fn rounds_to_three_places() {
        assert_eq!(scale_factor(&(1.0 / 3.0)).unwrap(), "0.333");
        assert_eq!(scale_factor(&(2.0 / 3.0)).unwrap(), "0.667");
        assert_eq!(scale_factor(&(5.0 / 3.0)).unwrap(), "1.667");
        // Rounding up to a whole number leaves no decimal point behind.
        assert_eq!(scale_factor(&4.9999999).unwrap(), "5");
    }
}

#[cfg(test)]
mod is_web_url_tests {
    use super::filters::is_web_url;

    #[test]
    fn accepts_http_and_https() {
        assert!(is_web_url("https://example.com/recipe").unwrap());
        assert!(is_web_url("http://example.com").unwrap());
        assert!(is_web_url(" https://example.com ").unwrap());
    }

    #[test]
    fn rejects_other_schemes_and_plain_text() {
        assert!(!is_web_url("javascript:alert(1)").unwrap());
        assert!(!is_web_url("mailto:me@example.com").unwrap());
        assert!(!is_web_url("Grandma's cookbook").unwrap());
        assert!(!is_web_url("example.com").unwrap());
        assert!(!is_web_url("see https://example.com").unwrap());
    }
}

#[cfg(test)]
mod reference_link_tests {
    use super::{reference_label, reference_path};

    #[test]
    fn a_recipe_links_to_its_page() {
        assert_eq!(
            reference_path("./Salads/Caprese", false, false),
            "recipe/Salads/Caprese"
        );
        assert_eq!(
            reference_path("./Salads/Caprese", false, true),
            "recipe/Salads/Caprese.html"
        );
    }

    #[test]
    fn a_menu_links_to_the_menu_however_it_was_written() {
        for name in ["./Brunches/Sunday.menu", "./Brunches/Sunday"] {
            assert_eq!(
                reference_path(name, true, false),
                "recipe/Brunches/Sunday.menu"
            );
            // `cook build web` writes menus under `menu/`, not `recipe/`.
            assert_eq!(
                reference_path(name, true, true),
                "menu/Brunches/Sunday.html"
            );
        }
    }

    #[test]
    fn labels_drop_the_dot_slash_and_the_extension() {
        assert_eq!(
            reference_label("./Brunches/Sunday.menu"),
            "Brunches › Sunday"
        );
        assert_eq!(reference_label("./Risotto"), "Risotto");
    }
}

#[cfg(all(test, feature = "server"))]
mod inline_code_tests {
    use super::filters::inline_code;

    #[test]
    fn wraps_backticked_spans() {
        assert_eq!(
            inline_code("the `recipe` key").unwrap(),
            "the <code>recipe</code> key"
        );
    }

    #[test]
    fn escapes_html_outside_and_inside_code() {
        assert_eq!(
            inline_code("a <b> and `<i>`").unwrap(),
            "a &lt;b&gt; and <code>&lt;i&gt;</code>"
        );
    }

    #[test]
    fn passes_through_text_with_no_backticks() {
        assert_eq!(inline_code("plain text").unwrap(), "plain text");
    }

    #[test]
    fn unmatched_backtick_does_not_panic() {
        // Odd backtick count: `split` still alternates segments by position,
        // so the trailing segment after the lone backtick is treated as
        // "inside code" and gets wrapped rather than dropped. Still valid,
        // balanced HTML — just not what the author meant — and, critically,
        // no panic.
        assert_eq!(inline_code("one `two").unwrap(), "one <code>two</code>");
    }
}

mod filters {
    use askama::Result;
    use url::Url;

    pub fn hostname(url: &str) -> Result<String> {
        Ok(Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(String::from))
            .unwrap_or_else(|| url.to_string()))
    }

    /// True when a metadata value is an absolute http(s) URL, so templates
    /// can render it as a link. Other schemes (`javascript:`, `data:`, …)
    /// stay plain text.
    pub fn is_web_url(value: &str) -> Result<bool> {
        let value = value.trim();
        if value.contains(char::is_whitespace) {
            return Ok(false);
        }
        Ok(Url::parse(value)
            .map(|u| matches!(u.scheme(), "http" | "https") && u.host_str().is_some())
            .unwrap_or(false))
    }

    /// A scale factor as short decimal text, for a menu's badges and recipe
    /// links alike: rounded to 3 places, without trailing zeros. `1/3` reads
    /// `0.333`, `2.0` reads `2`.
    pub fn scale_factor(value: &f64) -> Result<String> {
        let text = format!("{value:.3}");
        if !text.contains('.') {
            return Ok(text);
        }
        Ok(text.trim_end_matches('0').trim_end_matches('.').to_string())
    }

    /// Render Markdown-style `inline code` spans as `<code>` elements.
    ///
    /// The API docs describe payload keys and content types in prose, and
    /// backticks carry real meaning there. Askama would otherwise print them
    /// literally.
    ///
    /// Escapes every segment itself, so the result is safe to pass through
    /// `|safe`. Callers MUST use `|safe` or the markup will be double-escaped
    /// and shown as text.
    ///
    /// Only `api_docs.html` uses this filter, and that template is server-only,
    /// so the filter is gated too — `src/web/` still compiles without the
    /// `server` feature for the static-site export, and an ungated filter would
    /// be dead code in that build.
    #[cfg(feature = "server")]
    pub fn inline_code(s: &str) -> Result<String> {
        let mut out = String::with_capacity(s.len());
        for (i, segment) in s.split('`').enumerate() {
            let escaped = askama::filters::escape(askama::Html, segment)?.to_string();
            if i % 2 == 1 {
                out.push_str("<code>");
                out.push_str(&escaped);
                out.push_str("</code>");
            } else {
                out.push_str(&escaped);
            }
        }
        Ok(out)
    }
}

#[cfg(feature = "server")]
#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate {
    pub active: String,
    pub error_message: String,
    pub tr: Tr,
    pub prefix: String,
    pub static_mode: bool,
    pub repo_url: Option<String>,
    pub features: FeatureFlags,
    /// Who the page is for; decides whether editing controls are shown.
    pub viewer: Viewer,
}

pub struct TodaysMenu {
    pub menu_name: String,
    pub menu_path: String,
    pub date_display: String,
}

#[derive(Template)]
#[template(path = "recipes.html")]
pub struct RecipesTemplate {
    pub active: String,
    pub current_name: String,
    pub breadcrumbs: Vec<Breadcrumb>,
    pub items: Vec<RecipeItem>,
    pub todays_menu: Option<TodaysMenu>,
    pub new_recipe_url: String,
    pub new_menu_url: String,
    pub new_plan_url: String,
    /// `/random` link for the current folder; `None` hides the button.
    pub random_recipe_url: Option<String>,
    pub tr: Tr,
    pub prefix: String,
    pub static_mode: bool,
    pub repo_url: Option<String>,
    pub features: FeatureFlags,
    /// Who the page is for; decides whether editing controls are shown.
    pub viewer: Viewer,
}

/// The `/search` page: every hit of the search box's query, best first, as
/// the recipe index's cards.
#[cfg(feature = "server")]
#[derive(Template)]
#[template(path = "search.html")]
pub struct SearchTemplate {
    pub active: String,
    pub query: String,
    pub count: usize,
    pub items: Vec<RecipeItem>,
    pub tr: Tr,
    pub prefix: String,
    pub static_mode: bool,
    pub repo_url: Option<String>,
    pub features: FeatureFlags,
    /// Who the page is for; decides whether editing controls are shown.
    pub viewer: Viewer,
}

/// The servings stepper of a recipe that declares a number of servings.
pub struct ServingsScale {
    /// The recipe's own `servings`.
    pub base: u32,
    /// The servings the page is scaled to.
    pub chosen: f64,
}

impl ServingsScale {
    /// The servings the recipe was written for, when the page is scaled away
    /// from them: times, pan sizes and seasoning do not always scale with the
    /// quantities, so the page says what the author wrote it for.
    pub fn original_servings(&self) -> Option<usize> {
        (self.chosen != f64::from(self.base)).then_some(self.base as usize)
    }
}

#[derive(Template)]
#[template(path = "recipe.html")]
pub struct RecipeTemplate {
    pub active: String,
    pub recipe: RecipeData,
    pub recipe_path: String,
    pub breadcrumbs: Vec<String>,
    /// The factor the quantities were scaled by.
    pub scale: f64,
    /// Set when the recipe declares servings: the stepper then counts
    /// servings instead of showing `scale`.
    pub servings: Option<ServingsScale>,
    pub tags: Vec<String>,
    pub ingredients: Vec<IngredientData>,
    pub cookware: Vec<CookwareData>,
    pub sections: Vec<RecipeSection>,
    pub image_path: Option<String>,
    pub tr: Tr,
    pub prefix: String,
    pub static_mode: bool,
    pub repo_url: Option<String>,
    pub features: FeatureFlags,
    /// Who the page is for; decides whether editing controls are shown.
    pub viewer: Viewer,
}

impl RecipeTemplate {
    /// Build JSON data for the cooking mode feature.
    /// This is called from the template to embed structured recipe data.
    pub fn cooking_mode_json(&self) -> String {
        let sections: Vec<serde_json::Value> = self
            .sections
            .iter()
            .map(|section| {
                let ingredients: Vec<serde_json::Value> = section
                    .cooking_mode_ingredients
                    .iter()
                    .map(|ing| {
                        serde_json::json!({
                            "name": ing.name,
                            "quantity": ing.quantity,
                            "unit": ing.unit,
                            "note": ing.note,
                            "optional": ing.optional,
                        })
                    })
                    .collect();

                let steps: Vec<serde_json::Value> = section
                    .items
                    .iter()
                    .filter_map(|item| match item {
                        RecipeSectionItem::Step(step) => {
                            let step_ingredients: Vec<serde_json::Value> = step
                                .ingredients
                                .iter()
                                .map(|ing| {
                                    serde_json::json!({
                                        "name": ing.name,
                                        "quantity": ing.quantity,
                                        "unit": ing.unit,
                                        "note": ing.note,
                                    })
                                })
                                .collect();

                            Some(serde_json::json!({
                                "type": "step",
                                "number": step.number,
                                "globalNumber": section.step_offset + step.number,
                                "image": step.image_path,
                                "ingredients": step_ingredients,
                            }))
                        }
                        RecipeSectionItem::Note(_) => None,
                    })
                    .collect();

                serde_json::json!({
                    "name": section.name,
                    "stepOffset": section.step_offset,
                    "ingredients": ingredients,
                    "steps": steps,
                })
            })
            .collect();

        let data = serde_json::json!({
            "name": self.recipe.name,
            "scale": self.scale,
            "image": self.image_path,
            "sections": sections,
            "optionalLabel": self.tr.t("recipe-optional"),
        });

        // Escape </script> sequences to prevent premature script tag closing
        serde_json::to_string(&data)
            .map(|s| s.replace("</", "<\\/"))
            .unwrap_or_else(|_| "{}".to_string())
    }

    /// Build schema.org Recipe JSON-LD for SEO.
    /// Embedded as `<script type="application/ld+json">` on the recipe page.
    pub fn recipe_jsonld(&self) -> String {
        let recipe_ingredient: Vec<String> = self
            .ingredients
            .iter()
            .map(|ing| format_ingredient_string(&ing.name, &ing.quantity, &ing.unit, &ing.note))
            .collect();

        let recipe_instructions: Vec<serde_json::Value> = self
            .sections
            .iter()
            .flat_map(|section| {
                section
                    .items
                    .iter()
                    .filter_map(|item| match item {
                        RecipeSectionItem::Step(step) => {
                            let text = step_text(step);
                            if text.trim().is_empty() {
                                None
                            } else {
                                Some(serde_json::json!({
                                    "@type": "HowToStep",
                                    "text": text,
                                }))
                            }
                        }
                        RecipeSectionItem::Note(_) => None,
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        let mut data = serde_json::Map::new();
        data.insert("@context".into(), "https://schema.org".into());
        data.insert("@type".into(), "Recipe".into());
        data.insert("name".into(), self.recipe.name.clone().into());
        if let Some(img) = &self.image_path {
            data.insert("image".into(), img.clone().into());
        }
        data.insert("recipeIngredient".into(), recipe_ingredient.into());
        data.insert("recipeInstructions".into(), recipe_instructions.into());
        if !self.tags.is_empty() {
            data.insert("keywords".into(), self.tags.join(", ").into());
        }

        if let Some(meta) = &self.recipe.metadata {
            if let Some(desc) = &meta.description {
                data.insert("description".into(), desc.clone().into());
            }
            if let Some(author) = &meta.author {
                data.insert(
                    "author".into(),
                    serde_json::json!({ "@type": "Person", "name": author }),
                );
            }
            if let Some(servings) = &meta.servings {
                data.insert("recipeYield".into(), servings.clone().into());
            }
            if let Some(course) = &meta.course {
                data.insert("recipeCategory".into(), course.clone().into());
            }
            if let Some(cuisine) = &meta.cuisine {
                data.insert("recipeCuisine".into(), cuisine.clone().into());
            }
            if let Some(prep) = meta.prep_time.as_deref().and_then(to_iso8601_duration) {
                data.insert("prepTime".into(), prep.into());
            }
            if let Some(cook) = meta.cook_time.as_deref().and_then(to_iso8601_duration) {
                data.insert("cookTime".into(), cook.into());
            }
            if let Some(total) = meta.time.as_deref().and_then(to_iso8601_duration) {
                data.insert("totalTime".into(), total.into());
            }
        }

        // Escape </script> sequences to prevent premature script tag closing
        serde_json::to_string(&serde_json::Value::Object(data))
            .map(|s| s.replace("</", "<\\/"))
            .unwrap_or_else(|_| "{}".to_string())
    }
}

fn format_ingredient_string(
    name: &str,
    quantity: &Option<String>,
    unit: &Option<String>,
    note: &Option<String>,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(q) = quantity {
        if !q.is_empty() {
            parts.push(q.clone());
        }
    }
    if let Some(u) = unit {
        if !u.is_empty() {
            parts.push(u.clone());
        }
    }
    parts.push(name.to_string());
    let mut s = parts.join(" ");
    if let Some(n) = note {
        if !n.is_empty() {
            s.push_str(&format!(" ({n})"));
        }
    }
    s
}

fn step_text(step: &StepData) -> String {
    let mut out = String::new();
    for item in &step.items {
        match item {
            StepItem::Text(t) => out.push_str(t),
            StepItem::Ingredient { name, .. } => out.push_str(name),
            StepItem::Cookware { name, note } => {
                out.push_str(name);
                if let Some(n) = note {
                    out.push_str(" (");
                    out.push_str(n);
                    out.push(')');
                }
            }
            StepItem::Timer { display, .. } => out.push_str(display),
            StepItem::Quantity(q) => out.push_str(q),
            StepItem::LineBreak => out.push(' '),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Best-effort conversion of free-form time strings (e.g. "30 minutes",
/// "1 hour 15 min", "1h30m") to an ISO 8601 duration like "PT1H30M".
/// Returns `None` if the input cannot be parsed confidently — callers should
/// then skip the field rather than emit an invalid schema.org Duration.
fn to_iso8601_duration(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    // If already ISO 8601, pass through.
    if trimmed.starts_with('P') && trimmed.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Some(trimmed.to_string());
    }

    let lower = trimmed.to_lowercase();
    let mut hours: u32 = 0;
    let mut minutes: u32 = 0;
    let mut found = false;

    // Capture all "<number> <unit>" pairs, where unit is hour(s)/h or minute(s)/min/m.
    let mut chars = lower.char_indices().peekable();
    while let Some(&(idx, c)) = chars.peek() {
        if c.is_ascii_digit() {
            let start = idx;
            while let Some(&(_, ch)) = chars.peek() {
                if ch.is_ascii_digit() {
                    chars.next();
                } else {
                    break;
                }
            }
            let end = chars.peek().map(|&(i, _)| i).unwrap_or(lower.len());
            let n: u32 = lower[start..end].parse().ok()?;
            // Skip whitespace
            while let Some(&(_, ch)) = chars.peek() {
                if ch.is_whitespace() {
                    chars.next();
                } else {
                    break;
                }
            }
            // Read unit letters
            let unit_start = chars.peek().map(|&(i, _)| i).unwrap_or(lower.len());
            while let Some(&(_, ch)) = chars.peek() {
                if ch.is_ascii_alphabetic() {
                    chars.next();
                } else {
                    break;
                }
            }
            let unit_end = chars.peek().map(|&(i, _)| i).unwrap_or(lower.len());
            let unit = &lower[unit_start..unit_end];
            match unit {
                "h" | "hr" | "hrs" | "hour" | "hours" => {
                    hours += n;
                    found = true;
                }
                "m" | "min" | "mins" | "minute" | "minutes" => {
                    minutes += n;
                    found = true;
                }
                _ => return None,
            }
        } else {
            chars.next();
        }
    }

    if !found {
        return None;
    }

    let mut out = String::from("PT");
    if hours > 0 {
        out.push_str(&format!("{hours}H"));
    }
    if minutes > 0 {
        out.push_str(&format!("{minutes}M"));
    }
    if out == "PT" {
        return None;
    }
    Some(out)
}

#[derive(Template)]
#[template(path = "menu.html")]
pub struct MenuTemplate {
    pub active: String,
    pub name: String,
    pub recipe_path: String,
    pub breadcrumbs: Vec<String>,
    /// The factor the menu was scaled by.
    pub scale: f64,
    /// Set when the menu declares servings: the stepper then counts servings
    /// instead of showing `scale`.
    pub servings: Option<ServingsScale>,
    pub metadata: Option<RecipeMetadata>,
    pub sections: Vec<MenuSection>,
    /// Set when the menu is a meal plan: the page shows `sections` as a
    /// calendar instead of one card each.
    pub plan: Option<crate::web::plan::PlanView>,
    pub image_path: Option<String>,
    pub tr: Tr,
    pub prefix: String,
    pub static_mode: bool,
    pub repo_url: Option<String>,
    pub features: FeatureFlags,
    /// Who the page is for; decides whether editing controls are shown.
    pub viewer: Viewer,
}

#[cfg(feature = "server")]
#[derive(Template)]
#[template(path = "shopping_list.html")]
pub struct ShoppingListTemplate {
    pub active: String,
    pub tr: Tr,
    pub prefix: String,
    pub static_mode: bool,
    pub repo_url: Option<String>,
    pub features: FeatureFlags,
    /// Who the page is for; decides whether editing controls are shown.
    pub viewer: Viewer,
}

#[cfg(feature = "server")]
#[derive(Template)]
#[template(path = "preferences.html")]
pub struct PreferencesTemplate {
    pub active: String,
    pub aisle_path: String,
    pub pantry_path: String,
    pub base_path: String,
    pub version: String,
    pub tr: Tr,
    pub sync_enabled: bool,
    pub sync_logged_in: bool,
    pub sync_email: Option<String>,
    pub sync_syncing: bool,
    pub prefix: String,
    pub static_mode: bool,
    pub repo_url: Option<String>,
    pub features: FeatureFlags,
    /// Who the page is for; decides whether editing controls are shown.
    pub viewer: Viewer,
    /// The nutrition standards to choose from: one per file of the standards folder.
    pub standards: Vec<StandardChoice>,
}

/// A nutrition standard in the picker of the preferences page.
#[cfg(feature = "server")]
#[derive(Debug, Clone)]
pub struct StandardChoice {
    pub id: String,
    pub name: String,
    pub description: String,
    pub selected: bool,
}

#[cfg(feature = "server")]
#[derive(Template)]
#[template(path = "aisles.html")]
pub struct AislesTemplate {
    pub active: String,
    /// What `GET /api/aisles` returns; the page draws itself from it and
    /// from each change's answer.
    pub aisles: serde_json::Value,
    pub tr: Tr,
    pub prefix: String,
    pub static_mode: bool,
    pub repo_url: Option<String>,
    pub features: FeatureFlags,
    /// Who the page is for; decides whether editing controls are shown.
    pub viewer: Viewer,
}

#[cfg(feature = "server")]
#[derive(Template)]
#[template(path = "pantry.html")]
pub struct PantryTemplate {
    pub active: String,
    pub configured: bool,
    pub sections: Vec<PantrySection>,
    /// How many items each filter above the list would show.
    pub counts: PantryCounts,
    /// Why the pantry file could not be read, when it could not; the page
    /// shows it rather than an empty pantry someone might start filling.
    pub load_error: Option<String>,
    pub tr: Tr,
    pub prefix: String,
    pub static_mode: bool,
    pub repo_url: Option<String>,
    pub features: FeatureFlags,
    /// Who the page is for; decides whether editing controls are shown.
    pub viewer: Viewer,
}

#[cfg(feature = "server")]
#[derive(Template)]
#[template(path = "edit.html")]
pub struct EditTemplate {
    pub active: String,
    pub recipe_name: String,
    /// A `.menu` file: the toolbar gets the Day, Meal and Add recipe helpers.
    pub is_menu: bool,
    pub recipe_path: String,
    pub content: String,
    /// Largest title picture upload the server takes, checked in the browser
    /// before sending so a too-big file fails at once rather than after it.
    pub max_image_bytes: usize,
    /// Longest edge the server stores a title picture at; the browser scales
    /// a photo down to it before sending.
    pub max_image_edge: u32,
    pub tr: Tr,
    pub prefix: String,
    pub static_mode: bool,
    pub repo_url: Option<String>,
    pub features: FeatureFlags,
    /// Who the page is for; decides whether editing controls are shown.
    pub viewer: Viewer,
}

/// The frame fields of the new-plan form, as the form shows them.
#[cfg(feature = "server")]
pub struct NewPlanForm {
    /// `YYYY-MM-DD`.
    pub start: String,
    pub days: String,
    pub min_days: u32,
    pub max_days: u32,
    pub servings: String,
    pub meals: Vec<NewPlanMeal>,
}

#[cfg(feature = "server")]
pub struct NewPlanMeal {
    /// The form field, e.g. `breakfast`.
    pub field: &'static str,
    /// The meal's name in the page language, as the plan will write it.
    pub label: String,
    pub checked: bool,
}

#[cfg(feature = "server")]
#[derive(Template)]
#[template(path = "new.html")]
pub struct NewTemplate {
    pub active: String,
    pub tr: Tr,
    pub error: Option<String>,
    pub filename: Option<String>,
    /// The form creates a `.menu` file rather than a `.cook` recipe.
    pub is_menu: bool,
    /// The `.menu` is a meal plan: the form asks for its frame too.
    pub is_plan: bool,
    pub plan: NewPlanForm,
    pub prefix: String,
    pub static_mode: bool,
    pub repo_url: Option<String>,
    pub features: FeatureFlags,
    /// Who the page is for; decides whether editing controls are shown.
    pub viewer: Viewer,
}

#[cfg(feature = "server")]
#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub active: String,
    /// Where to go after signing in, relative to the URL prefix.
    pub next: String,
    /// Echoed back after a failed attempt so it need not be retyped.
    pub username: String,
    pub failed: bool,
    pub tr: Tr,
    pub prefix: String,
    pub static_mode: bool,
    pub repo_url: Option<String>,
    pub features: FeatureFlags,
    pub viewer: Viewer,
}

#[cfg(feature = "server")]
#[derive(Debug, Clone, Serialize)]
pub struct PantrySection {
    pub name: String,
    pub items: Vec<PantryItem>,
}

#[cfg(feature = "server")]
#[derive(Debug, Clone, Serialize)]
pub struct PantryItem {
    pub name: String,
    pub quantity: Option<String>,
    pub bought: Option<String>,
    pub expire: Option<String>,
    pub low: Option<String>,
    /// `quantity` as the page shows it, `250 g` for `250%g`; empty without one.
    pub quantity_text: String,
    /// `low` shown the same way.
    pub low_text: String,
    /// `"out"`, `"low"` or `"ok"`, judged as `cook pantry depleted` judges it.
    pub stock: &'static str,
    /// `"expired"`, `"today"`, `"soon"` (within the week `cook pantry
    /// expiring` looks ahead), `"later"`, or `""` with no readable date.
    pub expiry: &'static str,
    /// Days until it expires, or since it did; 0 without a date.
    pub expiry_days: usize,
}

/// The number of items behind each filter on the pantry page.
#[cfg(feature = "server")]
#[derive(Debug, Clone, Default, Serialize)]
pub struct PantryCounts {
    pub all: usize,
    pub low: usize,
    pub out: usize,
    pub expiring: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breadcrumb {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeItem {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub count: Option<usize>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    /// Calorie entry from a `nutrition:` metadata list (e.g. `"258 kcal"`),
    /// shown as a compact badge next to the tags.
    pub nutrition_kcal: Option<String>,
    /// The donut and macros of a recipe that gives all three, shown instead
    /// of the kcal badge.
    pub nutrition_ring: Option<ListNutrition>,
    pub image_path: Option<String>,
    pub is_menu: bool,
    pub modified_at: Option<u64>,
    pub created_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecipeData {
    pub name: String,
    pub metadata: Option<RecipeMetadata>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecipeMetadata {
    pub servings: Option<String>,
    pub time: Option<String>,
    pub difficulty: Option<String>,
    pub course: Option<String>,
    pub prep_time: Option<String>,
    pub cook_time: Option<String>,
    pub cuisine: Option<String>,
    pub diet: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub source: Option<String>,
    pub source_url: Option<String>,
    pub custom: Vec<(String, String)>,
    /// One panel per non-standard metadata family (`nutrition:`, `file:`, ...),
    /// each built by that family's renderer and drawn by its own template
    /// under `templates/panels/`.
    pub panels: Vec<FamilyPanel>,
}

/// The panel of one metadata family. Each variant is drawn by its own
/// template (`templates/panels/<variant>.html`); `panels/families.html`
/// picks the template.
#[derive(Debug, Clone, Serialize)]
pub enum FamilyPanel {
    /// `nutrition:` as a mapping of numbers: a table of values.
    /// Boxed: the table is far larger than the other variants.
    Nutrition(Box<NutritionPanel>),
    /// `file:` / `meta:`: dated and attributed entries with icons.
    File(MetaListFamily),
    /// Any other list or mapping: one line of chips.
    Generic(MetaListFamily),
}

/// The nutrition panel of a recipe page, with every label already translated:
/// a donut and the macros and a few small cards for the servings chosen, and
/// the full table behind a button.
#[derive(Debug, Clone, Serialize)]
pub struct NutritionPanel {
    pub title: String,
    pub per_serving_label: String,
    pub per_100g_label: String,
    pub whole_recipe_label: String,
    /// "of which", in front of the label of a sub-nutrient row.
    pub of_which: String,
    /// "for", as in "for 4 servings".
    pub for_label: String,
    /// The label of the button that opens the table and the tiles behind it.
    pub full_table_label: String,
    /// The same button once it is open.
    pub less_label: String,
    /// The buttons above the table: fold every row, unfold every row.
    pub collapse_all_label: String,
    pub expand_all_label: String,
    /// What the fold button of a row says to a screen reader.
    pub fold_label: String,
    /// What the list above the table offers: one entry per standard file, then the essential table.
    pub picks: Vec<NutritionPick>,
    /// Whether the table is the essential one: the basic rows, flat.
    pub essential: bool,
    /// What the list says to a screen reader.
    pub pick_label: String,
    /// The vitamins and minerals the standard's rule calls well present.
    pub well_present: Option<WellPresent>,
    /// What the explanation bubbles say, as JSON for the page's script: the labels of their
    /// sections, then for each nutrient what it is, what it is for, where it is found, the source of
    /// that text, the reference intake and the rating of the standard in use, and the recipe's figure.
    pub tips_json: String,
    /// The servings the amounts are for.
    pub servings: String,
    /// Whether the recipe gave a `weight:`, which the per-100 g column needs.
    pub has_100g: bool,
    /// True when the recipe has no whole number of servings: everything is
    /// per serving, with no per-100 g and no whole-recipe column.
    pub per_serving_only: bool,
    /// The energy for the servings chosen, in kcal.
    pub kcal_total: Option<String>,
    /// The energy of one serving, in kcal.
    pub kcal_per_serving: Option<String>,
    /// The donut, when carbohydrates, fat and proteins are all given.
    pub ring: Option<Vec<RingArc>>,
    /// Fat, carbohydrates and proteins for the servings chosen.
    pub macros: Vec<MacroLine>,
    /// Saturated fat, sugars, fibre and salt as small cards.
    pub cards: Vec<NutrientCard>,
    /// "Health rating, per 100 g:", in front of the legend.
    pub legend_label: String,
    /// What each colour of the cards means; empty without a `weight:`, when
    /// every card is neutral.
    pub legend: Vec<LegendItem>,
    /// The rows of the full table, laid out by the tree of the standard.
    pub rows: Vec<NutritionRow>,
}

/// The tiles of the vitamins and minerals that are well present, with the rule that picked them.
#[derive(Debug, Clone, Serialize)]
pub struct WellPresent {
    pub title: String,
    /// "at least 15 % of the reference intake per 100 g ...", for the standard in use.
    pub rule: String,
    pub tiles: Vec<NutrientCard>,
}

/// One arc of the donut. `kind` is `carb`, `fat` or `prot`, and picks the
/// colour in `components.css`; the dash values are on a circle of
/// circumference 100.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RingArc {
    pub kind: String,
    pub dasharray: String,
    pub dashoffset: String,
}

/// One macro beside the donut: its amount and its share of the energy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroLine {
    pub kind: String,
    pub label: String,
    pub amount: String,
    pub pct: Option<u32>,
}

/// A small card for one nutrient, coloured by `level` (`low`, `mid`, `high`,
/// `good` or `neutral`); `word` says the level in words.
#[derive(Debug, Clone, Serialize)]
pub struct NutrientCard {
    pub icon: Option<String>,
    pub label: String,
    pub value: String,
    pub level: String,
    pub word: Option<String>,
    /// Which macro it belongs to (`fat`, `carb`, `prot`), `other` for none, so
    /// a click on a slice of the donut can highlight it.
    pub group: String,
    /// "rich in" or "source of", on the tiles of the nutrients that are well present.
    pub claim: Option<String>,
    /// The canonical name of the nutrient, which the row of the table shares.
    pub key: String,
}

/// One colour of the cards and its meaning, translated.
#[derive(Debug, Clone, Serialize)]
pub struct LegendItem {
    /// The colour: `low` (green), `mid` (amber), `high` (red) or `neutral` (blue).
    pub level: String,
    pub word: String,
    /// Whether a card of this page has that rating. A rating the standard can give but no card
    /// has stays in the legend, greyed, so the legend is always there to explain the colours.
    pub used: bool,
}

/// What a recipe card shows of the nutrition: the donut and the macros, per
/// serving.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListNutrition {
    pub kcal: String,
    pub arcs: Vec<RingArc>,
    pub macros: Vec<MacroLine>,
    /// "per serving", translated.
    pub per_serving: String,
}

/// One row of a [`NutritionPanel`]'s table, values already formatted with unit: a nutrient the
/// recipe gave, or a family of the standard's tree that holds some.
#[derive(Debug, Clone, Serialize)]
pub struct NutritionRow {
    pub icon: Option<String>,
    pub label: String,
    /// A row that is read in a lighter style: a nutrient under another one.
    pub sub: bool,
    /// Read as "of which ...": a part of the nutrient above it (saturated fat, sugars),
    /// not merely listed with it (cholesterol with the fats, fibre where the norm keeps it apart).
    pub part: bool,
    /// A family (macronutrients, minerals, vitamins...), not a nutrient.
    pub fam: bool,
    /// The top-level family it sits in: `macro`, `other`, `min`, `vit`, or empty for the energy.
    pub family: String,
    /// The slice of the donut it belongs to (`fat`, `carb`, `prot`), or empty.
    pub mac: String,
    /// How deep it sits in the tree: 0 for the energy and the families, 1 for a macro.
    /// Drives the indentation and the folding.
    pub depth: u8,
    /// `fat`, `carb` or `prot` for a slice and what belongs to it, `all` for
    /// the energy, `other` for the rest.
    pub group: String,
    /// The canonical name of the nutrient or family (`saturatedFatContent`, `minerals`).
    pub key: String,
    pub per_serving: String,
    pub per_100g: Option<String>,
    pub total: String,
    /// The share of the standard's reference intake that one serving covers, in percent.
    pub pct: Option<u32>,
    /// Whether rows sit below it, so a fold button goes on it.
    pub foldable: bool,
    /// Its place in the simple view (the table as it was before the standards): `0` not shown,
    /// `1` a line of its own, `2` an "of which" line under the nutrient above.
    pub simple: u8,
}

/// A standard in the list above the nutrition table.
#[derive(Debug, Clone, Serialize)]
pub struct NutritionPick {
    pub id: String,
    pub name: String,
    pub selected: bool,
}

/// One family line of list-valued custom metadata (e.g. `nutrition`).
#[derive(Debug, Clone, Serialize)]
pub struct MetaListFamily {
    pub label: String,
    /// Small translated qualifier shown under the label (e.g. `nutrition`'s
    /// "per serving"), or `None` for families without one.
    pub note: Option<String>,
    pub items: Vec<MetaListItem>,
}

/// One entry within a [`MetaListFamily`] (e.g. `"258 kcal"`).
#[derive(Debug, Clone, Serialize)]
pub struct MetaListItem {
    /// The address, when the value is a web link: `text` then shows only its
    /// host, so a long URL cannot push the page wider than the screen.
    pub url: Option<String>,
    /// Inline SVG markup (Tabler Icons, MIT), or `None` for a plain bullet.
    pub icon: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IngredientData {
    pub name: String,
    pub quantity: Option<String>,
    pub unit: Option<String>,
    /// Preparation note from Cooklang shorthand notation (e.g., "@tomatoes{2}(diced)" -> "diced")
    pub note: Option<String>,
    pub reference_path: Option<String>,
    /// What the link to `reference_path` adds so the recipe opens at the size
    /// this one asks for — see [`reference_query`]. Empty otherwise.
    pub reference_query: String,
    /// Marked optional in the recipe (`@?chives`). Required and optional
    /// amounts of one ingredient are listed as two entries.
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CookwareData {
    pub name: String,
    /// Preparation or descriptive note from Cooklang shorthand notation,
    /// e.g. `#pot{}(big)` -> "big".
    pub note: Option<String>,
    /// Marked optional in the recipe (`#?splatter guard{}`).
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecipeSection {
    /// One-based position among the parsed recipe's sections, counting the
    /// empty ones this page leaves out: the `S` of a `Recipe.S.N.jpg` step
    /// picture.
    pub number: usize,
    pub name: Option<String>,
    pub items: Vec<RecipeSectionItem>,
    pub step_offset: usize,
    pub ingredients: Vec<IngredientData>,
    /// Uncombined ingredients for cooking mode mise en place, sorted by aisle order.
    /// Each ingredient occurrence appears separately (not merged by name).
    pub cooking_mode_ingredients: Vec<IngredientData>,
}

#[derive(Debug, Clone, Serialize)]
pub enum RecipeSectionItem {
    Step(StepData),
    Note(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct StepData {
    pub number: usize,
    pub items: Vec<StepItem>,
    pub ingredients: Vec<StepIngredient>,
    pub image_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StepIngredient {
    pub name: String,
    pub quantity: Option<String>,
    pub unit: Option<String>,
    /// Preparation note from Cooklang shorthand notation (e.g., "@tomatoes{2}(diced)" -> "diced")
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum StepItem {
    Text(String),
    Ingredient {
        name: String,
        reference_path: Option<String>,
        /// As on [`IngredientData::reference_query`].
        reference_query: String,
    },
    Cookware {
        name: String,
        note: Option<String>,
    },
    Timer {
        display: String,
        seconds: Option<i64>,
    },
    Quantity(String),
    LineBreak,
}

/// The text of a menu line that is only a meal heading (`Breakfast:`).
pub fn meal_header(line: &[MenuSectionItem]) -> Option<&str> {
    match line {
        [MenuSectionItem::Text(text)] if text.trim().ends_with(':') => Some(text),
        _ => None,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MenuSection {
    pub name: Option<String>,
    pub lines: Vec<Vec<MenuSectionItem>>,
}

/// Where a menu's reference to `name` leads, below the site prefix: the
/// recipe's page, or for a `menu` the other menu's page — `menu/<path>.html`
/// on a static site, and `recipe/<path>.menu` on the server, whose extension
/// keeps a recipe of the same name from answering instead.
pub fn reference_path(name: &str, menu: bool, static_mode: bool) -> String {
    let path = name.strip_prefix("./").unwrap_or(name);
    match (menu, static_mode) {
        (true, true) => format!("menu/{}.html", path.trim_end_matches(".menu")),
        (true, false) => format!("recipe/{}.menu", path.trim_end_matches(".menu")),
        (false, true) => format!("recipe/{path}.html"),
        (false, false) => format!("recipe/{path}"),
    }
}

/// The query that opens a referenced recipe `factor` times its own size, as
/// a menu's links do: by servings when it declares a whole number of them,
/// since its page counts servings, otherwise by the multiplier. Empty at ×1,
/// or when the factor is unknown.
pub fn reference_query(factor: Option<f64>, servings: Option<u32>) -> String {
    let Some(factor) = factor.filter(|f| f.is_finite() && *f > 0.0) else {
        return String::new();
    };
    // Compared as written, so that a factor a hair off 1 links plainly too.
    let text = |value: f64| filters::scale_factor(&value).unwrap_or_default();
    if text(factor) == "1" {
        return String::new();
    }
    match servings.filter(|&n| n > 0) {
        Some(n) => format!("?servings={}", text(f64::from(n) * factor)),
        None => format!("?scale={}", text(factor)),
    }
}

/// A menu's reference as the page names it: `Brunches › Sunday`, without the
/// `./` or a `.menu` it was written with.
pub fn reference_label(name: &str) -> String {
    let path = name.strip_prefix("./").unwrap_or(name);
    path.trim_end_matches(".menu").replace('/', " › ")
}

#[derive(Debug, Clone, Serialize)]
pub enum MenuSectionItem {
    Text(String),
    RecipeReference {
        name: String,
        scale: Option<f64>,
        /// `scale` as servings of the referenced recipe, when it declares
        /// them: its link then carries `?servings=` instead of `?scale=`.
        servings: Option<f64>,
        /// The reference is to another menu (`@./Brunches/Sunday.menu{}`):
        /// it links to that menu's page and is labelled as a menu.
        menu: bool,
    },
    Ingredient {
        name: String,
        quantity: Option<String>,
        unit: Option<String>,
    },
}

// -- API documentation page --

/// One parameter of a documented endpoint.
#[cfg(feature = "server")]
pub struct ParamDoc {
    pub name: String,
    /// Where the parameter goes: "path", "query", or "body".
    pub kind: String,
    pub required: bool,
    /// Human-facing type, e.g. "string", "number", "string[]".
    pub type_name: String,
    pub description: String,
}

/// One documented API endpoint.
///
/// Constructed via private builder helpers in `web::api_docs` (not part of
/// this type's public surface) — `method_classes` below is the only method
/// kept here.
#[cfg(feature = "server")]
pub struct EndpointDoc {
    pub method: String,
    /// Path in axum route syntax, e.g. "/api/pantry/{section}/{name}".
    pub path: String,
    pub summary: String,
    /// Longer prose. Empty string means "no extra detail".
    pub description: String,
    pub params: Vec<ParamDoc>,
    /// Pretty-printed JSON request body, if the endpoint takes one.
    pub request_example: Option<String>,
    /// Pretty-printed JSON response body, if the endpoint returns one.
    pub response_example: Option<String>,
    /// Cargo feature required for this endpoint to exist, e.g. "sync".
    pub feature: Option<String>,
}

#[cfg(feature = "server")]
impl EndpointDoc {
    /// Tailwind classes for the method badge. Kept here rather than in the
    /// template so the template needs no conditional chain per endpoint.
    pub fn method_classes(&self) -> &'static str {
        match self.method.as_str() {
            "GET" => "bg-sunk text-text",
            "POST" => "bg-ok-soft text-ok",
            "PUT" => "bg-accent-soft text-accent-text",
            "DELETE" => "bg-danger-soft text-danger",
            _ => "bg-sunk text-muted",
        }
    }
}

/// A group of related endpoints, rendered as one section with an anchor.
#[cfg(feature = "server")]
pub struct ApiSection {
    /// Anchor target, e.g. "shopping-list".
    pub id: String,
    pub title: String,
    pub description: String,
    pub endpoints: Vec<EndpointDoc>,
}

/// A labelled line of prose: either a "Before you start" fact or a status
/// code and what it means.
#[cfg(feature = "server")]
pub struct ApiNote {
    pub label: String,
    pub text: String,
}

/// The fixed prose opening the API reference — connection facts and the error
/// contract.
///
/// Data rather than markup in `api_docs.html` so `web::api_docs_md` renders
/// the same words into `docs/api.md` instead of keeping a second copy that
/// drifts. The base URL is deliberately absent: the page derives it from the
/// request host, and the markdown export has no host to derive it from.
#[cfg(feature = "server")]
pub struct ApiPreamble {
    pub intro: String,
    pub notes: Vec<ApiNote>,
    pub error_intro: String,
    /// The error envelope, as a literal JSON body.
    pub error_example: String,
    /// Status codes, labelled by code.
    pub error_codes: Vec<ApiNote>,
}

#[cfg(feature = "server")]
#[derive(Template)]
#[template(path = "api_docs.html")]
pub struct ApiDocsTemplate {
    pub active: String,
    /// Fully-qualified API root, e.g. "http://localhost:9080/api".
    pub base_url: String,
    pub preamble: ApiPreamble,
    pub sections: Vec<ApiSection>,
    pub tr: Tr,
    pub prefix: String,
    pub static_mode: bool,
    pub repo_url: Option<String>,
    pub features: FeatureFlags,
    /// Who the page is for; decides whether editing controls are shown.
    pub viewer: Viewer,
}

/// Askama's axum integration lived in the `askama_axum` crate, which was
/// deprecated and never updated for axum 0.8. It provided a blanket
/// `IntoResponse` for every `Template`; a blanket impl is not available to us
/// (both traits are foreign), so each response template opts in explicitly.
#[cfg(feature = "server")]
macro_rules! impl_into_response {
    ($($t:ty),+ $(,)?) => {
        $(impl axum::response::IntoResponse for $t {
            fn into_response(self) -> axum::response::Response {
                match askama::Template::render(&self) {
                    Ok(body) => axum::response::Html(body).into_response(),
                    Err(err) => {
                        tracing::error!("failed to render template: {err}");
                        (
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                            "Template rendering failed",
                        )
                            .into_response()
                    }
                }
            }
        })+
    };
}

#[cfg(feature = "server")]
impl_into_response!(
    ErrorTemplate,
    RecipesTemplate,
    SearchTemplate,
    RecipeTemplate,
    MenuTemplate,
    ShoppingListTemplate,
    PreferencesTemplate,
    PantryTemplate,
    AislesTemplate,
    EditTemplate,
    NewTemplate,
    ApiDocsTemplate,
    LoginTemplate,
);

/// The two large templates are handed around boxed, and `Box<T>` is not
/// itself a `Template`.
#[cfg(feature = "server")]
macro_rules! impl_into_response_boxed {
    ($($t:ty),+ $(,)?) => {
        $(impl axum::response::IntoResponse for Box<$t> {
            fn into_response(self) -> axum::response::Response {
                (*self).into_response()
            }
        })+
    };
}

#[cfg(feature = "server")]
impl_into_response_boxed!(RecipeTemplate, MenuTemplate);
