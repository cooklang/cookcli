# Web UI Token Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild the web UI's styling on a Tailwind v4 CSS-first token layer with the Cooklang palette and flat surfaces, while keeping every page's layout, spacing and dimensions exactly as they are on `main`.

**Architecture:** `static/css/input.css` owns the tokens, the `@theme` registration and the print/CodeMirror rules; `static/css/components.css` owns the component vocabulary (`.card`, `.btn`, `.nav-pill`, `.recipe-card`, `.step-box`, …). Each Askama template keeps its current markup and swaps palette utilities and gradients for token utilities and component classes. The `.dark .*` override block in `base.html` is deleted last, once no page depends on it.

**Tech Stack:** Rust (Axum + Askama templates), Tailwind CSS 4.3 via `@tailwindcss/cli`, Playwright E2E tests, `cargo test`.

**Spec:** `docs/superpowers/specs/2026-09-04-web-ui-tokens-design.md`. Read it first. Section 1.4 is the component table every template task refers to.

**Branch:** `design/web-ui-tokens`, cut from `main`. Final step force-pushes it over `design/web-ui-refresh` (PR #456).

**Rules that apply to every task:**

- Never commit `static/css/output.css` (gitignored). Rebuild it with `npm run build-css` after any CSS change and before running E2E tests.
- No Tailwind palette utility (`gray-*`, `orange-*`, `purple-*`, `blue-*`, `green-*`, `red-*`, `pink-*`, `yellow-*`, `indigo-*`, `lime-*`, `amber-*`, `cyan-*`, `emerald-*`, `white`, `black`) may appear in a file you touch, except `bg-black/50` for modal backdrops. No gradients. The only `dark:` usage allowed is the theme-toggle icon swap (`hidden dark:block` / `block dark:hidden`).
- Keep every structural utility (grid columns, `p-6`, `gap-6`, `mb-8`, `h-48`, `w-16`, `sticky top-6`, responsive variants) exactly as `main` has it. Only colour, border, radius, shadow and typography classes change.
- Commit messages use Conventional Commits (`feat(ui):`, `fix(ui):`, `refactor(ui):`, `test(ui):`, `docs:`) and end with the trailer line `Claude-Session: https://claude.ai/code/session_013urND2B6Y3Z7WQuDpE8ZDu`.
- E2E: `npm test -- --project=chromium <spec>` runs one spec file in Chromium. The Playwright web server builds CSS, JS and the binary automatically; if a server is already on port 9080 it is reused, so restart it after Rust changes. The `shopping-list*.spec.ts` files race on the shared fixture; run them with `--workers=1`.

---

## File map

| File | Responsibility | Action |
|---|---|---|
| `static/css/input.css` | Tailwind entry: custom variant, sources, tokens, `@theme`, type scale, print token reset, CodeMirror | Rewrite |
| `static/css/components.css` | Component vocabulary in `@layer components` | Create |
| `static/css/cooking-mode.css` | Cook mode overlay, on tokens | Replace with PR #456's version |
| `static/css/custom-styles.css`, `static/css/styles.css`, `tailwind.config.js` | Dead / superseded | Delete |
| `templates/base.html` | Nav card, search, footer, inline style block | Edit (Tasks 2 and 12) |
| `templates/recipes.html` | Index cards + sorter | Edit |
| `templates/recipe.html` | Recipe page + scale scripts | Edit |
| `templates/shopping_list.html` | Shopping list markup + JS-rendered strings | Edit |
| `templates/pantry.html` | Pantry blocks + status JS | Edit |
| `templates/menu.html` | Menu page | Edit |
| `templates/preferences.html` | Preferences cards and toggles | Edit |
| `templates/edit.html`, `templates/new.html` | Editor and new-recipe form | Edit |
| `templates/api_docs.html`, `templates/error.html` | API docs and error page | Edit |
| `src/web/templates.rs` | `method_classes()` badge classes | Edit |
| `static/js/keyboard-shortcuts.js` | Modal classes, `adjustScale` export | Edit |
| `static/js/search.js` | Result row classes | Edit |
| `static/js/cooking-mode.js` | Step capture selectors | Edit |
| `tests/e2e/navigation.spec.ts`, `preferences.spec.ts`, `recipe-display.spec.ts` | Selector updates | Edit |
| `tests/e2e/recipes-sort.spec.ts` | Sorter coverage | Create |
| `tests/menu_api_test.rs` | Class-agnostic badge regex | Edit |

---

### Task 1: CSS foundation

**Files:**
- Rewrite: `static/css/input.css`
- Create: `static/css/components.css`
- Replace: `static/css/cooking-mode.css`
- Delete: `static/css/custom-styles.css`, `static/css/styles.css`, `tailwind.config.js`
- Modify: `templates/base.html:13` (remove the `custom-styles.css` link)

- [ ] **Step 1: Confirm the build is green before touching anything**

Run: `npm run build-css && cargo build 2>&1 | tail -1`
Expected: `output.css` written, `Finished` line from cargo.

- [ ] **Step 2: Write `static/css/input.css`**

Replace the whole file with:

```css
@import "tailwindcss";

/* Tailwind v4 is CSS-first. The two things tailwind.config.js used to own —
   class-based dark mode and the list of files to scan for class names — are
   declared here instead. */
@custom-variant dark (&:where(.dark, .dark *));

@source "../../templates";
@source "../../static/js";
/* The one Rust file that emits class names: method_classes() for the API
   docs method badge. */
@source "../../src/web/templates.rs";

/* ============================================================
   DESIGN TOKENS
   Seventeen semantic colour names plus radii and shadows, defined once per
   theme. Every colour in the UI resolves through one of these. Do not
   introduce raw hex values or Tailwind palette utilities in templates.
   --accent-ink is the foreground for text sitting ON a filled --accent
   background: the design system's near-black, which clears AA on the
   accent fill where white would not.
   ============================================================ */
:root {
    color-scheme: light;

    --bg:            #fcfcfb;
    --surface:       #ffffff;
    --surface-sunk:  #f5f3f0;
    --border:        #e4e0da;
    --border-strong: #c3bcb1;
    --text:          #16161d;   /* DS Text/Primary */
    --text-muted:    #5f5a51;   /* DS Text/Secondary, darkened for AA */
    --text-faint:    #6a645b;
    --accent:        #e15a29;   /* DS Controls/Primary, Icons/Primary */
    --accent-text:   #715329;   /* DS Text/Tags */
    --accent-soft:   #f5dacf;   /* DS Background/UI One */
    --accent-ink:    #16161d;
    --ok:            #3d6849;
    --ok-soft:       #e2e8df;
    --danger:        #c4261c;   /* DS Text/Warning, darkened for AA */
    --danger-soft:   #f7dfdc;
    --danger-ink:    #ffffff;
    /* Links. The design system is entirely warm, so this is the dark end of
       the DS orange rather than a blue; every link also carries an underline
       so the affordance never rests on hue alone. */
    --info:          #8a3d14;
    --disabled:      #d3cdcb;   /* DS Controls/Disabled */
    --inactive:      #8a8075;   /* DS Controls/Inactive, darkened for AA */

    --radius-control: 6px;
    --radius-card:    6px;

    /* Two elevations. In-flow surfaces get --shadow-card, which is barely a
       shadow at all — the border does the work. --shadow-overlay is only for
       things floating above the page: dropdowns, dialogs, search results. */
    --shadow-card:    0 1px 0 rgba(27, 31, 36, .04);
    --shadow-overlay: 0 8px 24px rgba(27, 31, 36, .12);
}

/* .cooking-overlay is listed here deliberately: cook mode is always dark
   regardless of the site theme, so its inline entity badges must resolve the
   dark token values or they render at ~2.5:1 on the dark card. */
.dark,
.cooking-overlay {
    color-scheme: dark;

    --bg:            #16161d;
    --surface:       #1c1c24;
    --surface-sunk:  #23232c;
    --border:        #30303b;
    --border-strong: #43434f;
    --text:          #efeae6;
    --text-muted:    #ada69b;
    --text-faint:    #948d83;
    --accent:        #e15a29;
    --accent-text:   #f08050;
    --accent-soft:   #3a2820;
    --accent-ink:    #16161d;
    --ok:            #6fb283;
    --ok-soft:       #1e2a22;
    --danger:        #ff6b60;
    --danger-soft:   #2e1b1a;
    --danger-ink:    #16161d;
    --info:          #e59a6d;
    --disabled:      #4a4a55;
    --inactive:      #8f8880;

    --shadow-card:    none;
    --shadow-overlay: 0 8px 24px rgba(0, 0, 0, .5);
}

/* Register tokens with Tailwind so they generate real utilities
   (bg-surface, text-muted, border-line, …). `inline` makes the generated
   utilities reference the custom property, so they flip automatically under
   .dark with no override rules. Border tokens are named `line` so the
   utility reads `border-line` rather than `border-border`. */
@theme inline {
    --color-bg:          var(--bg);
    --color-surface:     var(--surface);
    --color-sunk:        var(--surface-sunk);
    --color-line:        var(--border);
    --color-line-strong: var(--border-strong);
    --color-text:        var(--text);
    --color-muted:       var(--text-muted);
    --color-faint:       var(--text-faint);
    --color-accent:      var(--accent);
    --color-accent-text: var(--accent-text);
    --color-accent-soft: var(--accent-soft);
    --color-accent-ink:  var(--accent-ink);
    --color-ok:          var(--ok);
    --color-ok-soft:     var(--ok-soft);
    --color-danger:      var(--danger);
    --color-danger-soft: var(--danger-soft);
    --color-danger-ink:  var(--danger-ink);
    --color-info:        var(--info);
    --color-disabled:    var(--disabled);
    --color-inactive:    var(--inactive);
}

/* ============================================================
   TYPE SCALE
   Seven steps, each with a role and a line-height chosen for that role.
   `display` is 30px — the size main's page titles have always been — the
   rest are the PR #456 scale. Tailwind's own names are aliased onto the
   steps so a stray `text-sm` cannot drift off the scale.
   ============================================================ */
@theme {
    --text-display: 30px;
    --text-display--line-height: 1.2;
    --text-display--letter-spacing: -.01em;

    --text-title: 18px;
    --text-title--line-height: 1.35;
    --text-title--letter-spacing: -.005em;

    --text-read: 16px;
    --text-read--line-height: 1.6;

    --text-body: 14px;
    --text-body--line-height: 1.5;

    --text-ui: 13px;
    --text-ui--line-height: 1.4;

    --text-meta: 12px;
    --text-meta--line-height: 1.4;

    --text-label: 11px;
    --text-label--line-height: 1.3;
    --text-label--letter-spacing: .06em;

    --text-xs:   var(--text-meta);
    --text-xs--line-height: 1.4;
    --text-sm:   var(--text-body);
    --text-sm--line-height: 1.5;
    --text-base: var(--text-read);
    --text-base--line-height: 1.6;
    --text-lg:   var(--text-title);
    --text-lg--line-height: 1.35;
    --text-2xl:  var(--text-display);
    --text-2xl--line-height: 1.2;
    --text-3xl:  var(--text-display);
    --text-3xl--line-height: 1.2;
}

body {
    font-size: var(--text-body);
    line-height: 1.5;
}

/* Component vocabulary. Kept in its own file so this one stays the small,
   stable theme contract. */
@import "./components.css";

/* Print: force the light token values regardless of the active theme.
   Layout flattening lives in base.html's print block. */
@media print {
    :root,
    .dark,
    .cooking-overlay {
        --bg:            #ffffff;
        --surface:       #ffffff;
        --surface-sunk:  #f5f4f2;
        --border:        #dddad5;
        --border-strong: #bbb6ae;
        --text:          #111111;
        --text-muted:    #444444;
        --text-faint:    #666666;
        --accent:        #a8380c;
        --accent-text:   #a8380c;
        --accent-soft:   #f7ece5;
        --accent-ink:    #ffffff;
        --ok:            #23624a;
        --ok-soft:       #eaf3ee;
        --danger:        #96201a;
        --danger-soft:   #f8eae9;
        --danger-ink:    #ffffff;
        --info:          #1f4f86;
        --disabled:      #cccccc;
        --inactive:      #767676;

        --shadow-card:    none;
        --shadow-overlay: none;
    }

    .card,
    .btn,
    .stepper,
    .nav-card,
    .recipe-card,
    .step-box,
    .ingredient-row,
    .pantry-item {
        border: 1px solid var(--border) !important;
        box-shadow: none !important;
    }

    .card,
    .step-box {
        break-inside: avoid;
    }
}

/* ============================================================
   CODEMIRROR
   Token-driven in BOTH themes. These rules live OUTSIDE @layer components on
   purpose: CodeMirror injects its base theme as an unlayered <style> at
   runtime, and unlayered always beats layered regardless of specificity.
   `.cm-editor.cm-editor` reaches 0,3,0 to beat CodeMirror's own `.ͼ2 .cm-*`
   selectors in source order without !important.
   ============================================================ */
.cm-editor {
    background: var(--surface);
    color: var(--text);
}

.cm-editor.cm-editor .cm-gutters {
    background: var(--surface-sunk);
    border-color: var(--border);
    color: var(--text-faint);
}

.cm-editor.cm-editor .cm-activeLineGutter {
    background: var(--surface-sunk);
    color: var(--text-muted);
}

.cm-editor.cm-editor .cm-activeLine {
    background: color-mix(in srgb, var(--surface-sunk) 60%, transparent);
}

.cm-editor.cm-editor .cm-content { caret-color: var(--text); }

.cm-editor.cm-editor .cm-cursor,
.cm-editor.cm-editor .cm-dropCursor {
    border-left-color: var(--text);
}

/* CodeMirror's focused-selection rule is 0,5,0, which no reasonable selector
   beats, so this keeps the !important the old rule had. */
.cm-editor .cm-selectionBackground,
.cm-editor.cm-focused .cm-selectionBackground,
.cm-editor ::selection {
    background: color-mix(in srgb, var(--info) 30%, transparent) !important;
}
```

- [ ] **Step 3: Create `static/css/components.css`**

```css
/* ============================================================
   COMPONENT LAYER
   Imported from input.css after the token registration. Everything here
   resolves through the tokens; never introduce raw hex values or Tailwind
   palette utilities. Dimensions match what main rendered before the token
   migration — this file changes how things look, not how big they are.
   Colour transitions are deliberately absent: transitioning a token-valued
   property races Chrome's custom-property invalidation when .dark flips,
   and elements keep the outgoing theme's colour until the next recalc.
   ============================================================ */
@layer components {
    /* ---------- Focus ---------- */
    :focus-visible {
        outline: 2px solid var(--accent);
        outline-offset: 2px;
        border-radius: var(--radius-control);
    }

    /* ---------- Surfaces ---------- */
    .card {
        background: var(--surface);
        border: 1px solid var(--border);
        border-radius: var(--radius-card);
        box-shadow: var(--shadow-card);
    }

    .card-head {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 10px 14px;
        border-bottom: 1px solid var(--border);
    }

    .card-head h2,
    .card-head h3 {
        margin: 0;
        font-size: var(--text-label);
        font-weight: 600;
        letter-spacing: .06em;
        text-transform: uppercase;
        color: var(--text-muted);
    }

    .card-head.plain h2,
    .card-head.plain h3 {
        font-size: var(--text-body);
        font-weight: 600;
        letter-spacing: 0;
        text-transform: none;
        color: var(--text);
    }

    .card-head .count {
        font-size: var(--text-meta);
        color: var(--text-faint);
        font-variant-numeric: tabular-nums;
    }

    /* The nav container: main's rounded card, not sticky. */
    .nav-card {
        background: var(--surface);
        border: 1px solid var(--border);
        border-radius: var(--radius-card);
        box-shadow: var(--shadow-card);
        margin-bottom: 2rem;
    }

    /* ---------- Controls ---------- */
    /* 40px: main's px-4 py-2 buttons at 16px text. */
    .btn {
        display: inline-flex;
        align-items: center;
        gap: 8px;
        height: 40px;
        padding: 0 16px;
        border-radius: var(--radius-control);
        font-size: var(--text-body);
        font-weight: 500;
        white-space: nowrap;
        cursor: pointer;
        text-decoration: none;
        background: var(--surface);
        color: var(--text);
        border: 1px solid var(--border-strong);
    }

    .btn:hover {
        background: var(--surface-sunk);
        border-color: var(--border-strong);
    }

    .btn:active { background: var(--surface-sunk); filter: brightness(.97); }

    .btn svg { width: 20px; height: 20px; flex: 0 0 auto; }

    /* One accent-filled button per view. Everything else is neutral. */
    .btn-primary {
        background: var(--accent);
        border-color: var(--accent);
        color: var(--accent-ink);
    }

    .btn-primary:hover {
        background: var(--accent);
        filter: brightness(1.06);
    }

    /* The commit half of a destructive confirmation. */
    .btn-danger {
        background: var(--danger);
        border-color: var(--danger);
        color: var(--danger-ink);
    }

    .btn-danger:hover {
        background: var(--danger);
        filter: brightness(1.06);
    }

    .icon-btn {
        width: 36px;
        height: 36px;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        border: 0;
        border-radius: var(--radius-control);
        background: transparent;
        color: var(--text-muted);
        cursor: pointer;
        text-decoration: none;
    }

    .icon-btn:hover {
        background: var(--surface-sunk);
        color: var(--text);
    }

    .icon-btn svg { width: 20px; height: 20px; }

    .select {
        height: 40px;
        padding: 0 10px;
        border-radius: var(--radius-control);
        border: 1px solid var(--border-strong);
        background: var(--surface);
        color: var(--text);
        font-size: var(--text-body);
        font-weight: 500;
        cursor: pointer;
    }

    .select:hover { background: var(--surface-sunk); }

    .stepper {
        display: inline-flex;
        align-items: center;
        height: 40px;
        overflow: hidden;
        background: var(--surface);
        border: 1px solid var(--border-strong);
        border-radius: var(--radius-control);
    }

    .stepper button {
        width: 32px;
        height: 100%;
        border: 0;
        background: transparent;
        color: var(--text-muted);
        cursor: pointer;
        font-size: var(--text-read);
    }

    .stepper button:hover {
        background: var(--surface-sunk);
        color: var(--text);
    }

    .stepper button:active { background: var(--surface-sunk); filter: brightness(.97); }

    .stepper input {
        width: 56px;
        height: 100%;
        border: 0;
        background: transparent;
        color: var(--text);
        text-align: center;
        font-size: var(--text-body);
        font-weight: 600;
        font-variant-numeric: tabular-nums;
        -moz-appearance: textfield;
        appearance: textfield;
    }

    .stepper input::-webkit-outer-spin-button,
    .stepper input::-webkit-inner-spin-button {
        -webkit-appearance: none;
        margin: 0;
    }

    .stepper input:focus { outline: none; }

    .search-input {
        width: 100%;
        height: 44px;
        padding: 0 16px 0 40px;
        border-radius: var(--radius-control);
        border: 1px solid var(--border-strong);
        background: var(--surface);
        color: var(--text);
        font-size: var(--text-body);
    }

    .search-input::placeholder { color: var(--text-faint); }

    .search-input:focus {
        outline: none;
        border-color: var(--accent);
    }

    /* ---------- Navigation ---------- */
    .nav-pill {
        padding: .5rem 1.25rem;
        border-radius: 9999px;
        font-size: var(--text-body);
        font-weight: 500;
        color: var(--text-muted);
        text-decoration: none;
        white-space: nowrap;
    }

    .nav-pill:hover {
        background: var(--surface-sunk);
        color: var(--text);
    }

    .nav-pill.active {
        background: var(--accent-soft);
        color: var(--accent-text);
        font-weight: 600;
    }

    /* ---------- Lists ---------- */
    .row {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 7px 14px;
        font-size: var(--text-body);
    }

    .row + .row { border-top: 1px solid var(--border); }

    .row .row-name { flex: 1; min-width: 0; }

    .row-value {
        white-space: nowrap;
        font-weight: 600;
        color: var(--accent-text);
        font-variant-numeric: tabular-nums;
    }

    .row-note {
        color: var(--text-faint);
        font-size: var(--text-meta);
    }

    /* Checkbox sizes live here, not in w-* / h-* utilities, so the
       coarse-pointer override further down can actually win. */
    .ref-checkbox { width: 16px; height: 16px; }
    .list-checkbox { width: 20px; height: 20px; }

    /* Recipe ingredient rows: main's tinted row, on the sunk surface. */
    .ingredient-list { list-style: none; margin: 0; padding: 0; }

    .ingredient-row {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: .5rem .75rem;
        border-radius: var(--radius-control);
        background: var(--surface-sunk);
    }

    .section-label {
        font-size: var(--text-label);
        font-weight: 600;
        letter-spacing: .06em;
        text-transform: uppercase;
        color: var(--text-faint);
        margin: 18px 0 8px;
        padding: 0 14px;
    }

    /* ---------- Text ---------- */
    .metaline {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 6px 0;
        font-size: var(--text-ui);
        color: var(--text-muted);
    }

    .metaline > *:not(:last-child)::after {
        content: "·";
        margin: 0 8px;
        color: var(--text-faint);
    }

    .metaline b { font-weight: 600; color: var(--text); }

    /* Recipe-reference links. --info is a warm orange, so the underline is
       what makes it read as a link. */
    a.text-info { text-decoration: underline; text-underline-offset: 2px; }

    /* ---------- Status ---------- */
    .item-status-dot {
        flex: 0 0 auto;
        width: 8px;
        height: 8px;
        border-radius: 50%;
    }

    .item-status-dot.in-stock { background: var(--ok); }

    .item-status-dot.low-stock { background: var(--accent); }

    .item-status-dot.out-of-stock { background: var(--danger); }

    /* ---------- Inline recipe entities ---------- */
    /* Weight + tint, not bordered gradient pills, so they stop competing with
       the prose they sit inside. */
    .ingredient-badge {
        color: var(--accent-text);
        font-weight: 600;
        white-space: nowrap;
    }

    /* Distinguished from an ingredient by texture, not hue, so the split is
       visible to red-green deficient readers. */
    .cookware-badge {
        color: var(--text);
        font-weight: 600;
        white-space: nowrap;
        text-decoration: underline dotted var(--inactive);
        text-underline-offset: 3px;
    }

    .timer-badge {
        padding: 1px 6px;
        border-radius: 5px;
        background: var(--surface-sunk);
        border: 1px solid var(--border);
        color: var(--text);
        font-weight: 600;
        font-variant-numeric: tabular-nums;
        white-space: nowrap;
    }

    /* main's 32px circle, on a flat accent tint. */
    .step-number {
        flex: 0 0 32px;
        width: 32px;
        height: 32px;
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        background: var(--accent-soft);
        color: var(--accent-text);
        font-size: var(--text-body);
        font-weight: 700;
        font-variant-numeric: tabular-nums;
    }

    .tag {
        display: inline-block;
        padding: 1px 8px;
        border-radius: 9999px;
        font-size: var(--text-meta);
        font-weight: 500;
        background: var(--surface-sunk);
        border: 1px solid var(--border);
        color: var(--text-muted);
    }

    /* Neutral bordered pill. The metadata-* key classes stay in the markup as
       hooks but carry no colour of their own any more. */
    .metadata-pill {
        display: inline-flex;
        align-items: center;
        padding: .25rem .75rem;
        border-radius: 9999px;
        font-size: var(--text-body);
        font-weight: 500;
        white-space: nowrap;
        background: var(--surface);
        border: 1px solid var(--border);
        color: var(--text-muted);
    }

    .metadata-pill svg {
        width: 1rem;
        height: 1rem;
        margin-right: .5rem;
    }

    /* ---------- Recipes index ---------- */
    /* main's tall card: block layout, p-6 content from the template, no
       gradient stripe, no scale transform. */
    .recipe-card {
        display: flex;
        flex-direction: column;
        overflow: hidden;
        text-decoration: none;
        background: var(--surface);
        border: 1px solid var(--border);
        border-radius: var(--radius-card);
        box-shadow: var(--shadow-card);
    }

    .recipe-card:hover { border-color: var(--border-strong); }

    .recipe-card-icon {
        width: 64px;
        height: 64px;
        display: flex;
        align-items: center;
        justify-content: center;
        border-radius: 50%;
        background: var(--surface-sunk);
        border: 1px solid var(--border);
        font-size: 24px;
        margin-bottom: 1rem;
    }

    /* An <h2>, not an <h3>: the index has an <h1> and nothing between. */
    .recipe-card-title {
        display: block;
        margin: 0 0 .5rem;
        font-size: var(--text-title);
        font-weight: 700;
        line-height: 1.35;
        color: var(--text);
    }

    .recipe-card-sub {
        display: block;
        font-size: var(--text-meta);
        color: var(--text-faint);
    }

    /* ---------- Recipe page ---------- */
    .step-list { list-style: none; margin: 0; padding: 0; }

    /* main's per-step box. */
    .step-box {
        background: var(--surface-sunk);
        border: 1px solid var(--border);
        border-radius: var(--radius-card);
        padding: 1rem;
    }

    /* Step prose keeps main's leading-8 (2.0) on 16px text — the one place
       the reading step's line-height is overridden, by decision. */
    .step-body {
        font-size: var(--text-read);
        line-height: 2;
        color: var(--text);
    }

    .step-refs {
        margin-top: .5rem;
        padding-left: 1rem;
        border-left: 2px solid var(--accent);
        font-size: var(--text-body);
        color: var(--text-muted);
    }

    .recipe-note {
        display: flex;
        gap: 8px;
        padding: 12px 14px;
        border-radius: var(--radius-card);
        border: 1px solid var(--border);
        border-left: 3px solid var(--accent);
        background: var(--surface-sunk);
        color: var(--text-muted);
        font-style: italic;
    }

    .image-step {
        @apply w-full max-h-80 object-contain rounded-xl;
    }

    .recipe-image-placeholder {
        background: var(--surface-sunk);
        border: 1px solid var(--border);
        color: var(--accent-text);
        @apply w-30 h-30 rounded-full flex items-center justify-center text-5xl;
    }

    /* ---------- Pantry ---------- */
    /* main's four-line stat block. The transparent border keeps the box from
       shifting when a state border appears. */
    .pantry-item {
        background: var(--surface-sunk);
        border: 1px solid transparent;
        border-radius: var(--radius-control);
        padding: 1rem;
    }

    .pantry-item:hover { border-color: var(--border); }

    .pantry-item.out-of-stock {
        background: var(--danger-soft);
        border-color: var(--danger);
    }

    .pantry-item.out-of-stock .quantity-display,
    .pantry-item.out-of-stock .out-of-stock-icon {
        color: var(--danger);
        font-weight: 600;
    }

    .pantry-item.low-stock {
        background: var(--accent-soft);
        border-color: var(--accent);
    }

    .pantry-item.low-stock .quantity-display,
    .pantry-item.low-stock .out-of-stock-icon {
        color: var(--accent-text);
        font-weight: 600;
    }

    /* Row actions appear on hover or focus, as main's group-hover did. */
    .pantry-actions { opacity: 0; }

    .pantry-item:hover .pantry-actions,
    .pantry-item:focus-within .pantry-actions { opacity: 1; }

    /* ---------- Coarse pointers ---------- */
    /* Touch targets per WCAG 2.2 SC 2.5.8. Only active on touch devices. */
    @media (pointer: coarse) {
        .btn { height: 44px; }
        .select { height: 44px; }
        .stepper { height: 46px; }
        .stepper button { width: 44px; }
        .icon-btn { width: 44px; height: 44px; }
        .ref-checkbox,
        #list-content input[type="checkbox"] { width: 24px; height: 24px; }
        .pantry-actions { opacity: 1; }
    }
}
```

- [ ] **Step 4: Replace `static/css/cooking-mode.css` with PR #456's tokenised version**

Run:
```bash
git show origin/design/web-ui-refresh:static/css/cooking-mode.css > static/css/cooking-mode.css
grep -c 'var(--' static/css/cooking-mode.css
```
Expected: a count above 40, and `grep -nE '#[0-9a-f]{6}|rgba\(' static/css/cooking-mode.css` prints nothing.

- [ ] **Step 5: Delete the superseded files and the stale link**

```bash
git rm -q static/css/custom-styles.css static/css/styles.css tailwind.config.js
```

In `templates/base.html`, delete line 13:

```html
    <link href="{{ prefix }}/static/css/custom-styles.css" rel="stylesheet">
```

- [ ] **Step 6: Build and check the generated utilities exist**

Run: `npm run build-css 2>&1 | tail -3 && grep -oE -- '\.btn-primary|\.nav-card|\.step-box|\.ingredient-row|--accent-soft:' static/css/output.css | sort -u`
Expected: no errors from Tailwind, and all five names printed. Component classes are always emitted; token utilities such as `bg-surface` only appear once a template uses them, so do not look for those yet.

- [ ] **Step 7: Smoke-check the server still renders**

Run: `cargo build 2>&1 | tail -1`, then in a second terminal `./target/debug/cook server ./seed --port 9080`, and `curl -s -o /dev/null -w '%{http_code}\n' http://localhost:9080/`.
Expected: `200`. The pages still look like main at this point because the old `.dark .*` block and the utilities are untouched; only `custom-styles.css` is gone, so the gradient badges and step numbers already render flat. Leave that server running for the rest of the plan; restart it after any Rust change.

- [ ] **Step 8: Commit**

```bash
git add static/css/input.css static/css/components.css static/css/cooking-mode.css templates/base.html
git commit -q -F - <<'EOF'
feat(ui): add token layer and component vocabulary

Tailwind moves fully CSS-first: @custom-variant dark and @source lines
replace tailwind.config.js. custom-styles.css (which shadowed output.css)
and the unreferenced styles.css are deleted. Component dimensions match
what main rendered; only the colour system changes.

Claude-Session: https://claude.ai/code/session_013urND2B6Y3Z7WQuDpE8ZDu
EOF
```

---

### Task 2: `base.html` chrome on tokens

**Files:**
- Modify: `templates/base.html` (body/nav/footer markup, search-result strings, search-selected style rule)

The `.dark .*` override block and the print block stay until Task 12. Only the search-specific dark rules go now, because the search panel is restyled here and those rules would fight it.

- [ ] **Step 1: Replace the search-selected style rules**

In the `<style>` block, find the run that starts with `/* Search result keyboard selection */` and ends just before `@media print {` (on main this is `#search-results a.search-selected { … }` through `.dark #search-results .border-gray-100 { … }`). Replace the whole run with:

```css
        /* Search result keyboard selection. --surface-sunk alone is barely
           visible against --surface in dark mode, so the accent bar carries
           the signal. */
        #search-results a.search-selected {
            background: var(--accent-soft) !important;
            box-shadow: inset 3px 0 0 var(--accent);
        }

```

- [ ] **Step 2: Replace the body, nav and footer markup**

Replace everything from `<body class="bg-gray-50">` through the closing `</footer>{% endif %}</div>` (the `viewport` div) with:

```html
<body class="bg-bg text-text">
    <div class="viewport">
        <nav class="nav-card relative">
            <div class="px-3 lg:px-6 py-4 relative">
                <div class="flex items-center justify-between flex-wrap gap-y-3">
                    <a href="{{ prefix }}/{% if static_mode %}index.html{% endif %}" class="order-1 hidden md:flex items-center space-x-2">
                        <img src="{{ prefix }}/static/android-chrome-192x192.png" alt="Cook" class="brand-mark h-8 w-8 rounded-md" width="32" height="32">
                    </a>

                    <div class="relative z-50 order-2 flex-1 md:ml-2 md:mr-4" id="search-container">
                        <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                            <svg class="h-5 w-5 text-faint" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
                            </svg>
                        </div>
                        <input type="text"
                               id="search-input"
                               placeholder="{{ tr.t("search-placeholder") }}"
                               class="search-input">
                        <div id="search-results"
                             class="absolute top-full mt-2 w-full card shadow-[var(--shadow-overlay)] hidden max-h-96 overflow-y-auto"
                             style="z-index: 9999;">
                        </div>
                    </div>

                    <div class="order-3 w-full md:w-auto flex items-center gap-1 lg:gap-2">
                        {% if features.show_shopping_list || features.show_pantry %}
                        <div class="hidden md:flex items-center gap-1 mr-2 lg:mr-4">
                            <a href="{{ prefix }}/{% if static_mode %}index.html{% endif %}"
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
                        <!-- Inline buttons on md+ screens -->
                        {% if !static_mode %}
                        <a href="{{ prefix }}/preferences" class="icon-btn hidden md:inline-flex {% if active == "preferences" %}bg-accent-soft text-accent-text{% endif %}" aria-label="Preferences" title="Preferences">
                            <span>⚙️</span>
                        </a>
                        {% endif %}
                        <button onclick="showShortcutsHelp()" class="icon-btn hidden md:inline-flex ml-2 print:hidden" aria-label="Keyboard shortcuts" title="Keyboard shortcuts (?)">
                            <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                            </svg>
                        </button>
                        <button onclick="toggleTheme()" class="icon-btn hidden md:inline-flex ml-2 print:hidden" aria-label="Toggle theme">
                            <svg class="hidden dark:block" fill="currentColor" viewBox="0 0 20 20">
                                <path fill-rule="evenodd" d="M10 2a1 1 0 011 1v1a1 1 0 11-2 0V3a1 1 0 011-1zm4 8a4 4 0 11-8 0 4 4 0 018 0zm-.464 4.95l.707.707a1 1 0 001.414-1.414l-.707-.707a1 1 0 00-1.414 1.414zm2.12-10.607a1 1 0 010 1.414l-.706.707a1 1 0 11-1.414-1.414l.707-.707a1 1 0 011.414 0zM17 11a1 1 0 100-2h-1a1 1 0 100 2h1zm-7 4a1 1 0 011 1v1a1 1 0 11-2 0v-1a1 1 0 011-1zM5.05 6.464A1 1 0 106.465 5.05l-.708-.707a1 1 0 00-1.414 1.414l.707.707zm1.414 8.486l-.707.707a1 1 0 01-1.414-1.414l.707-.707a1 1 0 011.414 1.414zM4 11a1 1 0 100-2H3a1 1 0 000 2h1z" clip-rule="evenodd"></path>
                            </svg>
                            <svg class="block dark:hidden" fill="currentColor" viewBox="0 0 20 20">
                                <path d="M17.293 13.293A8 8 0 016.707 2.707a8.001 8.001 0 1010.586 10.586z"></path>
                            </svg>
                        </button>
                        <!-- Overflow menu for small screens -->
                        <div class="relative md:hidden ml-auto" id="more-menu-container">
                            <button onclick="document.getElementById('more-dropdown').classList.toggle('hidden')" class="icon-btn" aria-label="More options">
                                <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z"></path>
                                </svg>
                            </button>
                            <div id="more-dropdown" class="hidden absolute right-0 mt-2 w-56 card shadow-[var(--shadow-overlay)] z-50 py-1">
                                {% if features.show_shopping_list || features.show_pantry %}
                                <a href="{{ prefix }}/{% if static_mode %}index.html{% endif %}" class="menu-item {% if active == "recipes" %}active{% endif %}">
                                    <span>🍳</span> <span>{{ tr.t("nav-recipes") }}</span>
                                </a>
                                {% if features.show_shopping_list && !static_mode %}
                                <a href="{{ prefix }}/shopping-list" class="menu-item {% if active == "shopping" %}active{% endif %}">
                                    <span>🛒</span> <span>{{ tr.t("nav-shopping-list") }}</span>
                                </a>
                                {% endif %}
                                {% if features.show_pantry && !static_mode %}
                                <a href="{{ prefix }}/pantry" class="menu-item {% if active == "pantry" %}active{% endif %}">
                                    <span>🥫</span> <span>{{ tr.t("nav-pantry") }}</span>
                                </a>
                                {% endif %}
                                <div class="border-t border-line my-1"></div>
                                {% endif %}
                                {% if !static_mode %}
                                <a href="{{ prefix }}/preferences" class="menu-item">
                                    <span>⚙️</span> <span>Preferences</span>
                                </a>
                                {% endif %}
                                <button onclick="showShortcutsHelp(); document.getElementById('more-dropdown').classList.add('hidden');" class="menu-item">
                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>
                                    <span>Keyboard Shortcuts</span>
                                </button>
                                <button onclick="toggleTheme(); document.getElementById('more-dropdown').classList.add('hidden');" class="menu-item">
                                    <svg class="w-4 h-4 hidden dark:block" fill="currentColor" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M10 2a1 1 0 011 1v1a1 1 0 11-2 0V3a1 1 0 011-1zm4 8a4 4 0 11-8 0 4 4 0 018 0zm-.464 4.95l.707.707a1 1 0 001.414-1.414l-.707-.707a1 1 0 00-1.414 1.414zm2.12-10.607a1 1 0 010 1.414l-.706.707a1 1 0 11-1.414-1.414l.707-.707a1 1 0 011.414 0zM17 11a1 1 0 100-2h-1a1 1 0 100 2h1zm-7 4a1 1 0 011 1v1a1 1 0 11-2 0v-1a1 1 0 011-1zM5.05 6.464A1 1 0 106.465 5.05l-.708-.707a1 1 0 00-1.414 1.414l.707.707zm1.414 8.486l-.707.707a1 1 0 01-1.414-1.414l.707-.707a1 1 0 011.414 1.414zM4 11a1 1 0 100-2H3a1 1 0 000 2h1z" clip-rule="evenodd"></path></svg>
                                    <svg class="w-4 h-4 block dark:hidden" fill="currentColor" viewBox="0 0 20 20"><path d="M17.293 13.293A8 8 0 016.707 2.707a8.001 8.001 0 1010.586 10.586z"></path></svg>
                                    <span>Toggle Theme</span>
                                </button>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </nav>

        <main>
            {% block content %}{% endblock %}
        </main>

        {% if static_mode %}
        <footer class="mt-12 mb-6 text-center text-sm text-faint print:hidden">
            Built with
            <a href="https://cooklang.org/cli/" class="text-accent-text hover:underline" target="_blank" rel="noopener">CookCLI</a>
            {% if let Some(url) = repo_url %}
            &middot;
            <a href="{{ url }}" class="text-accent-text hover:underline" target="_blank" rel="noopener">View source</a>
            {% endif %}
        </footer>
        {% endif %}
    </div>
```

- [ ] **Step 3: Update the server-mode search result strings**

In the inline search script (inside `{% if !static_mode %}` after the theme toggle script), change the two rendering lines:

```js
                        searchResults.innerHTML = '<div class="p-4 text-muted text-center">' + translations.noRecipes + '</div>';
```

and

```js
                            `<a href="{{ prefix }}/recipe/${recipe.path}" class="search-result block px-4 py-3 hover:bg-sunk border-b border-line last:border-b-0">
                                <div class="font-medium text-text">${recipe.name}</div>
```

- [ ] **Step 4: Build, run the navigation and search specs**

Run: `npm run build-css && npm test -- --project=chromium tests/e2e/navigation.spec.ts tests/e2e/search.spec.ts`
Expected: all pass. (`navigation.spec.ts` still selects `h3` on recipe cards; that changes in Task 3.)

- [ ] **Step 5: Look at it**

Open `http://localhost:9080/` at 1440px and 820px, light and dark. The nav is the same rounded card at the same height as main, the search field is full width, the pills sit where they did. Dark mode: nav card is `#1c1c24` on a `#16161d` page.

- [ ] **Step 6: Commit**

```bash
git add templates/base.html
git commit -q -F - <<'EOF'
refactor(ui): move the nav card and search onto tokens

Claude-Session: https://claude.ai/code/session_013urND2B6Y3Z7WQuDpE8ZDu
EOF
```

---

### Task 3: Recipes index

**Files:**
- Modify: `templates/recipes.html`
- Modify: `tests/e2e/navigation.spec.ts:29,142`
- Create: `tests/e2e/recipes-sort.spec.ts`

- [ ] **Step 1: Retarget the navigation spec at the card title class**

In `tests/e2e/navigation.spec.ts` change both occurrences of

```ts
      const recipeName = await firstRecipe.locator('h3').textContent();
```
(line 29) and
```ts
    const recipeName = await simpleRecipeCard.locator('h3').textContent();
```
(line 142) to use `.locator('.recipe-card-title')` instead of `.locator('h3')`.

- [ ] **Step 2: Add the sorter spec**

Create `tests/e2e/recipes-sort.spec.ts`:

```ts
import { test, expect, Page } from '@playwright/test';
import { TestHelpers } from '../fixtures/test-helpers';

// Assertions read data-name rather than the card heading so these tests do
// not re-create the markup coupling the sorter itself was fixed to avoid.
const recipeNames = (page: Page) =>
  page.locator('#recipes-grid [data-type="recipe"]').evaluateAll((els) =>
    els.map((el) => el.getAttribute('data-name') ?? ''),
  );

const allNames = (page: Page) =>
  page.locator('#recipes-grid > [data-type]').evaluateAll((els) =>
    els.map((el) => `${el.getAttribute('data-type')}:${el.getAttribute('data-name')}`),
  );

test.describe('Recipes index sorting', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ page }) => {
    helpers = new TestHelpers(page);
    await helpers.navigateTo('/');
    await page.evaluate(() => sessionStorage.removeItem('recipes-sort'));
    await page.reload();
  });

  test('controls are visible and default to name ascending', async ({ page }) => {
    await expect(page.locator('#sort-controls')).toBeVisible();
    await expect(page.locator('#sort-field')).toHaveValue('name');
    await expect(page.locator('#sort-dir')).toHaveText('↑');

    const names = await recipeNames(page);
    expect(names.length).toBeGreaterThan(1);
    const sorted = [...names].sort((a, b) =>
      a.localeCompare(b, undefined, { numeric: true, sensitivity: 'base' }),
    );
    expect(names).toEqual(sorted);
  });

  test('direction toggle reverses the order', async ({ page }) => {
    const asc = await recipeNames(page);
    await page.locator('#sort-dir').click();
    await expect(page.locator('#sort-dir')).toHaveText('↓');

    const desc = await recipeNames(page);
    expect(desc).toEqual([...asc].reverse());
  });

  test('sorting by modified date defaults to newest first', async ({ page }) => {
    const byName = await recipeNames(page);
    const timestamps = await page
      .locator('#recipes-grid [data-type="recipe"]')
      .evaluateAll((els) => els.map((el) => Number(el.getAttribute('data-modified'))));
    // A fresh checkout gives every seed file the same mtime (second precision),
    // so "reorders" is only meaningful when at least two values differ.
    const distinct = new Set(timestamps).size > 1;

    await page.locator('#sort-field').selectOption('modified');
    await expect(page.locator('#sort-dir')).toHaveText('↓');

    const newestFirst = await recipeNames(page);
    expect([...newestFirst].sort()).toEqual([...byName].sort()); // same set

    const sorted = await page
      .locator('#recipes-grid [data-type="recipe"]')
      .evaluateAll((els) => els.map((el) => Number(el.getAttribute('data-modified'))));
    expect(sorted).toEqual([...sorted].sort((a, b) => b - a));
    if (distinct) {
      expect(newestFirst).not.toEqual(byName);
    }
  });

  test('directories stay grouped above recipes in both directions', async ({ page }) => {
    for (const _ of [0, 1]) {
      const entries = await allNames(page);
      const lastDir = entries.map((e) => e.startsWith('directory:')).lastIndexOf(true);
      const firstRecipe = entries.findIndex((e) => e.startsWith('recipe:'));
      if (lastDir !== -1 && firstRecipe !== -1) {
        expect(lastDir).toBeLessThan(firstRecipe);
      }
      await page.locator('#sort-dir').click();
    }
  });

  test('sort choice survives a reload', async ({ page }) => {
    await page.locator('#sort-dir').click();
    const before = await recipeNames(page);

    await page.reload();

    await expect(page.locator('#sort-dir')).toHaveText('↓');
    await expect(page.locator('#sort-field')).toHaveValue('name');
    expect(await recipeNames(page)).toEqual(before);
  });

  test('corrupt saved state falls back to defaults instead of throwing', async ({ page }) => {
    const pageErrors: string[] = [];
    page.on('pageerror', (e) => pageErrors.push(e.message));

    await page.evaluate(() => sessionStorage.setItem('recipes-sort', '{not json'));
    await page.reload();

    await expect(page.locator('#sort-controls')).toBeVisible();
    await expect(page.locator('#sort-field')).toHaveValue('name');
    await expect(page.locator('#sort-dir')).toHaveText('↑');

    // Prove the sorter script itself kept running past the corrupt value,
    // not just that the controls rendered.
    const before = await recipeNames(page);
    await page.locator('#sort-dir').click();
    await expect(page.locator('#sort-dir')).toHaveText('↓');
    expect(await recipeNames(page)).toEqual([...before].reverse());

    expect(pageErrors).toEqual([]);
  });

  test('controls stay hidden when there is nothing to sort', async ({ page }) => {
    await helpers.navigateTo('/directory/Salads');
    const recipes = await page.locator('#recipes-grid [data-type="recipe"]').count();
    expect(recipes).toBeLessThan(2);
    await expect(page.locator('#sort-controls')).toBeHidden();
  });

  test('direction toggle is labelled for assistive tech', async ({ page }) => {
    const label = await page.locator('#sort-dir').getAttribute('aria-label');
    expect(label).toBeTruthy();
  });
});
```

- [ ] **Step 3: Run both specs to see them fail**

Run: `npm test -- --project=chromium tests/e2e/navigation.spec.ts tests/e2e/recipes-sort.spec.ts`
Expected: navigation fails on `.recipe-card-title` (does not exist yet); the sort spec fails on `data-name`, the reload test and the aria-label test.

- [ ] **Step 4: Rewrite `templates/recipes.html`**

Replace the whole file with:

```html
{% extends "base.html" %}

{% block title %}{{ tr.t("nav-recipes") }} - Cook{% endblock %}

{% block content %}
<div>
    {% if !breadcrumbs.is_empty() %}
    <nav class="mb-4">
        <ol class="flex items-center space-x-2 text-sm text-muted">
            <li><a href="{{ prefix }}/{% if static_mode %}index.html{% endif %}" class="hover:text-text">{{ tr.t("nav-recipes") }}</a></li>
            {% for crumb in breadcrumbs %}
            <li class="flex items-center">
                <span class="mx-2 text-faint">/</span>
                <a href="{{ prefix }}/directory/{{ crumb.path }}{% if static_mode %}.html{% endif %}" class="hover:text-text">{{ crumb.name }}</a>
            </li>
            {% endfor %}
        </ol>
    </nav>
    {% endif %}

    <div class="flex items-center justify-between mb-8">
        <h1 class="text-display font-bold text-text">
            {{ current_name }}
        </h1>
        {% if !static_mode %}
        <a href="{{ new_recipe_url }}" class="btn btn-primary">
            <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
            </svg>
            {{ tr.t("new-recipe") }}
        </a>
        {% endif %}
    </div>

    <div id="sort-controls" class="hidden items-center gap-2 mb-6">
        <label for="sort-field" class="text-sm text-muted">{{ tr.t("sort-by") }}</label>
        <select id="sort-field" class="select">
            <option value="name">{{ tr.t("sort-name") }}</option>
            <option value="modified">{{ tr.t("sort-modified") }}</option>
            <option value="created" id="sort-created-option">{{ tr.t("sort-created") }}</option>
        </select>
        <button id="sort-dir" type="button" class="btn" aria-label="{{ tr.t("sort-direction-toggle") }}" title="{{ tr.t("sort-direction-toggle") }}">↑</button>
    </div>

    {% match todays_menu %}
    {% when Some with (menu) %}
    <div class="mb-8 card p-6 border-l-[3px] border-l-accent">
        <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
            <div class="flex-1">
                <div class="flex items-center gap-3 mb-2">
                    <span class="text-2xl">📋</span>
                    <h2 class="text-title font-bold text-text">{{ tr.t("todays-menu-title") }}</h2>
                    <span class="text-sm text-faint">{{ menu.date_display }}</span>
                </div>
                <p class="text-sm text-muted">{{ tr.t("todays-menu-from") }}: {{ menu.menu_name }}</p>
            </div>
            <a href="{{ prefix }}/{% if static_mode %}menu{% else %}recipe{% endif %}/{{ menu.menu_path }}{% if static_mode %}.html{% endif %}" class="btn self-start sm:self-center">
                {{ tr.t("todays-menu-view") }}
                <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"></path>
                </svg>
            </a>
        </div>
    </div>
    {% when None %}
    {% endmatch %}

    <div id="recipes-grid" class="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
        {% for item in items %}
        {% if item.is_directory %}
        <a href="{{ prefix }}/directory/{{ item.path }}{% if static_mode %}.html{% endif %}" data-type="directory" data-name="{{ item.name }}" class="recipe-card">
            <div class="p-6 flex-1">
                <div class="recipe-card-icon">
                    <span>📁</span>
                </div>
                <h2 class="recipe-card-title">{{ item.name }}</h2>
                {% match item.count %}
                {% when Some with (count) %}
                <span class="text-sm text-accent-text font-medium">{{ tr.tn("recipes-count", count) }}</span>
                {% when None %}
                {% endmatch %}
            </div>
        </a>
        {% else %}
        <a href="{{ prefix }}/{% if static_mode %}{% if item.is_menu %}menu{% else %}recipe{% endif %}{% else %}recipe{% endif %}/{{ item.path }}{% if static_mode %}.html{% endif %}"
           data-type="recipe"
           data-name="{{ item.name }}"
           {% if let Some(ts) = item.modified_at %}data-modified="{{ ts }}"{% endif %}
           {% if let Some(ts) = item.created_at %}data-created="{{ ts }}"{% endif %}
           class="recipe-card">
            {% match item.image_path %}
            {% when Some with (img) %}
            <div class="h-48 bg-sunk overflow-hidden">
                <img src="{{ img }}" alt="{{ item.name }}" class="w-full h-full object-cover">
            </div>
            <div class="p-6">
            {% when None %}
            <div class="p-6">
                <div class="recipe-card-icon">
                    <span>{% if item.is_menu %}📋{% else %}🍽️{% endif %}</span>
                </div>
            {% endmatch %}
                <h2 class="recipe-card-title">{{ item.name }}</h2>
                {% if item.is_menu %}
                <span class="tag mb-2 inline-block">{{ tr.t("recipe-type-menu") }}</span>
                {% endif %}
                {% match item.description %}
                {% when Some with (description) %}
                <p class="text-muted text-sm">{{ description }}</p>
                {% when None %}
                {% endmatch %}
                {% if !item.tags.is_empty() %}
                <div class="mt-3 flex flex-wrap gap-2">
                    {% for tag in item.tags.iter().take(3) %}
                    <span class="tag">{{ tag }}</span>
                    {% endfor %}
                    {% if item.tags.len() > 3 %}
                    <span class="text-xs text-faint">+{{ item.tags.len() - 3 }}</span>
                    {% endif %}
                </div>
                {% endif %}
            </div>
        </a>
        {% endif %}
        {% endfor %}

        {% if items.is_empty() %}
        <div class="text-faint text-center py-8">
            {{ tr.t("recipes-empty") }}
        </div>
        {% endif %}
    </div>
</div>
{% endblock %}

{% block scripts %}
<script>
(function () {
    const grid = document.getElementById('recipes-grid');
    const controls = document.getElementById('sort-controls');
    const fieldSelect = document.getElementById('sort-field');
    const dirBtn = document.getElementById('sort-dir');
    const createdOption = document.getElementById('sort-created-option');

    if (!grid || !controls || !fieldSelect || !dirBtn) return;

    // Sorting is client-side because this template is also rendered by the
    // static site builder, where there is no server to answer a ?sort= query.
    const STORE_KEY = 'recipes-sort';

    let sortField = 'name';
    let sortDir = 'asc';

    const recipeCards = Array.from(grid.querySelectorAll('[data-type="recipe"]'));

    // "Created" is only offered when every recipe has a creation date;
    // otherwise the column is partly empty and the ordering is arbitrary.
    const allHaveCreated = recipeCards.length > 0 &&
        recipeCards.every(el => el.hasAttribute('data-created'));
    if (!allHaveCreated && createdOption) {
        createdOption.remove();
    }

    // Nothing to sort with fewer than two recipes.
    if (recipeCards.length < 2) return;
    controls.classList.remove('hidden');
    controls.classList.add('flex');

    function restore() {
        let saved = null;
        try {
            saved = JSON.parse(sessionStorage.getItem(STORE_KEY) || 'null');
        } catch (e) {
            // Corrupt value: fall through to the defaults.
        }
        if (!saved) return;
        // The saved field may not exist here — "created" is removed on pages
        // where not every recipe has a date. Treat an invalid field as fully
        // corrupt so a stale direction cannot pair with the wrong field.
        const valid = Array.from(fieldSelect.options).some(o => o.value === saved.field);
        if (!valid) return;
        sortField = saved.field;
        fieldSelect.value = saved.field;
        if (saved.dir === 'asc' || saved.dir === 'desc') {
            sortDir = saved.dir;
        }
    }

    function persist() {
        try {
            sessionStorage.setItem(STORE_KEY, JSON.stringify({ field: sortField, dir: sortDir }));
        } catch (e) {
            // Private browsing or a full quota: sorting still works, it just
            // does not survive navigation.
        }
    }

    function getValue(el) {
        if (sortField === 'name') {
            // data-name rather than the heading text, so a markup change
            // cannot silently break sorting.
            return (el.getAttribute('data-name') || '').trim();
        }
        const raw = el.getAttribute(sortField === 'modified' ? 'data-modified' : 'data-created');
        if (raw === null) return null;
        const n = parseInt(raw, 10);
        return Number.isNaN(n) ? null : n;
    }

    // numeric so "Recipe 10" sorts after "Recipe 9"; sensitivity 'base' so
    // case and accents do not split otherwise-equal names.
    const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' });

    function compare(a, b) {
        const av = getValue(a);
        const bv = getValue(b);
        // Missing values sort last in both directions.
        if (av === null && bv === null) return 0;
        if (av === null) return 1;
        if (bv === null) return -1;
        const cmp = typeof av === 'string' ? collator.compare(av, bv) : av - bv;
        return sortDir === 'asc' ? cmp : -cmp;
    }

    function applySort() {
        const all = Array.from(grid.children);
        const dirs = all.filter(el => el.dataset.type === 'directory');
        const recipes = all.filter(el => el.dataset.type === 'recipe');
        const others = all.filter(el => !el.dataset.type);

        recipes.sort(compare);
        // Directories stay grouped above recipes, sorted by name among
        // themselves so their order is deterministic. They only follow
        // sortDir when sorting by name; for date fields they stay A→Z since
        // directories have no modified/created date of their own.
        dirs.sort((a, b) => {
            const cmp = collator.compare(a.getAttribute('data-name') || '', b.getAttribute('data-name') || '');
            return sortField === 'name' && sortDir === 'desc' ? -cmp : cmp;
        });

        const frag = document.createDocumentFragment();
        dirs.forEach(el => frag.appendChild(el));
        recipes.forEach(el => frag.appendChild(el));
        others.forEach(el => frag.appendChild(el));
        grid.appendChild(frag);

        dirBtn.textContent = sortDir === 'asc' ? '↑' : '↓';
    }

    fieldSelect.addEventListener('change', function () {
        sortField = this.value;
        // Dates default to newest first; names to A–Z.
        sortDir = sortField === 'name' ? 'asc' : 'desc';
        applySort();
        persist();
    });

    dirBtn.addEventListener('click', function () {
        sortDir = sortDir === 'asc' ? 'desc' : 'asc';
        applySort();
        persist();
    });

    restore();
    applySort();
})();
</script>
{% endblock %}
```

- [ ] **Step 5: Run the specs**

Run: `npm run build-css && npm test -- --project=chromium tests/e2e/navigation.spec.ts tests/e2e/recipes-sort.spec.ts`
Expected: all pass.

- [ ] **Step 6: Look at it**

`http://localhost:9080/` at 1440px: three columns of tall cards, 64px icon discs, 192px image bands, same card heights as main. Dark mode cards are `#1c1c24` with a `#30303b` hairline.

- [ ] **Step 7: Commit**

```bash
git add templates/recipes.html tests/e2e/navigation.spec.ts tests/e2e/recipes-sort.spec.ts
git commit -q -F - <<'EOF'
refactor(ui): recipes index on tokens, sorter reads data-name

The sorter now uses Intl.Collator with numeric collation and persists
the choice in sessionStorage. Card headings are h2 so the outline has
no skipped level; the E2E suite selects .recipe-card-title.

Claude-Session: https://claude.ai/code/session_013urND2B6Y3Z7WQuDpE8ZDu
EOF
```

---

### Task 4: Shared scripts

**Files:**
- Modify: `static/js/keyboard-shortcuts.js`
- Modify: `static/js/search.js`
- Modify: `static/js/cooking-mode.js`

- [ ] **Step 1: Export `adjustScale` and retint the shortcuts modal**

In `static/js/keyboard-shortcuts.js`, directly after the closing brace of `function adjustScale(delta) { … }` (around line 376), add:

```js
    // The recipe page's −/+ stepper buttons call this too, so the clamping,
    // rounding and no-op guard live in exactly one place.
    window.adjustScale = adjustScale;
```

Then in `window.showShortcutsHelp` make these replacements (each is a whole-line replacement of the class string):

| Old | New |
|---|---|
| `const kbd = 'class="px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-sm font-mono"';` | `const kbd = 'class="px-2 py-1 bg-sunk rounded text-sm font-mono"';` |
| `<span class="text-gray-600 dark:text-gray-400">${label}</span>` | `<span class="text-muted">${label}</span>` |
| `<h3 class="font-semibold text-gray-900 dark:text-white mb-3">Shopping List</h3>` | `<h3 class="font-semibold text-text mb-3">Shopping List</h3>` |
| `<div class="bg-white dark:bg-gray-800 rounded-2xl shadow-xl max-w-2xl w-full mx-4 max-h-[80vh] overflow-hidden">` | `<div class="card shadow-[var(--shadow-overlay)] max-w-2xl w-full mx-4 max-h-[80vh] overflow-hidden">` |
| `<div class="p-6 border-b border-gray-200 dark:border-gray-700 flex justify-between items-center">` | `<div class="p-6 border-b border-line flex justify-between items-center">` |
| `<h2 class="text-xl font-bold text-gray-900 dark:text-white">Keyboard Shortcuts</h2>` | `<h2 class="text-title font-bold text-text">Keyboard Shortcuts</h2>` |
| `<button onclick="closeShortcutsHelp()" class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">` | `<button onclick="closeShortcutsHelp()" class="icon-btn" aria-label="Close">` |
| `<h3 class="font-semibold text-gray-900 dark:text-white mb-3">Navigation</h3>` | `<h3 class="font-semibold text-text mb-3">Navigation</h3>` |
| `<h3 class="font-semibold text-gray-900 dark:text-white mb-3">General</h3>` | `<h3 class="font-semibold text-text mb-3">General</h3>` |
| `<h3 class="font-semibold text-gray-900 dark:text-white mb-3">Recipe Page</h3>` | `<h3 class="font-semibold text-text mb-3">Recipe Page</h3>` |
| `<div class="p-4 border-t border-gray-200 dark:border-gray-700 text-center text-sm text-gray-500 dark:text-gray-400">` | `<div class="p-4 border-t border-line text-center text-sm text-faint">` |
| `Press <kbd class="px-1 py-0.5 bg-gray-100 dark:bg-gray-700 rounded text-xs font-mono">Esc</kbd> to close` | `Press <kbd class="px-1 py-0.5 bg-sunk rounded text-xs font-mono">Esc</kbd> to close` |

The close button's inner `<svg class="w-6 h-6" …>` keeps its size classes; `.icon-btn svg` is overridden by the utility, which is fine.

- [ ] **Step 2: Retint the static-mode search results**

In `static/js/search.js`, inside `render(matches)`, change the two strings:

```js
      results.innerHTML = '<div class="p-4 text-muted text-center">No recipes found</div>';
```

```js
        return '<a href="' + escapeHtml(href) + '" class="search-result block px-4 py-3 hover:bg-sunk border-b border-line last:border-b-0">' +
          '<div class="font-medium text-text">' + escapeHtml(m.title) + '</div>' +
```

Do not touch `loadIndex()`; main's `<script>`-tag loader is the correct one.

- [ ] **Step 3: Stop cook mode scraping layout utilities**

In `static/js/cooking-mode.js`, `captureStepHTML()`:

```js
    function captureStepHTML() {
        const stepElements = [];
        const sectionEls = document.querySelectorAll('ol.step-list');
        sectionEls.forEach(function(ol) {
            ol.querySelectorAll(':scope > li').forEach(function(li) {
                const textDiv = li.querySelector('.step-body');
                if (textDiv) {
                    stepElements.push(textDiv.innerHTML);
                }
            });
        });
        return stepElements;
    }
```

`ol.step-list` and `.step-body` do not exist until Task 5, so cook mode captures nothing between this commit and the next. That is why Tasks 4 and 5 are committed back to back.

- [ ] **Step 4: Verify the palette scan is clean for these files**

Run: `grep -nE 'gray-|orange-|purple-|pink-|dark:' static/js/keyboard-shortcuts.js static/js/search.js static/js/cooking-mode.js || echo clean`
Expected: `clean`.

- [ ] **Step 5: Commit**

```bash
git add static/js/keyboard-shortcuts.js static/js/search.js static/js/cooking-mode.js
git commit -q -F - <<'EOF'
refactor(ui): shared scripts on tokens, export adjustScale

Cook mode captures steps from ol.step-list/.step-body instead of the
layout utilities it used to scrape.

Claude-Session: https://claude.ai/code/session_013urND2B6Y3Z7WQuDpE8ZDu
EOF
```

---

### Task 5: Recipe page

**Files:**
- Modify: `templates/recipe.html`
- Modify: `tests/e2e/recipe-display.spec.ts:109-170`

- [ ] **Step 1: Retarget the recipe-display spec**

In `tests/e2e/recipe-display.spec.ts`, replace the body of the metadata test (the block starting `// Check for metadata pills` through the end of the `if (await metadataPills.count() > 0) { … }` block, lines 112–130) with:

```ts
    // Pills keep the .metadata-pill class. No count guard: Easy Pancakes
    // declares metadata, so zero pills is a failure, not a skip.
    const metadataPills = page.locator('.metadata-pill');
    expect(await metadataPills.count()).toBeGreaterThan(0);
    await expect(metadataPills.first()).toBeVisible();

    const metadataText = (await metadataPills.allTextContents()).join(' ').toLowerCase();
    expect(metadataText).toContain('2');
    expect(metadataText).toContain('servings');
    expect(metadataText).toContain('5 min');
    expect(metadataText).toContain('20 min');
    expect(metadataText).toContain('cookcli team');
```

Then change three selectors further down in the same file:

| Line | Old | New |
|---|---|---|
| 139 | `page.locator('ul.space-y-3 li')` | `page.locator('ul.ingredient-list li')` |
| 148 | `ingredientWithNote.locator('span.italic.text-gray-600')` | `ingredientWithNote.locator('span.row-note')` |
| 164 | `page.locator('.text-sm.text-gray-600.mt-2')` | `page.locator('.step-refs')` |

- [ ] **Step 2: Run the spec to see it fail**

Run: `npm test -- --project=chromium tests/e2e/recipe-display.spec.ts`
Expected: the notes test fails on `ul.ingredient-list li` (count 0).

- [ ] **Step 3: Rewrite the markup section of `templates/recipe.html`**

Replace everything from `{% block content %}` up to (not including) `<script id="cooking-mode-data"` with:

```html
{% block content %}
<script type="application/ld+json">{{ self.recipe_jsonld()|safe }}</script>
<div>
    <nav class="mb-4 breadcrumb print:hidden">
        <ol class="flex items-center space-x-2 text-sm text-muted">
            <li><a href="{{ prefix }}/{% if static_mode %}index.html{% endif %}" class="hover:text-text">{{ tr.t("nav-recipes") }}</a></li>
            {% for crumb in breadcrumbs %}
            <li class="flex items-center">
                <span class="mx-2 text-faint">/</span>
                {% if loop.last %}
                    <span class="text-text">{{ crumb }}</span>
                {% else %}
                    <a href="{{ prefix }}/directory/{{ breadcrumbs[..loop.index0 + 1].join("/") }}{% if static_mode %}.html{% endif %}" class="hover:text-text">{{ crumb }}</a>
                {% endif %}
            </li>
            {% endfor %}
        </ol>
    </nav>

    <div class="mb-8">
        <!-- Recipe image if available -->
        {% match image_path %}
        {% when Some with (img) %}
        <div class="mb-6 max-w-4xl mx-auto flex justify-center">
            <img src="{{ img }}" alt="{{ recipe.name }}" class="rounded-xl shadow-[var(--shadow-card)] max-w-full h-auto max-h-[500px]">
        </div>
        {% when None %}
        {% endmatch %}

        <!-- Title bar with scale and shopping list button -->
        <div class="flex flex-col md:flex-row md:items-center md:justify-between gap-4 mb-6 print:flex-row print:items-center">
            <h1 class="text-display font-bold text-text print:text-2xl">
                {{ recipe.name }}
            </h1>
            {% if scale != 1.0 %}
            <div class="hidden print:block text-lg font-normal text-muted">{{ tr.t("recipe-scale-label") }}: {{ scale }}x</div>
            {% endif %}
            <div class="flex flex-wrap items-center gap-2 lg:gap-3 print:hidden">
                {% if !static_mode %}
                <div class="flex items-center">
                    <label for="scale" class="text-sm font-medium text-muted mr-2">{{ tr.t("recipe-scale-label") }}:</label>
                    <div class="stepper">
                        <button type="button" aria-label="Decrease scale" onclick="adjustScale(-0.5)">&minus;</button>
                        <input type="number"
                               id="scale"
                               value="{{ scale }}"
                               min="0.5"
                               max="200"
                               step="0.5"
                               onchange="goToScale(this.value)">
                        <button type="button" aria-label="Increase scale" onclick="adjustScale(0.5)">+</button>
                    </div>
                </div>
                <a href="{{ prefix }}/edit/{{ recipe_path }}" class="btn" title="{{ tr.t("action-edit") }}">
                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"></path>
                    </svg>
                    <span>{{ tr.t("action-edit") }}</span>
                </a>
                <button onclick="addToShoppingList(event, {{ recipe_path|json }})" class="btn" title="{{ tr.t("recipe-add-to-shopping") }}">
                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 3h2l.4 2M7 13h10l4-8H5.4M7 13L5.4 5M7 13l-2.293 2.293c-.63.63-.184 1.707.707 1.707H17m0 0a2 2 0 100 4 2 2 0 000-4zm-8 2a2 2 0 11-4 0 2 2 0 014 0z"></path>
                    </svg>
                    <span>{{ tr.t("recipe-add-to-shopping") }}</span>
                </button>
                {% endif %}
                <button id="start-cooking-btn" onclick="startCookingMode()" class="btn btn-primary" title="Start Cooking">
                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"></path>
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                    </svg>
                    <span>Cook</span>
                </button>
                {% if static_mode %}
                <a href="{{ prefix }}/recipe/{{ recipe_path }}.cook" download class="btn" title="Download .cook source">
                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v2a2 2 0 002 2h12a2 2 0 002-2v-2M7 10l5 5 5-5M12 15V3"></path>
                    </svg>
                    <span>.cook</span>
                </a>
                {% endif %}
            </div>
        </div>

        <!-- Recipe info section -->
        <div class="space-y-4">
            <!-- Tags row -->
            {% if !tags.is_empty() %}
            <div class="flex flex-wrap gap-2">
                {% for tag in tags %}
                <span class="tag">#{{ tag }}</span>
                {% endfor %}
            </div>
            {% endif %}

            {% match recipe.metadata %}
            {% when Some with (metadata) %}

            <!-- Description if present -->
            {% match metadata.description %}
            {% when Some with (description) %}
            <div class="recipe-note">
                <p class="m-0">{{ description }}</p>
            </div>
            {% when None %}
            {% endmatch %}

            <!-- Metadata row -->
            <div id="metadata-container" class="flex flex-wrap gap-3">
                {% match metadata.servings %}
                {% when Some with (servings) %}
                <span class="metadata-pill metadata-servings">👥 {{ servings }} {{ tr.t("recipe-servings-label") }}</span>
                {% when None %}
                {% endmatch %}

                {% match metadata.time %}
                {% when Some with (time) %}
                <span class="metadata-pill metadata-time">⏱️ {{ time }}</span>
                {% when None %}
                {% endmatch %}

                {% match metadata.difficulty %}
                {% when Some with (difficulty) %}
                <span class="metadata-pill metadata-difficulty">📊 {{ difficulty }}</span>
                {% when None %}
                {% endmatch %}

                {% match metadata.course %}
                {% when Some with (course) %}
                <span class="metadata-pill metadata-course">🍽️ {{ course }}</span>
                {% when None %}
                {% endmatch %}

                {% match metadata.prep_time %}
                {% when Some with (prep_time) %}
                <span class="metadata-pill metadata-prep">⏱️ {{ tr.t("meta-prep-time") }}: {{ prep_time }}</span>
                {% when None %}
                {% endmatch %}

                {% match metadata.cook_time %}
                {% when Some with (cook_time) %}
                <span class="metadata-pill metadata-cook">🔥 {{ tr.t("meta-cook-time") }}: {{ cook_time }}</span>
                {% when None %}
                {% endmatch %}

                {% match metadata.cuisine %}
                {% when Some with (cuisine) %}
                <span class="metadata-pill metadata-cuisine">🌍 {{ cuisine }}</span>
                {% when None %}
                {% endmatch %}

                {% match metadata.diet %}
                {% when Some with (diet) %}
                <span class="metadata-pill metadata-diet">🥗 {{ diet }}</span>
                {% when None %}
                {% endmatch %}

                {% match metadata.author %}
                {% when Some with (author) %}
                <span class="metadata-pill metadata-author">👤 {{ author }}</span>
                {% when None %}
                {% endmatch %}

                {% match metadata.source %}
                {% when Some with (source) %}
                <span class="metadata-pill metadata-source">📖 {{ source }}</span>
                {% when None %}
                {% endmatch %}

                {% match metadata.source_url %}
                {% when Some with (source_url) %}
                <span class="metadata-pill metadata-source-url">🔗 <a href="{{ source_url }}" class="text-info hover:underline ml-1">{{ source_url|hostname }}</a></span>
                {% when None %}
                {% endmatch %}

                <!-- Custom metadata -->
                {% for (key, value) in metadata.custom %}
                <span class="metadata-pill metadata-custom">{{ key }}: {{ value }}</span>
                {% endfor %}
            </div>

            {% when None %}
            {% endmatch %}
        </div>
    </div>

    <div id="recipe-body" class="grid md:grid-cols-3 gap-8 mb-8">
        <div class="md:col-span-1">
            <div class="card p-6">
                <h2 class="text-title font-bold mb-4 text-accent-text">🥘 {{ tr.t("recipe-ingredients") }}</h2>
                {% if sections.len() > 1 %}
                <!-- Display ingredients grouped by section -->
                {% for section in sections %}
                {% if section.ingredients.len() > 0 %}
                {% match section.name %}
                {% when Some with (name) %}
                <h3 class="text-title font-semibold mt-4 mb-2 text-accent-text">{{ name }}</h3>
                {% when None %}
                {% if !loop.first %}<h3 class="text-title font-semibold mt-4 mb-2 text-accent-text">{{ tr.t("recipe-main-section") }}</h3>{% endif %}
                {% endmatch %}
                <ul class="ingredient-list space-y-3 {% if !loop.last %}mb-4{% endif %}">
                    {% for ingredient in section.ingredients %}
                    <li class="ingredient-row">
                        <div class="flex-1 flex items-center">
                            {% match ingredient.reference_path %}
                            {% when Some with (path) %}
                            {% if !static_mode %}
                            <input type="checkbox" checked
                                   class="ref-checkbox accent-[var(--accent)] mr-2 shrink-0"
                                   data-ref-path="{{ path }}"
                                   title="{{ tr.t("shopping-include-in-list") }}">
                            {% endif %}
                            <a href="{{ prefix }}/recipe/{{ path }}{% if static_mode %}.html{% endif %}" class="font-medium text-info hover:underline flex items-center gap-1">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"></path>
                                </svg>
                                {{ ingredient.name }}
                            </a>
                            {% when None %}
                            <span class="font-medium">{{ ingredient.name }}</span>
                            {% endmatch %}
                            {% match ingredient.note %}
                            {% when Some with (note) %}
                            <span class="row-note italic ml-1 break-words" aria-label="{{ tr.t("recipe-preparation") }}: {{ note }}">({{ note }})</span>
                            {% when None %}
                            {% endmatch %}
                        </div>
                        <span class="row-value ml-2">
                            {% match ingredient.quantity %}
                            {% when Some with (quantity) %}{{ quantity }}{% when None %}{% endmatch %}
                            {% match ingredient.unit %}
                            {% when Some with (unit) %} {{ unit }}{% when None %}{% endmatch %}
                        </span>
                    </li>
                    {% endfor %}
                </ul>
                {% endif %}
                {% endfor %}
                {% else %}
                <!-- Fallback to original flat list for recipes without sections -->
                <ul class="ingredient-list space-y-3">
                    {% for ingredient in ingredients %}
                    <li class="ingredient-row">
                        <div class="flex-1 flex items-center">
                            {% match ingredient.reference_path %}
                            {% when Some with (path) %}
                            {% if !static_mode %}
                            <input type="checkbox" checked
                                   class="ref-checkbox accent-[var(--accent)] mr-2 shrink-0"
                                   data-ref-path="{{ path }}"
                                   title="{{ tr.t("shopping-include-in-list") }}">
                            {% endif %}
                            <a href="{{ prefix }}/recipe/{{ path }}{% if static_mode %}.html{% endif %}" class="font-medium text-info hover:underline flex items-center gap-1">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"></path>
                                </svg>
                                {{ ingredient.name }}
                            </a>
                            {% when None %}
                            <span class="font-medium">{{ ingredient.name }}</span>
                            {% endmatch %}
                            {% match ingredient.note %}
                            {% when Some with (note) %}
                            <span class="row-note italic ml-1 break-words" aria-label="{{ tr.t("recipe-preparation") }}: {{ note }}">({{ note }})</span>
                            {% when None %}
                            {% endmatch %}
                        </div>
                        <span class="row-value ml-2">
                            {% match ingredient.quantity %}
                            {% when Some with (quantity) %}{{ quantity }}{% when None %}{% endmatch %}
                            {% match ingredient.unit %}
                            {% when Some with (unit) %} {{ unit }}{% when None %}{% endmatch %}
                        </span>
                    </li>
                    {% endfor %}
                </ul>
                {% endif %}

                {% if cookware.len() > 0 %}
                <h2 class="text-title font-bold mt-6 mb-4 text-text">🍳 {{ tr.t("recipe-cookware") }}</h2>
                <ul class="cookware-list ingredient-list space-y-2">
                    {% for item in cookware %}
                    <li class="ingredient-row">
                        <span class="font-medium">{{ item.name }}</span>
                    </li>
                    {% endfor %}
                </ul>
                {% endif %}
            </div>
        </div>

        <div class="md:col-span-2">
            <div class="card p-6">
                {% for section in sections %}
                {% match section.name %}
                {% when Some with (name) %}
                <h3 class="text-title font-semibold mt-6 mb-4 text-accent-text border-b border-line pb-2">{{ name }}</h3>
                {% when None %}
                {% endmatch %}
                <ol class="step-list space-y-4 {% if !loop.first %}mt-4{% endif %}">
                    {% for item in section.items %}
                    {% match item %}
                    {% when crate::web::templates::RecipeSectionItem::Step with (step) %}
                    <li class="step-box">
                        <div class="flex flex-col gap-4">
                            {% match step.image_path %}
                            {% when Some with (img) %}
                            <img class="image-step" src="{{ img }}" />
                            {% when None %}
                            {% endmatch %}
                            <div class="flex gap-4">
                                <div class="step-number">{{ step.number }}</div>
                                <div class="flex-1">
                                    <div class="step-body mb-2">
                                        {% for step_item in step.items %}
                                        {% match step_item %}
                                        {% when crate::web::templates::StepItem::Text with (text) %}{{ text }}{% when crate::web::templates::StepItem::Ingredient with { name, reference_path } %}{% match reference_path %}{% when Some with (path) %}<a href="{{ prefix }}/recipe/{{ path }}{% if static_mode %}.html{% endif %}" class="ingredient-badge hover:underline" title="View recipe: {{ name }}">{{ name }}</a>{% when None %}<span class="ingredient-badge">{{ name }}</span>{% endmatch %}{% when crate::web::templates::StepItem::Cookware with (name) %}<span class="cookware-badge">{{ name }}</span>{% when crate::web::templates::StepItem::Timer with (name) %}<span class="timer-badge">⏱️ {{ name }}</span>{% when crate::web::templates::StepItem::Quantity with (qty) %}<span class="font-bold text-accent-text">{{ qty }}</span>{% when crate::web::templates::StepItem::LineBreak %}<br>{% endmatch %}{% endfor %}
                                    </div>
                                    {% if step.ingredients.len() > 0 %}
                                    <div class="step-refs">
                                        {% for ing in step.ingredients %}
                                        <span class="inline-block mr-3">
                                            {{ ing.name }}{% match ing.quantity %}{% when Some with (q) %}: {{ q }}{% when None %}{% endmatch %}{% match ing.unit %}{% when Some with (u) %} {{ u }}{% when None %}{% endmatch %}{% match ing.note %}{% when Some with (note) %} <span class="italic text-faint break-words" aria-label="{{ tr.t("recipe-preparation") }}: {{ note }}">({{ note }})</span>{% when None %}{% endmatch %}{% if !loop.last %},{% endif %}
                                        </span>
                                        {% endfor %}
                                    </div>
                                    {% endif %}
                                </div>
                            </div>
                        </div>
                    </li>
                    {% when crate::web::templates::RecipeSectionItem::Note with (note) %}
                    <li class="recipe-note mb-3">
                        <span>📝</span>
                        <p class="m-0" style="white-space: pre-line">{{ note }}</p>
                    </li>
                    {% endmatch %}
                    {% endfor %}
                </ol>
                {% endfor %}
            </div>
        </div>
    </div>
</div>

```

Compare the `StepItem` line against main's before saving: it is identical except `text-orange-600` → `text-accent-text`. The `step.ingredients` line is identical except `text-gray-600` → `text-faint`.

- [ ] **Step 4: Update the recipe scripts**

Still in `templates/recipe.html`, inside the `{% if !static_mode %}<script>` block, replace `addToShoppingList` and `showRecipeError` with:

```js
async function addToShoppingList(event, recipePath) {
    const scale = document.getElementById('scale').value;
    // Hide any previous error
    const existingError = document.getElementById('recipe-error-banner');
    if (existingError) existingError.remove();

    // Collect checked recipe references
    const refCheckboxes = document.querySelectorAll('.ref-checkbox');
    let payload = {
        path: recipePath,
        scale: parseFloat(scale)
    };
    if (refCheckboxes.length > 0) {
        payload.included_references = Array.from(refCheckboxes)
            .filter(cb => cb.checked)
            .map(cb => cb.dataset.refPath);
    }

    try {
        const response = await fetch('{{ prefix }}/api/shopping_list/add', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(payload)
        });

        if (response.ok) {
            // Change button temporarily to show success
            const button = event.target.closest('button');
            const originalContent = button.innerHTML;
            button.innerHTML = `
                <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                </svg>
                {{ tr.t("recipe-added") }}
            `;
            button.classList.add('bg-ok-soft', 'text-ok', 'border-ok');

            // Reset after 2 seconds
            setTimeout(() => {
                button.innerHTML = originalContent;
                button.classList.remove('bg-ok-soft', 'text-ok', 'border-ok');
            }, 2000);
        } else {
            const data = await response.json().catch(() => ({}));
            showRecipeError(data.error || {{ tr.t("shopping-failed-to-add")|json|safe }});
        }
    } catch (error) {
        console.error('Failed to add to shopping list:', error);
        showRecipeError({{ tr.t("shopping-failed-to-add")|json|safe }});
    }
}

function showRecipeError(message) {
    const existing = document.getElementById('recipe-error-banner');
    if (existing) existing.remove();

    const banner = document.createElement('div');
    banner.id = 'recipe-error-banner';
    banner.className = 'mb-4 card p-4 border-l-[3px] border-l-danger bg-danger-soft';
    banner.innerHTML = `
        <div class="flex items-start gap-3">
            <div class="shrink-0 text-danger">
                <svg class="w-5 h-5 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z"></path>
                </svg>
            </div>
            <p class="flex-1 text-text text-sm font-mono whitespace-pre-wrap"></p>
            <button type="button" onclick="this.closest('#recipe-error-banner').remove()" class="icon-btn shrink-0" aria-label="Dismiss">
                <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                </svg>
            </button>
        </div>
    `;
    banner.querySelector('p').textContent = message;
    // Insert before the main content
    const content = document.getElementById('recipe-body');
    content.parentNode.insertBefore(banner, content);
}

// Scale navigation stashes the scroll position so a scale tap does not
// jump the reader back to the top of the recipe.
const SCALE_SCROLL_KEY = 'recipe-scale-scroll';
// |json puts the path in a JS string literal with <, >, & escaped as
// \uXXXX, so it survives the <script> context (Askama's HTML escaping
// would not be decoded here the way it is inside an attribute).
const RECIPE_URL = {{ prefix|json|safe }} + '/recipe/' + {{ recipe_path|json|safe }};
const DEFAULT_SCALE = {{ scale }};

function goToScale(value) {
    const input = document.getElementById('scale');
    const n = parseFloat(value);
    if (!Number.isFinite(n)) {
        // Cleared or non-numeric: put the current scale back, do not navigate.
        input.value = DEFAULT_SCALE;
        return;
    }
    const min = parseFloat(input.min) || 0.5;
    const max = parseFloat(input.max) || 200;
    const clamped = Math.min(max, Math.max(min, n));
    try {
        sessionStorage.setItem(SCALE_SCROLL_KEY, window.location.pathname + '|' + window.scrollY);
    } catch (e) { /* private mode / storage disabled */ }
    window.location.href = RECIPE_URL + '?scale=' + encodeURIComponent(clamped);
}

// Only ever set immediately before a scale navigation, cleared as soon as it
// is read, and only honoured on the same recipe path, so a stale stash from a
// failed navigation cannot scroll a different recipe.
(function restoreScaleScroll() {
    let stashed = null;
    try {
        stashed = sessionStorage.getItem(SCALE_SCROLL_KEY);
        if (stashed !== null) sessionStorage.removeItem(SCALE_SCROLL_KEY);
    } catch (e) { return; }
    if (stashed === null) return;
    const sep = stashed.lastIndexOf('|');
    if (sep === -1 || stashed.slice(0, sep) !== window.location.pathname) return;
    const y = parseInt(stashed.slice(sep + 1), 10);
    if (!Number.isFinite(y) || y <= 0) return;
    window.addEventListener('load', function () { window.scrollTo(0, y); });
})();
```

- [ ] **Step 5: Run the recipe specs and cook mode**

Run: `npm run build-css && npm test -- --project=chromium tests/e2e/recipe-display.spec.ts tests/e2e/recipe-scaling.spec.ts tests/e2e/cooking-mode.spec.ts`
Expected: all pass. If `recipe-scaling` fails on the input, check that `adjustScale` dispatches `change` (it does on main) and that `goToScale` is defined before the stepper is clicked.

- [ ] **Step 6: Look at it**

`http://localhost:9080/recipe/Neapolitan%20Pizza` at 1440px and 820px: 30px title, neutral emoji pills, the three-column grid with a boxed ingredient card, tinted rows, boxed steps with 32px step discs and 2.0 leading. Buttons show their labels at 820px. Press Cook: step cards are populated and the entity badges are readable on the dark card in light theme.

- [ ] **Step 7: Commit**

```bash
git add templates/recipe.html tests/e2e/recipe-display.spec.ts
git commit -q -F - <<'EOF'
refactor(ui): recipe page on tokens with scale stepper

Layout is unchanged from main. Scale changes keep the scroll position
and the stepper shares adjustScale with the keyboard shortcuts.

Claude-Session: https://claude.ai/code/session_013urND2B6Y3Z7WQuDpE8ZDu
EOF
```

---

### Task 6: Shopping list

**Files:**
- Modify: `templates/shopping_list.html` (markup block and four JS template-literal regions)

The page's list is rendered by JavaScript, so most substitutions are inside template literals. The DOM hooks the tests use (`#list-content li`, `label .item-name`, `data-action`, `.aisle-name`, `h3, h4`) all stay.

- [ ] **Step 1: Replace the markup block**

Replace everything from `{% block content %}` up to (not including) the first `<script>` with:

```html
{% block content %}
<div class="flex flex-col lg:flex-row gap-6">
    <!-- Sidebar with selected recipes and pantry -->
    <div class="lg:w-2/5 xl:w-1/3">
        <div class="card p-6 sticky top-6">
            <h2 class="font-bold text-title mb-3 text-accent-text">{{ tr.t("shopping-selected-recipes") }}</h2>
            <div id="selected-recipes" class="space-y-2 mb-6">
            </div>

            <!-- Pantry items section -->
            <div id="pantry-section" class="hidden">
                <h2 class="font-bold text-title mb-3 text-text">{{ tr.t("shopping-in-pantry") }}</h2>
                <div id="pantry-items" class="bg-sunk rounded-[var(--radius-control)] p-3 border border-line max-h-64 overflow-y-auto">
                </div>
            </div>
        </div>
    </div>

    <!-- Main content area with shopping list -->
    <div class="flex-1">
        <div id="error-banner" class="hidden mb-4 card p-4 border-l-[3px] border-l-danger bg-danger-soft">
            <div class="flex items-start gap-3">
                <div class="shrink-0 text-danger">
                    <svg class="w-5 h-5 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z"></path>
                    </svg>
                </div>
                <div class="flex-1">
                    <p id="error-message" class="text-text text-sm font-mono whitespace-pre-wrap"></p>
                </div>
                <button type="button" onclick="hideError()" class="icon-btn shrink-0" aria-label="Dismiss">
                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                    </svg>
                </button>
            </div>
        </div>

        <!-- Heading and list actions share a row; the heading lives here rather
             than in the JS-rendered list so the buttons stay put while the list
             below them re-renders. -->
        <div id="shopping-list-header" class="hidden flex-wrap items-center justify-between gap-3 mb-4">
            <h1 class="text-display font-bold text-text">{{ tr.t("shopping-title") }}</h1>
            <div class="flex items-center gap-2">
                <!-- Split button: the left half copies, the right half opens the
                     checkboxes that decide what the copied text contains. -->
                <div id="copy-list-group" class="hidden relative">
                    <div class="flex">
                        <button id="copy-list-button" type="button" onclick="copyList()" class="btn btn-primary rounded-r-none">
                            {{ tr.t("shopping-copy") }}
                        </button>
                        <button id="copy-options-toggle" type="button" onclick="toggleCopyOptions()"
                                aria-haspopup="true" aria-expanded="false" aria-controls="copy-options-menu"
                                aria-label="{{ tr.t("shopping-copy-options") }}"
                                class="btn btn-primary rounded-l-none -ml-px px-3 border-l-accent-ink/25">
                            <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path>
                            </svg>
                        </button>
                    </div>
                    <div id="copy-options-menu" class="hidden absolute right-0 mt-2 w-64 card shadow-[var(--shadow-overlay)] z-50 py-1">
                        <label class="flex items-center gap-3 px-4 py-2.5 text-sm text-text hover:bg-sunk cursor-pointer">
                            <input type="checkbox" id="copy-option-aisles" onchange="onCopyOptionChange()"
                                   class="w-4 h-4 accent-[var(--accent)]">
                            <span>{{ tr.t("shopping-copy-include-aisles") }}</span>
                        </label>
                        <label class="flex items-center gap-3 px-4 py-2.5 text-sm text-text hover:bg-sunk cursor-pointer">
                            <input type="checkbox" id="copy-option-amounts" onchange="onCopyOptionChange()"
                                   class="w-4 h-4 accent-[var(--accent)]">
                            <span>{{ tr.t("shopping-copy-include-amounts") }}</span>
                        </label>
                    </div>
                </div>
                <button type="button" onclick="clearList()" class="btn">
                    {{ tr.t("shopping-clear-all") }}
                </button>
            </div>
        </div>

        <div id="shopping-list-results">
            <div id="list-content"></div>
        </div>
    </div>
</div>

```

Note the page title becomes an `h1` (main had none on this page) and the sidebar headings become `h2`.

- [ ] **Step 2: Replace `renderSelectedRecipes`'s markup strings**

Inside `function renderSelectedRecipes()`, make these replacements:

```js
        container.innerHTML = '<p class="text-faint text-sm">' + escHtml({{ tr.t("shopping-no-recipes")|json|safe }}) + '</p>';
        document.getElementById('list-content').innerHTML = '<p class="text-faint">' + escHtml({{ tr.t("shopping-no-items")|json|safe }}) + '</p>';
```

Plan/menu entry (the `if (item.recipes)` branch):

```js
                const refsHtml = refs.length > 0 ? `
                    <ul class="mt-1 ml-8 pl-2 border-l border-line space-y-0.5">
                        ${refs.map(ref => {
                            const refName = ref.split('/').pop();
                            return `<li class="text-xs text-faint flex items-center gap-1">
                                <svg class="w-3 h-3 text-faint shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"></path>
                                </svg>
                                <a href="{{ prefix }}/recipe/${encodeRecipePath(ref)}" class="hover:text-accent-text hover:underline">${escHtml(refName)}</a>
                            </li>`;
                        }).join('')}
                    </ul>` : '';
                return `<li class="text-sm text-muted flex flex-col">
                    <div class="flex items-center gap-1">
                        <svg class="w-3 h-3 text-faint shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"></path>
                        </svg>
                        <a href="{{ prefix }}/recipe/${encodeRecipePath(recipe.path)}" class="hover:text-accent-text hover:underline">${escHtml(recipe.name)}</a>
                        <span class="text-faint text-xs">(×${escHtml(recipe.scale)})</span>
                    </div>
                    ${refsHtml}
                </li>`;
            }).join('');

            return `
            <div class="bg-sunk p-3 rounded-[var(--radius-control)] border border-line">
                <div class="flex items-center justify-between">
                    <div>
                        <a href="{{ prefix }}/recipe/${encodeRecipePath(item.path)}" class="font-medium text-text hover:text-accent-text underline decoration-[var(--border-strong)] hover:decoration-[var(--accent)]">${escHtml(item.name)}</a>
                        <span class="text-accent-text ml-2 text-xs font-semibold uppercase">plan</span>
                    </div>
                    <button type="button" data-action="remove-recipe" data-path="${escHtml(item.path)}" class="text-danger hover:underline font-medium text-sm">
                        {{ tr.t("shopping-remove") }}
                    </button>
                </div>
                <ul class="mt-2 ml-4 space-y-1">${recipesHtml}</ul>
            </div>`;
```

Regular recipe entry (after `// Regular recipe entry`):

```js
        const refsHtml = refs.length > 0 ? `
            <ul class="mt-1 ml-8 pl-2 border-l border-line space-y-0.5">
                ${refs.map(ref => {
                    const refName = ref.split('/').pop();
                    return `<li class="text-sm text-faint flex items-center gap-1">
                        <svg class="w-3 h-3 text-faint shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"></path>
                        </svg>
                        <a href="{{ prefix }}/recipe/${encodeRecipePath(ref)}" class="hover:text-accent-text hover:underline">${escHtml(refName)}</a>
                    </li>`;
                }).join('')}
            </ul>` : '';

        return `
        <div class="bg-sunk p-3 rounded-[var(--radius-control)] border border-line">
            <div class="flex items-center justify-between">
                <div>
                    <a href="{{ prefix }}/recipe/${encodeRecipePath(item.path)}" class="font-medium text-text hover:text-accent-text underline decoration-[var(--border-strong)] hover:decoration-[var(--accent)]">${escHtml(item.name)}</a>
                    <span class="text-muted ml-2 text-sm">(×${escHtml(item.scale)})</span>
                </div>
                <button type="button" data-action="remove-recipe" data-path="${escHtml(item.path)}" class="text-danger hover:underline font-medium text-sm">
                    {{ tr.t("shopping-remove") }}
                </button>
            </div>
            ${refsHtml}
        </div>`;
```

- [ ] **Step 3: Replace the aisle and pantry rendering in `displayShoppingList`**

The aisle block (`html += data.categories.map(category => …`):

```js
        html += data.categories.map(category => `
            <div class="aisle-section mb-6 card p-4">
                <h2 class="aisle-name font-semibold text-title mb-3 text-accent-text">${escHtml(category.category)}</h2>
                <ul class="space-y-2">
                    ${category.items.map((item, idx) => {
                        // Use ingredient name as the unique ID for localStorage
                        const itemId = `item-${item.name.replace(/\s+/g, '-')}`;
                        return `
                        <li class="flex items-center justify-between py-2 px-3 hover:bg-sunk rounded-[var(--radius-control)]">
                            <div class="flex items-center flex-1">
                                <input type="checkbox"
                                    id="${escHtml(itemId)}"
                                    class="list-checkbox accent-[var(--accent)] mr-3"
                                    data-action="toggle-item"
                                    data-item-id="${escHtml(itemId)}"
                                    data-ingredient-name="${escHtml(item.name)}">
                                <label for="${escHtml(itemId)}" class="cursor-pointer flex-1">
                                    <span class="item-name">${escHtml(item.name)}</span>
                                </label>
                            </div>
                            <span class="text-muted ml-4">
                                ${escHtml(formatQuantities(item.quantities))}
                            </span>
                        </li>
                    `}).join('')}
                </ul>
            </div>
        `).join('');
```

Aisle headings are `h2` under the page's `h1`; the `aisle-name` class keeps `shopping-list.spec.ts`'s `h3, h4, .aisle-name` selector matching.

The pantry block (`pantryItems.innerHTML = …`):

```js
        pantryItems.innerHTML = `
            <ul class="space-y-2 text-sm">
                ${data.pantry_items.map((item, idx) => {
                    // Handle both string array and object array formats
                    const itemName = typeof item === 'string' ? item : item.name;
                    const itemQuantities = typeof item === 'string' ? null : item.quantities;
                    return `
                    <li class="flex items-center justify-between py-1">
                        <div class="flex items-center flex-1">
                            <svg class="w-4 h-4 text-ok mr-2 shrink-0" fill="currentColor" viewBox="0 0 20 20">
                                <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd"></path>
                            </svg>
                            <span class="text-text">${escHtml(itemName)}</span>
                        </div>
                        <span class="text-muted text-xs ml-2">
                            ${itemQuantities ? escHtml(formatQuantities(itemQuantities)) : ''}
                        </span>
                    </li>
                `}).join('')}
            </ul>
        `;
```

And the empty-list line just below:

```js
        contentDiv.innerHTML = '<p class="text-faint">' + escHtml({{ tr.t("shopping-no-items")|json|safe }}) + '</p>';
```

- [ ] **Step 4: Check nothing is left**

Run: `grep -nE 'gray-|orange-|purple-|pink-|indigo-|green-|red-|blue-|gradient|shadow-(xs|lg|xl)|rounded-2xl' templates/shopping_list.html || echo clean`
Expected: `clean`.

- [ ] **Step 5: Run the shopping list specs serially**

Run: `npm run build-css && npm test -- --project=chromium --workers=1 tests/e2e/shopping-list.spec.ts tests/e2e/shopping-list-copy.spec.ts tests/e2e/shopping-list-live.spec.ts`
Expected: all pass.

- [ ] **Step 6: Look at it**

Add Neapolitan Pizza to the list, open `/shopping-list` at 1440px: sidebar card sticky at the same width, aisle cards with the same padding and row height as main, checked items struck through.

- [ ] **Step 7: Commit**

```bash
git add templates/shopping_list.html
git commit -q -F - <<'EOF'
refactor(ui): shopping list on tokens

The page gains its missing h1; sidebar and aisle headings become h2.

Claude-Session: https://claude.ai/code/session_013urND2B6Y3Z7WQuDpE8ZDu
EOF
```

---

### Task 7: Pantry

**Files:**
- Modify: `templates/pantry.html` (markup block, `applyFilter` selector, `updateItemStatus` class toggling)

- [ ] **Step 1: Replace the markup block**

Replace everything from `{% block content %}` up to (not including) `<!-- Add Item Modal -->` with:

```html
{% block content %}
<div class="px-4 sm:px-8 lg:px-12 xl:px-16">
    <div id="pantry-error-banner" class="hidden mb-4 card p-4 border-l-[3px] border-l-danger bg-danger-soft">
        <div class="flex items-start gap-3">
            <div class="shrink-0 text-danger">
                <svg class="w-5 h-5 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z"></path>
                </svg>
            </div>
            <p id="pantry-error-message" class="flex-1 text-text text-sm font-mono whitespace-pre-wrap"></p>
            <button type="button" onclick="document.getElementById('pantry-error-banner').classList.add('hidden')" class="icon-btn shrink-0" aria-label="Dismiss">
                <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                </svg>
            </button>
        </div>
    </div>

    <div class="mt-8 mb-6">
        <div class="flex items-center justify-between">
            <h1 class="text-display font-bold text-text">{{ tr.t("pantry-title") }}</h1>
            {% if !sections.is_empty() %}
            <div class="flex items-center gap-4">
                <span id="out-of-stock-count" class="text-sm text-faint"></span>
                <label class="flex items-center cursor-pointer group">
                    <input type="checkbox" id="filter-out-of-stock" class="mr-2 w-4 h-4 accent-[var(--accent)]">
                    <span class="text-sm text-muted group-hover:text-text">{{ tr.t("pantry-show-out-of-stock") }}</span>
                </label>
            </div>
            {% endif %}
        </div>
    </div>

    {% if !configured %}
    <div class="card">
        <div class="p-8 text-center">
            <svg class="mx-auto h-12 w-12 text-faint mb-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"></path>
            </svg>
            <h2 class="text-title font-medium text-text mb-2">{{ tr.t("pantry-no-config") }}</h2>
            <p class="text-muted">{{ tr.t("pantry-create-config") }}</p>
            <a href="{{ prefix }}/preferences" class="mt-4 inline-block text-accent-text hover:underline">
                {{ tr.t("pantry-configure") }}
            </a>
        </div>
    </div>
    {% else %}
    <div class="space-y-6">
        {% for section in sections %}
        <div class="card pantry-section">
            <div class="p-6">
                <h2 class="text-title font-semibold text-text mb-4 capitalize">{{ section.name }}</h2>
                {% if section.items.is_empty() %}
                <p class="text-faint italic">{{ tr.t("pantry-no-items-section") }}</p>
                {% else %}
                <div class="grid gap-3 md:grid-cols-2 lg:grid-cols-3">
                    {% for item in section.items %}
                    <div class="pantry-item group"
                         data-section="{{ section.name }}"
                         data-name="{{ item.name }}"
                         data-quantity="{% if let Some(quantity) = item.quantity %}{{ quantity }}{% endif %}"
                         data-low="{% if let Some(low) = item.low %}{{ low }}{% endif %}">
                        <!-- Main content with aligned name and attributes -->
                        <div class="flex items-start justify-between">
                            <div class="grow min-w-0">
                                <!-- Name aligned with attributes -->
                                <h3 class="font-medium text-text mb-2">{{ item.name }}</h3>

                                <!-- Attributes section -->
                                <div class="item-display">
                                    <div class="space-y-1 text-sm">
                                        <div class="quantity-display flex items-center">
                                            <span class="text-faint mr-2">{{ tr.t("pantry-item-quantity-short") }}</span>
                                            <span class="item-quantity">{% if let Some(quantity) = item.quantity %}{{ quantity }}{% else %}-{% endif %}</span>
                                            <svg class="out-of-stock-icon w-4 h-4 ml-1 hidden" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path>
                                            </svg>
                                        </div>
                                        <div class="flex items-center text-muted">
                                            <span class="text-faint mr-2">{{ tr.t("pantry-item-bought") }}</span>
                                            <span class="item-bought">{% if let Some(bought) = item.bought %}{{ bought }}{% else %}-{% endif %}</span>
                                        </div>
                                        <div class="flex items-center text-muted">
                                            <span class="text-faint mr-2">{{ tr.t("pantry-item-expire") }}</span>
                                            <span class="item-expire">{% if let Some(expire) = item.expire %}{{ expire }}{% else %}-{% endif %}</span>
                                        </div>
                                        <div class="flex items-center text-muted">
                                            <span class="text-faint mr-2">{{ tr.t("pantry-item-low") }}</span>
                                            <span class="item-low">{% if let Some(low) = item.low %}{{ low }}{% else %}-{% endif %}</span>
                                        </div>
                                    </div>
                                </div>

                                <!-- Edit form (hidden by default) -->
                                <div class="item-edit hidden">
                                    <div class="space-y-2">
                                        <input type="text" class="edit-quantity w-full px-2 py-1 text-sm border border-line rounded-[var(--radius-control)] bg-surface text-text"
                                               placeholder="Quantity (e.g., 500%g)"
                                               value="{% if let Some(quantity) = item.quantity %}{{ quantity }}{% endif %}">
                                        <input type="text" class="edit-bought w-full px-2 py-1 text-sm border border-line rounded-[var(--radius-control)] bg-surface text-text"
                                               placeholder="Bought (DD.MM.YYYY)"
                                               value="{% if let Some(bought) = item.bought %}{{ bought }}{% endif %}">
                                        <input type="text" class="edit-expire w-full px-2 py-1 text-sm border border-line rounded-[var(--radius-control)] bg-surface text-text"
                                               placeholder="Expires (DD.MM.YYYY)"
                                               value="{% if let Some(expire) = item.expire %}{{ expire }}{% endif %}">
                                        <input type="text" class="edit-low w-full px-2 py-1 text-sm border border-line rounded-[var(--radius-control)] bg-surface text-text"
                                               placeholder="Low threshold (e.g., 100%g)"
                                               value="{% if let Some(low) = item.low %}{{ low }}{% endif %}">
                                        <div class="flex flex-wrap items-center justify-end gap-2 mt-2">
                                            <button type="button" class="cancel-btn btn btn-sm">{{ tr.t("pantry-cancel") }}</button>
                                            <button type="button" class="save-btn btn btn-primary btn-sm">{{ tr.t("pantry-save") }}</button>
                                        </div>
                                    </div>
                                </div>
                            </div>

                            <!-- Action buttons and status indicator -->
                            <div class="flex items-start space-x-2 ml-4">
                                <div class="item-status-dot in-stock mt-1"></div>
                                <div class="pantry-actions flex items-center space-x-1">
                                    <button type="button" class="edit-btn icon-btn"
                                            title="{{ tr.t("pantry-edit-item") }}">
                                        <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"></path>
                                        </svg>
                                    </button>
                                    <button type="button" class="delete-btn icon-btn text-danger"
                                            title="{{ tr.t("pantry-remove-item") }}">
                                        <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                                        </svg>
                                    </button>
                                </div>
                            </div>
                        </div>
                    </div>
                    {% endfor %}
                </div>
                {% endif %}
            </div>
        </div>
        {% endfor %}
    </div>

    <div class="mt-8 flex flex-col sm:flex-row sm:justify-between sm:items-center gap-4">
        <div class="text-sm text-faint">
            {{ tr.t("pantry-total-sections") }} {{ sections.len() }}
        </div>
        <div class="flex flex-wrap gap-2">
            <button type="button" id="add-item-btn" class="btn btn-primary">
                {{ tr.t("pantry-add-item") }}
            </button>
            <a href="{{ prefix }}/preferences" class="btn">
                {{ tr.t("pantry-edit-config") }}
            </a>
        </div>
    </div>
    {% endif %}
</div>

```

Compare against main's markup before saving: the only structural differences are `pantry-section` on the section card (the old `recipe-card` class was doing that job), `h3` → `h2` for the unconfigured heading, and `type="button"` on the row buttons.

- [ ] **Step 2: Restyle the add-item modal**

In the `<!-- Add Item Modal -->` block:

```html
<div id="add-modal" class="fixed inset-0 bg-black/50 hidden items-center justify-center z-50">
    <div class="card shadow-[var(--shadow-overlay)] p-6 w-full max-w-md mx-4">
        <h2 class="text-title font-bold text-text mb-4">{{ tr.t("pantry-add-pantry-item") }}</h2>
```

Every `<select>`/`<input>` in the form: replace `class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600"` with `class="w-full px-3 py-2 border border-line rounded-[var(--radius-control)] bg-surface text-text"`. The footer buttons:

```html
            <div class="flex justify-end space-x-3">
                <button type="button" onclick="closeAddModal()" class="btn">{{ tr.t("pantry-cancel") }}</button>
                <button type="submit" class="btn btn-primary">{{ tr.t("pantry-save") }}</button>
            </div>
```

- [ ] **Step 3: Point `applyFilter` at the section class**

```js
        const sections = document.querySelectorAll('.pantry-section');
```

and the empty message it creates:

```js
                        message.className = 'empty-section-message text-faint italic';
```

- [ ] **Step 4: Rewrite the stock-status class toggling**

Replace the block that starts `// Reset all styling first` and ends just before `// Update count after checking stock status` with:

```js
            // State lives on the item as one class; components.css styles
            // the block, dot, quantity and icon from it.
            item.classList.remove('out-of-stock', 'low-stock');

            const statusDot = item.querySelector('.item-status-dot');
            if (statusDot) {
                statusDot.classList.remove('in-stock', 'out-of-stock', 'low-stock', 'animate-pulse');
            }

            const warningIcon = item.querySelector('.out-of-stock-icon');
            if (warningIcon) {
                warningIcon.classList.add('hidden');
            }

            if (isOutOfStock) {
                item.classList.add('out-of-stock');
                if (statusDot) statusDot.classList.add('out-of-stock', 'animate-pulse');
                if (warningIcon) warningIcon.classList.remove('hidden');
            } else if (isLowStock) {
                item.classList.add('low-stock');
                if (statusDot) statusDot.classList.add('low-stock', 'animate-pulse');
                if (warningIcon) warningIcon.classList.remove('hidden');
            } else {
                if (statusDot) statusDot.classList.add('in-stock');
            }
        });
```

Keep the `});` that closes the `items.forEach` if your replacement range did not include it; the function must still end with `updateOutOfStockCount();`.

- [ ] **Step 5: Check nothing is left**

Run: `grep -nE 'gray-|orange-|red-|green-|blue-|dark:|recipe-card' templates/pantry.html || echo clean`
Expected: `clean`.

- [ ] **Step 6: Run the pantry spec**

Run: `npm run build-css && npm test -- --project=chromium tests/e2e/pantry.spec.ts`
Expected: all pass.

- [ ] **Step 7: Look at it**

`/pantry` at 1440px: section cards with three columns of stat blocks, the same block size as main; hover reveals edit/delete; an item with quantity `0` shows a danger-tinted block and pulsing dot; the out-of-stock filter still hides sections.

- [ ] **Step 8: Commit**

```bash
git add templates/pantry.html
git commit -q -F - <<'EOF'
refactor(ui): pantry on tokens, stock state as one class

Claude-Session: https://claude.ai/code/session_013urND2B6Y3Z7WQuDpE8ZDu
EOF
```

---

### Task 8: Menu page

**Files:**
- Modify: `tests/menu_api_test.rs:305-308`
- Modify: `templates/menu.html`

- [ ] **Step 1: Make the menu API test class-agnostic**

In `tests/menu_api_test.rs`, the regex inside `html_menu_page_agrees_with_the_menu_api` pins the badge's classes. Change it to:

```rust
    // Every reference link, paired with the badge following it (if any).
    // The badge's classes are deliberately not pinned — this test is about
    // the factors, and hard-coding the styling made a purely visual change
    // to menu.html fail here.
    let re = regex::Regex::new(
        r#"/recipe/(?:[^"?]*?)"[^>]*>\s*[^<]+?\s*</a>\s*(?:<span[^>]*>\(×([0-9.]+)\)</span>)?"#,
    )
    .unwrap();
```

Run: `cargo test --test menu_api_test html_menu_page_agrees_with_the_menu_api 2>&1 | tail -3`
Expected: passes (the regex is a superset of the old one).

- [ ] **Step 2: Rewrite the markup block of `templates/menu.html`**

Replace everything from `{% block content %}` up to (not including) `{% if !static_mode %}<script>` with:

```html
{% block content %}
<div id="menu-content">
    <nav class="mb-4 print:hidden">
        <ol class="flex items-center space-x-2 text-sm text-muted">
            <li><a href="{{ prefix }}/{% if static_mode %}index.html{% endif %}" class="hover:text-text">{{ tr.t("nav-recipes") }}</a></li>
            {% for crumb in breadcrumbs %}
            <li class="flex items-center">
                <span class="mx-2 text-faint">/</span>
                {% if loop.last %}
                    <span class="text-text">{{ crumb }}</span>
                {% else %}
                    <a href="{{ prefix }}/directory/{{ breadcrumbs[..loop.index0 + 1].join("/") }}{% if static_mode %}.html{% endif %}" class="hover:text-text">{{ crumb }}</a>
                {% endif %}
            </li>
            {% endfor %}
        </ol>
    </nav>

    <div class="mb-4">
        <!-- Menu image if available -->
        {% match image_path %}
        {% when Some with (img) %}
        <div class="mb-4 max-w-4xl mx-auto print:hidden">
            <div class="overflow-hidden card bg-sunk">
                <img src="{{ img }}" alt="{{ name }}" class="w-full h-auto max-h-[500px] object-contain mx-auto">
            </div>
        </div>
        {% when None %}
        {% endmatch %}

        <!-- Title bar with scale and shopping list button -->
        <div class="flex flex-col md:flex-row md:items-center md:justify-between gap-4 mb-4 print:mb-1">
            <div class="flex items-center gap-3">
                <h1 class="text-display font-bold text-text print:text-2xl">{{ name }}</h1>
                <span class="tag print:hidden">{{ tr.t("recipe-type-menu") }}</span>
            </div>
            {% if !static_mode %}
            <div class="flex flex-wrap items-center gap-2 lg:gap-3 print:hidden">
                <div class="flex items-center">
                    <label for="scale" class="text-sm font-medium text-muted mr-2">{{ tr.t("recipe-scale-label") }}:</label>
                    <div class="stepper">
                        <button type="button" aria-label="Decrease scale" onclick="adjustScale(-0.5)">&minus;</button>
                        <input type="number"
                               id="scale"
                               value="{{ scale }}"
                               min="0.5"
                               max="200"
                               step="0.5"
                               onchange="goToScale(this.value)">
                        <button type="button" aria-label="Increase scale" onclick="adjustScale(0.5)">+</button>
                    </div>
                </div>
                <a href="{{ prefix }}/edit/{{ recipe_path }}" class="btn" title="{{ tr.t("action-edit") }}">
                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"></path>
                    </svg>
                    <span>{{ tr.t("action-edit") }}</span>
                </a>
                <button onclick="addToShoppingList(event, {{ recipe_path|json }})" class="btn btn-primary" title="{{ tr.t("recipe-add-all-to-shopping") }}">
                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 3h2l.4 2M7 13h10l4-8H5.4M7 13L5.4 5M7 13l-2.293 2.293c-.63.63-.184 1.707.707 1.707H17m0 0a2 2 0 100 4 2 2 0 000-4zm-8 2a2 2 0 11-4 0 2 2 0 014 0z"></path>
                    </svg>
                    <span>{{ tr.t("recipe-add-all-to-shopping") }}</span>
                </button>
            </div>
            {% endif %}
        </div>

        <!-- Menu info section -->
        <div class="space-y-2 print:hidden">
            {% match metadata %}
            {% when Some with (metadata) %}

            <!-- Description if present -->
            {% match metadata.description %}
            {% when Some with (description) %}
            <div class="recipe-note">
                <p class="m-0">{{ description }}</p>
            </div>
            {% when None %}
            {% endmatch %}

            <!-- Metadata row -->
            <div id="metadata-container" class="flex flex-wrap gap-3">
                {% match metadata.servings %}
                {% when Some with (servings) %}
                <span class="metadata-pill metadata-servings">👥 {{ servings }} {{ tr.t("recipe-servings-label") }}</span>
                {% when None %}
                {% endmatch %}

                {% match metadata.time %}
                {% when Some with (time) %}
                <span class="metadata-pill metadata-time">⏱️ {{ time }}</span>
                {% when None %}
                {% endmatch %}

                {% match metadata.author %}
                {% when Some with (author) %}
                <span class="metadata-pill metadata-author">👤 {{ author }}</span>
                {% when None %}
                {% endmatch %}

                {% match metadata.source %}
                {% when Some with (source) %}
                <span class="metadata-pill metadata-source">📖 {{ source }}</span>
                {% when None %}
                {% endmatch %}

                {% match metadata.source_url %}
                {% when Some with (source_url) %}
                <span class="metadata-pill metadata-source-url">🔗 <a href="{{ source_url }}" class="text-info hover:underline ml-1">{{ source_url|hostname }}</a></span>
                {% when None %}
                {% endmatch %}

                <!-- Custom metadata -->
                {% for (key, value) in metadata.custom %}
                <span class="metadata-pill metadata-custom">{{ key }}: {{ value }}</span>
                {% endfor %}
            </div>

            {% when None %}
            {% endmatch %}
        </div>
    </div>

    <!-- Menu sections -->
    <div class="space-y-6">
            {% for section in sections %}
            <div class="menu-section card overflow-hidden">
                {% match section.name %}
                {% when Some with (name) %}
                <div class="menu-section-header bg-sunk border-b border-line px-6 py-3">
                    <h2 class="text-title font-semibold text-text">{{ name }}</h2>
                </div>
                {% when None %}
                {% endmatch %}

                <div class="menu-section-body p-6 space-y-2">
                    {% for line in section.lines %}
                    {% if line.len() == 1 %}
                        {% match line[0] %}
                        {% when crate::web::templates::MenuSectionItem::Text with (text) %}
                            {% if text.trim().ends_with(":") %}
                            <!-- Meal type header -->
                            <h3 class="text-title font-semibold text-text mt-2 mb-1">{{ text }}</h3>
                            {% else %}
                            <!-- Regular text line -->
                            <div class="pl-6">
                                <span class="text-text">{{ text }}</span>
                            </div>
                            {% endif %}
                        {% when _ %}
                            <!-- Single non-text item -->
                            <div class="pl-6 flex items-baseline gap-x-2">
                                {% match line[0] %}
                                {% when crate::web::templates::MenuSectionItem::RecipeReference with { name, scale } %}
                                    <span class="inline-flex items-center gap-1">
                                        <a href="{{ prefix }}/recipe/{{ name.strip_prefix("./").unwrap_or(name) }}{% if static_mode %}.html{% endif %}"
                                           class="text-info font-medium hover:underline">
                                            {{ name.strip_prefix("./").unwrap_or(name).replace("/", " › ") }}
                                        </a>
                                        {% match scale %}
                                        {% when Some with (s) %}
                                            <span class="text-sm text-faint">(×{{ s }})</span>
                                        {% when None %}
                                        {% endmatch %}
                                    </span>
                                {% when crate::web::templates::MenuSectionItem::Ingredient with { name, quantity, unit } %}
                                    <span class="inline-flex items-center gap-1">
                                        <span class="ingredient-badge">
                                            {{ name }}
                                            {% match quantity %}
                                            {% when Some with (q) %}
                                                <span>{{ q }}</span>
                                            {% when None %}
                                            {% endmatch %}
                                            {% match unit %}
                                            {% when Some with (u) %}
                                                <span class="font-normal text-muted">{{ u }}</span>
                                            {% when None %}
                                            {% endmatch %}
                                        </span>
                                    </span>
                                {% when _ %}
                                {% endmatch %}
                            </div>
                        {% endmatch %}
                    {% else %}
                    <!-- Multi-item line (meal item with ingredients) -->
                    <div class="pl-6 flex flex-wrap items-baseline gap-x-2">
                        {% for item in line %}
                            {% match item %}
                            {% when crate::web::templates::MenuSectionItem::Text with (text) %}
                                <span class="text-text">{{ text }}</span>

                            {% when crate::web::templates::MenuSectionItem::RecipeReference with { name, scale } %}
                                <span class="inline-flex items-center gap-1">
                                    <a href="{{ prefix }}/recipe/{{ name.strip_prefix("./").unwrap_or(name) }}{% if static_mode %}.html{% endif %}"
                                       class="text-info font-medium hover:underline">
                                        {{ name.strip_prefix("./").unwrap_or(name).replace("/", " › ") }}
                                    </a>
                                    {% match scale %}
                                    {% when Some with (s) %}
                                        <span class="text-sm text-faint">(×{{ s }})</span>
                                    {% when None %}
                                    {% endmatch %}
                                </span>

                            {% when crate::web::templates::MenuSectionItem::Ingredient with { name, quantity, unit } %}
                                <span class="inline-flex items-center gap-1">
                                    <span class="ingredient-badge">
                                        {{ name }}
                                        {% match quantity %}
                                        {% when Some with (q) %}
                                            <span>{{ q }}</span>
                                        {% when None %}
                                        {% endmatch %}
                                        {% match unit %}
                                        {% when Some with (u) %}
                                            <span class="font-normal text-muted">{{ u }}</span>
                                        {% when None %}
                                        {% endmatch %}
                                    </span>
                                </span>
                            {% endmatch %}
                        {% endfor %}
                    </div>
                    {% endif %}
                    {% endfor %}
                </div>
            </div>
            {% endfor %}
    </div>
</div>
```

- [ ] **Step 3a: Safe scale navigation**

The stepper's input uses `onchange="goToScale(this.value)"`. Inside the `{% if !static_mode %}<script>` block, before `escHtml`, add the same guarded navigation the recipe page uses (no scroll stash here):

```js
// |json puts the path in a JS string literal with <, >, & escaped as
// \uXXXX, so it survives the <script> context.
const RECIPE_URL = {{ prefix|json|safe }} + '/recipe/' + {{ recipe_path|json|safe }};
const DEFAULT_SCALE = {{ scale }};

function goToScale(value) {
    const input = document.getElementById('scale');
    const n = parseFloat(value);
    if (!Number.isFinite(n)) {
        // Cleared or non-numeric: put the current scale back, do not navigate.
        input.value = DEFAULT_SCALE;
        return;
    }
    const min = parseFloat(input.min) || 0.5;
    const max = parseFloat(input.max) || 200;
    const clamped = Math.min(max, Math.max(min, n));
    window.location.href = RECIPE_URL + '?scale=' + encodeURIComponent(clamped);
}
```

- [ ] **Step 3: Retint the success/error states in the menu script**

In `addToShoppingList` inside `templates/menu.html`, replace the four `classList` lines:

```js
            button.classList.add('bg-ok-soft', 'text-ok', 'border-ok');

            setTimeout(() => {
                button.innerHTML = originalText;
                button.classList.remove('bg-ok-soft', 'text-ok', 'border-ok');
            }, 2000);
```

and in the `catch`:

```js
        button.classList.add('bg-danger-soft', 'text-danger', 'border-danger');

        setTimeout(() => {
            button.innerHTML = originalText;
            button.classList.remove('bg-danger-soft', 'text-danger', 'border-danger');
        }, 2000);
```

Also change the two `<svg class="w-5 h-5" …>` inside the success/error `innerHTML` strings to `<svg …>` without the size classes, so `.btn svg` sizes them.

- [ ] **Step 4: Check nothing is left, run the tests**

Run: `grep -nE 'gray-|orange-|purple-|pink-|blue-|cyan-|green-|red-|gradient|shadow-(xs|md|lg)' templates/menu.html || echo clean`
Expected: `clean`.

Run: `npm run build-css && cargo build && cargo test --test menu_api_test 2>&1 | tail -3 && npm test -- --project=chromium tests/e2e/navigation.spec.ts`
Expected: Rust tests pass; navigation still passes.

- [ ] **Step 5: Look at it**

Open a menu from the index (any item with the 📋 icon): 30px title, tag chip, sections as cards with a sunk header band.

- [ ] **Step 6: Commit**

```bash
git add templates/menu.html tests/menu_api_test.rs
git commit -q -F - <<'EOF'
refactor(ui): menu page on tokens

The menu API test stops pinning the scale badge's classes.

Claude-Session: https://claude.ai/code/session_013urND2B6Y3Z7WQuDpE8ZDu
EOF
```

---

### Task 9: Preferences

**Files:**
- Modify: `tests/e2e/preferences.spec.ts:231-245`
- Modify: `templates/preferences.html`

- [ ] **Step 1: Assert on `aria-pressed` instead of gradient classes**

In `tests/e2e/preferences.spec.ts`:

```ts
    // Both enabled → toggles report pressed state via aria-pressed
    await expect(shoppingBtn).toHaveAttribute('aria-pressed', 'true');
    await expect(pantryBtn).toHaveAttribute('aria-pressed', 'true');
```

and further down:

```ts
    // Button is now inactive
    const shoppingBtn = page.getByRole('button', { name: /Shopping/i });
    await expect(shoppingBtn).toHaveAttribute('aria-pressed', 'false');
```

Run: `npm test -- --project=chromium tests/e2e/preferences.spec.ts`
Expected: the two feature-toggle tests fail (no `aria-pressed` yet).

- [ ] **Step 2: Rewrite the markup block of `templates/preferences.html`**

Replace everything from `{% block content %}` through `{% endblock %}` (the content block only, not the scripts block) with:

```html
{% block content %}
<div>
    <h1 class="text-display font-bold text-text mb-6">{{ tr.t("pref-title") }}</h1>

    <div class="space-y-6">
        <!-- Language Selector -->
        <div class="card p-6">
            <h2 class="text-title font-semibold mb-4 text-text">{{ tr.t("pref-language") }}</h2>
            <div class="flex flex-wrap gap-3">
                <button onclick="setLanguage('en-US')" aria-pressed="{% if tr.lang_string() == "en-US" %}true{% else %}false{% endif %}"
                        class="language-btn btn {% if tr.lang_string() == "en-US" %}btn-primary{% endif %}">
                    🇺🇸 English
                </button>
                <button onclick="setLanguage('de-DE')" aria-pressed="{% if tr.lang_string() == "de-DE" %}true{% else %}false{% endif %}"
                        class="language-btn btn {% if tr.lang_string() == "de-DE" %}btn-primary{% endif %}">
                    🇩🇪 Deutsch
                </button>
                <button onclick="setLanguage('nl-NL')" aria-pressed="{% if tr.lang_string() == "nl-NL" %}true{% else %}false{% endif %}"
                        class="language-btn btn {% if tr.lang_string() == "nl-NL" %}btn-primary{% endif %}">
                    🇳🇱 Nederlands
                </button>
                <button onclick="setLanguage('fr-FR')" aria-pressed="{% if tr.lang_string() == "fr-FR" %}true{% else %}false{% endif %}"
                        class="language-btn btn {% if tr.lang_string() == "fr-FR" %}btn-primary{% endif %}">
                    🇫🇷 Français
                </button>
                <button onclick="setLanguage('es-ES')" aria-pressed="{% if tr.lang_string() == "es-ES" %}true{% else %}false{% endif %}"
                        class="language-btn btn {% if tr.lang_string() == "es-ES" %}btn-primary{% endif %}">
                    🇪🇸 Español
                </button>
                <button onclick="setLanguage('eu-ES')" aria-pressed="{% if tr.lang_string() == "eu-ES" %}true{% else %}false{% endif %}"
                        class="language-btn btn {% if tr.lang_string() == "eu-ES" %}btn-primary{% endif %}">
                    Euskara
                </button>
                <button onclick="setLanguage('sv-SE')" aria-pressed="{% if tr.lang_string() == "sv-SE" %}true{% else %}false{% endif %}"
                        class="language-btn btn {% if tr.lang_string() == "sv-SE" %}btn-primary{% endif %}">
                    🇸🇪 Svenska
                </button>
            </div>
            <p class="text-sm text-muted mt-3">Your language preference will be saved in a cookie.</p>
        </div>

        <!-- Features -->
        {% if !static_mode %}
        <div class="card p-6">
            <h2 class="text-title font-semibold mb-2 text-text">{{ tr.t("pref-features") }}</h2>
            <p class="text-sm text-muted mb-4">{{ tr.t("pref-features-desc") }}</p>
            <div class="flex flex-wrap gap-3">
                <button onclick="toggleFeature('show_shopping_list', {{ features.show_shopping_list }})"
                        aria-pressed="{% if features.show_shopping_list %}true{% else %}false{% endif %}"
                        class="btn {% if features.show_shopping_list %}btn-primary{% endif %}">
                    🛒 {{ tr.t("nav-shopping-list") }}
                </button>
                <button onclick="toggleFeature('show_pantry', {{ features.show_pantry }})"
                        aria-pressed="{% if features.show_pantry %}true{% else %}false{% endif %}"
                        class="btn {% if features.show_pantry %}btn-primary{% endif %}">
                    🥫 {{ tr.t("nav-pantry") }}
                </button>
            </div>
        </div>
        {% endif %}

        <!-- CookCloud Sync -->
        {% if sync_enabled %}
        <div class="card p-6">
            <h2 class="text-title font-semibold mb-3 text-text">CookCloud Sync</h2>
            <div id="sync-section">
                {% if sync_logged_in %}
                <div class="flex items-center justify-between">
                    <div>
                        <p class="text-sm">
                            Signed in as <span class="font-medium">{{ sync_email.as_deref().unwrap_or("Unknown") }}</span>
                        </p>
                        <p class="text-xs text-muted mt-1" id="sync-status-text">{% if sync_syncing %}Syncing recipes...{% else %}Sync idle{% endif %}</p>
                    </div>
                    <button onclick="syncLogout()" class="btn text-danger border-danger">
                        Logout
                    </button>
                </div>
                {% else %}
                <div id="sync-login-section" class="flex items-center justify-between">
                    <p class="text-sm text-muted" id="sync-login-message">Sync your recipes across devices with CookCloud.</p>
                    <button id="sync-login-btn" onclick="syncLogin()" class="btn btn-primary">
                        Login to CookCloud
                    </button>
                </div>

                <div id="sync-login-card" class="hidden mt-4 card p-6 bg-sunk">
                    <h3 class="text-title font-semibold mb-2 text-text">Sign in to CookCloud</h3>
                    <ol class="mt-3 space-y-2 text-sm text-text">
                        <li>1. Open <a id="sync-login-link" href="#" target="_blank" rel="noopener" class="underline text-accent-text">cook.md/device</a> in any browser.</li>
                        <li>2. Enter this code:</li>
                    </ol>
                    <div class="mt-3 flex items-center gap-2">
                        <span id="sync-login-code" class="text-2xl tracking-widest bg-surface border border-line-strong px-4 py-2 rounded-[var(--radius-control)] font-mono">----  ----</span>
                        <button id="sync-login-copy" type="button" class="btn">Copy</button>
                    </div>
                    <p id="sync-login-expires" class="mt-3 text-sm text-muted"></p>
                    <div class="mt-4 flex gap-2">
                        <a id="sync-login-open" href="#" target="_blank" rel="noopener" class="btn btn-primary">Open cook.md/device</a>
                        <button id="sync-login-cancel" type="button" class="btn">Cancel</button>
                    </div>
                </div>
                {% endif %}
            </div>
        </div>
        {% endif %}

        <div class="card p-6">
            <h2 class="text-title font-semibold mb-3 text-text">Configuration Files</h2>
            <div class="space-y-2 text-sm">
                <div>
                    <span class="font-medium">{{ tr.t("pref-aisle-path") }}:</span>
                    <span class="text-muted ml-2">{{ aisle_path }}</span>
                </div>
                <div>
                    <span class="font-medium">{{ tr.t("pref-pantry-path") }}:</span>
                    <span class="text-muted ml-2">{{ pantry_path }}</span>
                </div>
            </div>
        </div>

        <div class="card p-6">
            <h2 class="text-title font-semibold mb-3 text-text">Recipe Directory</h2>
            <div class="text-sm">
                <span class="font-medium">{{ tr.t("pref-base-path") }}:</span>
                <span class="text-muted ml-2">{{ base_path }}</span>
            </div>
        </div>

        <div class="card p-6">
            <h2 class="text-title font-semibold mb-3 text-text">About</h2>
            <div class="space-y-2 text-sm">
                <div>
                    <span class="font-medium">{{ tr.t("pref-version") }}:</span>
                    <span class="text-muted ml-2">{{ version }}</span>
                </div>
                <div>
                    <span class="font-medium">Cooklang CLI:</span>
                    <span class="text-muted ml-2">A command-line tool for managing Cooklang recipes</span>
                </div>
            </div>
        </div>

        <div class="card p-6">
            <h2 class="text-title font-semibold mb-3 text-text">Documentation & Resources</h2>
            <div class="space-y-2 text-sm">
                {% if !static_mode %}
                <div>
                    <a href="{{ prefix }}/api-docs"
                       class="text-accent-text hover:underline font-medium">
                        🔌 Server API
                    </a>
                    <span class="text-muted ml-2">- HTTP endpoints for building integrations</span>
                </div>
                {% endif %}
                <div>
                    <a href="https://cooklang.org/cli/" target="_blank" rel="noopener noreferrer"
                       class="text-accent-text hover:underline font-medium">
                        📚 CLI Documentation
                    </a>
                    <span class="text-muted ml-2">- Learn about all CLI commands and features</span>
                </div>
                <div>
                    <a href="https://cooklang.org/docs/spec/" target="_blank" rel="noopener noreferrer"
                       class="text-accent-text hover:underline font-medium">
                        📖 Cooklang Specification
                    </a>
                    <span class="text-muted ml-2">- Recipe markup language syntax guide</span>
                </div>
                <div>
                    <a href="https://cooklang.org/docs/getting-started/" target="_blank" rel="noopener noreferrer"
                       class="text-accent-text hover:underline font-medium">
                        🚀 Getting Started
                    </a>
                    <span class="text-muted ml-2">- Introduction to Cooklang basics</span>
                </div>
                <div>
                    <a href="https://github.com/cooklang/CookCLI" target="_blank" rel="noopener noreferrer"
                       class="text-accent-text hover:underline font-medium">
                        🔧 GitHub Repository
                    </a>
                    <span class="text-muted ml-2">- Source code and issue tracker</span>
                </div>
            </div>
        </div>
    </div>
</div>
{% endblock %}
```

The `{% block scripts %}` block is unchanged.

- [ ] **Step 3: Check and test**

Run: `grep -nE 'gray-|orange-|purple-|pink-|blue-|indigo-|red-|gradient|shadow-lg|rounded-2xl|rounded-sm' templates/preferences.html || echo clean`
Expected: `clean`.

Run: `npm run build-css && npm test -- --project=chromium tests/e2e/preferences.spec.ts`
Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add templates/preferences.html tests/e2e/preferences.spec.ts
git commit -q -F - <<'EOF'
refactor(ui): preferences on tokens, toggles expose aria-pressed

Claude-Session: https://claude.ai/code/session_013urND2B6Y3Z7WQuDpE8ZDu
EOF
```

---

### Task 10: Editor and new-recipe form

**Files:**
- Modify: `templates/edit.html` (header, modal, editor container, status bar, three JS class strings)
- Modify: `templates/new.html`

- [ ] **Step 1: Restyle the edit page markup**

In `templates/edit.html`, replace everything from `{% block content %}` up to (not including) `<script src="{{ prefix }}/static/js/editor.bundle.js"></script>` with:

```html
{% block content %}
<div class="flex flex-col h-[calc(100vh-12rem)]">
    <!-- Header bar -->
    <div class="flex items-center justify-between mb-4">
        <div class="flex items-center gap-4">
            <a href="{{ prefix }}/recipe/{{ recipe_path }}" class="btn">
                <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18"></path>
                </svg>
                {{ tr.t("action-back") }}
            </a>
            <h1 class="text-display font-bold text-text">{{ recipe_name }}</h1>
        </div>
        <div class="flex items-center gap-3">
            <span id="save-status" class="text-sm text-muted"></span>
            <button onclick="showDeleteModal()" class="btn btn-danger">
                <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                </svg>
                {{ tr.t("action-delete") }}
            </button>
        </div>
    </div>

    <!-- Delete confirmation modal -->
    <div id="delete-modal" class="fixed inset-0 bg-black/50 hidden items-center justify-center z-50">
        <div class="card shadow-[var(--shadow-overlay)] p-6 max-w-md mx-4">
            <div class="flex items-center gap-3 mb-4">
                <div class="w-12 h-12 rounded-full bg-danger-soft flex items-center justify-center">
                    <svg class="w-6 h-6 text-danger" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path>
                    </svg>
                </div>
                <h2 class="text-title font-bold text-text">{{ tr.t("delete-recipe") }}</h2>
            </div>
            <p class="text-muted mb-2">{{ tr.t("delete-recipe-confirm") }}</p>
            <p class="text-sm text-danger mb-6">{{ tr.t("delete-recipe-warning") }}</p>
            <div class="flex justify-end gap-3">
                <button onclick="hideDeleteModal()" class="btn">
                    {{ tr.t("action-cancel") }}
                </button>
                <button onclick="deleteRecipe()" class="btn btn-danger">
                    {{ tr.t("action-delete") }}
                </button>
            </div>
        </div>
    </div>

    <!-- Editor area -->
    <div id="editor-container" class="flex-1 card overflow-hidden"></div>

    <!-- Status bar -->
    <div id="status-bar" class="mt-2 px-4 py-2 bg-sunk rounded-[var(--radius-control)] flex items-center justify-between text-sm">
        <div class="flex items-center gap-2">
            <span id="lsp-status" class="flex items-center gap-1">
                <span id="lsp-indicator" class="w-2 h-2 rounded-full bg-inactive"></span>
                <span id="lsp-text">Disconnected</span>
            </span>
        </div>
        <div id="cursor-position" class="text-muted">
            Line 1, Col 1
        </div>
    </div>
</div>

```

- [ ] **Step 2: Retint the edit page scripts**

`updateSaveStatus`:

```js
function updateSaveStatus(state) {
    const status = document.getElementById('save-status');
    switch (state) {
        case 'modified':
            status.textContent = 'Modified';
            status.className = 'text-sm text-accent-text';
            break;
        case 'saving':
            status.textContent = 'Saving...';
            status.className = 'text-sm text-muted';
            break;
        case 'saved':
            status.textContent = 'Saved';
            status.className = 'text-sm text-ok';
            break;
        case 'error':
            status.textContent = 'Save failed';
            status.className = 'text-sm text-danger';
            break;
    }
}
```

`showToast`'s class line:

```js
    toast.className = `fixed bottom-4 right-4 px-4 py-2 rounded-[var(--radius-control)] border shadow-[var(--shadow-overlay)] z-50 ${
        type === 'error' ? 'bg-danger-soft text-danger border-danger' : 'bg-ok-soft text-ok border-ok'
    }`;
```

The LSP indicator (around lines 350–366): replace the six `className` assignments:

| State | `indicator.className` | `text.className` |
|---|---|---|
| connected | `'w-2 h-2 rounded-full bg-ok'` | `'text-ok'` |
| disconnected | `'w-2 h-2 rounded-full bg-inactive'` | `'text-muted'` |
| error | `'w-2 h-2 rounded-full bg-danger'` | `'text-danger'` |

- [ ] **Step 3: Rewrite `templates/new.html`**

```html
{% extends "base.html" %}

{% block title %}{{ tr.t("new-recipe") }} - Cook{% endblock %}

{% block content %}
<div class="max-w-xl mx-auto">
    <h1 class="text-display font-bold text-text mb-8">{{ tr.t("new-recipe") }}</h1>

    {% if let Some(err) = error %}
    <div class="mb-6 card p-4 border-l-[3px] border-l-danger bg-danger-soft">
        <div class="flex items-center gap-2">
            <svg class="w-5 h-5 text-danger shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
            </svg>
            <p class="text-text">{{ err }}</p>
        </div>
    </div>
    {% endif %}

    <form action="{{ prefix }}/new" method="POST" class="space-y-6">
        <div>
            <label for="filename" class="block text-sm font-medium text-muted mb-2">
                {{ tr.t("new-recipe-path") }}
            </label>
            <div class="flex items-center gap-2">
                <input
                    type="text"
                    id="filename"
                    name="filename"
                    required
                    pattern="[a-zA-Z0-9_/ -]+"
                    placeholder="{{ tr.t("new-recipe-placeholder") }}"
                    value="{{ filename.as_deref().unwrap_or("") }}"
                    class="flex-1 px-4 py-3 border border-line rounded-[var(--radius-control)] bg-surface text-text focus:border-accent outline-hidden{% if error.is_some() %} border-danger{% endif %}"
                >
                <span class="text-faint">.cook</span>
            </div>
            <p class="mt-2 text-sm text-faint">{{ tr.t("new-recipe-hint") }}</p>
        </div>

        <div class="flex gap-4">
            <a href="{{ prefix }}/" class="btn">
                {{ tr.t("action-cancel") }}
            </a>
            <button type="submit" class="btn btn-primary">
                {{ tr.t("new-recipe-create") }}
            </button>
        </div>
    </form>
</div>
{% endblock %}
```

- [ ] **Step 4: Check and test**

Run: `grep -nE 'gray-|orange-|red-|green-|dark:|gradient|shadow-(lg|xl)|rounded-2xl' templates/edit.html templates/new.html || echo clean`
Expected: `clean`.

Run: `npm run build-css && npm test -- --project=chromium tests/e2e/navigation.spec.ts` then open `/edit/Neapolitan%20Pizza` in light and dark: the CodeMirror gutter is sunk-coloured and the caret is visible in dark mode. Open `/new`: the form renders on tokens.

- [ ] **Step 5: Commit**

```bash
git add templates/edit.html templates/new.html
git commit -q -F - <<'EOF'
refactor(ui): editor and new-recipe form on tokens

Claude-Session: https://claude.ai/code/session_013urND2B6Y3Z7WQuDpE8ZDu
EOF
```

---

### Task 11: API docs and error page

**Files:**
- Modify: `src/web/templates.rs:745-753`
- Modify: `templates/api_docs.html`
- Modify: `templates/error.html`

- [ ] **Step 1: Token classes for the method badge**

In `src/web/templates.rs`:

```rust
    pub fn method_classes(&self) -> &'static str {
        match self.method.as_str() {
            "GET" => "bg-sunk text-text",
            "POST" => "bg-ok-soft text-ok",
            "PUT" => "bg-accent-soft text-accent-text",
            "DELETE" => "bg-danger-soft text-danger",
            _ => "bg-sunk text-muted",
        }
    }
```

Run: `cargo test --lib api_docs 2>&1 | tail -3`
Expected: passes (no test pins the strings).

- [ ] **Step 2: Rewrite `templates/api_docs.html`**

```html
{% extends "base.html" %}

{% block title %}Server API - Cook{% endblock %}

{% block content %}
<div class="max-w-5xl">
    <h1 class="text-display font-bold text-text mb-2">Server API</h1>
    <p class="text-muted mb-6">
        {{ preamble.intro }}
    </p>

    <!-- Ground rules -->
    <div class="card p-6 border-l-[3px] border-l-accent mb-8">
        <h2 class="text-title font-semibold mb-3 text-text">Before you start</h2>
        <div class="text-sm space-y-2 text-text">
            <div>
                <span class="font-medium">Base URL:</span>
                <code class="ml-2 px-2 py-0.5 rounded bg-sunk font-mono">{{ base_url }}</code>
            </div>
            {% for n in preamble.notes %}
            <div>
                <span class="font-medium">{{ n.label }}:</span>
                <span class="ml-2">{{ n.text|inline_code|safe }}</span>
            </div>
            {% endfor %}
        </div>
    </div>

    <!-- Errors -->
    <div class="card p-6 mb-8">
        <h2 class="text-title font-semibold mb-3 text-text">Errors</h2>
        <p class="text-sm text-text mb-3">
            {{ preamble.error_intro }}
        </p>
        <pre class="bg-sunk border border-line rounded-[var(--radius-control)] p-3 text-xs overflow-x-auto font-mono">{{ preamble.error_example }}</pre>
        <ul class="mt-3 text-sm space-y-1 text-text">
            {% for c in preamble.error_codes %}
            <li><span class="font-mono font-medium">{{ c.label }}</span> — {{ c.text|inline_code|safe }}</li>
            {% endfor %}
        </ul>
    </div>

    <!-- Contents -->
    <div class="card p-6 mb-8">
        <h2 class="text-title font-semibold mb-3 text-text">Contents</h2>
        <div class="flex flex-wrap gap-2">
            {% for section in sections %}
            <a href="#{{ section.id }}" class="btn">
                {{ section.title }}
            </a>
            {% endfor %}
        </div>
    </div>

    <!-- Endpoints -->
    {% for section in sections %}
    <section id="{{ section.id }}" class="mb-10 scroll-mt-24">
        <h2 class="text-title font-bold mb-2 text-text">{{ section.title }}</h2>
        <p class="text-sm text-muted mb-4">{{ section.description|inline_code|safe }}</p>

        <div class="space-y-4">
            {% for endpoint in section.endpoints %}
            <article class="card p-5">
                <div class="flex flex-wrap items-center gap-2 mb-2">
                    <span class="px-2 py-0.5 rounded text-xs font-bold font-mono {{ endpoint.method_classes() }}">{{ endpoint.method }}</span>
                    <code class="font-mono text-sm break-all">{{ endpoint.path }}</code>
                    {% if let Some(feature) = endpoint.feature %}
                    <span class="px-2 py-0.5 rounded text-xs font-medium bg-accent-soft text-accent-text">
                        requires <span class="font-mono">{{ feature }}</span> build
                    </span>
                    {% endif %}
                </div>

                <p class="text-sm font-medium mb-1">{{ endpoint.summary|inline_code|safe }}</p>
                {% if !endpoint.description.is_empty() %}
                <p class="text-sm text-muted">{{ endpoint.description|inline_code|safe }}</p>
                {% endif %}

                {% if !endpoint.params.is_empty() %}
                <div class="mt-4 overflow-x-auto">
                    <table class="w-full text-sm">
                        <thead>
                            <tr class="text-left text-xs uppercase tracking-wide text-faint border-b border-line">
                                <th class="py-1.5 pr-4 font-medium">Name</th>
                                <th class="py-1.5 pr-4 font-medium">In</th>
                                <th class="py-1.5 pr-4 font-medium">Type</th>
                                <th class="py-1.5 font-medium">Description</th>
                            </tr>
                        </thead>
                        <tbody>
                            {% for p in endpoint.params %}
                            <tr class="border-b border-line align-top">
                                <td class="py-1.5 pr-4 font-mono whitespace-nowrap">
                                    {{ p.name }}{% if p.required %}<span class="text-danger" title="required">*</span>{% endif %}
                                </td>
                                <td class="py-1.5 pr-4 text-faint">{{ p.kind }}</td>
                                <td class="py-1.5 pr-4 font-mono text-faint whitespace-nowrap">{{ p.type_name }}</td>
                                <td class="py-1.5 text-muted">{{ p.description|inline_code|safe }}</td>
                            </tr>
                            {% endfor %}
                        </tbody>
                    </table>
                    <p class="mt-1 text-xs text-faint">
                        <span class="text-danger">*</span> required
                    </p>
                </div>
                {% endif %}

                {% if let Some(request) = endpoint.request_example %}
                <div class="mt-4">
                    <h4 class="text-xs uppercase tracking-wide text-faint mb-1">Request body</h4>
                    <pre class="bg-sunk border border-line rounded-[var(--radius-control)] p-3 text-xs overflow-x-auto font-mono">{{ request }}</pre>
                </div>
                {% endif %}

                {% if let Some(response) = endpoint.response_example %}
                <div class="mt-4">
                    <h4 class="text-xs uppercase tracking-wide text-faint mb-1">Response</h4>
                    <pre class="bg-sunk border border-line rounded-[var(--radius-control)] p-3 text-xs overflow-x-auto font-mono">{{ response }}</pre>
                </div>
                {% endif %}
            </article>
            {% endfor %}
        </div>
    </section>
    {% endfor %}
</div>
{% endblock %}
```

- [ ] **Step 3: Rewrite `templates/error.html`**

```html
{% extends "base.html" %}

{% block title %}{{ tr.t("error-title") }} - Cook{% endblock %}

{% block content %}
<div class="max-w-2xl mx-auto mt-12">
    <div class="card p-6 border-l-[3px] border-l-danger bg-danger-soft">
        <div class="flex items-start gap-4">
            <div class="shrink-0 text-danger">
                <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z"></path>
                </svg>
            </div>
            <div>
                <h1 class="text-title font-bold text-danger mb-2">{{ tr.t("error-title") }}</h1>
                <p class="text-text font-mono text-sm whitespace-pre-wrap">{{ error_message }}</p>
            </div>
        </div>
        <div class="mt-6">
            <a href="{{ prefix }}/" class="btn">
                {{ tr.t("error-back-home") }}
            </a>
        </div>
    </div>
</div>
{% endblock %}
```

The heading becomes an `h1`: the error page had none.

- [ ] **Step 4: Check and test**

Run: `grep -nE 'gray-|blue-|purple-|red-|indigo-|dark:|gradient|shadow-lg|rounded-2xl' templates/api_docs.html templates/error.html || echo clean`
Expected: `clean`.

Run: `cargo build && npm run build-css && npm test -- --project=chromium tests/e2e/api-docs.spec.ts`
Expected: passes. Restart the dev server first if one is running, because the Rust change needs the new binary. Open `/api-docs`: GET badges are sunk-grey, POST green-tinted, DELETE red-tinted. Open `/recipe/does-not-exist`: the error card renders.

- [ ] **Step 5: Commit**

```bash
git add src/web/templates.rs templates/api_docs.html templates/error.html
git commit -q -F - <<'EOF'
refactor(ui): API docs and error page on tokens

Claude-Session: https://claude.ai/code/session_013urND2B6Y3Z7WQuDpE8ZDu
EOF
```

---

### Task 12: Delete the dark override block, rewrite the print block

**Files:**
- Modify: `templates/base.html` (the whole `<style>…</style>` block in `<head>`)

Every page is now on tokens, so nothing depends on the `.dark .*` overrides. The print block is rewritten to target the new markup only.

- [ ] **Step 1: Replace the entire `<style>` block**

Replace everything from `<style>` through `</style>` in `<head>` with:

```html
    <style>
        .viewport {
            width: 100%;
            max-width: 72rem;
            margin: 2rem auto;
            padding: 0 1rem;
        }

        /* Search result keyboard selection. --surface-sunk alone is barely
           visible against --surface in dark mode, so the accent bar carries
           the signal. */
        #search-results a.search-selected {
            background: var(--accent-soft) !important;
            box-shadow: inset 3px 0 0 var(--accent);
        }

        @media print {
            /* Token values are reset to light in input.css; this resets the
               browser's own defaults so unstyled text is dark too. */
            html, html.dark {
                color-scheme: light !important;
            }

            * {
                print-color-adjust: exact;
                -webkit-print-color-adjust: exact;
            }

            body {
                background: white !important;
                margin: 0;
                padding: 0;
            }

            .viewport {
                max-width: 100%;
                margin: 0;
                padding: 0;
            }

            /* Hide navigation and non-essential elements */
            nav, #search-container, #search-input, #search-results, .print\:hidden, .breadcrumb {
                display: none !important;
            }

            /* Hide all navigation links */
            a[href="{{ prefix }}/"]{% if !static_mode %}, a[href="{{ prefix }}/shopping-list"], a[href="{{ prefix }}/preferences"]{% endif %} {
                display: none !important;
            }

            /* Recipe layout for print: one column */
            .grid {
                display: block !important;
            }

            .md\:col-span-1, .md\:col-span-2 {
                page-break-inside: avoid;
                width: 100% !important;
            }

            /* Square corners on paper */
            .card, .step-box, .ingredient-row, .recipe-note, .rounded-xl {
                border-radius: 0 !important;
            }

            /* Step numbers */
            .step-number {
                background: #000 !important;
                color: #fff !important;
            }

            /* Images */
            img {
                max-height: 300px !important;
                max-width: 100% !important;
                height: auto !important;
                object-fit: contain !important;
                page-break-inside: avoid;
                page-break-after: avoid;
            }

            /* Recipe sections on same page if possible */
            h2, h3 {
                page-break-after: avoid;
            }

            ol, ul {
                page-break-inside: avoid;
            }

            /* Compact spacing */
            .mb-8, .mb-6, .mb-4 {
                margin-bottom: 0.5rem !important;
            }

            .p-6 {
                padding: 0.5rem !important;
            }

            /* Recipe header */
            h1 {
                color: #000 !important;
                font-size: 1.5rem !important;
                margin-bottom: 0 !important;
            }

            /* Print-specific flexbox utilities */
            .print\:flex-row {
                flex-direction: row !important;
            }

            .print\:items-center {
                align-items: center !important;
            }

            .print\:block {
                display: block !important;
            }

            .print\:text-2xl {
                font-size: 1.5rem !important;
            }

            /* Remove truncation in print so text is fully visible */
            .truncate {
                overflow: visible !important;
                text-overflow: unset !important;
                white-space: normal !important;
                max-width: none !important;
                display: inline !important;
            }

            /* Shopping list print styles */
            .lg\:w-2\/5, .xl\:w-1\/3 {
                display: none !important;
            }

            #shopping-list-results ~ .flex-1,
            .flex.min-h-screen > .flex-1 {
                width: 100% !important;
                flex: none !important;
            }

            #shopping-list-results {
                font-size: 11pt !important;
                line-height: 1.2 !important;
            }

            #shopping-list-results input[type="checkbox"] {
                display: none !important;
            }

            #shopping-list-results li .flex-1 {
                width: auto !important;
                flex: 1 1 0% !important;
            }

            #shopping-list-results label {
                cursor: default !important;
            }

            #shopping-list-results h2 {
                font-size: 14pt !important;
                margin-bottom: 0.1rem !important;
            }

            #shopping-list-results .space-y-2 > * + *,
            #shopping-list-results ul > * + * {
                margin-top: 0 !important;
            }

            #shopping-list-results li {
                padding: 0 !important;
                line-height: 1.3 !important;
                border-bottom: 1px dotted #ccc;
            }

            #shopping-list-results .card {
                margin-bottom: 0.15rem !important;
                padding: 0 !important;
                border: none !important;
            }

            /* Menu print styles */
            #menu-content {
                font-size: 10pt !important;
                line-height: 1.4 !important;
            }

            #menu-content h1 {
                font-size: 14pt !important;
                margin-bottom: 0.25rem !important;
                color: #000 !important;
            }

            #menu-content .space-y-6 > * + * {
                margin-top: 0.3rem !important;
            }

            #menu-content .menu-section {
                border: 1px solid #ccc !important;
                border-radius: 0 !important;
                page-break-inside: avoid;
            }

            #menu-content .menu-section-header {
                background: #eee !important;
                color: #000 !important;
                padding: 0.15rem 0.5rem !important;
            }

            #menu-content .menu-section-header h2 {
                font-size: 11pt !important;
                color: #000 !important;
            }

            #menu-content .menu-section-body {
                padding: 0.25rem 0.5rem !important;
            }

            #menu-content .menu-section-body .space-y-2 > * + *,
            #menu-content .menu-section-body > * + * {
                margin-top: 0.05rem !important;
            }

            #menu-content .menu-section-body h3 {
                font-size: 10pt !important;
                margin-top: 0.2rem !important;
                margin-bottom: 0 !important;
            }

            #menu-content .menu-section-body .pl-6 {
                padding-left: 1rem !important;
            }

            /* Preserve word spacing in flex lines */
            #menu-content .menu-section-body .flex {
                gap: 0.25em !important;
            }

            #menu-content .ingredient-badge {
                font-weight: bold !important;
                color: #000 !important;
            }

            #menu-content a {
                color: #000 !important;
                text-decoration: none !important;
            }
        }
    </style>
```

- [ ] **Step 2: Confirm the override block is gone and no palette utility survives anywhere**

Run:
```bash
grep -c '\.dark ' templates/base.html
grep -rnE '(^|[ "'"'"'])(hover:|focus:|group-hover:)?(bg|text|border|from|to|via|ring|placeholder|decoration)-(gray|orange|purple|pink|blue|indigo|green|emerald|red|yellow|lime|amber|cyan|slate|white)\b' templates static/js/*.js static/css/input.css static/css/components.css static/css/cooking-mode.css src/web/templates.rs
grep -rn 'gradient' templates static/js/*.js static/css/input.css static/css/components.css static/css/cooking-mode.css
grep -rn 'dark:' templates static/js/*.js | grep -v 'dark:block\|dark:hidden'
```
Expected: `0` from the first command, nothing from the other three except comment lines in `components.css` that mention the word "gradient" (those are fine). Fix any class or property occurrence that prints before continuing.

- [ ] **Step 3: Build everything and run the full suites**

```bash
cargo fmt && cargo clippy 2>&1 | tail -3 && cargo test 2>&1 | grep -E 'test result|FAILED' 
npm run build-css && npm run build-js
npm test -- --project=chromium --workers=1
```
Expected: clippy clean, every cargo test result line `ok`, Playwright reports 0 failed. The accessibility spec asserts one `h1` per page and AA contrast; the shopping list, pantry and error pages now have an `h1`, which is what it wants.

- [ ] **Step 4: Print check**

In dark mode, open `/recipe/Neapolitan%20Pizza`, `Cmd+P`, preview: dark text on white, one column, no nav. Cancel.

Dark-mode values to verify once the override block is gone:
- Body background `#16161d` (the old `.dark body { background-color: #111827 }` used to win)
- Idle `.nav-pill` colour `#ada69b`
- Hover fill `--surface-sunk`
- Active pill `--accent-soft` / `--accent-text`

- [ ] **Step 5: Commit**

```bash
git add templates/base.html
git commit -q -F - <<'EOF'
refactor(ui): delete the dark-mode override block

Every page resolves its colours through tokens now, so the ~450 lines
of `.dark .*` utility overrides are dead. The print block is rewritten
against the current markup.

Claude-Session: https://claude.ai/code/session_013urND2B6Y3Z7WQuDpE8ZDu
EOF
```

---

### Task 13: Visual pass, PR description, force-push

**Files:** none new. This task verifies, documents and ships.

- [ ] **Step 1: Side-by-side against main**

Start a second server on main for comparison:

```bash
git worktree add /tmp/cookcli-main origin/main
cd /tmp/cookcli-main && npm install --silent && npm run build-css && npm run build-js && cargo build
```

then, in a second terminal, `/tmp/cookcli-main/target/debug/cook server /tmp/cookcli-main/seed --port 9081`.

For each of `/`, `/recipe/Neapolitan%20Pizza`, `/shopping-list` (with a recipe added), `/pantry`, `/preferences`, `/edit/Neapolitan%20Pizza`, `/api-docs`, and a menu page, compare `:9080` against `:9081` at 1440, 1024 and 820px in light and dark. What must match: nav height, card positions and heights, grid columns, button heights, step box padding, pantry block size. What must differ: colours, gradients gone, hairline borders, 6px radii, one accent.

When done, stop that server and run `git worktree remove /tmp/cookcli-main`.

- [ ] **Step 2: Cook mode in light theme**

On `:9080` in light theme open a recipe, press Cook. Entity badges on the step card are readable; the header pill for the active section is accent-filled.

- [ ] **Step 3: Static build smoke test**

```bash
cargo run -q -- build ./seed --output /tmp/cook-static 2>&1 | tail -2
ls /tmp/cook-static/static/css/
open /tmp/cook-static/index.html
```
Expected: `output.css` and `cooking-mode.css` present, no `custom-styles.css`; the index renders on tokens from `file://`, and search works (main's script-tag loader was kept).

- [ ] **Step 4: Write the PR description**

Save to `/tmp/pr-body.md`:

```markdown
Rebuilds the web UI's styling on a Tailwind v4 CSS-first token layer and adopts the Cooklang design-system palette with flat, hairline-bordered surfaces. Every page keeps the layout, spacing and dimensions it has on `main`.

This replaces the earlier version of this PR, which bundled the same foundation with a density pass (48px app bar, 60px index rows, sticky ingredient rail, compact lists). The density work is dropped; the foundation, palette, type scale and bug fixes are kept.

## Foundation

- `input.css` is fully CSS-first: `@custom-variant dark` and `@source` replace `tailwind.config.js`, which is deleted.
- Twenty semantic tokens (`--bg`, `--surface`, `--text`, `--accent`, …) registered with `@theme inline`, so `bg-surface` / `text-muted` / `border-line` are real utilities that flip under `.dark` with no override rules.
- `components.css` holds the component vocabulary. Every colour resolves through a token; no raw hex outside `@media print`.
- Deleted: `custom-styles.css` (shadowed `output.css`), `styles.css` (unreferenced), and the ~450-line `.dark .*` override block in `base.html`.

## Look

Cooklang DS palette, no gradients, one accent, 6px radii, hairline borders, two-value elevation scale. Inline entities are weight + tint (ingredients) and a dotted underline (cookware), so the distinction no longer rests on a red/green hue pair.

## Type

Seven-step scale with per-step line-heights; Tailwind's size names are aliased onto it. Page titles stay at 30px. Step text keeps its 2.0 leading.

## Fixes carried over

- Index sorter reads `data-name`, collates numerically, persists in `sessionStorage`; now covered by `recipes-sort.spec.ts`.
- Scale changes preserve scroll position; −/+ stepper shares `adjustScale` with the keyboard shortcuts.
- Every page has exactly one `h1` and no skipped heading level.
- Cook mode legible in light theme; its step capture no longer scrapes layout utilities.
- Print path works from dark theme.
- CodeMirror dark caret/gutters (rules moved out of `@layer`).
- `menu_api_test.rs` no longer pins CSS classes; `recipe-display.spec.ts` lost its vacuous guard.

## Verification

- `cargo fmt`, `cargo clippy`, `cargo test` clean.
- Playwright: full suite green in Chromium.
- Each page compared against `main` at 1440/1024/820px, light and dark.

Spec: `docs/superpowers/specs/2026-09-04-web-ui-tokens-design.md`.

https://claude.ai/code/session_013urND2B6Y3Z7WQuDpE8ZDu
```

- [ ] **Step 5: Force-push over the PR branch and update the description**

The user approved replacing PR #456's branch. Its old commits stay reachable from the PR's history.

```bash
git push --force-with-lease=design/web-ui-refresh origin design/web-ui-tokens:design/web-ui-refresh
gh pr edit 456 --title "feat(ui): token foundation and Cooklang palette, existing layout kept" --body-file /tmp/pr-body.md
gh pr view 456 --json url,title -q '.url + " " + .title'
```

If `--force-with-lease` is refused because the remote moved, fetch, look at what changed on `design/web-ui-refresh`, and ask the user before retrying.

- [ ] **Step 6: Report**

Tell the user: the PR URL, the number of commits on the branch, which E2E projects were run, and anything skipped.

---

## Self-review against the spec

| Spec section | Task |
|---|---|
| 1.1 `input.css` structure, `@custom-variant`, `@source`, no `@config` | 1 |
| 1.2 tokens (light, dark, print) | 1 |
| 1.3 type scale, 30px display, `text-3xl` alias, 2.0 step leading | 1 (`.step-body` in components) |
| 1.4 component retuning and additions | 1 |
| 1.5 stylesheet deletions, cook-mode replacement | 1 |
| 1.6 `base.html` style block | 2 (search rules), 12 (dark block, print) |
| 3.1 base | 2 |
| 3.2 recipes | 3 |
| 3.3 recipe | 5 |
| 3.4 shopping list | 6 |
| 3.5 pantry | 7 |
| 3.6 menu | 8 |
| 3.7 preferences | 9 |
| 3.8 edit, new | 10 |
| 3.9 api docs, error, `method_classes` | 11 |
| 3.10 scripts | 4 |
| 4 heading semantics | 3, 5, 6, 7, 11 |
| 5 behaviour fixes | 3 (sorter), 5 (scale), 1 (cook-mode tokens, print, CodeMirror, no transitions), 4 (cook-mode JS), 8 (menu test) |
| 6 tests | 3, 5, 8, 9 |
| 7 verification | 12, 13 |
| 8 sequencing | task order |
