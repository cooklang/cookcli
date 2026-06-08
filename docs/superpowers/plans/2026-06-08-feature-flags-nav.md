# Feature Flags: Configurable Nav Bar — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let users enable/disable Shopping List and Pantry nav links from the Preferences page; both features are shown by default and the setting persists via cookies refreshed on every request.

**Architecture:** A `FeatureFlags` struct lives in `src/server/language.rs` alongside the existing language cookie logic. An Axum middleware reads feature flag cookies, injects `FeatureFlags` as a request extension, and refreshes the cookie expiry on every response. Every template struct gains a `features: FeatureFlags` field; `base.html` renders nav pills conditionally; `preferences.html` shows toggle buttons.

**Tech Stack:** Rust/Axum, Askama templates, Tailwind CSS, Fluent (FTL) i18n, Playwright (E2E tests)

---

## File Map

| File | Change |
|------|--------|
| `src/server/language.rs` | Add `FeatureFlags`, `parse_feature_flags`, `features_middleware` |
| `src/server/mod.rs` | Register `features_middleware` with `from_fn_with_state` |
| `src/server/templates.rs` | Add `features: FeatureFlags` to all 9 template structs |
| `src/server/builders.rs` | Add `features` to `RecipesBuildInput` and `RecipeBuildInput`; thread through |
| `src/server/ui.rs` | Extract `Extension(features)` in every handler; update `error_page` |
| `src/build/renderer.rs` | Pass `FeatureFlags::default()` to builder inputs (static mode) |
| `templates/base.html` | Conditional nav pills (desktop + mobile dropdown); dark mode CSS |
| `templates/preferences.html` | Features section card + `toggleFeature` JS |
| `locales/en-US/preferences.ftl` | Add `pref-features`, `pref-features-desc` |
| `locales/de-DE/preferences.ftl` | Same |
| `locales/nl-NL/preferences.ftl` | Same |
| `locales/fr-FR/preferences.ftl` | Same |
| `locales/es-ES/preferences.ftl` | Same |
| `locales/eu-ES/preferences.ftl` | Same |
| `locales/sv-SE/preferences.ftl` | Same |
| `tests/e2e/navigation.spec.ts` | Tests for feature-flag nav visibility |
| `tests/e2e/preferences.spec.ts` | Tests for feature toggle in preferences UI |

---

## Task 1: `FeatureFlags` struct and cookie parsing

**Files:**
- Modify: `src/server/language.rs`

- [ ] **Step 1.1 — Write the failing unit tests**

Add at the bottom of `src/server/language.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn make_cookie_headers(cookies: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(header::COOKIE, cookies.parse().unwrap());
        h
    }

    #[test]
    fn test_feature_flags_default_true_when_no_cookies() {
        let flags = parse_feature_flags(&HeaderMap::new());
        assert!(flags.show_shopping_list);
        assert!(flags.show_pantry);
    }

    #[test]
    fn test_feature_flags_disabled_by_zero() {
        let flags = parse_feature_flags(&make_cookie_headers(
            "show_shopping_list=0; show_pantry=0",
        ));
        assert!(!flags.show_shopping_list);
        assert!(!flags.show_pantry);
    }

    #[test]
    fn test_feature_flags_enabled_by_one() {
        let flags = parse_feature_flags(&make_cookie_headers(
            "show_shopping_list=1; show_pantry=1",
        ));
        assert!(flags.show_shopping_list);
        assert!(flags.show_pantry);
    }

    #[test]
    fn test_feature_flags_partial_override() {
        let flags =
            parse_feature_flags(&make_cookie_headers("show_shopping_list=0"));
        assert!(!flags.show_shopping_list);
        assert!(flags.show_pantry); // absent → default true
    }

    #[test]
    fn test_feature_flags_unknown_value_treated_as_enabled() {
        // Anything that isn't "0" is truthy
        let flags = parse_feature_flags(&make_cookie_headers(
            "show_shopping_list=yes",
        ));
        assert!(flags.show_shopping_list);
    }
}
```

- [ ] **Step 1.2 — Run tests to confirm they fail**

```bash
cd /Users/romain/Projects/cooklang/cookcli
cargo test test_feature_flags 2>&1 | head -30
```

Expected: compile errors — `parse_feature_flags` and `FeatureFlags` are not defined yet.

- [ ] **Step 1.3 — Implement `FeatureFlags` and `parse_feature_flags`**

Add the following to `src/server/language.rs` after the `SUPPORTED_LANGUAGES` constant (around line 20):

```rust
/// Per-request feature visibility flags, read from cookies.
#[derive(Clone, Debug)]
pub struct FeatureFlags {
    pub show_shopping_list: bool,
    pub show_pantry: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            show_shopping_list: true,
            show_pantry: true,
        }
    }
}

/// Parse feature flag cookies from request headers.
/// Absent cookie → feature enabled (default true).
/// Cookie value "0" → disabled. Any other value → enabled.
pub fn parse_feature_flags(headers: &HeaderMap) -> FeatureFlags {
    let mut flags = FeatureFlags::default();

    if let Some(cookie_header) = headers.get(header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
                if parts.len() == 2 {
                    match parts[0] {
                        "show_shopping_list" => {
                            flags.show_shopping_list = parts[1] != "0"
                        }
                        "show_pantry" => flags.show_pantry = parts[1] != "0",
                        _ => {}
                    }
                }
            }
        }
    }

    flags
}
```

- [ ] **Step 1.4 — Run tests to confirm they pass**

```bash
cargo test test_feature_flags 2>&1
```

Expected: 5 tests pass.

- [ ] **Step 1.5 — Commit**

```bash
git add src/server/language.rs
git commit -m "feat: add FeatureFlags struct and cookie parsing for nav feature toggles"
```

---

## Task 2: Features middleware

**Files:**
- Modify: `src/server/language.rs`
- Modify: `src/server/mod.rs`

- [ ] **Step 2.1 — Add `State` to imports in `language.rs`**

In `src/server/language.rs`, change the existing `use axum::{...}` block to include `State`:

```rust
use axum::{
    extract::{Request, State},
    http::{header, HeaderMap},
    middleware::Next,
    response::Response,
};
```

- [ ] **Step 2.2 — Implement `features_middleware` in `language.rs`**

Add after the `language_middleware` function:

```rust
/// Middleware that reads feature flag cookies, injects them as a request
/// extension, and refreshes the cookie expiry on every response.
/// Takes the URL prefix as state so Set-Cookie headers use the correct path.
pub async fn features_middleware(
    State(url_prefix): State<String>,
    mut req: Request,
    next: Next,
) -> Response {
    let features = parse_feature_flags(req.headers());
    req.extensions_mut().insert(features.clone());
    let mut response = next.run(req).await;

    let max_age = 365 * 24 * 60 * 60_u32;
    let cookie_path = if url_prefix.is_empty() {
        "/".to_string()
    } else {
        url_prefix.clone()
    };

    for (name, val) in [
        (
            "show_shopping_list",
            if features.show_shopping_list { "1" } else { "0" },
        ),
        ("show_pantry", if features.show_pantry { "1" } else { "0" }),
    ] {
        let cookie = format!(
            "{name}={val}; path={cookie_path}; max-age={max_age}; SameSite=Lax"
        );
        if let Ok(header_val) = cookie.parse() {
            response.headers_mut().append(header::SET_COOKIE, header_val);
        }
    }

    response
}
```

- [ ] **Step 2.3 — Register the middleware in `mod.rs`**

In `src/server/mod.rs`, in the `run` function, extract `url_prefix` before `with_state` consumes `state`, then register the middleware. Find the block starting with `#[cfg(feature = "sync")] let state_for_shutdown = state.clone();` and add the prefix capture right before it:

```rust
    // Capture url_prefix before state is consumed by with_state.
    let url_prefix_for_features = state.url_prefix.clone();

    #[cfg(feature = "sync")]
    let state_for_shutdown = state.clone();

    let app = app
        .with_state(state)
        .layer(DefaultBodyLimit::max(MAX_BODY_SIZE))
        .layer(axum::middleware::from_fn_with_state(
            url_prefix_for_features,
            language::features_middleware,
        ))
        .layer(axum::middleware::from_fn(language::language_middleware))
        .layer(
            CorsLayer::new()
                .allow_origin("*".parse::<HeaderValue>().unwrap())
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE]),
        );
```

- [ ] **Step 2.4 — Build to confirm it compiles**

```bash
cargo build -p cookcli 2>&1 | grep -E "^error"
```

Expected: no errors.

- [ ] **Step 2.5 — Commit**

```bash
git add src/server/language.rs src/server/mod.rs
git commit -m "feat: add features_middleware that injects FeatureFlags and refreshes cookies"
```

---

## Task 3: Add `FeatureFlags` to template structs, builders, and handlers

This task must be done in one step because Askama validates template fields at compile time. All structs must have `features` before templates can reference it.

**Files:**
- Modify: `src/server/templates.rs`
- Modify: `src/server/builders.rs`
- Modify: `src/server/ui.rs`
- Modify: `src/build/renderer.rs`

- [ ] **Step 3.1 — Add import and `features` field to all template structs in `templates.rs`**

At the top of `src/server/templates.rs`, add the import after the existing imports:

```rust
use crate::server::language::FeatureFlags;
```

Then add `pub features: FeatureFlags,` to each template struct. The complete list of structs to update and their new fields (add after the last existing field in each):

```rust
// ErrorTemplate
pub struct ErrorTemplate {
    pub active: String,
    pub error_message: String,
    pub tr: Tr,
    pub prefix: String,
    pub static_mode: bool,
    pub features: FeatureFlags,        // ADD
}

// RecipesTemplate
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
    pub features: FeatureFlags,        // ADD
}

// RecipeTemplate
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
    pub features: FeatureFlags,        // ADD
}

// MenuTemplate
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
    pub features: FeatureFlags,        // ADD
}

// ShoppingListTemplate
pub struct ShoppingListTemplate {
    pub active: String,
    pub tr: Tr,
    pub prefix: String,
    pub static_mode: bool,
    pub features: FeatureFlags,        // ADD
}

// PreferencesTemplate
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
    pub features: FeatureFlags,        // ADD
}

// PantryTemplate
pub struct PantryTemplate {
    pub active: String,
    pub configured: bool,
    pub sections: Vec<PantrySection>,
    pub tr: Tr,
    pub prefix: String,
    pub static_mode: bool,
    pub features: FeatureFlags,        // ADD
}

// EditTemplate
pub struct EditTemplate {
    pub active: String,
    pub recipe_name: String,
    pub recipe_path: String,
    pub content: String,
    pub base_path: String,
    pub tr: Tr,
    pub prefix: String,
    pub static_mode: bool,
    pub features: FeatureFlags,        // ADD
}

// NewTemplate
pub struct NewTemplate {
    pub active: String,
    pub tr: Tr,
    pub error: Option<String>,
    pub filename: Option<String>,
    pub prefix: String,
    pub static_mode: bool,
    pub features: FeatureFlags,        // ADD
}
```

- [ ] **Step 3.2 — Add `features` to builder input structs in `builders.rs`**

In `src/server/builders.rs`, add to imports at top:

```rust
use crate::server::language::FeatureFlags;
```

Update `RecipesBuildInput`:

```rust
pub struct RecipesBuildInput<'a> {
    pub base_path: &'a Utf8Path,
    pub url_prefix: &'a str,
    pub sub_path: Option<&'a str>,
    pub lang: LanguageIdentifier,
    pub static_mode: bool,
    pub features: FeatureFlags,        // ADD
}
```

Update `RecipeBuildInput`:

```rust
pub struct RecipeBuildInput<'a> {
    pub base_path: &'a Utf8Path,
    pub url_prefix: &'a str,
    pub recipe_path: &'a str,
    pub aisle_path: Option<&'a Utf8PathBuf>,
    pub scale: f64,
    pub lang: LanguageIdentifier,
    pub static_mode: bool,
    pub features: FeatureFlags,        // ADD
}
```

In `build_recipes_template`, destructure the new field and pass it to the template:

```rust
pub fn build_recipes_template(input: RecipesBuildInput<'_>) -> Result<RecipesTemplate> {
    let RecipesBuildInput {
        base_path,
        url_prefix,
        sub_path,
        lang,
        static_mode,
        features,          // ADD
    } = input;
    // ... existing body unchanged ...
    Ok(RecipesTemplate {
        active: "recipes".to_string(),
        current_name,
        breadcrumbs,
        items,
        todays_menu,
        new_recipe_url,
        tr: Tr::new(lang),
        prefix: url_prefix.to_string(),
        static_mode,
        features,          // ADD
    })
}
```

In `build_recipe_template`, destructure and thread through. In the `RecipeBuildInput` destructuring block:

```rust
    let RecipeBuildInput {
        base_path,
        url_prefix,
        recipe_path,
        aisle_path,
        scale,
        lang,
        static_mode,
        features,          // ADD
    } = input;
```

Pass to `RecipeTemplate` at the bottom of `build_recipe_template`:

```rust
    let template = RecipeTemplate {
        active: "recipes".to_string(),
        recipe: RecipeData { name: recipe_name, metadata },
        recipe_path: recipe_path.to_string(),
        breadcrumbs,
        scale,
        tags,
        ingredients,
        cookware,
        sections,
        image_path,
        tr: Tr::new(lang),
        prefix: url_prefix.to_string(),
        static_mode,
        features,              // ADD
    };
```

Pass to `build_menu_template_inner` (which builds `MenuTemplate`). Update the call in `build_recipe_template`:

```rust
        let template = build_menu_template_inner(
            recipe_path.to_string(),
            scale,
            entry,
            base_path,
            url_prefix,
            lang,
            static_mode,
            features,          // ADD
        )?;
```

Update `build_menu_template_inner` signature and constructor:

```rust
fn build_menu_template_inner(
    path: String,
    scale: f64,
    entry: cooklang_find::RecipeEntry,
    base_path: &Utf8Path,
    url_prefix: &str,
    lang: LanguageIdentifier,
    static_mode: bool,
    features: FeatureFlags,    // ADD
) -> Result<MenuTemplate> {
    // ... existing body unchanged ...
    Ok(MenuTemplate {
        active: "recipes".to_string(),
        name: menu_name,
        recipe_path: path,
        breadcrumbs,
        scale,
        metadata,
        sections,
        image_path,
        tr: Tr::new(lang),
        prefix: url_prefix.to_string(),
        static_mode,
        features,              // ADD
    })
}
```

- [ ] **Step 3.3 — Update `renderer.rs` to pass `FeatureFlags::default()`**

In `src/build/renderer.rs`, add import at top:

```rust
use crate::server::language::FeatureFlags;
```

In `render_index`, update the `RecipesBuildInput`:

```rust
    let template = build_recipes_template(RecipesBuildInput {
        base_path: source,
        url_prefix: &prefix,
        sub_path: None,
        lang: lang.clone(),
        static_mode: true,
        features: FeatureFlags::default(),    // ADD
    })?;
```

In `render_directory`, same change:

```rust
    let template = build_recipes_template(RecipesBuildInput {
        base_path: source,
        url_prefix: &prefix,
        sub_path: Some(sub_path),
        lang: lang.clone(),
        static_mode: true,
        features: FeatureFlags::default(),    // ADD
    })?;
```

In `render_recipe`, update `RecipeBuildInput`:

```rust
    let kind = build_recipe_template(RecipeBuildInput {
        base_path: source,
        url_prefix: &prefix,
        recipe_path: trimmed,
        aisle_path,
        scale: 1.0,
        lang: lang.clone(),
        static_mode: true,
        features: FeatureFlags::default(),    // ADD
    })?;
```

- [ ] **Step 3.4 — Update all handlers in `ui.rs`**

Add import at the top of `src/server/ui.rs`:

```rust
use crate::server::language::FeatureFlags;
```

Update `error_page` helper to accept and pass features:

```rust
fn error_page(
    lang: LanguageIdentifier,
    prefix: &str,
    msg: impl std::fmt::Display,
    features: FeatureFlags,
) -> axum::response::Response {
    let template = ErrorTemplate {
        active: String::new(),
        error_message: msg.to_string(),
        tr: Tr::new(lang),
        prefix: prefix.to_string(),
        static_mode: false,
        features,
    };
    template.into_response()
}
```

Update `recipes_page`:

```rust
async fn recipes_page(
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
) -> axum::response::Response {
    recipes_handler(state, None, lang, features).await
}
```

Update `recipes_directory`:

```rust
async fn recipes_directory(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
) -> axum::response::Response {
    recipes_handler(state, Some(path), lang, features).await
}
```

Update `recipes_handler` to accept and thread features:

```rust
async fn recipes_handler(
    state: Arc<AppState>,
    path: Option<String>,
    lang: LanguageIdentifier,
    features: FeatureFlags,
) -> axum::response::Response {
    let input = crate::server::builders::RecipesBuildInput {
        base_path: &state.base_path,
        url_prefix: &state.url_prefix,
        sub_path: path.as_deref(),
        lang: lang.clone(),
        static_mode: false,
        features: features.clone(),
    };
    match crate::server::builders::build_recipes_template(input) {
        Ok(template) => template.into_response(),
        Err(e) => {
            tracing::error!("Failed to build recipes template: {:?}", e);
            error_page(lang, &state.url_prefix, &e, features)
        }
    }
}
```

Update `recipe_page`:

```rust
async fn recipe_page(
    Path(path): Path<String>,
    Query(query): Query<RecipeQuery>,
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
) -> axum::response::Response {
    let scale = query.scale.unwrap_or(1.0);

    let input = crate::server::builders::RecipeBuildInput {
        base_path: &state.base_path,
        url_prefix: &state.url_prefix,
        recipe_path: &path,
        aisle_path: state.aisle_path.as_ref(),
        scale,
        lang: lang.clone(),
        static_mode: false,
        features: features.clone(),
    };

    match crate::server::builders::build_recipe_template(input) {
        Ok(crate::server::builders::RecipeBuildOutput::Recipe(template)) => {
            template.into_response()
        }
        Ok(crate::server::builders::RecipeBuildOutput::Menu(template)) => template.into_response(),
        Err(e) => {
            tracing::error!("Failed to build recipe template: {:?}", e);
            error_page(lang, &state.url_prefix, &e, features)
        }
    }
}
```

Update `edit_page`:

```rust
async fn edit_page(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
) -> axum::response::Response {
    // ... existing validation/reading logic unchanged ...
    let template = crate::server::templates::EditTemplate {
        active: "recipes".to_string(),
        recipe_name,
        recipe_path: path,
        content,
        base_path: state.base_path.to_string(),
        tr: crate::server::templates::Tr::new(lang),
        prefix: state.url_prefix.clone(),
        static_mode: false,
        features,
    };
    template.into_response()
}
```

(Keep all the existing path validation and file reading logic unchanged; just add `Extension(features): Extension<FeatureFlags>` to the signature and `features,` to the template constructor.)

Update `new_page`:

```rust
async fn new_page(
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
    Query(query): Query<NewPageQuery>,
) -> impl askama_axum::IntoResponse {
    crate::server::templates::NewTemplate {
        active: "recipes".to_string(),
        tr: Tr::new(lang),
        error: query.error,
        filename: query.filename,
        prefix: state.url_prefix.clone(),
        static_mode: false,
        features,
    }
}
```

Update `shopping_list_page`:

```rust
async fn shopping_list_page(
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
) -> impl askama_axum::IntoResponse {
    ShoppingListTemplate {
        active: "shopping".to_string(),
        tr: Tr::new(lang),
        prefix: state.url_prefix.clone(),
        static_mode: false,
        features,
    }
}
```

Update `pantry_page`:

```rust
async fn pantry_page(
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
) -> Result<impl askama_axum::IntoResponse, StatusCode> {
    // ... existing pantry loading logic unchanged ...
    Ok(PantryTemplate {
        active: "pantry".to_string(),
        configured: pantry_path.is_some(),
        sections,
        tr: Tr::new(lang),
        prefix: state.url_prefix.clone(),
        static_mode: false,
        features,
    })
}
```

Update `preferences_page`:

```rust
async fn preferences_page(
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
    Extension(features): Extension<FeatureFlags>,
) -> impl askama_axum::IntoResponse {
    #[cfg(feature = "sync")]
    let (sync_logged_in, sync_email, sync_syncing) = state.sync_status().await;
    #[cfg(not(feature = "sync"))]
    let (sync_logged_in, sync_email, sync_syncing) = (false, None, false);

    PreferencesTemplate {
        active: "preferences".to_string(),
        aisle_path: state
            .aisle_path
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_else(|| "Not configured".to_string()),
        pantry_path: state
            .pantry_path
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_else(|| "Not configured".to_string()),
        base_path: state.base_path.to_string(),
        version: format!("{} - in food we trust", env!("CARGO_PKG_VERSION")),
        tr: Tr::new(lang),
        sync_enabled: cfg!(feature = "sync"),
        sync_logged_in,
        sync_email,
        sync_syncing,
        prefix: state.url_prefix.clone(),
        static_mode: false,
        features,
    }
}
```

- [ ] **Step 3.5 — Build to confirm it compiles cleanly**

```bash
cargo build -p cookcli 2>&1 | grep -E "^error"
```

Expected: no errors. If Askama reports a missing `features` field for a template, check that all 9 structs were updated.

- [ ] **Step 3.6 — Commit**

```bash
git add src/server/templates.rs src/server/builders.rs src/server/ui.rs src/build/renderer.rs
git commit -m "feat: thread FeatureFlags through all template structs, builders, and handlers"
```

---

## Task 4: Conditional nav links in `base.html`

**Files:**
- Modify: `templates/base.html`

- [ ] **Step 4.1 — Add dark mode CSS for `.nav-pill`**

In `templates/base.html`, inside the existing `<style>` block, after the existing `.dark .hover\:text-purple-700:hover` rule (around line 241), add:

```css
        /* Nav pill dark mode */
        .dark .nav-pill {
            color: #9ca3af;
        }

        .dark .nav-pill:hover {
            background: #374151;
            color: #fb923c;
        }

        .dark .nav-pill.active {
            background: linear-gradient(135deg, #ff6b35, #f97316);
            color: white;
        }
```

- [ ] **Step 4.2 — Add desktop nav pills inside the nav bar**

In `templates/base.html`, find the `order-3` div (around line 773):

```html
<div class="order-3 w-full md:w-auto flex items-center gap-1 lg:gap-2">
```

Insert the following block immediately after the opening `<div class="order-3 ...">` tag, before the existing preferences button:

```html
                        {% if features.show_shopping_list || features.show_pantry %}
                        <div class="hidden md:flex items-center mr-2 lg:mr-4">
                            <a href="{{ prefix }}{% if static_mode %}/index.html{% endif %}"
                               class="nav-pill {% if active == "recipes" %}active{% endif %}">
                                {{ tr.t("nav-recipes") }}
                            </a>
                            {% if features.show_shopping_list && !static_mode %}
                            <a href="{{ prefix }}/shopping-list"
                               class="nav-pill {% if active == "shopping" %}active{% endif %}">
                                {{ tr.t("nav-shopping-list") }}
                            </a>
                            {% endif %}
                            {% if features.show_pantry && !static_mode %}
                            <a href="{{ prefix }}/pantry"
                               class="nav-pill {% if active == "pantry" %}active{% endif %}">
                                {{ tr.t("nav-pantry") }}
                            </a>
                            {% endif %}
                        </div>
                        {% endif %}
```

- [ ] **Step 4.3 — Add nav links to the mobile overflow dropdown**

In `templates/base.html`, find the `more-dropdown` div (around line 800). Inside it, just before the existing `{% if !static_mode %}` preferences link, insert:

```html
                                {% if features.show_shopping_list || features.show_pantry %}
                                <a href="{{ prefix }}{% if static_mode %}/index.html{% endif %}" class="flex items-center gap-3 px-4 py-2.5 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors {% if active == "recipes" %}font-semibold text-orange-600{% endif %}">
                                    <span>🍳</span> <span>{{ tr.t("nav-recipes") }}</span>
                                </a>
                                {% if features.show_shopping_list && !static_mode %}
                                <a href="{{ prefix }}/shopping-list" class="flex items-center gap-3 px-4 py-2.5 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors {% if active == "shopping" %}font-semibold text-orange-600{% endif %}">
                                    <span>🛒</span> <span>{{ tr.t("nav-shopping-list") }}</span>
                                </a>
                                {% endif %}
                                {% if features.show_pantry && !static_mode %}
                                <a href="{{ prefix }}/pantry" class="flex items-center gap-3 px-4 py-2.5 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors {% if active == "pantry" %}font-semibold text-orange-600{% endif %}">
                                    <span>🥫</span> <span>{{ tr.t("nav-pantry") }}</span>
                                </a>
                                {% endif %}
                                <div class="border-t border-gray-100 dark:border-gray-700 my-1"></div>
                                {% endif %}
```

- [ ] **Step 4.4 — Build and verify no template errors**

```bash
cargo build -p cookcli 2>&1 | grep -E "^error"
```

Expected: no errors.

- [ ] **Step 4.5 — Commit**

```bash
git add templates/base.html
git commit -m "feat: add conditional nav pills for shopping list and pantry in base.html"
```

---

## Task 5: Features section in `preferences.html`

**Files:**
- Modify: `templates/preferences.html`

- [ ] **Step 5.1 — Add the Features section card**

In `templates/preferences.html`, find the Language Selector section (the `bg-gradient-to-r from-purple-50 to-pink-50` div). Add the Features section immediately after it, before the CookCloud sync section:

```html
        <!-- Features -->
        {% if !static_mode %}
        <div class="bg-gradient-to-r from-blue-50 to-indigo-50 p-6 rounded-2xl border-2 border-blue-200">
            <h2 class="text-lg font-semibold mb-2 text-blue-900">{{ tr.t("pref-features") }}</h2>
            <p class="text-sm text-blue-700 mb-4">{{ tr.t("pref-features-desc") }}</p>
            <div class="flex flex-wrap gap-3">
                <button onclick="toggleFeature('show_shopping_list', {{ features.show_shopping_list }})"
                        class="px-4 py-2 rounded-lg font-medium transition-all duration-200 border-2 {% if features.show_shopping_list %}bg-gradient-to-r from-orange-500 to-orange-600 text-white border-orange-600 shadow-lg scale-105{% else %}bg-white text-gray-700 border-gray-300 hover:border-orange-400 hover:bg-orange-50 hover:scale-105{% endif %}">
                    🛒 {{ tr.t("nav-shopping-list") }}
                </button>
                <button onclick="toggleFeature('show_pantry', {{ features.show_pantry }})"
                        class="px-4 py-2 rounded-lg font-medium transition-all duration-200 border-2 {% if features.show_pantry %}bg-gradient-to-r from-orange-500 to-orange-600 text-white border-orange-600 shadow-lg scale-105{% else %}bg-white text-gray-700 border-gray-300 hover:border-orange-400 hover:bg-orange-50 hover:scale-105{% endif %}">
                    🥫 {{ tr.t("nav-pantry") }}
                </button>
            </div>
        </div>
        {% endif %}
```

- [ ] **Step 5.2 — Add `toggleFeature` JS function**

In `templates/preferences.html`, inside the `{% block scripts %}<script>` block, add before the closing `</script>` tag (before the `{% if sync_enabled %}` block):

```js
    function toggleFeature(name, current) {
        const val = current ? '0' : '1';
        const maxAge = 365 * 24 * 60 * 60;
        document.cookie = `${name}=${val}; path={{ prefix }}/; max-age=${maxAge}; SameSite=Lax`;
        window.location.reload();
    }
```

- [ ] **Step 5.3 — Build to confirm no errors**

```bash
cargo build -p cookcli 2>&1 | grep -E "^error"
```

Expected: no errors.

- [ ] **Step 5.4 — Commit**

```bash
git add templates/preferences.html
git commit -m "feat: add Features section to preferences page with shopping list and pantry toggles"
```

---

## Task 6: Add translations to all 7 locale files

**Files:**
- Modify: `locales/en-US/preferences.ftl`
- Modify: `locales/de-DE/preferences.ftl`
- Modify: `locales/nl-NL/preferences.ftl`
- Modify: `locales/fr-FR/preferences.ftl`
- Modify: `locales/es-ES/preferences.ftl`
- Modify: `locales/eu-ES/preferences.ftl`
- Modify: `locales/sv-SE/preferences.ftl`

- [ ] **Step 6.1 — Add keys to `locales/en-US/preferences.ftl`**

Append to end of file:

```
# Features
pref-features = Features
pref-features-desc = Choose which features appear in the navigation bar.
```

- [ ] **Step 6.2 — Add keys to `locales/de-DE/preferences.ftl`**

Append to end of file:

```
# Features
pref-features = Funktionen
pref-features-desc = Wähle, welche Funktionen in der Navigationsleiste angezeigt werden.
```

- [ ] **Step 6.3 — Add keys to `locales/nl-NL/preferences.ftl`**

Append to end of file:

```
# Features
pref-features = Functies
pref-features-desc = Kies welke functies in de navigatiebalk worden weergegeven.
```

- [ ] **Step 6.4 — Add keys to `locales/fr-FR/preferences.ftl`**

Append to end of file:

```
# Features
pref-features = Fonctionnalités
pref-features-desc = Choisissez les fonctionnalités à afficher dans la barre de navigation.
```

- [ ] **Step 6.5 — Add keys to `locales/es-ES/preferences.ftl`**

Append to end of file:

```
# Features
pref-features = Funcionalidades
pref-features-desc = Elige qué funciones se muestran en la barra de navegación.
```

- [ ] **Step 6.6 — Add keys to `locales/eu-ES/preferences.ftl`**

Append to end of file:

```
# Features
pref-features = Eginbideak
pref-features-desc = Aukeratu nabigazio-barran zer eginbide erakutsi nahi duzun.
```

- [ ] **Step 6.7 — Add keys to `locales/sv-SE/preferences.ftl`**

Append to end of file:

```
# Features
pref-features = Funktioner
pref-features-desc = Välj vilka funktioner som visas i navigeringsfältet.
```

- [ ] **Step 6.8 — Build to confirm Fluent keys are valid**

```bash
cargo build -p cookcli 2>&1 | grep -E "^error"
```

Expected: no errors.

- [ ] **Step 6.9 — Commit**

```bash
git add locales/
git commit -m "feat: add pref-features translation keys to all 7 locales"
```

---

## Task 7: Manual smoke test before E2E tests

- [ ] **Step 7.1 — Build CSS**

```bash
cd /Users/romain/Projects/cooklang/cookcli && npm run build-css 2>&1 | tail -5
```

Expected: completes without error.

- [ ] **Step 7.2 — Start the dev server**

In a separate terminal (or background):

```bash
cargo run -- server ./seed 2>&1 &
sleep 3
```

- [ ] **Step 7.3 — Verify default nav shows Shopping List and Pantry**

Open `http://localhost:9080` in a browser. Expected: nav bar shows **Recipes**, **Shopping List**, **Pantry** pills.

- [ ] **Step 7.4 — Toggle off Shopping List**

Go to `http://localhost:9080/preferences`. Expected: Features section visible with two active (orange) buttons. Click **Shopping List** button. Page reloads. Expected: Shopping List button becomes inactive (white), nav no longer shows Shopping List pill.

- [ ] **Step 7.5 — Toggle off Pantry**

Click **Pantry** button. Page reloads. Expected: both buttons inactive, nav shows no feature pills (not even Recipes).

- [ ] **Step 7.6 — Toggle Shopping List back on**

Click **Shopping List** button. Page reloads. Expected: nav shows **Recipes** and **Shopping List** pills; Pantry still absent.

- [ ] **Step 7.7 — Verify cookie refresh**

Open browser DevTools → Application → Cookies. Expected: `show_shopping_list` and `show_pantry` cookies present with 1-year expiry.

---

## Task 8: E2E tests

**Files:**
- Modify: `tests/e2e/navigation.spec.ts`
- Modify: `tests/e2e/preferences.spec.ts`

- [ ] **Step 8.1 — Add feature flag nav tests to `navigation.spec.ts`**

Add the following test block at the end of `tests/e2e/navigation.spec.ts`:

```typescript
test.describe('Feature flag nav visibility', () => {
  test.beforeEach(async ({ context }) => {
    // Reset both feature flags to default (enabled) before each test
    await context.addCookies([
      { name: 'show_shopping_list', value: '1', url: 'http://localhost:9080' },
      { name: 'show_pantry', value: '1', url: 'http://localhost:9080' },
    ]);
  });

  test('shows Recipes, Shopping List, and Pantry nav links by default', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    await expect(page.locator('nav a.nav-pill', { hasText: /Recipes/i })).toBeVisible();
    await expect(page.locator('nav a.nav-pill', { hasText: /Shopping/i })).toBeVisible();
    await expect(page.locator('nav a.nav-pill', { hasText: /Pantry/i })).toBeVisible();
  });

  test('hides Shopping List nav link when cookie is 0', async ({ context, page }) => {
    await context.addCookies([
      { name: 'show_shopping_list', value: '0', url: 'http://localhost:9080' },
    ]);
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    await expect(page.locator('nav a.nav-pill', { hasText: /Shopping/i })).not.toBeVisible();
    await expect(page.locator('nav a.nav-pill', { hasText: /Recipes/i })).toBeVisible();
    await expect(page.locator('nav a.nav-pill', { hasText: /Pantry/i })).toBeVisible();
  });

  test('hides Pantry nav link when cookie is 0', async ({ context, page }) => {
    await context.addCookies([
      { name: 'show_pantry', value: '0', url: 'http://localhost:9080' },
    ]);
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    await expect(page.locator('nav a.nav-pill', { hasText: /Pantry/i })).not.toBeVisible();
    await expect(page.locator('nav a.nav-pill', { hasText: /Recipes/i })).toBeVisible();
    await expect(page.locator('nav a.nav-pill', { hasText: /Shopping/i })).toBeVisible();
  });

  test('shows no nav pills when both features are disabled', async ({ context, page }) => {
    await context.addCookies([
      { name: 'show_shopping_list', value: '0', url: 'http://localhost:9080' },
      { name: 'show_pantry', value: '0', url: 'http://localhost:9080' },
    ]);
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    await expect(page.locator('nav a.nav-pill')).toHaveCount(0);
  });

  test('nav pill is active on its corresponding page', async ({ page }) => {
    await page.goto('/shopping-list');
    await page.waitForLoadState('domcontentloaded');

    const shoppingPill = page.locator('nav a.nav-pill.active');
    await expect(shoppingPill).toBeVisible();
    await expect(shoppingPill).toContainText(/Shopping/i);
  });
});
```

- [ ] **Step 8.2 — Add feature toggle test to `preferences.spec.ts`**

Add the following test block at the end of `tests/e2e/preferences.spec.ts`:

```typescript
test.describe('Feature toggles in preferences', () => {
  test.beforeEach(async ({ context }) => {
    await context.addCookies([
      { name: 'show_shopping_list', value: '1', url: 'http://localhost:9080' },
      { name: 'show_pantry', value: '1', url: 'http://localhost:9080' },
    ]);
  });

  test('displays Features section with two active buttons', async ({ page }) => {
    await page.goto('/preferences');
    await page.waitForLoadState('networkidle');

    const shoppingBtn = page.getByRole('button', { name: /Shopping/i });
    const pantryBtn = page.getByRole('button', { name: /Pantry/i });

    await expect(shoppingBtn).toBeVisible();
    await expect(pantryBtn).toBeVisible();

    // Both enabled → should have the active (orange gradient) classes
    await expect(shoppingBtn).toHaveClass(/from-orange-500/);
    await expect(pantryBtn).toHaveClass(/from-orange-500/);
  });

  test('toggling Shopping List off removes it from nav', async ({ page }) => {
    await page.goto('/preferences');
    await page.waitForLoadState('networkidle');

    await page.getByRole('button', { name: /Shopping/i }).click();
    await page.waitForLoadState('networkidle');

    // Button is now inactive
    const shoppingBtn = page.getByRole('button', { name: /Shopping/i });
    await expect(shoppingBtn).not.toHaveClass(/from-orange-500/);

    // Nav no longer shows Shopping List
    await expect(page.locator('nav a.nav-pill', { hasText: /Shopping/i })).not.toBeVisible();
  });

  test('toggling a feature back on restores the nav link', async ({ context, page }) => {
    await context.addCookies([
      { name: 'show_shopping_list', value: '0', url: 'http://localhost:9080' },
    ]);
    await page.goto('/preferences');
    await page.waitForLoadState('networkidle');

    // Shopping is currently off — click to enable
    await page.getByRole('button', { name: /Shopping/i }).click();
    await page.waitForLoadState('networkidle');

    await expect(page.locator('nav a.nav-pill', { hasText: /Shopping/i })).toBeVisible();
  });
});
```

- [ ] **Step 8.3 — Run the new E2E tests**

```bash
cd /Users/romain/Projects/cooklang/cookcli
npx playwright test tests/e2e/navigation.spec.ts tests/e2e/preferences.spec.ts --reporter=line 2>&1 | tail -30
```

Expected: all new tests pass (existing tests continue to pass).

- [ ] **Step 8.4 — Run full test suite**

```bash
cargo test 2>&1 | tail -10
```

Expected: all Rust tests pass.

- [ ] **Step 8.5 — Commit**

```bash
git add tests/e2e/navigation.spec.ts tests/e2e/preferences.spec.ts
git commit -m "test: add E2E tests for feature flag nav visibility and preference toggles"
```
