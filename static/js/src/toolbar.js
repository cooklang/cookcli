import { EditorSelection } from "@codemirror/state";
import { isolateHistory } from "@codemirror/commands";

// Editing helpers behind the toolbar above the recipe editor. Every helper
// dispatches exactly one transaction, isolated in the undo history so each
// toolbar action is its own undo step even when clicked in quick succession,
// then hands focus back to the editor.

function dispatch(view, spec) {
  view.dispatch({
    ...spec,
    annotations: isolateHistory.of("full"),
    scrollIntoView: true
  });
  view.focus();
}

// Split a selection into leading whitespace, the text itself and trailing
// whitespace, so markup wraps the words and leaves the spacing outside.
function trimRange(state, range) {
  const text = state.sliceDoc(range.from, range.to);
  const lead = text.length - text.trimStart().length;
  const trimmed = text.trim();
  return { from: range.from + lead, to: range.from + lead + trimmed.length, text: trimmed };
}

// Wrap each selection in `open` + text + `close`.
// - With a selection, the cursor lands `cursorInClose` characters into `close`.
// - Without one, `open + close` is inserted and the cursor lands `emptyCursor`
//   characters into the inserted text.
export function wrapSelection(view, open, close, { cursorInClose = 0, emptyCursor = open.length } = {}) {
  const { state } = view;
  dispatch(view, state.changeByRange(range => {
    const sel = trimRange(state, range);
    if (!sel.text) {
      const at = range.head;
      return {
        changes: { from: at, insert: open + close },
        range: EditorSelection.cursor(at + emptyCursor)
      };
    }
    const insert = open + sel.text + close;
    return {
      changes: { from: sel.from, to: sel.to, insert },
      range: EditorSelection.cursor(sel.from + open.length + sel.text.length + cursorInClose)
    };
  }));
}

// Lines touched by the selection. A range ending at the very start of a line
// does not include that line, matching CodeMirror's own line commands.
function selectedLines(state) {
  const lines = new Map();
  for (const range of state.selection.ranges) {
    const first = state.doc.lineAt(range.from).number;
    let last = state.doc.lineAt(range.to).number;
    if (range.to > range.from && state.doc.line(last).from === range.to) last--;
    for (let n = first; n <= Math.max(first, last); n++) {
      lines.set(n, state.doc.line(n));
    }
  }
  return [...lines.values()];
}

// Toggle `prefix` at the start of every selected line. `pattern` recognises an
// existing prefix (for instance `-- ` or a bare `--`). When every non-empty
// line already carries it, it is removed; otherwise it is added where missing.
// Blank lines are skipped unless they are all there is.
export function prefixLines(view, prefix, pattern) {
  const { state } = view;
  const lines = selectedLines(state);
  const content = lines.filter(line => line.text.trim() !== "");
  const targets = content.length ? content : lines;
  const remove = content.length > 0 && content.every(line => pattern.test(line.text));

  const changes = state.changes(targets.flatMap(line => {
    const match = line.text.match(pattern);
    if (remove) return [{ from: line.from, to: line.from + match[0].length }];
    return match ? [] : [{ from: line.from, insert: prefix }];
  }));

  dispatch(view, { changes, selection: state.selection.map(changes, 1) });
}

// Blank-line separators needed so a block inserted at [from, to) stands in a
// paragraph of its own.
function blockSeparators(state, from, to) {
  const before = state.sliceDoc(Math.max(0, from - 2), from);
  const after = state.sliceDoc(to, to + 2);
  const lead = from === 0 || before === "\n\n" ? "" : before.endsWith("\n") ? "\n" : "\n\n";
  const trail = to === state.doc.length || after === "\n\n" ? "" : after.startsWith("\n") ? "\n" : "\n\n";
  return { lead, trail };
}

// Insert `text` as a block of its own, separated from its neighbours by blank
// lines. The block replaces the main selection; whitespace-only leftovers on
// the lines around it are dropped. `select` is a [from, to] pair relative to
// `text` for the selection afterwards (defaults to the end of the block).
export function insertBlock(view, text, { select = [text.length, text.length] } = {}) {
  const { state } = view;
  const range = state.selection.main;
  let from = range.from;
  let to = range.to;
  const startLine = state.doc.lineAt(from);
  const endLine = state.doc.lineAt(to);
  if (state.sliceDoc(startLine.from, from).trim() === "") from = startLine.from;
  if (state.sliceDoc(to, endLine.to).trim() === "") to = endLine.to;
  // Keep spaces that sat between the block and the text around it out of it.
  while (from > startLine.from && state.sliceDoc(from - 1, from) === " ") from--;
  while (to < endLine.to && state.sliceDoc(to, to + 1) === " ") to++;

  const { lead, trail } = blockSeparators(state, from, to);
  const start = from + lead.length;
  dispatch(view, {
    changes: { from, to, insert: lead + text + trail },
    selection: EditorSelection.single(start + select[0], start + select[1])
  });
}

// Locate a YAML frontmatter block: a first line of `---` closed by another
// `---` line. Returns the line numbers and the body offsets, or null.
export function findFrontmatter(state) {
  const { doc } = state;
  if (doc.lines < 2 || doc.line(1).text.trimEnd() !== "---") return null;
  for (let n = 2; n <= doc.lines; n++) {
    const line = doc.line(n);
    if (line.text.trimEnd() === "---") {
      return {
        openLine: 1,
        closeLine: n,
        bodyFrom: doc.line(2).from,
        bodyTo: line.from
      };
    }
  }
  return null;
}

// Without frontmatter, add one with an empty title and put the cursor on it.
// With frontmatter, open a new line just before the closing `---`.
export function ensureFrontmatter(view) {
  const { state } = view;
  const frontmatter = findFrontmatter(state);
  if (frontmatter) {
    const at = frontmatter.bodyTo;
    dispatch(view, {
      changes: { from: at, insert: "\n" },
      selection: EditorSelection.cursor(at)
    });
    return;
  }
  const head = "---\ntitle: ";
  const separator = state.doc.length && state.doc.line(1).text.trim() !== "" ? "\n" : "";
  dispatch(view, {
    changes: { from: 0, insert: head + "\n---\n" + separator },
    selection: EditorSelection.cursor(head.length)
  });
}

// A selection that sits inside one line without covering all of it.
function isInlineSelection(state, range) {
  if (range.empty) return false;
  const line = state.doc.lineAt(range.from);
  if (range.to > line.to) return false;
  return state.sliceDoc(range.from, range.to).trim() !== line.text.trim();
}

const BLOCK_COMMENT = /^\[-\s?([\s\S]*?)\s?-\]$/;

function toggleBlockComment(view) {
  const { state } = view;
  dispatch(view, state.changeByRange(range => {
    const sel = trimRange(state, range);
    const match = sel.text.match(BLOCK_COMMENT);
    const insert = match ? match[1] : `[- ${sel.text} -]`;
    return {
      changes: { from: sel.from, to: sel.to, insert },
      range: EditorSelection.range(sel.from, sel.from + insert.length)
    };
  }));
}

// Toolbar actions for recipe files, keyed by the buttons' `data-action`.
// Each receives the editor view and the button that triggered it.
export const recipeActions = {
  ingredient: view => wrapSelection(view, "@", "{}", { cursorInClose: 1 }),
  cookware: view => wrapSelection(view, "#", "{}", { cursorInClose: 1 }),
  timer: view => wrapSelection(view, "~", "{%minutes}", { cursorInClose: 1, emptyCursor: 2 }),
  section: (view, button) => {
    const selected = view.state.sliceDoc(view.state.selection.main.from, view.state.selection.main.to);
    const name = selected.replace(/\s+/g, " ").trim() || button?.dataset.default || "Section";
    insertBlock(view, `== ${name} ==`, { select: [3, 3 + name.length] });
  },
  // `>>` is legacy metadata, not a note, so it doesn't count as a prefix.
  note: view => prefixLines(view, "> ", /^>(?!>) ?/),
  comment: view => {
    if (isInlineSelection(view.state, view.state.selection.main)) {
      toggleBlockComment(view);
    } else {
      prefixLines(view, "-- ", /^-- ?/);
    }
  },
  metadata: view => ensureFrontmatter(view)
};

// Wire a `role="toolbar"` element to an editor view: clicks on `[data-action]`
// run the matching entry of `actions`, and the ARIA toolbar keyboard pattern
// (one tab stop, arrow keys / Home / End between items) applies to every
// button, select and input inside it.
export function initToolbar(root, view, actions = recipeActions) {
  const items = () => [...root.querySelectorAll("button, select, input")].filter(el => !el.disabled);

  function setCurrent(item) {
    for (const el of items()) el.tabIndex = el === item ? 0 : -1;
  }

  const initial = items();
  if (initial.length) setCurrent(initial[0]);

  root.addEventListener("click", event => {
    const button = event.target.closest("[data-action]");
    if (!button || !root.contains(button) || button.tagName === "SELECT") return;
    const action = actions[button.dataset.action];
    if (!action) return;
    setCurrent(button);
    action(view, button);
  });

  root.addEventListener("focusin", event => {
    if (items().includes(event.target)) setCurrent(event.target);
  });

  root.addEventListener("keydown", event => {
    // Arrow keys belong to text fields while the caret is in one.
    if (event.target.tagName === "INPUT") return;
    const list = items();
    const index = list.indexOf(event.target);
    if (index === -1) return;
    let next;
    switch (event.key) {
      case "ArrowRight": next = list[(index + 1) % list.length]; break;
      case "ArrowLeft": next = list[(index - 1 + list.length) % list.length]; break;
      case "Home": next = list[0]; break;
      case "End": next = list[list.length - 1]; break;
      default: return;
    }
    event.preventDefault();
    setCurrent(next);
    next.focus();
  });
}
