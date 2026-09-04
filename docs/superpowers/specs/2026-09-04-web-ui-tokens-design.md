# Web UI: Token Foundation With the Existing Layout

**Date:** 2026-09-04
**Status:** Approved, ready for planning
**Supersedes:** PR #456 (`design/web-ui-refresh`). This spec reuses that PR's CSS foundation and visual identity and discards its density work. At the end, this branch is force-pushed over `design/web-ui-refresh` so PR #456 keeps its number and discussion; its description is rewritten to match this spec.

## Goal

Rebuild the web UI's styling on Tailwind v4's CSS-first model with a semantic token layer, adopt the Cooklang design-system palette and the flat, hairline-bordered surface language from PR #456, and make the pages visually consistent with one another. Keep every page's structure, spacing, and dimensions as they are on `main` today.

## Non-goals

- No compaction. The 48px app bar, the 60px index rows, the sticky ingredient rail, the unboxed hairline step list, the single-line pantry rows, and the compact shopping list from PR #456 are not adopted.
- No backend, routing, template-data, or API changes.
- No change to cook mode's layout. Only its stylesheet moves onto tokens.
- No new pages, no new features beyond the behaviour fixes listed in section 5.

## Starting point

`main` is already on Tailwind v4 (`tailwindcss` and `@tailwindcss/cli` 4.3.3). Its `input.css` still drives Tailwind through `@config "../../tailwind.config.js"`, defines its components with `@apply` and raw hex gradients, and relies on a ~450-line block of `.dark .*` overrides in `templates/base.html` plus two extra stylesheets (`custom-styles.css`, which shadows `output.css`, and `styles.css`, which nothing references).

Work happens on a new branch from `main` (`design/web-ui-tokens`). Nothing is cherry-picked from PR #456; its final CSS files are used as the reference and copied where the spec says so.

## 1. Foundation

### 1.1 `static/css/input.css`

Structure, in order:

1. `@import "tailwindcss";`
2. `@custom-variant dark (&:where(.dark, .dark *));` — replaces `darkMode: 'class'`.
3. `@source "../../templates";`, `@source "../../static/js";` and `@source "../../src/web/templates.rs";` — replaces the content globs. The one Rust file that emits class names is `src/web/templates.rs` (`method_classes()` for the API docs method badge), so it is scanned explicitly; the rest of `src/` is not.
4. Token declarations: `:root` (light), `.dark, .cooking-overlay` (dark), each with `color-scheme`.
5. `@theme inline` registering every colour token as a Tailwind colour.
6. `@theme` type scale.
7. `body { font-size: var(--text-body); line-height: 1.5; }`
8. `@import "./components.css";`
9. `@media print` token reset and layout flattening.
10. CodeMirror rules, outside any layer.

`@config` is removed and `tailwind.config.js` is deleted. The gradient keyframes it defined are unused once gradients are gone.

### 1.2 Tokens

Twenty semantic colour tokens plus two radii, two shadows. Values are the PR's final values, verbatim.

| Token | Light | Dark |
|---|---|---|
| `--bg` | `#fcfcfb` | `#16161d` |
| `--surface` | `#ffffff` | `#1c1c24` |
| `--surface-sunk` | `#f5f3f0` | `#23232c` |
| `--border` | `#e4e0da` | `#30303b` |
| `--border-strong` | `#c3bcb1` | `#43434f` |
| `--text` | `#16161d` | `#efeae6` |
| `--text-muted` | `#5f5a51` | `#ada69b` |
| `--text-faint` | `#6a645b` | `#948d83` |
| `--accent` | `#e15a29` | `#e15a29` |
| `--accent-text` | `#715329` | `#f08050` |
| `--accent-soft` | `#f5dacf` | `#3a2820` |
| `--accent-ink` | `#16161d` | `#16161d` |
| `--ok` | `#3d6849` | `#6fb283` |
| `--ok-soft` | `#e2e8df` | `#1e2a22` |
| `--danger` | `#c4261c` | `#ff6b60` |
| `--danger-soft` | `#f7dfdc` | `#2e1b1a` |
| `--danger-ink` | `#ffffff` | `#16161d` |
| `--info` | `#8a3d14` | `#e59a6d` |
| `--disabled` | `#d3cdcb` | `#4a4a55` |
| `--inactive` | `#8a8075` | `#8f8880` |
| `--radius-control` | `6px` | `6px` |
| `--radius-card` | `6px` | `6px` |
| `--shadow-card` | `0 1px 0 rgba(27,31,36,.04)` | `none` |
| `--shadow-overlay` | `0 8px 24px rgba(27,31,36,.12)` | `0 8px 24px rgba(0,0,0,.5)` |

The print block resets all of these to the PR's print values (white surfaces, dark text, no shadows) under `:root, .dark, .cooking-overlay`.

Tailwind names, registered under `@theme inline`: `bg`, `surface`, `sunk`, `line`, `line-strong`, `text`, `muted`, `faint`, `accent`, `accent-text`, `accent-soft`, `accent-ink`, `ok`, `ok-soft`, `danger`, `danger-soft`, `danger-ink`, `info`, `disabled`, `inactive`. Border tokens are named `line` so the utility reads `border-line`.

No raw hex appears outside the token declarations and `@media print`. No Tailwind palette utility (`gray-*`, `orange-*`, `purple-*`, …) appears in any template, script, or stylesheet. No `dark:` variant appears in any template; every colour flips through its token.

### 1.3 Type scale

The PR's seven steps, with the page-title step raised to main's size.

| Step | Size | Line-height | Role |
|---|---|---|---|
| `display` | **30px** | 1.2 | page title (`h1`) |
| `title` | 18px | 1.35 | section headings |
| `read` | 16px | 1.6 | recipe step text, notes |
| `body` | 14px | 1.5 | list rows, card titles, default |
| `ui` | 13px | 1.4 | controls, metadata |
| `meta` | 12px | 1.4 | captions, counts, tags |
| `label` | 11px | 1.3 | uppercase section labels |

`display` and `title` carry the PR's negative letter-spacing. Tailwind's `text-xs`, `text-sm`, `text-base`, `text-lg`, `text-2xl` are aliased onto `meta`, `body`, `read`, `title`, `display` as in the PR; `text-3xl` is also aliased onto `display`. `text-4xl` is not used anywhere after this change. Four weights only: 400, 500, 600, 700.

Exception: recipe step text keeps main's `leading-8` (2.0 line-height) on 16px text. This is the one place the reading step's line-height is overridden, and it is a deliberate density choice.

### 1.4 `static/css/components.css`

Copied from the PR, then adjusted. Everything lives in `@layer components` and resolves through tokens.

**Kept as in the PR:** `:focus-visible` ring, `.card`, `.card-head` (including `.plain` and `.count`), `.btn-danger` colours, `.select` colours, `.row` family (used by the shopping list sidebar and JS-rendered lists), `.metaline` (available but unused), `.section-label`, `.item-status-dot` states, `.ingredient-badge`, `.cookware-badge`, `.timer-badge`, `.tag`, `.metadata-pill`, `.recipe-note`, `.step-refs`, `.image-step`, `.recipe-image-placeholder`, `.search-input` colours, `.nav-pill` colours, `a.text-info` underline, the coarse-pointer 44px target block, and the "no transitions on token colours" rule.

**Retuned to main's dimensions:**

| Class | Change |
|---|---|
| `.btn`, `.btn-primary`, `.btn-danger` | height 40px, padding `0 16px`, `text-body`, svg 20px. Matches main's `px-4 py-2` buttons. |
| `.select` | height 40px to sit level with `.btn`. |
| `.stepper` | height 40px; buttons 32px wide; input 56px wide. |
| `.nav-pill` | main's `px-5 py-2`, `border-radius: 9999px`, `text-body`. Flat fill states: hover `--surface-sunk`; active `--accent-soft` background, `--accent-text` colour, weight 600. |
| `.icon-btn` | 36px square (main's `p-2` + 20px icon), radius `--radius-control`, svg 20px. |
| `.search-input` | height 44px, padding `0 16px 0 40px`, `text-body`; keeps PR's border and focus rule. |
| `.step-number` | 32px circle (main), `--accent-soft` fill, `--accent-text` glyph, `text-body` weight 700, no border. |
| `.recipe-card` | block card: `display:flex; flex-direction:column; overflow:hidden`, `--surface`, hairline, `--radius-card`, `--shadow-card`; hover `border-color: var(--border-strong)`. No `::before` gradient stripe, no scale transform. Content padding is `p-6` in the template as today. |
| `.recipe-card-icon` | 64px circle, `--surface-sunk` fill, hairline, emoji at 24px. |
| `.recipe-card-title` | `h2` styled as main's `text-lg font-bold`, i.e. `text-title` 700, `--text`. |
| `.recipe-card-sub` | `text-meta`, `--text-faint`. |

**Removed:** `.appbar`, `.appbar-brand`, `.appbar-nav`, `--appbar-h`, `.recipe-layout`, `.recipe-rail`, `.step-list` / `.step-list > li` hairline rules, `.step-body` max-width, `.ingredient-list` padding, `.pantry-item` single-line rules, `.pantry-dates`, `.pantry-date` separators, `.pantry-actions` opacity rules.

**Added:**

| Class | Definition |
|---|---|
| `.nav-card` | main's nav container: `--surface`, hairline, `--radius-card`, `--shadow-card`, `margin-bottom: 2rem`. Not sticky. |
| `.step-list` | `list-style:none; margin:0; padding:0;` only. Step boxes are `li.step-box`. |
| `.step-box` | main's per-step box: `--surface-sunk` fill, hairline, `--radius-card`, `padding: 1rem`. |
| `.step-body` | `font-size: var(--text-read); line-height: 2;` |
| `.ingredient-list` | `list-style:none; margin:0; padding:0;` |
| `.ingredient-row` | main's tinted row: `display:flex; justify-content:space-between; align-items:center; padding: .5rem .75rem; border-radius: var(--radius-control); background: var(--surface-sunk);` Quantity uses `.row-value`. Note uses `.row-note`. |
| `.pantry-item` | main's block: `--surface-sunk` fill, `--radius-control`, `padding: 1rem`, `border: 1px solid transparent`; hover `border-color: var(--border)`. `.out-of-stock` variant: `--danger-soft` fill, `--danger` border; its `.quantity-display` is `--danger`. `.low-stock` variant: `--accent-soft` fill, `--accent` border; its `.quantity-display` is `--accent-text`. The JS toggles only these state classes; no utility classes are added at runtime. |
| `.pantry-actions` | opacity 0, revealed on `.pantry-item:hover`, `:focus-within`; always visible on coarse pointers. (Same behaviour as main's `group-hover:opacity-100`.) |

### 1.5 Other stylesheets

- `static/css/custom-styles.css` and `static/css/styles.css` are deleted; the `<link>` to `custom-styles.css` in `base.html` is removed.
- `static/css/cooking-mode.css` is replaced with the PR's tokenised version verbatim (66 added / 58 removed lines against main). Its layout is unchanged.
- `static/css/output.css` stays gitignored and is never committed.

### 1.6 `templates/base.html` `<style>` block

The `.dark .*` override block (roughly lines 20–450 on main) is deleted. What remains inline is `.viewport`, the search-selected rule (PR's accent-bar version), and the print block, rewritten so every selector matches the new markup: `.nav-card`, `.card`, `.step-box`, `.ingredient-row`, `#shopping-list-results .card`, `#menu-content .menu-section`. Print rules that targeted utility classes no longer present (`.bg-white`, `.md\:col-span-*`, `.bg-gradient-to-r`, `.text-orange-600`, `.rounded-2xl`, `.shadow-lg`) are removed rather than kept as dead code.

## 2. Component vocabulary

A template states intent through these classes and uses Tailwind utilities only for layout (flex, grid, gap, margin, padding, width, responsive variants) and for token colours (`bg-surface`, `text-muted`, `border-line`, …).

| Need | Use |
|---|---|
| Bordered surface | `.card`, optionally `.card-head` |
| Action | `.btn`; the one primary action per view `.btn-primary`; destructive confirm `.btn-danger` |
| Icon-only control | `.icon-btn` |
| Dropdown | `.select` |
| Recipe scale | `.stepper` with `−` / input / `+` |
| Tag | `.tag` |
| Recipe metadata | `.metadata-pill` with the emoji prefix; the `metadata-*` key classes stay in the markup as hooks but carry no colour |
| Inline entity in prose | `.ingredient-badge`, `.cookware-badge`, `.timer-badge` |
| Recipe-reference link | `text-info hover:underline` |
| Note or description | `.recipe-note` |
| Error banner | `.card p-4 border-l-[3px] border-l-danger bg-danger-soft` with `text-danger` icon |
| Floating panel (dropdown menu, search results, modal) | `.card` + `shadow-[var(--shadow-overlay)]` |

## 3. Pages

Every template starts from its `main` version. Edits are class substitutions plus the items called out here. Structure, grid columns, paddings, margins, and element sizes are otherwise untouched.

### 3.1 `base.html`

- `<body class="bg-bg text-text">`.
- Nav: `<nav class="nav-card relative">` wrapping main's `px-3 lg:px-6 py-4` inner layout. Brand image, search container (`w-full` input with `.search-input`, results panel `.card shadow-[var(--shadow-overlay)]`), nav pills, gear link as `.icon-btn` (active state `bg-accent-soft text-accent-text`), shortcuts and theme buttons as `.icon-btn`, small-screen more menu as `.card shadow-[var(--shadow-overlay)]` with `hover:bg-sunk` rows and `text-accent-text font-semibold` for the active item.
- Search: main's inline search script is kept (server mode) and main's `static/js/search.js` loader is kept (static mode). Only the result-row classes change: `hover:bg-sunk border-b border-line`, title `text-text`, empty state `text-muted`.
- Footer links `text-accent-text hover:underline`.

### 3.2 `recipes.html`

- Header: `h1.text-display.font-bold.text-text`; New recipe `.btn.btn-primary`.
- Sort controls: label `text-ui text-muted`, `.select`, direction `.btn` with `aria-label` from `tr.t("sort-direction-toggle")`, a key that already exists in every locale on `main`.
- Today's menu: `.card p-6 border-l-[3px] border-l-accent`; heading `h2.text-title`; view link `.btn`.
- Grid: `grid md:grid-cols-2 lg:grid-cols-3 gap-6` unchanged.
- Cards: `a.recipe-card` with `data-name`. Folder card: `.recipe-card-icon` 📁, `h2.recipe-card-title`, count `text-accent-text text-sm font-medium`. Recipe card: image band `h-48` unchanged, or `.recipe-card-icon` with 🍽️ / 📋; `h2.recipe-card-title`; menu badge `.tag`; description `text-muted text-sm`; tags `.tag`; overflow `+n` `text-faint text-xs`.
- Sorter script: the PR's version (reads `data-name`, `Intl.Collator` numeric, `sessionStorage` persistence with guarded read).

### 3.3 `recipe.html`

- Breadcrumb colours on tokens.
- Title row: `h1.text-display.font-bold.text-text.print:text-2xl`. Actions: `.stepper` (`−`, input `#scale`, `+`; label `for="scale"` kept visible as `text-sm font-medium text-muted mr-2` as on main), Edit `.btn`, Add to shopping list `.btn`, Cook `.btn.btn-primary`, static-mode download `.btn`. Labels are visible at every width: the `hidden lg:inline` spans lose both classes.
- Tags row `.tag`; description `.recipe-note`.
- Metadata: `#metadata-container` `flex flex-wrap gap-3` of `.metadata-pill` with emoji, keeping each `metadata-*` class. Source URL link `text-info hover:underline`.
- Grid `grid md:grid-cols-3 gap-8 mb-8` unchanged. Ingredients column: `.card p-6`, `h2.text-title.font-bold.text-accent-text` with emoji as on main, section `h3.text-title.font-semibold.text-accent-text`, `ul.ingredient-list.space-y-3` of `li.ingredient-row`; reference checkbox `accent-[var(--accent)]`; reference link `text-info hover:underline`; note `.row-note.italic.break-words`; quantity `.row-value`. Cookware heading `h2.text-title.font-bold.text-text` with its emoji (the design system has no green, so it does not get a second accent); list rows `.ingredient-row`.
- Steps column: `.card p-6`; section `h3.text-title.font-semibold.text-accent-text.border-b.border-line.pb-2`; `ol.step-list.space-y-4`; each step `li.step-box` containing main's flex layout, `.step-number`, `.step-body` for the prose, `.step-refs` for the per-step ingredient line at main's position (`mt-2 pl-4 border-l-2 border-accent`). Notes `li.recipe-note`.
- Scripts: `goToScale` stashes `scrollY` in `sessionStorage` before navigating; a restore IIFE reads and clears it on load. `adjustScale` is the shared one exported from `keyboard-shortcuts.js`; the stepper buttons call it.

### 3.4 `shopping_list.html`

- Sidebar `.card p-6 sticky top-6`; headings become `h2.text-title.font-bold` (`text-accent-text` for selected recipes, `text-text` for pantry). Pantry box `bg-sunk rounded-[var(--radius-control)] p-3 border border-line`.
- Error banner per section 2. Page title becomes `h1.text-display.font-bold.text-text` (main has no `h1` on this page).
- Copy split button `.btn.btn-primary` halves; options menu `.card shadow-[var(--shadow-overlay)]`; Clear `.btn`.
- JS-rendered markup: selected-recipe entries and nested references on tokens; aisle groups `.card p-4` with `h3.text-title.font-semibold`; items keep main's row structure with `accent-[var(--accent)]` checkboxes and `line-through text-faint` when checked. The `aisle-name` hook stays.

### 3.5 `pantry.html`

- Title `h1.text-display`; filter label and count on tokens.
- Unconfigured state `.card p-8 text-center`.
- Sections `.card p-6`, `h2.text-title.font-semibold.capitalize`; grid `gap-3 md:grid-cols-2 lg:grid-cols-3` unchanged; items `div.pantry-item.group` with main's four-line stat block; status dot `.item-status-dot` with `in-stock` / `low-stock` / `out-of-stock`; edit `.icon-btn` (title only, no aria-label, per the PR's pantry test note), delete `.icon-btn text-danger`; edit form inputs `border border-line rounded-[var(--radius-control)] bg-surface text-text`; Save `.btn.btn-primary`, Cancel `.btn`.
- Footer buttons `.btn.btn-primary` / `.btn`. Add-item modal `.card` with `shadow-[var(--shadow-overlay)]`.

### 3.6 `menu.html`

Six gradients become `.card`. Section header keeps its `menu-section-header` hook with `bg-sunk border-b border-line`. Head aligned with `recipe.html` (title `text-display`, actions `.btn`). The scale badge after each reference link keeps its element but its classes become `text-sm text-faint`.

### 3.7 `preferences.html`

Eleven gradients become `.card p-6` sections with `h2.text-title`. Language and feature toggle buttons are `.btn` with `.btn-primary` when active and carry `data-active="true|false"`.

### 3.8 `edit.html`, `new.html`

`.btn` / `.btn-primary` / `.btn-danger`, `.card`, form fields `border-line bg-surface text-text`. All `dark:` utilities removed. CodeMirror theming comes from the unlayered rules in `input.css`.

### 3.9 `api_docs.html`, `error.html`

Cards and tokens; all `dark:` utilities removed; code blocks `bg-sunk`. The HTTP method badge classes come from `method_classes()` in `src/web/templates.rs`; they become `bg-sunk text-text` (GET), `bg-ok-soft text-ok` (POST), `bg-accent-soft text-accent-text` (PUT), `bg-danger-soft text-danger` (DELETE), `bg-sunk text-muted` (other). This is a string change only, no behaviour change.

### 3.10 Scripts

- `static/js/keyboard-shortcuts.js`: modal and `kbd` classes on tokens; `window.adjustScale = adjustScale;` exported.
- `static/js/search.js`: result-row classes only. The loader is not touched.
- `static/js/cooking-mode.js`: `captureStepHTML` reads `ol.step-list > li` and `.step-body`.

## 4. Heading semantics

Every page has exactly one `h1` and no skipped level. Concretely: recipes index cards are `h2`; shopping list gets an `h1` and its sidebar headings become `h2`; pantry sections are `h2`, items `h3`; recipe page ingredients/steps headings are `h2`, sections `h3`.

## 5. Behaviour fixes ported from PR #456

1. Index sorter: `data-name`, numeric collation, persisted choice.
2. Scale change preserves scroll position; stepper buttons and keyboard shortcuts share one `adjustScale`.
3. Cook mode legible in light theme (`.cooking-overlay` in the dark token selector).
4. Cook mode step capture no longer scrapes layout utilities.
5. Print path: token reset under `@media print`; dark-theme recipes print dark-on-white.
6. CodeMirror dark caret and gutters: rules outside `@layer`.
7. No CSS transitions on token-valued colours (theme-toggle race).
8. `tests/menu_api_test.rs` regex is class-agnostic.

## 6. Tests

- `tests/e2e/navigation.spec.ts`: `h3` → `.recipe-card-title`.
- `tests/e2e/preferences.spec.ts`: gradient class assertions → `data-active`.
- `tests/e2e/recipe-display.spec.ts`: `ul.space-y-3 li` → `ul.ingredient-list li`; `span.italic.text-gray-600` → `span.row-note`; `.text-sm.text-gray-600.mt-2` → `.step-refs`. The metadata test keeps `.metadata-pill` but drops the `if (count > 0)` guard and asserts the Easy Pancakes values.
- `tests/e2e/recipes-sort.spec.ts`: added from the PR.
- `tests/e2e/tablet.spec.ts`: not added.
- `tests/menu_api_test.rs`: regex change.

## 7. Verification

- `cargo fmt`, `cargo clippy`, `cargo test` clean.
- `npm run build-css` succeeds with no warnings; `grep -rE 'gray-|orange-|purple-|blue-|green-|pink-|yellow-|indigo-|lime-|red-|dark:|gradient' templates static/js static/css/input.css static/css/components.css` returns nothing outside comments.
- `npm test` passes (serial for the `shopping-list*.spec.ts` files, which race on the shared fixture).
- Browser pass over every page at 1440, 1024, and 820px in light and dark, compared against `main`: same element positions and sizes, new colours and surfaces. Print preview of a recipe in dark theme is dark-on-white.
- Cook mode opened from a light-theme recipe: entity badges readable.

## 8. Sequencing

1. Foundation: `input.css`, `components.css`, deletions, `cooking-mode.css`, CodeMirror. Additive; every page still renders through the old override block.
2. `base.html` markup, keeping the override block for now.
3. Pages one at a time: recipes, recipe, shopping list, pantry, menu, preferences, edit/new, api docs/error. Each with its test updates.
4. Scripts.
5. Delete the `.dark .*` block and rewrite the print block, last, once no page depends on it.
6. Full verification, PR description, force-push to `design/web-ui-refresh`.

## Deferred (unchanged from PR #456)

- `recipe-method` locale key; the recipe page's steps card has no heading today and this spec does not add one.
- Decouple `--ok` from its stock-status and cookware roles.
- Format pantry quantities (`250%g` → `250 g`).
- Cook mode's own type sizes.
