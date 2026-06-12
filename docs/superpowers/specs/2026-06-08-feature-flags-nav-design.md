# Feature Flags: Configurable Nav Bar

**Date:** 2026-06-08

## Problem

Shopping List and Pantry were removed from the nav bar in a previous commit. Users who want those features back have no way to re-enable them. Recipes is always the default/mandatory page.

## Goal

Let users choose which features appear in the nav bar via the Preferences page. Shopping List and Pantry are shown by default (opt-out). Recipes is always the home page and cannot be hidden. If only Recipes is enabled, no nav links are shown at all (the bar still shows search, preferences, and theme toggle).

## Approach: Cookie-based, server-side rendering

Feature flags are stored as cookies, read in middleware, and injected as an Axum extension — exactly mirroring how the `lang` cookie and `LanguageIdentifier` extension work today. Askama templates render conditionally; no JS is needed for the nav itself.

---

## Data & Storage

### `FeatureFlags` struct

New struct in `src/server/language.rs` (alongside the existing language helpers):

```rust
#[derive(Clone, Debug)]
pub struct FeatureFlags {
    pub show_shopping_list: bool,
    pub show_pantry: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self { show_shopping_list: true, show_pantry: true }
    }
}
```

### Cookies

| Cookie name          | Values  | Absent means |
|----------------------|---------|--------------|
| `show_shopping_list` | `1`/`0` | enabled (true) |
| `show_pantry`        | `1`/`0` | enabled (true) |

Set with `path=<prefix>/; max-age=31536000; SameSite=Lax` (1-year expiry).

The middleware refreshes both cookies on every response, resetting their expiry to 1 year from the current request. This means preferences persist indefinitely as long as the user visits occasionally — they only expire if the user hasn't visited for a full year.

### Middleware

`features_middleware` in `src/server/language.rs` (or inline into the existing `language_middleware`) parses the two cookies, calls `req.extensions_mut().insert(feature_flags)`, then after calling `next.run(req)`, appends two `Set-Cookie` headers to the response to refresh the expiry.

---

## Template Changes

### `templates.rs`

Add `pub features: FeatureFlags` to every template struct:
- `ErrorTemplate`, `RecipesTemplate`, `RecipeTemplate`, `MenuTemplate`
- `ShoppingListTemplate`, `PantryTemplate`, `PreferencesTemplate`
- `EditTemplate`, `NewTemplate`

### `builders.rs`

The builder input structs for recipes/recipe/menu gain a `features: FeatureFlags` field, which is passed through to the constructed template.

### `ui.rs`

Every handler extracts `Extension(features): Extension<FeatureFlags>` and passes it to the template.

---

## `base.html` — Nav links

Inside the existing nav `<div class="order-3 ...">`, before the preferences button, add:

```html
{% if features.show_shopping_list || features.show_pantry %}
  <!-- Recipes link (only shown when at least one other feature is enabled) -->
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
{% endif %}
```

The same conditional block is duplicated in the mobile overflow dropdown.

---

## `preferences.html` — Features section

New card added after the Language section:

```html
<div class="bg-gradient-to-r from-blue-50 to-indigo-50 p-6 rounded-2xl border-2 border-blue-200">
    <h2 class="text-lg font-semibold mb-2 text-blue-900">{{ tr.t("pref-features") }}</h2>
    <p class="text-sm text-blue-700 mb-4">{{ tr.t("pref-features-desc") }}</p>
    <div class="flex flex-wrap gap-3">
        <button onclick="toggleFeature('show_shopping_list', {{ features.show_shopping_list }})"
            class="px-4 py-2 rounded-lg font-medium transition-all duration-200 border-2 {% if features.show_shopping_list %}bg-gradient-to-r from-orange-500 to-orange-600 text-white border-orange-600 shadow-lg scale-105{% else %}bg-white text-gray-700 border-gray-300 hover:border-orange-400 hover:bg-orange-50 hover:scale-105{% endif %}">
            {{ tr.t("nav-shopping-list") }}
        </button>
        <button onclick="toggleFeature('show_pantry', {{ features.show_pantry }})"
            class="px-4 py-2 rounded-lg font-medium transition-all duration-200 border-2 {% if features.show_pantry %}bg-gradient-to-r from-orange-500 to-orange-600 text-white border-orange-600 shadow-lg scale-105{% else %}bg-white text-gray-700 border-gray-300 hover:border-orange-400 hover:bg-orange-50 hover:scale-105{% endif %}">
            {{ tr.t("nav-pantry") }}
        </button>
    </div>
</div>
```

JavaScript (same pattern as `setLanguage`):

```js
function toggleFeature(name, current) {
    const val = current ? '0' : '1';
    const maxAge = 365 * 24 * 60 * 60;
    document.cookie = `${name}=${val}; path={{ prefix }}/; max-age=${maxAge}; SameSite=Lax`;
    window.location.reload();
}
```

---

## Translations

Two new keys added to `preferences.ftl` in all 7 locales:

| Locale  | `pref-features`       | `pref-features-desc`                                              |
|---------|-----------------------|-------------------------------------------------------------------|
| en-US   | Features              | Choose which features appear in the navigation bar.               |
| de-DE   | Funktionen            | Wähle, welche Funktionen in der Navigationsleiste angezeigt werden. |
| nl-NL   | Functies              | Kies welke functies in de navigatiebalk worden weergegeven.       |
| fr-FR   | Fonctionnalités       | Choisissez les fonctionnalités à afficher dans la barre de navigation. |
| es-ES   | Funcionalidades       | Elige qué funciones se muestran en la barra de navegación.        |
| eu-ES   | Eginbideak            | Aukeratu nabigazio-barran zer eginbide erakutsi nahi duzun.       |
| sv-SE   | Funktioner            | Välj vilka funktioner som visas i navigeringsfältet.              |

Nav labels (`nav-shopping-list`, `nav-pantry`, `nav-recipes`) already exist in all locales — no changes needed there.

---

## File Checklist

| File | Change |
|------|--------|
| `src/server/language.rs` | Add `FeatureFlags` struct + `features_middleware` |
| `src/server/mod.rs` | Register `features_middleware` in the middleware stack |
| `src/server/templates.rs` | Add `features: FeatureFlags` to all template structs |
| `src/server/builders.rs` | Thread `FeatureFlags` through builder inputs/outputs |
| `src/server/ui.rs` | Extract `Extension(features)` in every handler |
| `templates/base.html` | Add conditional nav links |
| `templates/preferences.html` | Add Features section + `toggleFeature` JS |
| `locales/*/preferences.ftl` (×7) | Add `pref-features` and `pref-features-desc` |
