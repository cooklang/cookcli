import { EditorState } from "@codemirror/state";
import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { syntaxHighlighting, defaultHighlightStyle, bracketMatching } from "@codemirror/language";
import { searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import { linter } from "@codemirror/lint";
import { autocompletion } from "@codemirror/autocomplete";
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
            options: items.map(item => ({
                label: item.label,
                type: item.kind === 6 ? 'variable' : item.kind === 14 ? 'keyword' : 'text',
                detail: item.detail || '',
                apply: item.insertText || item.label
            }))
        };
    } catch (e) {
        console.error('Completion error:', e);
        return null;
    }
}

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
  setCompletionResolver
};
