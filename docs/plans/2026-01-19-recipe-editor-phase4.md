# Recipe Editor Phase 4: LSP Client & Full Features

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete the editor with LSP-powered autocomplete, diagnostics display, preview toggle, and new recipe creation.

**Architecture:** Connect CodeMirror to the LSP WebSocket with document sync, completions, and diagnostics.

**Tech Stack:** CodeMirror 6 autocomplete extension, LSP textDocument protocol

---

## Task 1: Add LSP document sync (didOpen/didChange)

**Files:**
- Modify: `templates/edit.html`

**Step 1: Add document URI tracking**

After `let lspMessageId = 1;`, add:
```javascript
let documentUri = null;
let documentVersion = 0;
```

**Step 2: Update initialize to send didOpen after initialized**

Replace the `handleLspMessage` function to track initialization and send didOpen:

```javascript
let lspInitialized = false;

function handleLspMessage(message) {
    if (message.id !== undefined && message.result !== undefined) {
        console.log('LSP response:', message);
        if (message.result && message.result.capabilities) {
            sendLspNotification('initialized', {});
            lspInitialized = true;
            // Open the document
            openDocument();
        }
    } else if (message.method) {
        console.log('LSP notification:', message.method);
        if (message.method === 'textDocument/publishDiagnostics') {
            handleDiagnostics(message.params);
        }
    }
}

function openDocument() {
    documentUri = 'file://' + recipePath;
    documentVersion = 1;
    sendLspNotification('textDocument/didOpen', {
        textDocument: {
            uri: documentUri,
            languageId: 'cooklang',
            version: documentVersion,
            text: window.CooklangEditor.getContent(editorView)
        }
    });
}
```

**Step 3: Send didChange on content changes**

Update the editor initialization to send didChange:
```javascript
document.addEventListener('DOMContentLoaded', function() {
    const container = document.getElementById('editor-container');
    editorView = window.CooklangEditor.initEditor(container, originalContent, function(newContent) {
        hasUnsavedChanges = newContent !== originalContent;
        updateSaveStatus();

        // Send didChange to LSP
        if (lspInitialized && documentUri) {
            documentVersion++;
            sendLspNotification('textDocument/didChange', {
                textDocument: {
                    uri: documentUri,
                    version: documentVersion
                },
                contentChanges: [{ text: newContent }]
            });
        }
    });
});
```

**Step 4: Commit**

```bash
git add templates/edit.html
git commit -m "feat(editor): add LSP document synchronization

Send textDocument/didOpen on connect and textDocument/didChange on edits.
This enables the LSP to track document state for completions and diagnostics."
```

---

## Task 2: Add diagnostics display

**Files:**
- Modify: `static/js/src/editor.js`
- Modify: `templates/edit.html`

**Step 1: Add linter extension to editor.js**

Update `static/js/src/editor.js` to export a function for setting diagnostics:

Add these imports at the top:
```javascript
import { linter, Diagnostic } from "@codemirror/lint";
```

Add a diagnostics state and update function:
```javascript
// Diagnostics support
let currentDiagnostics = [];
let diagnosticsCallback = null;

export function setDiagnostics(view, diagnostics) {
    currentDiagnostics = diagnostics;
    // Force linter to rerun
    if (view) {
        view.dispatch({});
    }
}

// Create linter that returns current diagnostics
const cooklangLinter = linter(() => currentDiagnostics);
```

Add `cooklangLinter` to the extensions array in `initEditor`.

Update `window.CooklangEditor`:
```javascript
window.CooklangEditor = {
    initEditor,
    getContent,
    setContent,
    setDiagnostics
};
```

**Step 2: Add @codemirror/lint dependency**

```bash
npm install --save-dev @codemirror/lint
```

**Step 3: Rebuild JS bundle**

```bash
npm run build-js
```

**Step 4: Add handleDiagnostics function in template**

In `templates/edit.html`, add the diagnostics handler:
```javascript
function handleDiagnostics(params) {
    if (!editorView) return;

    const diagnostics = (params.diagnostics || []).map(d => {
        // LSP positions are 0-based, CodeMirror uses character offsets
        const from = positionToOffset(d.range.start);
        const to = positionToOffset(d.range.end);

        return {
            from: from,
            to: to,
            severity: d.severity === 1 ? 'error' : 'warning',
            message: d.message
        };
    });

    window.CooklangEditor.setDiagnostics(editorView, diagnostics);

    // Update status bar with diagnostic count
    const errorCount = diagnostics.filter(d => d.severity === 'error').length;
    const warningCount = diagnostics.filter(d => d.severity === 'warning').length;
    updateDiagnosticsStatus(errorCount, warningCount);
}

function positionToOffset(position) {
    const doc = editorView.state.doc;
    const line = doc.line(position.line + 1); // LSP is 0-based, CM is 1-based
    return line.from + position.character;
}

function updateDiagnosticsStatus(errors, warnings) {
    // Could add to status bar
    console.log(`Diagnostics: ${errors} errors, ${warnings} warnings`);
}
```

**Step 5: Commit**

```bash
git add static/js/src/editor.js static/js/editor.bundle.js templates/edit.html package.json package-lock.json
git commit -m "feat(editor): display LSP diagnostics in editor

- Add @codemirror/lint for error display
- Convert LSP diagnostics to CodeMirror format
- Show error squiggles under problematic code"
```

---

## Task 3: Add autocomplete from LSP

**Files:**
- Modify: `static/js/src/editor.js`
- Modify: `templates/edit.html`

**Step 1: Add autocomplete imports to editor.js**

```javascript
import { autocompletion, CompletionContext } from "@codemirror/autocomplete";
```

**Step 2: Create completion source that calls back to template**

In editor.js, add:
```javascript
// Completion callback - set from template
let completionCallback = null;

export function setCompletionCallback(callback) {
    completionCallback = callback;
}

// Async completion source
async function cooklangCompletionSource(context) {
    if (!completionCallback) return null;

    const pos = context.pos;
    const line = context.state.doc.lineAt(pos);
    const lineText = line.text;
    const col = pos - line.from;

    // Check if we should trigger completion (after @, #, or ~)
    const beforeCursor = lineText.slice(0, col);
    const triggerMatch = beforeCursor.match(/[@#~](\w*)$/);
    if (!triggerMatch) return null;

    const from = pos - triggerMatch[1].length;

    try {
        const completions = await completionCallback(pos, line.number - 1, col);
        if (!completions || completions.length === 0) return null;

        return {
            from: from,
            options: completions.map(c => ({
                label: c.label,
                type: c.kind === 6 ? 'variable' : 'keyword', // 6 = Variable in LSP
                detail: c.detail
            }))
        };
    } catch (e) {
        console.error('Completion error:', e);
        return null;
    }
}
```

**Step 3: Add autocompletion to extensions**

In `initEditor`, add autocompletion to extensions:
```javascript
autocompletion({
    override: [cooklangCompletionSource],
    activateOnTyping: true
}),
```

**Step 4: Update window.CooklangEditor**

```javascript
window.CooklangEditor = {
    initEditor,
    getContent,
    setContent,
    setDiagnostics,
    setCompletionCallback
};
```

**Step 5: In template, implement completion callback**

```javascript
// Pending completion requests
let pendingCompletions = new Map();

// Set up completion callback
window.CooklangEditor.setCompletionCallback(async (offset, line, character) => {
    return new Promise((resolve) => {
        const id = sendLspRequest('textDocument/completion', {
            textDocument: { uri: documentUri },
            position: { line: line, character: character }
        });

        if (id) {
            pendingCompletions.set(id, resolve);
            // Timeout after 2 seconds
            setTimeout(() => {
                if (pendingCompletions.has(id)) {
                    pendingCompletions.delete(id);
                    resolve([]);
                }
            }, 2000);
        } else {
            resolve([]);
        }
    });
});

// Update handleLspMessage to handle completion responses
function handleLspMessage(message) {
    if (message.id !== undefined) {
        // Check if this is a completion response
        if (pendingCompletions.has(message.id)) {
            const resolve = pendingCompletions.get(message.id);
            pendingCompletions.delete(message.id);

            const items = message.result?.items || message.result || [];
            resolve(items);
            return;
        }

        // Other responses...
        if (message.result && message.result.capabilities) {
            sendLspNotification('initialized', {});
            lspInitialized = true;
            openDocument();
        }
    } else if (message.method) {
        if (message.method === 'textDocument/publishDiagnostics') {
            handleDiagnostics(message.params);
        }
    }
}
```

**Step 6: Rebuild and commit**

```bash
npm run build-js
git add static/js/src/editor.js static/js/editor.bundle.js templates/edit.html
git commit -m "feat(editor): add LSP-powered autocomplete

- Trigger completions after @, #, ~ characters
- Request completions from LSP via WebSocket
- Display completion popup with ingredient/cookware suggestions"
```

---

## Task 4: Add preview toggle

**Files:**
- Modify: `templates/edit.html`

**Step 1: Add Preview button to header**

After the Save button, add:
```html
<button onclick="togglePreview()" id="preview-btn" class="px-4 py-2 bg-gray-200 text-gray-700 rounded-lg hover:bg-gray-300 transition-colors flex items-center gap-2">
    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"></path>
    </svg>
    {{ tr.t("action-preview") }}
</button>
```

**Step 2: Add preview container (hidden by default)**

After editor-container div:
```html
<div id="preview-container" class="flex-1 bg-white rounded-2xl shadow-lg overflow-auto p-6 hidden"></div>
```

**Step 3: Add toggle JavaScript**

```javascript
let isPreviewMode = false;

async function togglePreview() {
    const editorContainer = document.getElementById('editor-container');
    const previewContainer = document.getElementById('preview-container');
    const previewBtn = document.getElementById('preview-btn');

    isPreviewMode = !isPreviewMode;

    if (isPreviewMode) {
        // Fetch rendered recipe
        try {
            const response = await fetch(`/recipe/${encodeURIComponent(recipePath)}`);
            const html = await response.text();

            // Extract just the recipe content from the full page
            const parser = new DOMParser();
            const doc = parser.parseFromString(html, 'text/html');
            const recipeContent = doc.querySelector('.recipe-content') || doc.querySelector('main');

            previewContainer.innerHTML = recipeContent ? recipeContent.innerHTML : html;
        } catch (e) {
            previewContainer.innerHTML = '<p class="text-red-500">Failed to load preview</p>';
        }

        editorContainer.classList.add('hidden');
        previewContainer.classList.remove('hidden');
        previewBtn.innerHTML = `
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"></path>
            </svg>
            {{ tr.t("action-edit") }}
        `;
    } else {
        editorContainer.classList.remove('hidden');
        previewContainer.classList.add('hidden');
        previewBtn.innerHTML = `
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"></path>
            </svg>
            {{ tr.t("action-preview") }}
        `;
    }
}
```

**Step 4: Add i18n key for preview**

Add to all locale files:
```ftl
action-preview = Preview
```

**Step 5: Commit**

```bash
git add templates/edit.html locales/
git commit -m "feat(editor): add preview toggle

Toggle between edit mode and rendered preview.
Fetches current recipe render from server."
```

---

## Task 5: Add new recipe creation (/new route)

**Files:**
- Modify: `src/server/ui.rs`
- Create: `templates/new.html`
- Modify: `src/server/templates.rs`
- Modify: `src/server/handlers/recipes.rs`

**Step 1: Add NewTemplate struct**

In `src/server/templates.rs`:
```rust
#[derive(Template)]
#[template(path = "new.html")]
pub struct NewTemplate {
    pub active: String,
    pub tr: Tr,
}
```

**Step 2: Create new.html template**

Create `templates/new.html` for creating new recipes with a filename input.

**Step 3: Add /new route in ui.rs**

```rust
.route("/new", get(new_page).post(create_recipe))
```

**Step 4: Add create_recipe handler**

In handlers/recipes.rs, add a POST handler that creates a new file.

**Step 5: Add i18n keys**

```ftl
new-recipe = New Recipe
new-recipe-filename = Filename
new-recipe-placeholder = my-recipe
```

**Step 6: Commit**

```bash
git add src/server/ui.rs src/server/templates.rs src/server/handlers/recipes.rs templates/new.html locales/
git commit -m "feat(editor): add new recipe creation page

- Add /new route for creating recipes
- Form to enter filename
- Creates .cook file and redirects to editor"
```

---

## Task 6: Add "New Recipe" button to recipes list

**Files:**
- Modify: `templates/recipes.html`

**Step 1: Add New Recipe button**

Add a button in the recipes list header that links to /new.

**Step 2: Commit**

```bash
git add templates/recipes.html
git commit -m "feat(ui): add New Recipe button to recipes list"
```

---

## Summary

Phase 4 completes the recipe editor with:
- **Task 1**: LSP document sync (didOpen/didChange)
- **Task 2**: Diagnostics display (error squiggles)
- **Task 3**: LSP-powered autocomplete
- **Task 4**: Preview toggle
- **Task 5**: New recipe creation
- **Task 6**: New Recipe button in list

After Phase 4, the editor feature is complete with full LSP integration.
