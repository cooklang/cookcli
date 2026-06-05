# Nutrition Info Display — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Display nutrition metadata stored in `.cook` recipes as a collapsible section in the web UI, supporting both flat top-level keys and the nested `nutrition:` mapping produced by the importer.

**Architecture:** Add a `NutritionData` struct to `templates.rs` and a field on `RecipeMetadata`. Extract it in `builders.rs` via a shared `extract_nutrition` helper that handles both storage formats and suppresses the processed keys from the generic `custom` list. Render a native `<details>/<summary>` collapsible block in `recipe.html` using translated labels from all 7 locale files.

**Tech Stack:** Rust, Askama templates, Tailwind CSS v3.4+, Fluent (FTL) i18n

---

## File Map

| Action | File | Responsibility |
|--------|------|----------------|
| Modify | `src/server/templates.rs` | Add `NutritionData` struct + `nutrition` field on `RecipeMetadata` |
| Modify | `src/server/builders.rs` | Add `extract_nutrition()` helper; call it from both `RecipeMetadata` builder sites; exclude nutrition keys from `custom` |
| Modify | `templates/recipe.html` | Collapsible nutrition section after `#metadata-container` |
| Modify | `locales/en-US/recipes.ftl` | 10 new translation keys |
| Modify | `locales/fr-FR/recipes.ftl` | French translations |
| Modify | `locales/de-DE/recipes.ftl` | German translations |
| Modify | `locales/es-ES/recipes.ftl` | Spanish translations |
| Modify | `locales/eu-ES/recipes.ftl` | Basque translations |
| Modify | `locales/nl-NL/recipes.ftl` | Dutch translations |
| Modify | `locales/sv-SE/recipes.ftl` | Swedish translations |

---

## Task 1: Add `NutritionData` struct to `templates.rs`

**Files:**
- Modify: `src/server/templates.rs` (after line 503, inside the existing structs block)

Context: `RecipeMetadata` is defined at line 490. Add `NutritionData` before it, then add a `nutrition` field to `RecipeMetadata`.

- [ ] **Step 1: Insert `NutritionData` struct and update `RecipeMetadata`**

In `src/server/templates.rs`, add the `NutritionData` struct immediately before the `RecipeMetadata` struct definition, then add `nutrition: Option<NutritionData>` as the last field of `RecipeMetadata`:

```rust
// Add this struct before RecipeMetadata:
#[derive(Debug, Clone, Serialize)]
pub struct NutritionData {
    pub calories: Option<String>,
    pub protein: Option<String>,
    pub fat: Option<String>,
    pub saturated_fat: Option<String>,
    pub carbohydrates: Option<String>,
    pub fiber: Option<String>,
    pub sugar: Option<String>,
    pub sodium: Option<String>,
    pub serving_size: Option<String>,
}

// Update RecipeMetadata to add the last field:
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
    pub nutrition: Option<NutritionData>,   // ← new field
}
```

- [ ] **Step 2: Compile-check**

```bash
cargo check -p cookcli 2>&1 | head -30
```

Expected: two struct instantiation errors in `builders.rs` (missing `nutrition` field) — this is correct, we fix them in Task 2.

---

## Task 2: Implement `extract_nutrition` and wire it into both builders

**Files:**
- Modify: `src/server/builders.rs`

Context: `RecipeMetadata` is instantiated at two locations:
1. ~line 687 inside `build_recipe_template_inner` (recipe page)
2. ~line 937 inside `build_menu_template_inner` (menu page)

Both share the same `map_filtered()` loop that builds `custom_metadata`. We add a free function `extract_nutrition` at the bottom of the file, call it from both sites, and exclude nutrition keys from `custom_metadata`.

- [ ] **Step 1: Add `extract_nutrition` helper at the bottom of `builders.rs`**

Append to the end of `src/server/builders.rs`:

```rust
const NUTRITION_KEYS: &[&str] = &[
    "nutrition",
    "calories",
    "protein",
    "fat",
    "saturated fat",
    "saturated_fat",
    "carbohydrates",
    "fiber",
    "sugar",
    "sodium",
    "serving size",
    "serving_size",
];

fn is_nutrition_key(key: &str) -> bool {
    NUTRITION_KEYS.contains(&key)
}

fn yaml_value_to_string(v: &serde_yaml::Value) -> Option<String> {
    if let Some(s) = v.as_str() {
        Some(s.to_string())
    } else if let Some(n) = v.as_i64() {
        Some(n.to_string())
    } else {
        v.as_f64().map(|f| crate::util::format::format_number(f))
    }
}

fn extract_nutrition(metadata: &cooklang::Metadata) -> Option<NutritionData> {
    // Try nested `nutrition:` mapping first (importer format)
    if let Some(nutrition_val) = metadata.map.get("nutrition") {
        if let Some(nutrition_map) = nutrition_val.as_mapping() {
            let get = |key: &str| -> Option<String> {
                nutrition_map.get(key).and_then(yaml_value_to_string)
            };

            let data = NutritionData {
                calories: get("calories"),
                protein: get("protein"),
                fat: get("fat"),
                saturated_fat: get("saturated fat").or_else(|| get("saturated_fat")),
                carbohydrates: get("carbohydrates"),
                fiber: get("fiber"),
                sugar: get("sugar"),
                sodium: get("sodium"),
                serving_size: get("serving size").or_else(|| get("serving_size")),
            };

            let has_data = data.calories.is_some()
                || data.protein.is_some()
                || data.fat.is_some()
                || data.carbohydrates.is_some()
                || data.fiber.is_some()
                || data.sugar.is_some()
                || data.sodium.is_some();

            if has_data {
                return Some(data);
            }
        }
    }

    // Fall back to flat top-level keys (hand-authored format)
    let get = |key: &str| -> Option<String> {
        metadata.map.get(key).and_then(yaml_value_to_string)
    };

    let data = NutritionData {
        calories: get("calories"),
        protein: get("protein"),
        fat: get("fat"),
        saturated_fat: get("saturated fat").or_else(|| get("saturated_fat")),
        carbohydrates: get("carbohydrates"),
        fiber: get("fiber"),
        sugar: get("sugar"),
        sodium: get("sodium"),
        serving_size: get("serving size").or_else(|| get("serving_size")),
    };

    let has_data = data.calories.is_some()
        || data.protein.is_some()
        || data.fat.is_some()
        || data.carbohydrates.is_some()
        || data.fiber.is_some()
        || data.sugar.is_some()
        || data.sodium.is_some();

    if has_data { Some(data) } else { None }
}
```

- [ ] **Step 2: Update the recipe builder (~line 676) to exclude nutrition keys and add the field**

Find this block in `build_recipe_template_inner`:

```rust
        let mut custom_metadata = Vec::new();
        for (key, value) in recipe.metadata.map_filtered() {
            if let (Some(key_str), Some(val_str)) = (key.as_str(), value.as_str()) {
                if key_str.starts_with("source.") || key_str.starts_with("time.") {
                    continue;
                }

                custom_metadata.push((key_str.to_string(), val_str.to_string()));
            }
        }

        Some(RecipeMetadata {
            ...
            custom: custom_metadata,
        })
```

Replace with:

```rust
        let nutrition = extract_nutrition(&recipe.metadata);

        let mut custom_metadata = Vec::new();
        for (key, value) in recipe.metadata.map_filtered() {
            if let (Some(key_str), Some(val_str)) = (key.as_str(), value.as_str()) {
                if key_str.starts_with("source.")
                    || key_str.starts_with("time.")
                    || is_nutrition_key(key_str)
                {
                    continue;
                }

                custom_metadata.push((key_str.to_string(), val_str.to_string()));
            }
        }

        Some(RecipeMetadata {
            servings: get_field("servings"),
            time: get_field("time"),
            difficulty: get_field("difficulty"),
            course: get_field("course"),
            prep_time: get_field("prep time")
                .or_else(|| get_field("prep_time"))
                .or_else(|| get_field("preptime"))
                .or_else(|| get_field("time.prep")),
            cook_time: get_field("cook time")
                .or_else(|| get_field("cook_time"))
                .or_else(|| get_field("cooktime"))
                .or_else(|| get_field("time.cook")),
            cuisine: get_field("cuisine"),
            diet: get_field("diet"),
            author: get_field("author").or_else(|| get_field("source.author")),
            description: get_field("description"),
            source: get_field("source").or_else(|| get_field("source.name")),
            source_url: get_field("source.url"),
            custom: custom_metadata,
            nutrition,
        })
```

- [ ] **Step 3: Update the menu builder (~line 930) to exclude nutrition keys and add the field**

Find the analogous block in `build_menu_template_inner`:

```rust
        let mut custom_metadata = Vec::new();
        for (key, value) in recipe.metadata.map_filtered() {
            if let (Some(key_str), Some(val_str)) = (key.as_str(), value.as_str()) {
                custom_metadata.push((key_str.to_string(), val_str.to_string()));
            }
        }

        Some(RecipeMetadata {
            ...
            custom: custom_metadata,
        })
```

Replace with:

```rust
        let nutrition = extract_nutrition(&recipe.metadata);

        let mut custom_metadata = Vec::new();
        for (key, value) in recipe.metadata.map_filtered() {
            if let (Some(key_str), Some(val_str)) = (key.as_str(), value.as_str()) {
                if is_nutrition_key(key_str) {
                    continue;
                }
                custom_metadata.push((key_str.to_string(), val_str.to_string()));
            }
        }

        Some(RecipeMetadata {
            servings: get_field("servings"),
            time: get_field("time"),
            difficulty: get_field("difficulty"),
            course: get_field("course"),
            prep_time: get_field("prep time")
                .or_else(|| get_field("prep_time"))
                .or_else(|| get_field("preptime")),
            cook_time: get_field("cook time")
                .or_else(|| get_field("cook_time"))
                .or_else(|| get_field("cooktime")),
            cuisine: get_field("cuisine"),
            diet: get_field("diet"),
            author: get_field("author").or_else(|| get_field("source.author")),
            description: get_field("description"),
            source: get_field("source").or_else(|| get_field("source.name")),
            source_url: get_field("source.url"),
            custom: custom_metadata,
            nutrition,
        })
```

- [ ] **Step 4: Compile-check**

```bash
cargo check -p cookcli 2>&1 | head -30
```

Expected: clean compile (no errors, possibly clippy-style warnings about unused imports — these are fine for now).

- [ ] **Step 5: Commit**

```bash
git add src/server/templates.rs src/server/builders.rs
git commit -m "feat: extract nutrition data from recipe metadata"
```

---

## Task 3: Add translation keys to all 7 locale files

**Files:**
- Modify: `locales/en-US/recipes.ftl`
- Modify: `locales/fr-FR/recipes.ftl`
- Modify: `locales/de-DE/recipes.ftl`
- Modify: `locales/es-ES/recipes.ftl`
- Modify: `locales/eu-ES/recipes.ftl`
- Modify: `locales/nl-NL/recipes.ftl`
- Modify: `locales/sv-SE/recipes.ftl`

- [ ] **Step 1: Add keys to `locales/en-US/recipes.ftl`**

Append to the `# Recipe Metadata` section:

```fluent
meta-nutrition = Nutrition
meta-calories = Calories
meta-protein = Protein
meta-fat = Fat
meta-saturated-fat = Saturated Fat
meta-carbohydrates = Carbohydrates
meta-fiber = Fiber
meta-sugar = Sugar
meta-sodium = Sodium
meta-serving-size = Serving Size
```

- [ ] **Step 2: Add keys to `locales/fr-FR/recipes.ftl`**

```fluent
meta-nutrition = Valeurs nutritionnelles
meta-calories = Calories
meta-protein = Protéines
meta-fat = Lipides
meta-saturated-fat = Acides gras saturés
meta-carbohydrates = Glucides
meta-fiber = Fibres
meta-sugar = Sucres
meta-sodium = Sodium
meta-serving-size = Par portion
```

- [ ] **Step 3: Add keys to `locales/de-DE/recipes.ftl`**

```fluent
meta-nutrition = Nährwerte
meta-calories = Kalorien
meta-protein = Eiweiß
meta-fat = Fett
meta-saturated-fat = Gesättigte Fettsäuren
meta-carbohydrates = Kohlenhydrate
meta-fiber = Ballaststoffe
meta-sugar = Zucker
meta-sodium = Natrium
meta-serving-size = Portionsgröße
```

- [ ] **Step 4: Add keys to `locales/es-ES/recipes.ftl`**

```fluent
meta-nutrition = Información nutricional
meta-calories = Calorías
meta-protein = Proteínas
meta-fat = Grasas
meta-saturated-fat = Grasas saturadas
meta-carbohydrates = Carbohidratos
meta-fiber = Fibra
meta-sugar = Azúcares
meta-sodium = Sodio
meta-serving-size = Tamaño de porción
```

- [ ] **Step 5: Add keys to `locales/eu-ES/recipes.ftl`**

```fluent
meta-nutrition = Elikadura balioak
meta-calories = Kaloriak
meta-protein = Proteinak
meta-fat = Koipeak
meta-saturated-fat = Gantz aseak
meta-carbohydrates = Karbohidratoak
meta-fiber = Zuntzak
meta-sugar = Azukreak
meta-sodium = Sodioa
meta-serving-size = Zerbitzatu tamaina
```

- [ ] **Step 6: Add keys to `locales/nl-NL/recipes.ftl`**

```fluent
meta-nutrition = Voedingswaarden
meta-calories = Calorieën
meta-protein = Eiwitten
meta-fat = Vetten
meta-saturated-fat = Verzadigde vetten
meta-carbohydrates = Koolhydraten
meta-fiber = Vezels
meta-sugar = Suikers
meta-sodium = Natrium
meta-serving-size = Portiegrootte
```

- [ ] **Step 7: Add keys to `locales/sv-SE/recipes.ftl`**

```fluent
meta-nutrition = Näringsvärden
meta-calories = Kalorier
meta-protein = Protein
meta-fat = Fett
meta-saturated-fat = Mättat fett
meta-carbohydrates = Kolhydrater
meta-fiber = Fibrer
meta-sugar = Socker
meta-sodium = Natrium
meta-serving-size = Portionsstorlek
```

- [ ] **Step 8: Commit**

```bash
git add locales/
git commit -m "feat: add nutrition i18n keys to all 7 locales"
```

---

## Task 4: Add collapsible nutrition section to `recipe.html`

**Files:**
- Modify: `templates/recipe.html`

Context: The insertion point is inside the `{% when Some with (metadata) %}` block, after the closing `</div>` of `#metadata-container` (around line 195) and before `{% when None %}`.

Tailwind v3.4+ `open:` variant works on elements inside an open `<details>`: `details[open] .open\:rotate-180 { transform: rotate(180deg); }`.

- [ ] **Step 1: Add the collapsible nutrition block**

In `templates/recipe.html`, find the closing of the `#metadata-container` div and the next `{% when None %}` after the metadata block. The section looks like:

```html
                <!-- Custom metadata -->
                {% for (key, value) in metadata.custom %}
                <span class="metadata-pill metadata-custom">{{ key }}: {{ value }}</span>
                {% endfor %}
            </div>

            {% when None %}
            {% endmatch %}
```

Insert the following between `</div>` and `{% when None %}`:

```html
                <!-- Custom metadata -->
                {% for (key, value) in metadata.custom %}
                <span class="metadata-pill metadata-custom">{{ key }}: {{ value }}</span>
                {% endfor %}
            </div>

            {% match metadata.nutrition %}
            {% when Some with (nutrition) %}
            <details class="bg-white rounded-2xl shadow-sm border border-gray-100 overflow-hidden print:hidden">
                <summary class="px-6 py-4 cursor-pointer hover:bg-gray-50 select-none list-none flex items-center justify-between">
                    <span class="text-lg font-bold text-teal-600">🔬 {{ tr.t("meta-nutrition") }}</span>
                    <svg class="w-5 h-5 text-gray-400 transition-transform open:rotate-180" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path>
                    </svg>
                </summary>
                <div class="px-6 pb-5 pt-3 border-t border-gray-100">
                    <div class="grid grid-cols-2 sm:grid-cols-4 gap-3 mb-3">
                        {% match nutrition.calories %}
                        {% when Some with (val) %}
                        <div class="bg-teal-50 rounded-xl p-3 text-center">
                            <div class="text-xl font-bold text-teal-700">{{ val }}</div>
                            <div class="text-xs text-gray-500 mt-1">{{ tr.t("meta-calories") }}</div>
                        </div>
                        {% when None %}
                        {% endmatch %}
                        {% match nutrition.protein %}
                        {% when Some with (val) %}
                        <div class="bg-blue-50 rounded-xl p-3 text-center">
                            <div class="text-xl font-bold text-blue-700">{{ val }}</div>
                            <div class="text-xs text-gray-500 mt-1">{{ tr.t("meta-protein") }}</div>
                        </div>
                        {% when None %}
                        {% endmatch %}
                        {% match nutrition.fat %}
                        {% when Some with (val) %}
                        <div class="bg-yellow-50 rounded-xl p-3 text-center">
                            <div class="text-xl font-bold text-yellow-700">{{ val }}</div>
                            <div class="text-xs text-gray-500 mt-1">{{ tr.t("meta-fat") }}</div>
                        </div>
                        {% when None %}
                        {% endmatch %}
                        {% match nutrition.carbohydrates %}
                        {% when Some with (val) %}
                        <div class="bg-orange-50 rounded-xl p-3 text-center">
                            <div class="text-xl font-bold text-orange-700">{{ val }}</div>
                            <div class="text-xs text-gray-500 mt-1">{{ tr.t("meta-carbohydrates") }}</div>
                        </div>
                        {% when None %}
                        {% endmatch %}
                    </div>
                    <div class="flex flex-wrap gap-2 text-sm">
                        {% match nutrition.fiber %}
                        {% when Some with (val) %}
                        <span class="bg-green-50 text-green-700 rounded-lg px-3 py-1">{{ tr.t("meta-fiber") }}: <strong>{{ val }}</strong></span>
                        {% when None %}
                        {% endmatch %}
                        {% match nutrition.sugar %}
                        {% when Some with (val) %}
                        <span class="bg-pink-50 text-pink-700 rounded-lg px-3 py-1">{{ tr.t("meta-sugar") }}: <strong>{{ val }}</strong></span>
                        {% when None %}
                        {% endmatch %}
                        {% match nutrition.sodium %}
                        {% when Some with (val) %}
                        <span class="bg-gray-100 text-gray-700 rounded-lg px-3 py-1">{{ tr.t("meta-sodium") }}: <strong>{{ val }}</strong></span>
                        {% when None %}
                        {% endmatch %}
                        {% match nutrition.saturated_fat %}
                        {% when Some with (val) %}
                        <span class="bg-amber-50 text-amber-700 rounded-lg px-3 py-1">{{ tr.t("meta-saturated-fat") }}: <strong>{{ val }}</strong></span>
                        {% when None %}
                        {% endmatch %}
                    </div>
                    {% match nutrition.serving_size %}
                    {% when Some with (val) %}
                    <p class="mt-3 text-xs text-gray-400">{{ tr.t("meta-serving-size") }}: {{ val }}</p>
                    {% when None %}
                    {% endmatch %}
                </div>
            </details>
            {% when None %}
            {% endmatch %}

            {% when None %}
            {% endmatch %}
```

- [ ] **Step 2: Build CSS (Tailwind needs to scan the new classes)**

```bash
cd /path/to/cookcli && npm run build-css
```

Expected: `static/css/output.css` updated with teal/blue/yellow/orange/green/pink/amber/gray utility classes used in the template.

- [ ] **Step 3: Full build**

```bash
cargo build -p cookcli 2>&1 | head -30
```

Expected: clean compile.

- [ ] **Step 4: Commit**

```bash
git add templates/recipe.html static/css/output.css
git commit -m "feat: add collapsible nutrition section to recipe page"
```

---

## Task 5: Manual verification

**Files:** none — testing only

The `seed/Risotto.cook` file already contains a full nested `nutrition:` block with all fields (calories, fat, saturated fat, carbohydrates, sugar, protein, fiber, sodium, serving size). Use it to verify everything works end-to-end.

- [ ] **Step 1: Start the dev server**

```bash
cargo run -- server ./seed
```

Open a browser at `http://localhost:9080`.

- [ ] **Step 2: Navigate to the Risotto recipe**

Click on "Classic Risotto alla Milanese" from the recipe list.

Expected:
- A `🔬 Nutrition` collapsible bar appears between the metadata pills row and the ingredients/steps grid.
- The bar is collapsed by default (no nutrition details visible).

- [ ] **Step 3: Expand the nutrition section**

Click the `🔬 Nutrition` summary row.

Expected:
- Section expands to show 4 primary stat boxes: Calories (887 kcal), Protein (37.3 g), Fat (44.6 g), Carbohydrates (85.5 g)
- Secondary pills: Fiber (7.3 g), Sugar (17.9 g), Sodium (5.3 g), Saturated Fat (11.7 g)
- Serving size line: "Serving Size: 494"
- The chevron rotates 180° to point up

- [ ] **Step 4: Verify nutrition keys absent from custom metadata**

Check that no nutrition-related pills (`calories: …`, `fat: …`, etc.) appear in the grey metadata pill row above the nutrition section.

- [ ] **Step 5: Verify a recipe without nutrition shows no section**

Open a recipe that has no nutrition metadata (e.g., `Neapolitan Pizza.cook` or `lamb-chops.cook`).

Expected: no `🔬 Nutrition` section visible at all.

- [ ] **Step 6: Final quality checks**

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

Expected: no formatting changes, no clippy warnings, 0 tests run (project has no automated tests yet).

- [ ] **Step 7: Final commit**

```bash
git add -p  # stage only if anything changed from fmt
git commit -m "chore: fmt and clippy cleanup for nutrition feature"
```

If `cargo fmt` made no changes and clippy is clean, skip this commit.
