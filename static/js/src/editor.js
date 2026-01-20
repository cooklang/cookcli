import { EditorState } from "@codemirror/state";
import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { syntaxHighlighting, HighlightStyle, bracketMatching } from "@codemirror/language";
import { searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import { linter } from "@codemirror/lint";
import { autocompletion, snippet } from "@codemirror/autocomplete";
import { tags as t } from "@lezer/highlight";
import { cooklang } from "./cooklang-mode.js";

// Diagnostics support
let currentDiagnostics = [];

export function setDiagnostics(view, diagnostics) {
    currentDiagnostics = diagnostics;
    // Trigger a state update to rerun the linter
    if (view) {
        view.dispatch({
            effects: []
        });
    }
}

// Linter that returns current diagnostics
const cooklangLinter = linter((view) => currentDiagnostics, { delay: 0 });

// Completion support
let completionResolver = null;

export function setCompletionResolver(resolver) {
    completionResolver = resolver;
}

// Async completion source for Cooklang
async function cooklangCompletions(context) {
    if (!completionResolver) return null;

    const pos = context.pos;
    const line = context.state.doc.lineAt(pos);
    const textBefore = line.text.slice(0, pos - line.from);

    // Check for trigger characters (@, #, ~)
    const match = textBefore.match(/[@#~]([a-zA-Z0-9_]*)$/);
    if (!match) return null;

    const prefix = match[1];
    const from = pos - prefix.length;
    const triggerChar = match[0][0];

    try {
        const items = await completionResolver(line.number - 1, pos - line.from);
        if (!items || items.length === 0) return null;

        return {
            from: from,
            options: items.map(item => {
                const type = item.kind === 6 ? 'variable' : item.kind === 14 ? 'keyword' : 'text';
                const text = item.insertText || item.label;

                // Check if LSP sent a snippet (insertTextFormat === 2)
                if (item.insertTextFormat === 2 && text.includes('$')) {
                    // Convert LSP snippet syntax to CodeMirror: $0 -> ${}, $1 -> ${1}
                    const cmSnippet = text.replace(/\$0/g, '${}').replace(/\$(\d+)/g, '${$1}');
                    return {
                        label: item.label,
                        type: type,
                        detail: item.detail || '',
                        apply: snippet(cmSnippet)
                    };
                }

                return {
                    label: item.label,
                    type: type,
                    detail: item.detail || '',
                    apply: text
                };
            })
        };
    } catch (e) {
        console.error('Completion error:', e);
        return null;
    }
}

// Custom highlight style for Cooklang syntax
const cooklangHighlightStyle = HighlightStyle.define([
  { tag: t.variableName, color: "#ea580c", fontWeight: "600" },  // Ingredients (orange)
  { tag: t.keyword, color: "#16a34a", fontWeight: "600" },       // Cookware (green)
  { tag: t.number, color: "#dc2626", fontWeight: "600" },        // Timers (red)
  { tag: t.comment, color: "#9ca3af", fontStyle: "italic" },     // Comments
  { tag: t.meta, color: "#8b5cf6" },                             // Metadata
  { tag: t.unit, color: "#6366f1" },                             // Units
  { tag: t.heading, color: "#0891b2", fontWeight: "700", fontSize: "1.1em" },  // Sections (cyan)
  { tag: t.string, color: "#d97706", fontStyle: "italic" }       // Prep instructions (amber italic)
]);

// Editor base theme for layout
const cooklangBaseTheme = EditorView.theme({
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
  }
});

// Cursor position callback
let cursorPositionCallback = null;

export function setCursorPositionCallback(callback) {
  cursorPositionCallback = callback;
}

// Initialize editor
export function initEditor(container, initialContent, onChange) {
  const updateListener = EditorView.updateListener.of((update) => {
    if (update.docChanged && onChange) {
      onChange(update.state.doc.toString());
    }
    // Always update cursor position on any update
    const pos = update.state.selection.main.head;
    const line = update.state.doc.lineAt(pos);
    const col = pos - line.from + 1;
    if (cursorPositionCallback) {
      cursorPositionCallback(line.number, col);
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
      syntaxHighlighting(cooklangHighlightStyle),
      cooklangBaseTheme,
      cooklangLinter,
      autocompletion({
        override: [cooklangCompletions],
        activateOnTyping: true
      }),
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

  // Set cursor to beginning of document (line 1, col 1)
  view.dispatch({
    selection: { anchor: 0 }
  });
  view.focus();

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
  setContent,
  setDiagnostics,
  setCompletionResolver,
  setCursorPositionCallback
};
