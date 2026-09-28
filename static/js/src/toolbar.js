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

// Menu files. A meal is a `Meal:` header followed by `- ` bullets, and the
// menu page only keeps them together when every line but the last ends with
// ` \` (a blank line starts a new paragraph, so a new group).
const CONTINUED = /\\\s*$/;
const BULLET = /^\s*-(?!-)/;
const EMPTY_BULLET = /^\s*-\s*(\\\s*)?$/;

function isMealLine(text) {
  if (BULLET.test(text)) return true;
  const bare = text.replace(CONTINUED, "").trim();
  return bare.endsWith(":") && !bare.startsWith("--") && !bare.startsWith("=");
}

// Number of sections (`== Day ==` lines) after the frontmatter.
export function countSections(state) {
  const frontmatter = findFrontmatter(state);
  let count = 0;
  for (let n = frontmatter ? frontmatter.closeLine + 1 : 1; n <= state.doc.lines; n++) {
    if (/^\s*=/.test(state.doc.line(n).text)) count++;
  }
  return count;
}

// `2026-03-07` -> `Saturday (2026-03-07)`, the weekday in `locale`. The date
// in brackets is what the menu page reads to find today's menu.
export function datedDayName(value, locale) {
  const date = new Date(`${value}T00:00:00Z`);
  let weekday;
  try {
    weekday = date.toLocaleDateString(locale || undefined, { weekday: "long", timeZone: "UTC" });
  } catch {
    weekday = date.toLocaleDateString(undefined, { weekday: "long", timeZone: "UTC" });
  }
  return `${weekday.charAt(0).toLocaleUpperCase()}${weekday.slice(1)} (${value})`;
}

function nextDate(value) {
  const date = new Date(`${value}T00:00:00Z`);
  date.setUTCDate(date.getUTCDate() + 1);
  return date.toISOString().slice(0, 10);
}

// The number in the menu's `servings` frontmatter, or "" when there is none.
export function menuServings(state) {
  const frontmatter = findFrontmatter(state);
  if (!frontmatter) return "";
  const body = state.sliceDoc(frontmatter.bodyFrom, frontmatter.bodyTo);
  const match = body.match(/^servings\s*:\s*["']?(\d+(?:[.,]\d+)?)/m);
  return match ? match[1].replace(",", ".") : "";
}

// Add `item` to the menu as a bullet of the meal around the cursor:
// - on an empty bullet (`- `), fill it in;
// - on a blank line right after a meal, add to that meal;
// - on any other line, add a bullet below it.
// The line above the new bullet gets the ` \` that joins them when it is a
// meal header or bullet without one; the new bullet gets one when it lands
// between two lines of the same meal.
export function insertMenuItem(view, item) {
  const { state } = view;
  const { doc } = state;
  const lineBefore = n => (n > 1 ? doc.line(n - 1) : null);
  const lineAfter = n => (n < doc.lines ? doc.line(n + 1) : null);

  let line = doc.lineAt(state.selection.main.head);
  if (line.text.trim() === "") {
    const above = lineBefore(line.number);
    if (!above || !isMealLine(above.text)) {
      insertBlock(view, `- ${item}`);
      return;
    }
    line = above;
  }

  // ` \` at the end of `line`, replacing any trailing spaces.
  const joinLine = target => ({ from: target.from + target.text.trimEnd().length, to: target.to, insert: " \\" });
  const needsJoin = target => target && isMealLine(target.text) && !CONTINUED.test(target.text);

  const bullet = `- ${item}`;
  const changes = [];
  // The cursor lands right after the item: `offset` characters past where the
  // last change starts.
  let offset;
  if (EMPTY_BULLET.test(line.text)) {
    const above = lineBefore(line.number);
    if (needsJoin(above)) changes.push(joinLine(above));
    const tail = CONTINUED.test(line.text) ? " \\" : "";
    changes.push({ from: line.from, to: line.to, insert: bullet + tail });
    offset = bullet.length;
  } else {
    const below = lineAfter(line.number);
    // Already joined to the line below: the new bullet sits in between, so
    // it carries the join on.
    const tail = CONTINUED.test(line.text) && below && below.text.trim() !== "" ? " \\" : "";
    const change = needsJoin(line) ? joinLine(line) : { from: line.to, to: line.to, insert: "" };
    change.insert += "\n" + bullet + tail;
    changes.push(change);
    offset = change.insert.length - tail.length;
  }

  const changeSet = state.changes(changes);
  const last = changes[changes.length - 1];
  const cursor = changeSet.mapPos(last.from, -1) + offset;
  dispatch(view, { changes: changeSet, selection: EditorSelection.cursor(cursor) });
}

// Enter in a menu, at the end of a meal header or bullet: continue the meal,
// the way a list continues in a Markdown editor. The line gets the ` \` that
// keeps the next one in the same meal, and the next line starts with `- `.
// On an empty bullet, Enter ends the meal instead: the bullet and the ` \`
// leading to it go, and a blank line separates what comes next. Anywhere
// else it is a plain Enter. Returns whether it handled the key.
export function continueMeal(view) {
  const { state } = view;
  const range = state.selection.main;
  if (state.selection.ranges.length > 1 || !range.empty) return false;
  const line = state.doc.lineAt(range.head);
  const content = line.text.replace(CONTINUED, "").trimEnd();
  if (range.head < line.from + content.length || !isMealLine(line.text)) return false;

  const above = line.number > 1 ? state.doc.line(line.number - 1) : null;
  const below = line.number < state.doc.lines ? state.doc.line(line.number + 1) : null;

  if (EMPTY_BULLET.test(line.text)) {
    const changes = [{ from: line.from, to: line.to, insert: "\n" }];
    if (above && isMealLine(above.text) && CONTINUED.test(above.text)) {
      changes.push({ from: above.from + above.text.replace(CONTINUED, "").trimEnd().length, to: above.to });
    }
    const changeSet = state.changes(changes);
    view.dispatch({
      changes: changeSet,
      selection: EditorSelection.cursor(changeSet.mapPos(line.from, -1) + 1),
      scrollIntoView: true,
      userEvent: "input"
    });
    return true;
  }

  // Already joined to a non-blank line below: the new bullet sits in between,
  // so it carries the join on.
  const tail = CONTINUED.test(line.text) && below && below.text.trim() !== "" ? "\\" : "";
  const from = line.from + content.length;
  const insert = " \\\n- ";
  view.dispatch({
    changes: { from, to: line.to, insert: insert + tail },
    selection: EditorSelection.cursor(from + insert.length),
    scrollIntoView: true,
    userEvent: "input"
  });
  return true;
}

// Replace the selection with `text`, putting the cursor `cursorFromEnd`
// characters before its end.
function insertInline(view, text, cursorFromEnd = 0) {
  const { state } = view;
  dispatch(view, state.changeByRange(range => ({
    changes: { from: range.from, to: range.to, insert: text },
    range: EditorSelection.cursor(range.from + text.length - cursorFromEnd)
  })));
}

// Toolbar actions for menu files.
export const menuActions = {
  // `== Day N ==`, or `== Saturday (2026-03-07) ==` when the date field next
  // to the button holds a date. The date then moves on a day, ready for the
  // next one.
  day: (view, button) => {
    const dateInput = button?.closest('[role="toolbar"]')?.querySelector("[data-day-date]");
    let name;
    if (dateInput?.value) {
      name = datedDayName(dateInput.value, document.documentElement.lang);
      dateInput.value = nextDate(dateInput.value);
    } else {
      name = `${button?.dataset.default || "Day"} ${countSections(view.state) + 1}`;
    }
    insertBlock(view, `== ${name} ==`, { select: [3, 3 + name.length] });
  },
  meal: (view, item) => {
    const name = item?.dataset.meal || "Meal";
    insertBlock(view, `${name}: \\\n- `);
  }
};

// Actions that go through the recipe picker (see picker.js).
export function pickerActions(picker) {
  return {
    // A sub-recipe inside a step: `@./Path/Name{}`, cursor in the braces.
    "recipe-reference": async view => {
      const choice = await picker.open();
      if (choice) insertInline(view, `@${choice.reference}{}`, 1);
    },
    // A menu entry: `- @./Path/Name{N%servings}`, or `{}` for the recipe's
    // own servings.
    "add-recipe": async view => {
      const choice = await picker.open({ servings: menuServings(view.state) });
      if (!choice) return;
      const amount = choice.servings ? `${choice.servings}%servings` : "";
      insertMenuItem(view, `@${choice.reference}{${amount}}`);
    }
  };
}

// A button with `aria-haspopup="menu"` and the `role="menu"` it controls:
// the ARIA menu button pattern. Picking an item runs its `data-action`
// through the toolbar's click handler, then closes the menu.
function initMenuButton(button) {
  const menu = document.getElementById(button.getAttribute("aria-controls"));
  if (!menu) return;
  const menuItems = () => [...menu.querySelectorAll('[role="menuitem"]')];

  function open(focusIndex = 0) {
    menu.hidden = false;
    button.setAttribute("aria-expanded", "true");
    const list = menuItems();
    list[(focusIndex + list.length) % list.length]?.focus();
  }

  function close(refocus) {
    if (menu.hidden) return;
    menu.hidden = true;
    button.setAttribute("aria-expanded", "false");
    if (refocus) button.focus();
  }

  button.addEventListener("click", () => (menu.hidden ? open() : close(true)));
  button.addEventListener("keydown", event => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      open(event.key === "ArrowDown" ? 0 : -1);
    }
  });

  menu.addEventListener("keydown", event => {
    const list = menuItems();
    const index = list.indexOf(event.target);
    let next;
    switch (event.key) {
      case "ArrowDown": next = list[(index + 1) % list.length]; break;
      case "ArrowUp": next = list[(index - 1 + list.length) % list.length]; break;
      case "Home": next = list[0]; break;
      case "End": next = list[list.length - 1]; break;
      case "Escape":
        event.preventDefault();
        event.stopPropagation();
        close(true);
        return;
      case "Tab":
        close(false);
        return;
      default: return;
    }
    event.preventDefault();
    next.focus();
  });

  // Runs before the toolbar's own click handler, which then runs the action
  // and hands focus to the editor.
  menu.addEventListener("click", event => {
    if (event.target.closest('[role="menuitem"]')) close(true);
  });

  document.addEventListener("click", event => {
    if (!menu.hidden && !menu.contains(event.target) && !button.contains(event.target)) close(false);
  });
}

// Wire a `role="toolbar"` element to an editor view: clicks on `[data-action]`
// run the matching entry of `actions`, and the ARIA toolbar keyboard pattern
// (one tab stop, arrow keys / Home / End between items) applies to every
// button, select and input inside it, except the items of a popup menu, which
// have their own keyboard handling.
export function initToolbar(root, view, actions = recipeActions) {
  const items = () => [...root.querySelectorAll("button, select, input")]
    .filter(el => !el.disabled && !el.closest('[role="menu"]'));

  function setCurrent(item) {
    for (const el of items()) el.tabIndex = el === item ? 0 : -1;
  }

  const initial = items();
  if (initial.length) setCurrent(initial[0]);

  for (const button of root.querySelectorAll('[aria-haspopup="menu"]')) initMenuButton(button);

  root.addEventListener("click", event => {
    const button = event.target.closest("[data-action]");
    if (!button || !root.contains(button) || button.tagName === "SELECT") return;
    const action = actions[button.dataset.action];
    if (!action) return;
    if (items().includes(button)) setCurrent(button);
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
