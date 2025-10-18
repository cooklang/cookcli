use askama::Template;
use serde::{Deserialize, Serialize};

#[derive(Template)]
#[template(path = "recipes.html")]
pub struct RecipesTemplate {
    pub active: String,
    pub current_name: String,
    pub breadcrumbs: Vec<Breadcrumb>,
    pub items: Vec<RecipeItem>,
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
}

#[derive(Template)]
#[template(path = "shopping_list.html")]
pub struct ShoppingListTemplate {
    pub active: String,
}

#[derive(Template)]
#[template(path = "preferences.html")]
pub struct PreferencesTemplate {
    pub active: String,
    pub aisle_path: String,
    pub pantry_path: String,
    pub base_path: String,
    pub version: String,
}

#[derive(Template)]
#[template(path = "pantry.html")]
pub struct PantryTemplate {
    pub active: String,
    pub sections: Vec<PantrySection>,
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
    pub steps: Vec<StepData>,
    pub notes: Vec<String>,
    pub step_offset: usize,
    pub ingredients: Vec<IngredientData>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StepData {
    pub items: Vec<StepItem>,
    pub ingredients: Vec<StepIngredient>,
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
