# Recipe Editor Phase 2: CodeMirror Integration

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the basic textarea editor with CodeMirror 6 featuring Cooklang syntax highlighting.

**Architecture:** Add esbuild for JS bundling, port the Cooklang language mode from cooklang-obsidian, integrate CodeMirror into the edit page.

**Tech Stack:** CodeMirror 6, esbuild, vanilla JavaScript

---

## Task 1: Add esbuild and CodeMirror dependencies

**Files:**
- Modify: `package.json`

**Step 1: Install dependencies**

Run:
```bash
npm install --save-dev esbuild @codemirror/state @codemirror/view @codemirror/commands @codemirror/language @codemirror/search
```

**Step 2: Verify package.json updated**

Run: `cat package.json`

Expected: Dependencies section includes @codemirror/* packages and esbuild.

**Step 3: Commit**

```bash
git add package.json package-lock.json
git commit -m "$(cat <<'EOF'
build: add CodeMirror 6 and esbuild dependencies
EOF
)"
```

---

## Task 2: Create Cooklang language mode

**Files:**
- Create: `static/js/src/cooklang-mode.js`

**Step 1: Create src directory**

Run: `mkdir -p static/js/src`

**Step 2: Write the language mode file**

Port the TypeScript mode from cooklang-obsidian to vanilla JS:

```javascript
import { StreamLanguage } from "@codemirror/language";

// Cooklang syntax highlighting mode for CodeMirror 6
// Ported from cooklang-obsidian/src/mode/cook/cook.ts
export const cooklang = StreamLanguage.define({
  name: "cooklang",

  startState() {
    return {
      formatting: false,
      nextMultiline: false,
      inMultiline: false,
      afterSection: false,
      position: null,
      inFrontmatter: false,
      inMetadata: false,
      inNote: false,
      inComment: false
    };
  },

  token(stream, state) {
    const sol = stream.sol() || state.afterSection;
    const eol = stream.eol();

    state.afterSection = false;

    if (sol) {
      if (state.nextMultiline) {
        state.inMultiline = true;
        state.nextMultiline = false;
      } else {
        state.position = null;
      }
    }

    if (eol && !state.nextMultiline) {
      state.inMultiline = false;
      state.position = null;
    }

    if (sol) {
      while (stream.eatSpace()) {}
    }

    // Frontmatter delimiters (---)
    if (sol && stream.match(/^---\s*$/)) {
      state.inFrontmatter = !state.inFrontmatter;
      return "meta";
    }

    // Inside frontmatter
    if (state.inFrontmatter) {
      stream.skipToEnd();
      return "meta";
    }

    // Line comments (-- comment)
    if (sol && stream.match(/^--/)) {
      stream.skipToEnd();
      return "comment";
    }

    // Block comments ([- comment -])
    if (stream.match(/^\[-/)) {
      state.inComment = true;
      return "comment";
    }

    if (state.inComment) {
      if (stream.match(/-]/)) {
        state.inComment = false;
        return "comment";
      }
      stream.skipToEnd();
      return "comment";
    }

    // Metadata (>> key: value)
    if (sol && stream.match(/^>>/)) {
      state.inMetadata = true;
      state.position = "metadata-key";
      return "meta";
    }

    if (state.inMetadata) {
      if (state.position === "metadata-key") {
        if (stream.match(/^[^:]+:/)) {
          state.position = "metadata-value";
          return "meta";
        }
        stream.skipToEnd();
        return "meta";
      } else if (state.position === "metadata-value") {
        stream.skipToEnd();
        return "meta";
      }
    }

    // Notes (lines starting with >)
    if (sol && stream.match(/^>/)) {
      state.inNote = true;
      return "comment";
    }

    if (state.inNote) {
      stream.skipToEnd();
      return "comment";
    }

    // Ingredients (@ingredient{amount})
    if (stream.match(/^@([^@#~]+?(?={))/)) {
      return "variableName";
    } else if (stream.match(/^@(.+?\b)/)) {
      return "variableName";
    }

    // Cookware (#cookware{amount})
    if (stream.match(/^#([^@#~]+?(?={))/)) {
      return "keyword";
    } else if (stream.match(/^#(.+?\b)/)) {
      return "keyword";
    }

    // Timers (~timer{amount})
    if (stream.match(/^~([^@#~]+?(?={))/)) {
      return "number";
    } else if (stream.match(/^~(.+?\b)/)) {
      return "number";
    }

    // Amounts in curly braces
    const ch = stream.next();
    if (!ch) return null;

    if (ch === '{') {
      if (state.position !== "timer") state.position = "measurement";
      return null;
    }

    if (ch === '}') {
      state.position = null;
      return null;
    }

    if (ch === '%' && (state.position === "measurement" || state.position === "timer")) {
      state.position = "unit";
      return null;
    }

    return state.position;
  }
});
```

**Step 3: Commit**

```bash
git add static/js/src/cooklang-mode.js
git commit -m "$(cat <<'EOF'
feat(editor): add Cooklang syntax mode for CodeMirror 6

Ported from cooklang-obsidian plugin. Highlights:
- Ingredients (@) as variables
- Cookware (#) as keywords
- Timers (~) as numbers
- Comments (-- and [- -])
- Metadata (>> key: value)
- Frontmatter (---) blocks
EOF
)"
```

---

## Task 3: Create editor entry point

**Files:**
- Create: `static/js/src/editor.js`

**Step 1: Write the editor entry point**

```javascript
import { EditorState } from "@codemirror/state";
import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { syntaxHighlighting, defaultHighlightStyle, bracketMatching } from "@codemirror/language";
import { searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import { cooklang } from "./cooklang-mode.js";

// Custom theme for Cooklang highlighting
const cooklangTheme = EditorView.theme({
  "&": {
    height: "100%",
    fontSize: "14px"
  },
  ".cm-scroller": {
    fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
    overflow: "auto"
  },
  ".cm-content": {
    padding: "1rem"
  },
  ".cm-line": {
    padding: "0 0.5rem"
  },
  // Ingredient highlighting (orange)
  ".cm-variableName": {
    color: "#ea580c",
    fontWeight: "600"
  },
  // Cookware highlighting (green)
  ".cm-keyword": {
    color: "#16a34a",
    fontWeight: "600"
  },
  // Timer highlighting (red)
  ".cm-number": {
    color: "#dc2626",
    fontWeight: "600"
  },
  // Measurements
  ".cm-measurement": {
    color: "#6366f1"
  },
  // Comments
  ".cm-comment": {
    color: "#9ca3af",
    fontStyle: "italic"
  },
  // Metadata
  ".cm-meta": {
    color: "#8b5cf6"
  }
});

// Initialize editor
export function initEditor(container, initialContent, onChange) {
  const updateListener = EditorView.updateListener.of((update) => {
    if (update.docChanged && onChange) {
      onChange(update.state.doc.toString());
    }
  });

  const state = EditorState.create({
    doc: initialContent,
    extensions: [
      lineNumbers(),
      highlightActiveLine(),
      highlightActiveLineGutter(),
      history(),
      bracketMatching(),
      highlightSelectionMatches(),
      cooklang,
      syntaxHighlighting(defaultHighlightStyle),
      cooklangTheme,
      keymap.of([
        ...defaultKeymap,
        ...historyKeymap,
        ...searchKeymap
      ]),
      updateListener,
      EditorView.lineWrapping
    ]
  });

  const view = new EditorView({
    state,
    parent: container
  });

  return view;
}

// Get editor content
export function getContent(view) {
  return view.state.doc.toString();
}

// Set editor content
export function setContent(view, content) {
  view.dispatch({
    changes: {
      from: 0,
      to: view.state.doc.length,
      insert: content
    }
  });
}

// Export for global access
window.CooklangEditor = {
  initEditor,
  getContent,
  setContent
};
```

**Step 2: Commit**

```bash
git add static/js/src/editor.js
git commit -m "$(cat <<'EOF'
feat(editor): add CodeMirror editor entry point

Sets up CodeMirror 6 with:
- Line numbers and active line highlight
- History (undo/redo)
- Bracket matching
- Search highlighting
- Custom Cooklang theme with colored syntax
- Line wrapping
- Change callback for unsaved changes tracking
EOF
)"
```

---

## Task 4: Add esbuild configuration and build script

**Files:**
- Modify: `package.json`

**Step 1: Add build-js script to package.json**

Add to scripts section:
```json
"build-js": "esbuild static/js/src/editor.js --bundle --outfile=static/js/editor.bundle.js --format=iife --minify",
"watch-js": "esbuild static/js/src/editor.js --bundle --outfile=static/js/editor.bundle.js --format=iife --watch"
```

**Step 2: Run the build**

Run: `npm run build-js`

Expected: Creates `static/js/editor.bundle.js`

**Step 3: Verify bundle created**

Run: `ls -la static/js/editor.bundle.js`

Expected: File exists with reasonable size (~100-200KB minified)

**Step 4: Commit**

```bash
git add package.json static/js/editor.bundle.js
git commit -m "$(cat <<'EOF'
build: add esbuild configuration for JS bundling

Bundles CodeMirror and Cooklang mode into single IIFE file.
EOF
)"
```

---

## Task 5: Update Makefile for combined builds

**Files:**
- Modify: `Makefile`

**Step 1: Read current Makefile**

Read the Makefile to understand existing targets.

**Step 2: Add js and combined targets**

Add these targets:
```makefile
js:
	npm run build-js

assets: css js

dev_assets:
	npm run watch-css & npm run watch-js
```

Update existing targets to include JS where appropriate.

**Step 3: Run combined build**

Run: `make assets`

Expected: Both CSS and JS build successfully.

**Step 4: Commit**

```bash
git add Makefile
git commit -m "$(cat <<'EOF'
build: add JS bundling to Makefile

- js: Build JavaScript bundle
- assets: Build both CSS and JS
- dev_assets: Watch both CSS and JS
EOF
)"
```

---

## Task 6: Update edit.html to use CodeMirror

**Files:**
- Modify: `templates/edit.html`

**Step 1: Read current template**

Read the current edit.html template.

**Step 2: Replace textarea with CodeMirror container**

Key changes:
1. Add script tag for editor bundle
2. Replace textarea with div container
3. Initialize CodeMirror on page load
4. Update save function to use CodeMirror API

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
    <div id="editor-container" class="flex-1 bg-white rounded-2xl shadow-lg overflow-hidden"></div>
</div>

<script src="/static/js/editor.bundle.js"></script>
<script>
const recipePath = {{ recipe_path|json }};
let originalContent = {{ content|json }};
let hasUnsavedChanges = false;
let editorView = null;

// Initialize CodeMirror editor
document.addEventListener('DOMContentLoaded', function() {
    const container = document.getElementById('editor-container');
    editorView = window.CooklangEditor.initEditor(container, originalContent, function(newContent) {
        hasUnsavedChanges = newContent !== originalContent;
        updateSaveStatus();
    });
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
    const content = window.CooklangEditor.getContent(editorView);
    const saveBtn = document.getElementById('save-btn');
    const status = document.getElementById('save-status');

    saveBtn.disabled = true;
    status.textContent = 'Saving...';
    status.className = 'text-sm text-gray-500';

    try {
        const response = await fetch(`/api/recipes/${encodeURIComponent(recipePath)}`, {
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

**Step 3: Run build to verify**

Run: `cargo build`

Expected: Template compiles without errors.

**Step 4: Commit**

```bash
git add templates/edit.html
git commit -m "$(cat <<'EOF'
feat(editor): integrate CodeMirror with syntax highlighting

Replace textarea with CodeMirror 6 editor featuring:
- Cooklang syntax highlighting (ingredients, cookware, timers)
- Line numbers
- Undo/redo history
- Search functionality
- Line wrapping
EOF
)"
```

---

## Task 7: Update .gitignore for build artifacts

**Files:**
- Modify: `.gitignore`

**Step 1: Check if bundle should be committed or ignored**

Decision: Commit the bundle so that `cargo build` works without npm. The bundle is small (~150KB) and stable.

No changes needed to .gitignore.

**Step 2: Commit (if any changes)**

Skip if no changes.

---

## Task 8: Manual testing

**Step 1: Start the server**

Run: `cargo run -- server ./seed`

**Step 2: Test editing workflow**

1. Navigate to any recipe
2. Click "Edit" button
3. Verify CodeMirror loads with syntax highlighting:
   - Ingredients (@) should be orange
   - Cookware (#) should be green
   - Timers (~) should be red
   - Comments should be gray italic
4. Type some text, verify "Unsaved changes" appears
5. Press Ctrl+S, verify save works
6. Navigate away, verify unsaved changes warning

**Step 3: Test menu editing**

1. Navigate to a menu file
2. Click "Edit"
3. Verify editing works for menu files too

---

## Summary

Phase 2 adds CodeMirror 6 with Cooklang syntax highlighting:
- **Task 1**: Install dependencies
- **Task 2**: Create language mode
- **Task 3**: Create editor entry point
- **Task 4**: Add build configuration
- **Task 5**: Update Makefile
- **Task 6**: Update template
- **Task 7**: Gitignore check
- **Task 8**: Manual testing

After Phase 2, the editor will have full syntax highlighting. Phase 3 will add the LSP WebSocket bridge for autocomplete and diagnostics.
