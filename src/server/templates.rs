use askama::Template;
use fluent_templates::Loader;
use serde::{Deserialize, Serialize};
use unic_langid::LanguageIdentifier;

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
        crate::server::i18n::LOCALES.lookup(&self.lang, key)
    }

    pub fn lang_string(&self) -> String {
        self.lang.to_string()
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
}

#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate {
    pub active: String,
    pub error_message: String,
    pub tr: Tr,
    pub prefix: String,
    pub static_mode: bool,
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
    pub tr: Tr,
    pub prefix: String,
    pub static_mode: bool,
}

#[derive(Template)]
#[template(path = "recipe.html")]
pub struct RecipeTemplate {
    pub active: String,
    pub recipe: RecipeData,
    pub recipe_path: String,
    pub breadcrumbs: Vec<String>,
    pub scale: f64,
    pub tags: Vec<String>,
    pub ingredients: Vec<IngredientData>,
    pub cookware: Vec<CookwareData>,
    pub sections: Vec<RecipeSection>,
    pub image_path: Option<String>,
    pub tr: Tr,
    pub prefix: String,
    pub static_mode: bool,
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
            StepItem::Cookware(c) => out.push_str(c),
            StepItem::Timer(t) => out.push_str(t),
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
    pub scale: f64,
    pub metadata: Option<RecipeMetadata>,
    pub sections: Vec<MenuSection>,
    pub image_path: Option<String>,
    pub tr: Tr,
    pub prefix: String,
    pub static_mode: bool,
}

#[derive(Template)]
#[template(path = "shopping_list.html")]
pub struct ShoppingListTemplate {
    pub active: String,
    pub tr: Tr,
    pub prefix: String,
    pub static_mode: bool,
}

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
}

#[derive(Template)]
#[template(path = "pantry.html")]
pub struct PantryTemplate {
    pub active: String,
    pub configured: bool,
    pub sections: Vec<PantrySection>,
    pub tr: Tr,
    pub prefix: String,
    pub static_mode: bool,
}

#[derive(Template)]
#[template(path = "edit.html")]
pub struct EditTemplate {
    pub active: String,
    pub recipe_name: String,
    pub recipe_path: String,
    pub content: String,
    pub base_path: String,
    pub tr: Tr,
    pub prefix: String,
    pub static_mode: bool,
}

#[derive(Template)]
#[template(path = "new.html")]
pub struct NewTemplate {
    pub active: String,
    pub tr: Tr,
    pub error: Option<String>,
    pub filename: Option<String>,
    pub prefix: String,
    pub static_mode: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PantrySection {
    pub name: String,
    pub items: Vec<PantryItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PantryItem {
    pub name: String,
    pub quantity: Option<String>,
    pub bought: Option<String>,
    pub expire: Option<String>,
    pub low: Option<String>,
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
}

#[derive(Debug, Clone, Serialize)]
pub struct IngredientData {
    pub name: String,
    pub quantity: Option<String>,
    pub unit: Option<String>,
    /// Preparation note from Cooklang shorthand notation (e.g., "@tomatoes{2}(diced)" -> "diced")
    pub note: Option<String>,
    pub reference_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CookwareData {
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecipeSection {
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
    },
    Cookware(String),
    Timer(String),
    Quantity(String),
    LineBreak,
}

#[derive(Debug, Clone, Serialize)]
pub struct MenuSection {
    pub name: Option<String>,
    pub lines: Vec<Vec<MenuSectionItem>>,
}

#[derive(Debug, Clone, Serialize)]
pub enum MenuSectionItem {
    Text(String),
    RecipeReference {
        name: String,
        scale: Option<f64>,
    },
    Ingredient {
        name: String,
        quantity: Option<String>,
        unit: Option<String>,
    },
}
