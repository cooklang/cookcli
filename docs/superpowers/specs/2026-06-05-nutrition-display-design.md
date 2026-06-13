# Nutrition Info Display — Design Spec

**Date:** 2026-06-05  
**Status:** Approved

## Goal

Display nutrition information stored in `.cook` recipe metadata in the web UI, when available. Two storage formats exist: flat top-level keys (hand-authored) and a nested `nutrition:` mapping (from imported recipes).

## Data formats supported

**Flat keys** (hand-authored):
```
>> calories: 350
>> protein: 25g
>> fat: 12g
>> carbohydrates: 40g
>> fiber: 5g
>> sugar: 8g
>> sodium: 480mg
```

**Nested mapping** (importer output):
```
>> nutrition:
>>   calories: 350 calories
>>   fat: 18 grams fat
>>   protein: 25g
>>   carbohydrates: 40g
>>   fiber: 5g
>>   sugar: 8g
>>   sodium: 480mg
>>   serving size: 1 cup
```

## Architecture

### 1. `src/server/templates.rs` — new `NutritionData` struct

Add a new struct and a field to `RecipeMetadata`:

```rust
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

// RecipeMetadata gains:
pub nutrition: Option<NutritionData>,
```

### 2. `src/server/builders.rs` — extract nutrition

Before building `custom_metadata`, extract nutrition from both formats:

1. Check if `nutrition` key exists as a YAML mapping → read sub-keys
2. Otherwise look for flat top-level keys (`calories`, `protein`, `fat`, `saturated fat` / `saturated_fat`, `carbohydrates`, `fiber`, `sugar`, `sodium`, `serving size` / `serving_size`)
3. A `NutritionData` is `None` if no fields are found
4. Exclude all nutrition-related keys from the `custom` vec to avoid duplication

Known flat key names to exclude from custom:
`calories`, `protein`, `fat`, `saturated fat`, `saturated_fat`, `carbohydrates`, `fiber`, `sugar`, `sodium`, `serving size`, `serving_size`

The same extraction logic also applies to the second `RecipeMetadata` builder at line ~937 (menu template).

### 3. `templates/recipe.html` — collapsible section

Position: between the metadata pills container (`#metadata-container`) and the ingredients/steps grid.

Render only when `nutrition` is `Some`. Use a native HTML `<details>`/`<summary>` element (no JS required). Style with Tailwind to match the existing card aesthetic.

Visual design:
- Summary line: `🔬 Nutrition` (label via `tr.t("meta-nutrition")`) with a chevron indicator
- Expanded content: a row of pill-like stat boxes for each present field
- Primary fields (larger): Calories, Protein, Fat, Carbs
- Secondary fields (smaller): Fiber, Sugar, Sodium, Saturated Fat, Serving size
- Label/value layout inside each pill; all labels use `tr.t("meta-<field>")` keys

### 4. `locales/*/recipes.ftl` — translation keys

Add the following keys to all 7 locale files (en-US, de-DE, es-ES, eu-ES, fr-FR, nl-NL, sv-SE):

```
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

Translations per locale (best-effort; native speakers should review):

| Key | fr-FR | de-DE | es-ES | eu-ES | nl-NL | sv-SE |
|-----|-------|-------|-------|-------|-------|-------|
| meta-nutrition | Valeurs nutritionnelles | Nährwerte | Información nutricional | Elikadura balioak | Voedingswaarden | Näringsvärden |
| meta-calories | Calories | Kalorien | Calorías | Kaloriak | Calorieën | Kalorier |
| meta-protein | Protéines | Eiweiß | Proteínas | Proteinak | Eiwitten | Protein |
| meta-fat | Lipides | Fett | Grasas | Koipeak | Vetten | Fett |
| meta-saturated-fat | Acides gras saturés | Gesättigte Fettsäuren | Grasas saturadas | Gantz aseak | Verzadigde vetten | Mättat fett |
| meta-carbohydrates | Glucides | Kohlenhydrate | Carbohidratos | Karbohidratoak | Koolhydraten | Kolhydrater |
| meta-fiber | Fibres | Ballaststoffe | Fibra | Zuntzak | Vezels | Fibrer |
| meta-sugar | Sucres | Zucker | Azúcares | Azukreak | Suikers | Socker |
| meta-sodium | Sodium | Natrium | Sodio | Sodioa | Natrium | Natrium |
| meta-serving-size | Par portion | Portionsgröße | Tamaño de porción | Zerbitzatu tamaina | Portiegrootte | Portionsstorlek |

## Scope

- No changes to `cooklang-rs` — nutrition is not a `StdKey`; we read it from the raw metadata map
- No changes to the CLI `recipe` command output — this is UI-only
- No per-serving scaling of nutrition values — values are displayed as stored
- Both builder instances in `builders.rs` updated for consistency

## Out of scope

- Nutrition per-serving math when scaling
- Editing nutrition from the UI
- CLI text/JSON output of nutrition
