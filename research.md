# Multi-Language Translation Research for CookCLI Web UI

**Target Languages:** German (de), Dutch (nl), French (fr), Spanish (es)

**Date:** 2025-11-17

## Table of Contents

1. [Current Architecture](#current-architecture)
2. [I18n Solutions for Rust + Askama](#i18n-solutions-for-rust--askama)
3. [Recommended Approach](#recommended-approach)
4. [Implementation Strategy](#implementation-strategy)
5. [Translation File Structure](#translation-file-structure)
6. [Browser Language Detection](#browser-language-detection)
7. [User Preferences](#user-preferences)
8. [Example Implementation](#example-implementation)
9. [Alternative Approaches](#alternative-approaches)
10. [Effort Estimation](#effort-estimation)

---

## Current Architecture

### Web UI Stack
- **Backend Framework:** Axum (async Rust web framework)
- **Template Engine:** Askama (compile-time Jinja-like templates)
- **Styling:** Tailwind CSS (utility-first CSS)
- **Rendering:** Server-Side Rendering (SSR)

### Template Structure
```
templates/
├── base.html          # Base template with navigation
├── recipes.html       # Recipe listing
├── recipe.html        # Individual recipe display
├── shopping_list.html # Shopping list page
├── preferences.html   # User preferences
├── pantry.html        # Pantry management
└── menu.html          # Menu display
```

### Current Text Locations
All user-facing text is currently hardcoded in:
1. **HTML Templates** - Navigation labels, headings, buttons, placeholders
2. **Rust Handlers** - Error messages, status messages (src/server/ui.rs)
3. **JavaScript** - Search placeholder, "No recipes found" messages (base.html)

Examples from `base.html:489-493`:
- "Recipes" navigation button
- "Shopping List" navigation button
- "Pantry" navigation button
- "Search recipes..." placeholder

---

## I18n Solutions for Rust + Askama

### Research Findings

#### 1. No Native Askama Support
- **Askama does not have built-in i18n support** as of 2025
- GitHub issue #202 mentions i18n as "probably out of scope"
- Previous attempts to add native i18n were archived

#### 2. Available Rust i18n Libraries

##### fluent-rs / fluent-templates
- **Maintainer:** Mozilla (used in Firefox)
- **Format:** FTL (Fluent Translation List) files
- **Pros:**
  - Industry-standard, battle-tested in Firefox
  - Natural language features (plurals, gender, conjugations)
  - Clean, readable syntax
- **Cons:**
  - No official Askama integration
  - Only supports Tera and Handlebars out-of-the-box
  - Requires custom filter implementation for Askama

##### rust-i18n
- **Format:** YAML files
- **Pros:**
  - Simple setup
  - Compile-time validation
- **Cons:**
  - Less mature than Fluent
  - Limited natural language features

##### gettext-rs
- **Format:** PO/POT files
- **Pros:**
  - Traditional, widely used format
  - Many translation tools support it
- **Cons:**
  - More complex setup
  - Older approach

#### 3. Axum-Specific Solutions

##### axum_l10n
- **Purpose:** Language detection and routing for Axum
- **Features:**
  - Parses Accept-Language header
  - Middleware for language extraction
  - Can work with fluent-rs
- **Integration:**
  ```rust
  use axum_l10n::LanguageIdentifierExtractorLayer;

  .layer(LanguageIdentifierExtractorLayer::new(
      langid!("en-US"),                    // Default language
      vec![langid!("en-US"), langid!("de")], // Supported languages
      axum_l10n::RedirectMode::NoRedirect,
  ))
  ```

---

## Recommended Approach

### Primary Recommendation: Fluent + Custom Askama Filters

**Why Fluent?**
1. **Production-Ready:** Used by Mozilla Firefox
2. **Natural Language Support:** Handles plurals, gender, etc.
3. **Clean Syntax:** Easy for translators
4. **Future-Proof:** Active development and community

**Implementation Method:**
Create custom Askama filters that wrap fluent-templates functionality.

### Architecture Overview

```
┌─────────────────┐
│  HTTP Request   │
│ (Accept-Language)│
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ axum_l10n       │ ◄── Parse Accept-Language header
│ Middleware      │     Extract language preference
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Route Handler  │ ◄── Get LanguageIdentifier from Extension
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Template Struct │ ◄── Pass language to template
│ (with lang field)│
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Askama Template │ ◄── Use custom filter: {{ "key" | t(lang) }}
│ + Fluent Filter │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  HTML Response  │
└─────────────────┘
```

---

## Implementation Strategy

### Phase 1: Setup (Week 1)

1. **Add Dependencies** (Cargo.toml)
```toml
[dependencies]
fluent = "0.16"
fluent-templates = "0.10"
unic-langid = "0.9"
axum_l10n = "0.6"
```

2. **Create Locales Directory Structure**
```
locales/
├── en-US/
│   ├── common.ftl
│   ├── recipes.ftl
│   ├── shopping.ftl
│   └── preferences.ftl
├── de-DE/
│   ├── common.ftl
│   ├── recipes.ftl
│   ├── shopping.ftl
│   └── preferences.ftl
├── nl-NL/
│   └── [same files]
├── fr-FR/
│   └── [same files]
└── es-ES/
    └── [same files]
```

3. **Setup Fluent Loader** (src/server/i18n.rs)
```rust
use fluent_templates::static_loader;

static_loader! {
    pub static LOCALES = {
        locales: "./locales",
        fallback_language: "en-US",
        customise: |bundle| bundle.set_use_isolating(false),
    };
}
```

### Phase 2: Custom Askama Filter (Week 1-2)

Create a translation filter that works with Askama:

```rust
// src/server/filters.rs
use fluent_templates::Loader;
use unic_langid::LanguageIdentifier;
use std::collections::HashMap;

pub fn t(
    key: impl std::fmt::Display,
    lang: &LanguageIdentifier,
) -> askama::Result<String> {
    let key_str = key.to_string();
    Ok(crate::server::i18n::LOCALES.lookup(lang, &key_str))
}

pub fn t_with_args(
    key: impl std::fmt::Display,
    lang: &LanguageIdentifier,
    args: &HashMap<String, String>,
) -> askama::Result<String> {
    let key_str = key.to_string();
    let mut fluent_args = fluent::FluentArgs::new();

    for (k, v) in args {
        fluent_args.set(k.clone(), v.clone());
    }

    Ok(crate::server::i18n::LOCALES.lookup_with_args(lang, &key_str, &fluent_args))
}
```

### Phase 3: Middleware Integration (Week 2)

**Setup Language Detection:**

```rust
// src/server/mod.rs
use axum_l10n::LanguageIdentifierExtractorLayer;
use unic_langid::{langid, LanguageIdentifier};

const EN: LanguageIdentifier = langid!("en-US");
const DE: LanguageIdentifier = langid!("de-DE");
const NL: LanguageIdentifier = langid!("nl-NL");
const FR: LanguageIdentifier = langid!("fr-FR");
const ES: LanguageIdentifier = langid!("es-ES");

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .merge(ui())
        .merge(api())
        .layer(LanguageIdentifierExtractorLayer::new(
            EN,
            vec![EN, DE, NL, FR, ES],
            axum_l10n::RedirectMode::NoRedirect,
        ))
        .with_state(state)
}
```

### Phase 4: Template Updates (Week 2-3)

**Update Template Structs:**

```rust
// src/server/templates.rs
use unic_langid::LanguageIdentifier;

#[derive(Template)]
#[template(path = "recipes.html")]
pub struct RecipesTemplate {
    pub active: String,
    pub current_name: String,
    pub breadcrumbs: Vec<Breadcrumb>,
    pub items: Vec<RecipeItem>,
    pub lang: LanguageIdentifier,  // Add this
}
```

**Update Handlers:**

```rust
// src/server/ui.rs
use axum::extract::Extension;

async fn recipes_page(
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
) -> Result<impl askama_axum::IntoResponse, StatusCode> {
    let template = RecipesTemplate {
        active: "recipes".to_string(),
        current_name,
        breadcrumbs,
        items,
        lang,  // Pass language to template
    };
    Ok(template)
}
```

**Update Templates:**

```html
<!-- templates/base.html -->
<a href="/" class="...">
    <span>{{ "nav.recipes" | t(lang) }}</span>
</a>
<a href="/shopping-list" class="...">
    <span>{{ "nav.shopping-list" | t(lang) }}</span>
</a>
```

### Phase 5: Translation Content (Week 3-4)

Extract all hardcoded strings and create FTL files.

### Phase 6: Testing (Week 4)

1. Test each language manually
2. Add automated tests for translation loading
3. Test fallback to English for missing keys

---

## Translation File Structure

### Example: locales/en-US/common.ftl

```fluent
# Navigation
nav-recipes = Recipes
nav-shopping-list = Shopping List
nav-pantry = Pantry
nav-preferences = Preferences

# Search
search-placeholder = Search recipes...
search-no-results = No recipes found

# Common Actions
action-add = Add
action-remove = Remove
action-edit = Edit
action-save = Save
action-cancel = Cancel
action-delete = Delete
action-clear = Clear

# Common Labels
label-scale = Scale
label-servings = Servings
label-time = Time
label-difficulty = Difficulty
```

### Example: locales/en-US/recipes.ftl

```fluent
# Recipe Listing
recipes-title = All Recipes
recipes-directory-title = { $name }
recipes-empty = No recipes found in this directory.
recipes-count =
    { $count ->
        [one] { $count } recipe
       *[other] { $count } recipes
    }

# Recipe Display
recipe-ingredients = Ingredients
recipe-steps = Instructions
recipe-notes = Notes
recipe-cookware = Cookware
recipe-tags = Tags
recipe-add-to-shopping = Add to Shopping List

# Recipe Metadata
meta-course = Course
meta-cuisine = Cuisine
meta-diet = Diet
meta-author = Author
meta-source = Source
meta-prep-time = Prep Time
meta-cook-time = Cook Time
meta-total-time = Total Time
```

### Example: locales/de-DE/common.ftl

```fluent
# Navigation
nav-recipes = Rezepte
nav-shopping-list = Einkaufsliste
nav-pantry = Vorratskammer
nav-preferences = Einstellungen

# Search
search-placeholder = Rezepte suchen...
search-no-results = Keine Rezepte gefunden

# Common Actions
action-add = Hinzufügen
action-remove = Entfernen
action-edit = Bearbeiten
action-save = Speichern
action-cancel = Abbrechen
action-delete = Löschen
action-clear = Löschen
```

### Example with Variables: locales/en-US/shopping.ftl

```fluent
# Shopping List
shopping-title = Shopping List
shopping-empty = Your shopping list is empty
shopping-item-count =
    { $count ->
        [one] { $count } item
       *[other] { $count } items
    }
shopping-from-recipe = From { $recipeName }
shopping-clear-confirm = Are you sure you want to clear all items?
shopping-added = Added { $itemName } to shopping list
```

---

## Browser Language Detection

### Accept-Language Header

The `Accept-Language` HTTP header indicates the user's preferred languages:

```
Accept-Language: de-DE,de;q=0.9,en-US;q=0.8,en;q=0.7
```

This means:
1. German (Germany) - preferred
2. German (any) - 90% preference
3. English (US) - 80% preference
4. English (any) - 70% preference

### axum_l10n Middleware

The middleware automatically:
1. Parses the Accept-Language header
2. Finds the best match from supported languages
3. Extracts it as `Extension<LanguageIdentifier>`
4. Falls back to default if no match

### Manual Detection Alternative

```rust
async fn detect_language(
    headers: HeaderMap,
) -> LanguageIdentifier {
    if let Some(accept_lang) = headers.get(header::ACCEPT_LANGUAGE) {
        if let Ok(lang_str) = accept_lang.to_str() {
            // Parse and match against supported languages
            // Return best match
        }
    }
    langid!("en-US") // Default fallback
}
```

---

## User Preferences

### Cookie-Based Language Override

Allow users to manually select their language preference, overriding browser settings:

```rust
// Check cookie first, then Accept-Language header
async fn get_user_language(
    cookies: Option<TypedHeader<Cookie>>,
    Extension(detected_lang): Extension<LanguageIdentifier>,
) -> LanguageIdentifier {
    if let Some(cookie) = cookies {
        if let Some(lang_cookie) = cookie.get("lang") {
            if let Ok(lang) = lang_cookie.parse::<LanguageIdentifier>() {
                return lang;
            }
        }
    }
    detected_lang
}
```

### Language Selector UI

Add to preferences page:

```html
<div class="language-selector">
    <label>{{ "pref.language" | t(lang) }}</label>
    <select name="language" onchange="setLanguage(this.value)">
        <option value="en-US">English</option>
        <option value="de-DE">Deutsch</option>
        <option value="nl-NL">Nederlands</option>
        <option value="fr-FR">Français</option>
        <option value="es-ES">Español</option>
    </select>
</div>

<script>
function setLanguage(lang) {
    document.cookie = `lang=${lang}; path=/; max-age=31536000`;
    location.reload();
}
</script>
```

---

## Example Implementation

### Complete Flow Example

**1. FTL File (locales/en-US/recipes.ftl):**
```fluent
recipe-scale-label = Scale
recipe-scale-to = Scale to { $servings } servings
```

**2. Template (templates/recipe.html):**
```html
<div class="scale-control">
    <label>{{ "recipe-scale-label" | t(lang) }}</label>
    <input type="number" value="{{ scale }}" min="0.5" step="0.5">
    <span>{{ "recipe-scale-to" | t_args(lang, servings=metadata.servings) }}</span>
</div>
```

**3. Handler (src/server/ui.rs):**
```rust
async fn recipe_page(
    Path(path): Path<String>,
    Query(query): Query<RecipeQuery>,
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
) -> Result<axum::response::Response, StatusCode> {
    // ... existing code ...

    let template = RecipeTemplate {
        active: "recipes".to_string(),
        recipe,
        recipe_path: path,
        breadcrumbs,
        scale,
        tags,
        ingredients,
        cookware,
        sections,
        image_path,
        lang,  // Pass language
    };

    Ok(template.into_response())
}
```

---

## Alternative Approaches

### Alternative 1: Client-Side i18n with JavaScript

**Pros:**
- Don't need to modify Askama templates
- Can use i18next or similar JS libraries
- Dynamic language switching without page reload

**Cons:**
- Breaks SSR philosophy
- Slower initial load (translation after HTML)
- Flash of untranslated content (FOUC)
- SEO concerns
- Requires shipping translation files to client

### Alternative 2: Generate Separate Templates per Language

**Approach:** Use Askama's template inheritance to create language-specific templates

```
templates/
├── en/
│   ├── recipes.html
│   └── recipe.html
├── de/
│   ├── recipes.html
│   └── recipe.html
```

**Pros:**
- Simple implementation
- No filter needed
- Full control over text

**Cons:**
- Massive duplication
- Maintenance nightmare
- Translation changes require code changes
- Not scalable

### Alternative 3: Switch to Tera Templates

**Approach:** Replace Askama with Tera (which has native fluent-templates support)

**Pros:**
- Official fluent-templates integration
- More mature i18n ecosystem
- Runtime template compilation

**Cons:**
- **Major refactor** - rewrite all templates
- Lose compile-time template checking
- Runtime overhead
- Breaking change for existing codebase

**Verdict:** Not recommended for CookCLI

---

## Effort Estimation

### Development Timeline (Single Developer)

| Phase | Tasks | Estimated Time |
|-------|-------|----------------|
| **Phase 1: Setup** | Add dependencies, create directory structure, setup Fluent loader | 2-3 days |
| **Phase 2: Custom Filters** | Implement Askama filters for translation | 2-3 days |
| **Phase 3: Middleware** | Integrate axum_l10n, language detection | 1-2 days |
| **Phase 4: Template Updates** | Update all template structs and HTML templates | 3-4 days |
| **Phase 5: Translation** | Extract strings, create base English FTL files | 2-3 days |
| **Phase 6: Testing** | Manual and automated testing | 2-3 days |
| **Phase 7: Translation** | Translate to DE, NL, FR, ES (if doing yourself) | 4-5 days |
| **Total (English only)** | | **12-18 days** |
| **Total (All languages)** | | **16-23 days** |

### Translation Effort

**Strings to Translate (Estimated):**
- Navigation: ~10 strings
- Common actions/labels: ~20 strings
- Recipe pages: ~30 strings
- Shopping list: ~15 strings
- Preferences: ~15 strings
- Error messages: ~10 strings
- **Total: ~100 strings**

**Translation Options:**
1. **Professional Translation Service:** $0.10-0.20/word × ~500 words × 4 languages = **$200-400**
2. **Community Translation:** Free but requires coordination and review
3. **Machine Translation (DeepL/GPT) + Review:** Quick but needs native speaker review

---

## Recommendations Summary

### ✅ Recommended Stack

1. **fluent-templates** - Translation library
2. **axum_l10n** - Language detection middleware
3. **Custom Askama filters** - Bridge between Askama and Fluent
4. **Cookie-based preferences** - User language override

### 📋 Implementation Checklist

- [ ] Add fluent dependencies to Cargo.toml
- [ ] Create locales directory structure
- [ ] Implement Fluent loader (src/server/i18n.rs)
- [ ] Create custom Askama filters (src/server/filters.rs)
- [ ] Add axum_l10n middleware
- [ ] Update all template structs to include `lang` field
- [ ] Update all route handlers to extract language
- [ ] Extract all hardcoded strings to FTL files
- [ ] Update all HTML templates to use translation filters
- [ ] Update JavaScript strings in base.html
- [ ] Add language selector to preferences page
- [ ] Implement cookie-based language override
- [ ] Test all pages in all languages
- [ ] Add fallback handling for missing translations
- [ ] Document translation workflow for contributors

### 🎯 Next Steps

1. **Get approval** on the general approach
2. **Start with Phase 1** (setup and dependencies)
3. **Create POC** for one template (e.g., recipes.html)
4. **Validate POC** before rolling out to all templates
5. **Extract strings** to English FTL files
6. **Translate** to target languages (DE, NL, FR, ES)

---

## Questions to Resolve

1. **Should language preference be stored in:**
   - Cookie (recommended - simple, no backend changes)
   - Database (requires user accounts)
   - localStorage (client-side only, no SSR benefit)

2. **URL structure:**
   - Keep current URLs (recommended - simpler)
   - Add language prefix (`/de/recipes`, `/fr/recipes`)

3. **Fallback strategy:**
   - Always fall back to English for missing keys (recommended)
   - Show key name if translation missing (debug mode)

4. **Translation workflow:**
   - Professional translators
   - Community contributions via GitHub
   - Machine translation + review

---

## References

- [Project Fluent](https://projectfluent.org/)
- [fluent-templates crate](https://crates.io/crates/fluent-templates)
- [axum_l10n crate](https://crates.io/crates/axum_l10n)
- [Askama documentation](https://github.com/askama-rs/askama)
- [Fluent Syntax Guide](https://projectfluent.org/fluent/guide/)
- [Web app localization in Rust blog post](https://yieldcode.blog/post/webapp-localization-in-rust/)

---

**Document Version:** 1.0
**Last Updated:** 2025-11-17
**Author:** Claude (AI Research Assistant)
