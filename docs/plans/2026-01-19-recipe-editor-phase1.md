# Recipe Editor Phase 1: Basic Editor Infrastructure

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add basic recipe editing capability with textarea editor, save/load functionality, and edit button on recipe pages.

**Architecture:** New `/edit/{path}` UI route serves editor template. New API endpoints `/api/recipe/{path}/raw` (GET) and `/api/recipe/{path}` (PUT) handle file read/write. Path validation prevents directory traversal. Atomic writes prevent corruption.

**Tech Stack:** Rust/Axum backend, Askama templates, existing Tailwind CSS styling.

---

## Task 1: Add Raw Recipe API Endpoint (GET)

**Files:**
- Modify: `src/server/handlers/recipes.rs`
- Modify: `src/server/handlers/mod.rs`
- Modify: `src/server/mod.rs`

**Step 1: Add `recipe_raw` handler function**

In `src/server/handlers/recipes.rs`, add after the `recipe` function (around line 128):

```rust
pub async fn recipe_raw(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<String, StatusCode> {
    check_path(&path)?;

    let recipe_path = state.base_path.join(&path);

    // Try .cook extension first, then .menu
    let file_path = if recipe_path.exists() {
        recipe_path
    } else {
        let cook_path = Utf8PathBuf::from(format!("{}.cook", recipe_path));
        let menu_path = Utf8PathBuf::from(format!("{}.menu", recipe_path));

        if cook_path.exists() {
            cook_path
        } else if menu_path.exists() {
            menu_path
        } else {
            tracing::error!("Recipe file not found: {path}");
            return Err(StatusCode::NOT_FOUND);
        }
    };

    std::fs::read_to_string(&file_path).map_err(|e| {
        tracing::error!("Failed to read recipe file {}: {}", file_path, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}
```

**Step 2: Export in handlers/mod.rs**

In `src/server/handlers/mod.rs`, update the `recipes` re-export line:

```rust
pub use recipes::{all_recipes, recipe, recipe_raw, reload, search};
```

**Step 3: Add route in server/mod.rs**

In `src/server/mod.rs`, in the `api` function (around line 248), add after the `/recipes/*path` route:

```rust
        .route("/recipe/*path/raw", get(handlers::recipe_raw))
```

**Step 4: Verify it compiles**

Run: `cargo build`
Expected: Build succeeds

**Step 5: Manual test**

Run: `cargo run -- server ./seed`
Then in another terminal: `curl http://localhost:9080/api/recipe/Neapolitan%20Pizza/raw`
Expected: Raw content of the recipe file

**Step 6: Commit**

```bash
git add src/server/handlers/recipes.rs src/server/handlers/mod.rs src/server/mod.rs
git commit -m "$(cat <<'EOF'
feat(api): add GET /api/recipe/{path}/raw endpoint

Returns raw file content for recipe editing
EOF
)"
```

---

## Task 2: Add Recipe Save API Endpoint (PUT)

**Files:**
- Modify: `src/server/handlers/recipes.rs`
- Modify: `src/server/handlers/mod.rs`
- Modify: `src/server/mod.rs`

**Step 1: Add imports for atomic write**

In `src/server/handlers/recipes.rs`, add to the imports at the top:

```rust
use std::io::Write;
```

**Step 2: Add `recipe_save` handler function**

In `src/server/handlers/recipes.rs`, add after the `recipe_raw` function:

```rust
pub async fn recipe_save(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
    body: String,
) -> Result<Json<serde_json::Value>, StatusCode> {
    check_path(&path)?;

    let recipe_path = state.base_path.join(&path);

    // Determine actual file path (with extension)
    let file_path = if recipe_path.exists() {
        recipe_path
    } else {
        let cook_path = Utf8PathBuf::from(format!("{}.cook", recipe_path));
        let menu_path = Utf8PathBuf::from(format!("{}.menu", recipe_path));

        if cook_path.exists() {
            cook_path
        } else if menu_path.exists() {
            menu_path
        } else {
            // Default to .cook for new files
            Utf8PathBuf::from(format!("{}.cook", recipe_path))
        }
    };

    // Atomic write: write to temp file, then rename
    let temp_path = file_path.with_extension("tmp");

    let mut temp_file = std::fs::File::create(&temp_path).map_err(|e| {
        tracing::error!("Failed to create temp file {}: {}", temp_path, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    temp_file.write_all(body.as_bytes()).map_err(|e| {
        tracing::error!("Failed to write to temp file {}: {}", temp_path, e);
        let _ = std::fs::remove_file(&temp_path);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    std::fs::rename(&temp_path, &file_path).map_err(|e| {
        tracing::error!("Failed to rename temp file to {}: {}", file_path, e);
        let _ = std::fs::remove_file(&temp_path);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("Saved recipe: {}", file_path);

    Ok(Json(serde_json::json!({
        "status": "success",
        "path": path
    })))
}
```

**Step 3: Export in handlers/mod.rs**

In `src/server/handlers/mod.rs`, update the `recipes` re-export:

```rust
pub use recipes::{all_recipes, recipe, recipe_raw, recipe_save, reload, search};
```

**Step 4: Add route in server/mod.rs**

In `src/server/mod.rs`, in the `api` function, add PUT route (use `axum::routing::put`):

```rust
        .route(
            "/recipe/*path",
            get(handlers::recipe).put(handlers::recipe_save),
        )
```

Note: This replaces the existing `.route("/recipes/*path", get(handlers::recipe))` line.

**Step 5: Update CORS layer for PUT method**

In `src/server/mod.rs`, update the CORS layer (around line 139):

```rust
                .allow_methods([Method::GET, Method::POST, Method::PUT]),
```

**Step 6: Verify it compiles**

Run: `cargo build`
Expected: Build succeeds

**Step 7: Manual test**

Run: `cargo run -- server ./seed`
Then test save:
```bash
curl -X PUT http://localhost:9080/api/recipe/Neapolitan%20Pizza \
  -H "Content-Type: text/plain" \
  -d "---
servings: 8
---
Test content"
```
Expected: `{"status":"success","path":"Neapolitan Pizza"}`

**Step 8: Restore test file**

```bash
git checkout seed/Neapolitan\ Pizza.cook
```

**Step 9: Commit**

```bash
git add src/server/handlers/recipes.rs src/server/handlers/mod.rs src/server/mod.rs
git commit -m "$(cat <<'EOF'
feat(api): add PUT /api/recipe/{path} endpoint

Saves recipe content with atomic write (temp file + rename)
EOF
)"
```

---

## Task 3: Add Editor Template

**Files:**
- Create: `templates/edit.html`
- Modify: `src/server/templates.rs`

**Step 1: Create editor template**

Create `templates/edit.html`:

```html
{% extends "base.html" %}

{% block title %}Edit: {{ recipe_name }} - Cook{% endblock %}

{% block content %}
<div class="flex flex-col h-[calc(100vh-12rem)]">
    <!-- Header bar -->
    <div class="flex items-center justify-between mb-4">
        <div class="flex items-center gap-4">
            <a href="/recipe/{{ recipe_path }}" class="px-4 py-2 bg-gray-200 text-gray-700 rounded-lg hover:bg-gray-300 transition-colors flex items-center gap-2">
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18"></path>
                </svg>
                {{ tr.t("action-cancel") }}
            </a>
            <h1 class="text-2xl font-bold text-gray-800">{{ recipe_name }}</h1>
        </div>
        <div class="flex items-center gap-3">
            <span id="save-status" class="text-sm text-gray-500"></span>
            <button onclick="saveRecipe()" id="save-btn" class="px-6 py-2 bg-gradient-to-r from-green-500 to-emerald-500 text-white rounded-lg hover:from-green-600 hover:to-emerald-600 transition-all shadow-md flex items-center gap-2">
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                </svg>
                {{ tr.t("action-save") }}
            </button>
        </div>
    </div>

    <!-- Editor area -->
    <div class="flex-1 bg-white rounded-2xl shadow-lg overflow-hidden">
        <textarea
            id="editor"
            class="w-full h-full p-6 font-mono text-sm resize-none focus:outline-none"
            spellcheck="false"
            placeholder="Enter your recipe here...">{{ content }}</textarea>
    </div>
</div>

<script>
const recipePath = '{{ recipe_path }}';
let originalContent = document.getElementById('editor').value;
let hasUnsavedChanges = false;

// Track changes
document.getElementById('editor').addEventListener('input', function() {
    hasUnsavedChanges = this.value !== originalContent;
    updateSaveStatus();
});

function updateSaveStatus() {
    const status = document.getElementById('save-status');
    if (hasUnsavedChanges) {
        status.textContent = 'Unsaved changes';
        status.className = 'text-sm text-orange-500';
    } else {
        status.textContent = 'Saved';
        status.className = 'text-sm text-green-500';
    }
}

async function saveRecipe() {
    const content = document.getElementById('editor').value;
    const saveBtn = document.getElementById('save-btn');
    const status = document.getElementById('save-status');

    saveBtn.disabled = true;
    status.textContent = 'Saving...';
    status.className = 'text-sm text-gray-500';

    try {
        const response = await fetch(`/api/recipe/${encodeURIComponent(recipePath)}`, {
            method: 'PUT',
            headers: {
                'Content-Type': 'text/plain',
            },
            body: content
        });

        if (response.ok) {
            originalContent = content;
            hasUnsavedChanges = false;
            status.textContent = 'Saved';
            status.className = 'text-sm text-green-500';
        } else {
            const error = await response.text();
            status.textContent = 'Save failed';
            status.className = 'text-sm text-red-500';
            alert('Failed to save: ' + error);
        }
    } catch (error) {
        status.textContent = 'Save failed';
        status.className = 'text-sm text-red-500';
        alert('Failed to save: ' + error.message);
    } finally {
        saveBtn.disabled = false;
    }
}

// Keyboard shortcut: Ctrl+S / Cmd+S
document.addEventListener('keydown', function(e) {
    if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault();
        saveRecipe();
    }
});

// Warn before leaving with unsaved changes
window.addEventListener('beforeunload', function(e) {
    if (hasUnsavedChanges) {
        e.preventDefault();
        e.returnValue = '';
    }
});

// Initial status
updateSaveStatus();
</script>
{% endblock %}
```

**Step 2: Add EditTemplate struct**

In `src/server/templates.rs`, add after `PantryTemplate` (around line 104):

```rust
#[derive(Template)]
#[template(path = "edit.html")]
pub struct EditTemplate {
    pub active: String,
    pub recipe_name: String,
    pub recipe_path: String,
    pub content: String,
    pub tr: Tr,
}
```

**Step 3: Verify it compiles**

Run: `cargo build`
Expected: Build succeeds

**Step 4: Commit**

```bash
git add templates/edit.html src/server/templates.rs
git commit -m "$(cat <<'EOF'
feat(ui): add editor template with textarea

Basic editor with save button, keyboard shortcut (Ctrl+S),
and unsaved changes warning
EOF
)"
```

---

## Task 4: Add Editor UI Route

**Files:**
- Modify: `src/server/ui.rs`

**Step 1: Add edit route to router**

In `src/server/ui.rs`, in the `ui()` function (around line 15), add the edit route:

```rust
pub fn ui() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(recipes_page))
        .route("/directory/*path", get(recipes_directory))
        .route("/recipe/*path", get(recipe_page))
        .route("/edit/*path", get(edit_page))
        .route("/shopping-list", get(shopping_list_page))
        .route("/pantry", get(pantry_page))
        .route("/preferences", get(preferences_page))
}
```

**Step 2: Add edit_page handler**

In `src/server/ui.rs`, add the handler function (after `recipe_page` function, around line 643):

```rust
async fn edit_page(
    Path(path): Path<String>,
    State(state): State<Arc<AppState>>,
    Extension(lang): Extension<LanguageIdentifier>,
) -> Result<impl askama_axum::IntoResponse, StatusCode> {
    let recipe_path = Utf8PathBuf::from(&path);

    // Find the actual file
    let entry = cooklang_find::get_recipe(vec![&state.base_path], &recipe_path).map_err(|_| {
        tracing::error!("Recipe not found: {path}");
        StatusCode::NOT_FOUND
    })?;

    let file_path = entry.path().ok_or_else(|| {
        tracing::error!("Recipe has no file path: {path}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Read raw content
    let content = std::fs::read_to_string(file_path).map_err(|e| {
        tracing::error!("Failed to read recipe file: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Get recipe name from path
    let recipe_name = path
        .split('/')
        .next_back()
        .unwrap_or(&path)
        .replace(".cook", "")
        .replace(".menu", "");

    let template = crate::server::templates::EditTemplate {
        active: "recipes".to_string(),
        recipe_name,
        recipe_path: path,
        content,
        tr: crate::server::templates::Tr::new(lang),
    };

    Ok(template)
}
```

**Step 3: Verify it compiles**

Run: `cargo build`
Expected: Build succeeds

**Step 4: Manual test**

Run: `cargo run -- server ./seed`
Navigate to: `http://localhost:9080/edit/Neapolitan%20Pizza`
Expected: Editor page with recipe content in textarea

**Step 5: Commit**

```bash
git add src/server/ui.rs
git commit -m "$(cat <<'EOF'
feat(ui): add /edit/{path} route for recipe editor

Serves editor template with raw recipe content
EOF
)"
```

---

## Task 5: Add Edit Button to Recipe Page

**Files:**
- Modify: `templates/recipe.html`

**Step 1: Add edit button to recipe header**

In `templates/recipe.html`, find the button row (around line 53-66) and add an Edit button before the shopping list button:

```html
                <a href="/edit/{{ recipe_path }}"
                   class="px-4 py-2 bg-gradient-to-r from-blue-500 to-cyan-500 text-white rounded-lg hover:from-blue-600 hover:to-cyan-600 transition-all shadow-md flex items-center gap-2">
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"></path>
                    </svg>
                    {{ tr.t("action-edit") }}
                </a>
```

Insert this after the scale input div and before the "Add to Shopping List" button.

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: Build succeeds

**Step 3: Manual test**

Run: `cargo run -- server ./seed`
Navigate to: `http://localhost:9080/recipe/Neapolitan%20Pizza`
Expected: Edit button appears in header, clicking it goes to editor

**Step 4: Test full flow**

1. View recipe at `/recipe/Neapolitan%20Pizza`
2. Click Edit button
3. Make a change in editor
4. Press Ctrl+S or click Save
5. Click Cancel to return to recipe view
6. Verify change persists

**Step 5: Restore test file**

```bash
git checkout seed/Neapolitan\ Pizza.cook
```

**Step 6: Commit**

```bash
git add templates/recipe.html
git commit -m "$(cat <<'EOF'
feat(ui): add Edit button to recipe detail page

Links to /edit/{path} for editing recipe content
EOF
)"
```

---

## Task 6: Add Edit Button to Menu Page

**Files:**
- Modify: `templates/menu.html`

**Step 1: Add edit button to menu header**

In `templates/menu.html`, find the button row (around line 52-59) and add an Edit button before the "Add to Shopping List" button. Insert this after the scale input `</div>` (around line 52):

```html
                <a href="/edit/{{ recipe_path }}"
                   class="px-4 py-2 bg-gradient-to-r from-blue-500 to-cyan-500 text-white rounded-lg hover:from-blue-600 hover:to-cyan-600 transition-all shadow-md flex items-center gap-2">
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"></path>
                    </svg>
                    {{ tr.t("action-edit") }}
                </a>
```

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: Build succeeds

**Step 3: Manual test**

Run: `cargo run -- server ./seed`
Navigate to: `http://localhost:9080/recipe/2%20Day%20Plan`
Expected: Edit button appears, clicking it goes to editor

**Step 4: Commit**

```bash
git add templates/menu.html
git commit -m "$(cat <<'EOF'
feat(ui): add Edit button to menu page

Allows editing menu files from the menu view
EOF
)"
```

---

## Task 7: Add i18n Keys for Editor

**Files:**
- Modify: `locales/en-US/common.ftl`
- Modify: `locales/de-DE/common.ftl`
- Modify: `locales/es-ES/common.ftl`
- Modify: `locales/fr-FR/common.ftl`
- Modify: `locales/nl-NL/common.ftl`

**Step 1: Add editor keys to en-US**

In `locales/en-US/common.ftl`, add at the end:

```
# Editor
editor-unsaved-changes = Unsaved changes
editor-saved = Saved
editor-saving = Saving...
editor-save-failed = Save failed
editor-placeholder = Enter your recipe here...
```

**Step 2: Add editor keys to other locales**

Add equivalent translations (or English fallback) to each locale file.

de-DE:
```
# Editor
editor-unsaved-changes = Ungespeicherte Änderungen
editor-saved = Gespeichert
editor-saving = Speichern...
editor-save-failed = Speichern fehlgeschlagen
editor-placeholder = Geben Sie hier Ihr Rezept ein...
```

es-ES:
```
# Editor
editor-unsaved-changes = Cambios sin guardar
editor-saved = Guardado
editor-saving = Guardando...
editor-save-failed = Error al guardar
editor-placeholder = Escribe tu receta aquí...
```

fr-FR:
```
# Editor
editor-unsaved-changes = Modifications non enregistrées
editor-saved = Enregistré
editor-saving = Enregistrement...
editor-save-failed = Échec de l'enregistrement
editor-placeholder = Entrez votre recette ici...
```

nl-NL:
```
# Editor
editor-unsaved-changes = Niet-opgeslagen wijzigingen
editor-saved = Opgeslagen
editor-saving = Opslaan...
editor-save-failed = Opslaan mislukt
editor-placeholder = Voer hier uw recept in...
```

**Step 3: Verify it compiles**

Run: `cargo build`
Expected: Build succeeds

**Step 4: Commit**

```bash
git add locales/
git commit -m "$(cat <<'EOF'
feat(i18n): add editor translation keys

Translations for editor UI in all supported languages
EOF
)"
```

---

## Summary

After completing all tasks, you will have:

1. **GET /api/recipe/{path}/raw** - Returns raw recipe file content
2. **PUT /api/recipe/{path}** - Saves recipe content atomically
3. **GET /edit/{path}** - Editor page with textarea
4. **Edit button** on both recipe and menu pages
5. **i18n support** for all editor strings

The editor supports:
- Ctrl+S / Cmd+S keyboard shortcut
- Unsaved changes warning
- Basic save status indicator

**Next Phase:** CodeMirror integration for syntax highlighting
