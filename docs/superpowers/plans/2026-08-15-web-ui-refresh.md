# Web UI Design Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the CookCLI web UI's ad-hoc styling with a semantic design-token system, producing a consistent, compact interface that works well on a tablet.

**Architecture:** A token layer (CSS custom properties, flipped once per theme) feeds a small component layer (`.card`, `.btn`, `.row`, …) defined in `static/css/input.css`. Templates are then migrated page-by-page from raw Tailwind utility strings to those components. The ~330-line `.dark .*` utility-override block in `base.html` is removed in stages, as each set of classes stops depending on it.

**Tech Stack:** Rust + Axum + Askama templates, Tailwind CSS v4 (CSS-first, with a legacy `@config` shim), Playwright for E2E.

**Spec:** [`docs/superpowers/specs/2026-08-15-web-ui-refresh-design.md`](../specs/2026-08-15-web-ui-refresh-design.md)
**Visual reference:** [`docs/superpowers/specs/2026-08-15-web-ui-refresh-mockup.html`](../specs/2026-08-15-web-ui-refresh-mockup.html) — open in a browser; `⇄` switches recipe/list view, `◐` toggles theme.

---

## Critical workflow notes

Read these before starting. They are not obvious and will waste your time if you miss them.

**1. Askama templates compile into the binary.** Editing a `.html` file in `templates/` has *no effect* on a running server. After every template edit you must:

```bash
cargo build
```

and restart the server. `CLAUDE.md` claims templates recompile per request — that is wrong for this Askama 0.12 setup.

**2. CSS must be rebuilt too.** `static/css/output.css` is a build artifact. After editing `static/css/input.css`:

```bash
npm run build-css
```

During development `npm run watch-css` does this automatically.

**3. The standard loop for any change in this plan:**

```bash
npm run build-css && cargo build && ./target/debug/cook server ./seed --port 9080
```

**4. Playwright reuses a running server** (`reuseExistingServer: !process.env.CI`). If you leave a stale server on port 9080, tests will run against stale templates and give meaningless results. Kill it before running tests, or make sure you rebuilt and restarted it.

**4a. Measured E2E baseline** (established in Task 2, commit `742606d`). Compare every later run against *this*, not against "all green":

| | Count |
| --- | --- |
| Total | 134 |
| Passed | 127 |
| Failed | 2 (flaky — see below) |
| Skipped | 5 (pre-existing `test.skip` in source) |

The 2 failures are **pre-existing test-isolation flakiness, not regressions**:
- `shopping-list-copy.spec.ts:89` "leaves out items that are already ticked off"
- `shopping-list.spec.ts:29` "should add recipe ingredients to shopping list"

Both pass when rerun with `--workers=1`. Cause: the shopping list persists to single shared files — `seed/.shopping-list` and `seed/.shopping-checked` — and `playwright.config.ts` sets `fullyParallel: true` with unbounded local workers, which race on them. `shopping-list-copy.spec.ts` overwrites those files directly in `beforeEach`; `shopping-list.spec.ts:29` mutates the same files through the UI. When either fails, rerun that spec alone with `--workers=1` before concluding you broke something.

(Note: `CLAUDE.md` says the shopping list lives at `/tmp/shopping_list.txt`. That is stale — the real paths are `seed/.shopping-list` / `seed/.shopping-checked`, written relative to the server's base directory. Both are gitignored, so `git add -A` will not sweep them up.)

The 5 skips are `accessibility.spec.ts:44`, `recipe-display.spec.ts:31`, `recipe-display.spec.ts:44`, `shopping-list.spec.ts:54`, `shopping-list.spec.ts:91`.

**4b. Tests mutate a tracked seed fixture.** The pantry tests write to `seed/config/pantry.conf`, which IS tracked in git. After any test run, restore it before committing so it isn't swept into your commit:

```bash
git checkout -- seed/config/pantry.conf
```

**4c. Playwright browsers on this host.** macOS 13.7.8 has no published Playwright Chromium build. Install with:

```bash
PLAYWRIGHT_HOST_PLATFORM_OVERRIDE=mac14 npx playwright install chromium
```

This pulls the mac14 Chrome-for-Testing build, which runs correctly here. Local environment only — never commit anything for this.

**5. Do not rename these class names.** The E2E suite (134 tests) selects on them directly:

| Class | Used by |
| --- | --- |
| `.nav-pill`, `.nav-pill.active` | `navigation.spec.ts` (7 assertions), `preferences.spec.ts` |
| `.recipe-card` | `navigation.spec.ts`, `test-helpers.ts` |
| `.ingredient-badge` | `recipe-display.spec.ts`, `test-helpers.ts` |
| `.metadata-pill` | `test-helpers.ts` |

These classes are **restyled, not renamed**. Keeping the names is deliberate: it preserves the test suite as a regression net for a change that touches every page.

---

## File structure

| File | Responsibility after this plan |
| --- | --- |
| `static/css/input.css` | **Single source of styling truth.** Token definitions, `@theme` registration, component layer. |
| `static/css/output.css` | Build artifact. Never edit by hand, and **never commit it** — it is gitignored (`.gitignore:5`) and has never been tracked. `git add static/css/output.css` errors; `git add -A` correctly skips it. |
| `static/css/custom-styles.css` | **Deleted.** Currently duplicates `input.css`'s component layer with different values, and loads *after* `output.css` so it silently wins. |
| `static/css/styles.css` | **Deleted.** 444 lines, referenced by nothing. |
| `static/css/cooking-mode.css` | Keeps its own layout, but consumes tokens for colour and inherits the shared badge classes. |
| `templates/base.html` | App shell + 48px app bar. Its inline `<style>` block keeps only `.viewport` and the `@media print` rules. |
| `templates/*.html` | Semantic markup using the component layer. No raw colour utilities. |
| `tailwind.config.js` | Content globs + `darkMode: 'class'` only. Legacy `primary-orange` / `light-*` colours removed. |
| `tests/e2e/tablet.spec.ts` | **New.** Tablet-viewport regression tests (820×1180). |

---

## Task 1: Design tokens

Additive only. No page changes appearance in this task.

**Files:**
- Modify: `static/css/input.css:1-7` (after the `@config` line)

- [ ] **Step 1: Add the token layer**

Insert immediately after the `@config "../../tailwind.config.js";` line in `static/css/input.css`, before the existing `/* Custom component classes */` comment:

```css
/* ============================================================
   DESIGN TOKENS
   Fifteen semantic names, defined once per theme. Every colour
   in the UI resolves through one of these. Do not introduce raw
   hex values or Tailwind palette utilities in templates.
   --accent-ink is the foreground colour for text and icons sitting ON a
   filled --accent background. It is white in light mode and near-black in
   dark mode, because the dark accent is deliberately vivid and cannot
   carry white text at AA contrast.
   ============================================================ */
:root {
    --bg:            #faf9f7;
    --surface:       #ffffff;
    --surface-sunk:  #f4f2ef;
    --border:        #e6e2dc;
    --border-strong: #d5cfc6;
    --text:          #1b1917;
    --text-muted:    #6f6862;
    --text-faint:    #736c66;
    --accent:        #c2410c;
    --accent-text:   #b0400f;
    --accent-soft:   #fdefe6;
    --accent-ink:    #ffffff;
    --ok:            #2f7d55;
    --ok-soft:       #e7f4ec;
    --info:          #2b6cb0;

    --radius-control: 6px;
    --radius-card:   10px;
    --shadow-card:   0 1px 2px rgba(0, 0, 0, .04);
}

.dark {
    color-scheme: dark;
    --bg:            #131519;
    --surface:       #1a1d22;
    --surface-sunk:  #212530;
    --border:        #2b3038;
    --border-strong: #3a414b;
    --text:          #eceef1;
    --text-muted:    #9aa2ac;
    --text-faint:    #858e99;
    --accent:        #ff7a45;
    --accent-text:   #ff9166;
    --accent-soft:   #2c1e17;
    --accent-ink:    #131519;
    --ok:            #5fbe8b;
    --ok-soft:       #17271f;
    --info:          #7ab3ea;

    --shadow-card:   none;
}

/* Register tokens with Tailwind so they generate real utilities
   (bg-surface, text-muted, border-line, …). `inline` makes the
   generated utilities reference the custom property, so they flip
   automatically under .dark with no override rules.
   Border tokens are named `line` so the utility reads `border-line`
   rather than `border-border`. */
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
    --color-info:        var(--info);
}
```

- [ ] **Step 2: Build the CSS**

Run: `npm run build-css`
Expected: exits 0, no errors.

- [ ] **Step 3: Verify the utilities were generated**

Run: `grep -c "color-surface\|color-accent-text\|color-line-strong" static/css/output.css`
Expected: a number greater than `0`. If it prints `0`, the `@theme inline` block was not picked up — check it sits at the top level of `input.css`, not nested inside `@layer`.

- [ ] **Step 4: Verify no visual regression**

Run: `npm run build-css && cargo build && ./target/debug/cook server ./seed --port 9080`
Open `http://localhost:9080/recipe/Neapolitan Pizza` and toggle light/dark.
Expected: the page looks exactly as it did before. Tokens are defined but nothing consumes them yet.

- [ ] **Step 5: Commit**

```bash
git add static/css/input.css
git commit -m "feat(ui): add semantic design tokens"
```

---

## Task 2: Remove duplicate and dead stylesheets

`custom-styles.css` defines `.recipe-card`, `.ingredient-badge`, `.cookware-badge`, `.timer-badge`, `.btn-primary`, `.search-input`, `.recipe-image-placeholder`, `.step-number`, `.tag`, `.nav-pill`, `.metadata-pill` — all of which `input.css` *also* defines, with different values. Because it is linked after `output.css`, it wins. `styles.css` is referenced by nothing.

**Files:**
- Delete: `static/css/custom-styles.css`
- Delete: `static/css/styles.css`
- Modify: `templates/base.html:13`
- Modify: `static/css/input.css` (add the one rule `custom-styles.css` had that `input.css` lacks)

- [ ] **Step 1: Confirm `styles.css` is genuinely unreferenced**

Run: `grep -rn "styles.css" templates/ src/ crates/ static/js/ 2>/dev/null | grep -v custom-styles`
Expected: no output. If anything prints, stop and investigate before deleting.

- [ ] **Step 2: Port the one missing rule into `input.css`**

`custom-styles.css` sizes the icon inside a metadata pill; `input.css` does not. Add inside the existing `@layer components { … }` block in `static/css/input.css`:

```css
    .metadata-pill svg {
        @apply w-4 h-4 mr-2;
    }
```

- [ ] **Step 3: Delete both files**

```bash
git rm static/css/custom-styles.css static/css/styles.css
```

- [ ] **Step 4: Remove the stylesheet link**

In `templates/base.html`, delete line 13:

```html
    <link href="{{ prefix }}/static/css/custom-styles.css" rel="stylesheet">
```

- [ ] **Step 5: Rebuild and verify**

Run: `npm run build-css && cargo build && ./target/debug/cook server ./seed --port 9080`
Open `http://localhost:9080/recipe/Neapolitan Pizza`.
Expected: page still renders with badges, step numbers and nav pills styled. Minor visual shifts are expected and correct — `input.css`'s versions differ slightly from the deleted duplicates (for example `.metadata-pill` loses its grey gradient for a flat white background). Nothing should be *unstyled*.

- [ ] **Step 6: Run the full E2E suite as a baseline**

Run: `npm test`
Expected: all 134 tests: 127 passed, 2 flaky, 5 skipped (see baseline above). **Record any pre-existing failures now** — you need this baseline to tell your regressions apart from failures that were already there.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "refactor(ui): consolidate stylesheets into input.css"
```

---

## Task 3: Component layer

Additive. Adds the new classes; does not yet change the old ones or any template.

**Files:**
- Modify: `static/css/input.css` (inside the existing `@layer components` block)

- [ ] **Step 1: Add the components**

Add at the top of the existing `@layer components { … }` block in `static/css/input.css`:

```css
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
        font-size: 11.5px;
        font-weight: 650;
        letter-spacing: .06em;
        text-transform: uppercase;
        color: var(--text-muted);
    }

    .card-head .count {
        font-size: 12px;
        color: var(--text-faint);
        font-variant-numeric: tabular-nums;
    }

    /* ---------- Controls ---------- */
    .btn {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        height: 32px;
        padding: 0 11px;
        border-radius: var(--radius-control);
        font-size: 13.5px;
        font-weight: 550;
        white-space: nowrap;
        cursor: pointer;
        text-decoration: none;
        background: var(--surface);
        color: var(--text);
        border: 1px solid var(--border-strong);
    }

    .btn:hover { background: var(--surface-sunk); }

    .btn svg { width: 15px; height: 15px; }

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

    .stepper {
        display: inline-flex;
        align-items: center;
        height: 32px;
        overflow: hidden;
        background: var(--surface);
        border: 1px solid var(--border-strong);
        border-radius: var(--radius-control);
    }

    .stepper button {
        width: 26px;
        height: 100%;
        border: 0;
        background: transparent;
        color: var(--text-muted);
        cursor: pointer;
        font-size: 14px;
    }

    .stepper button:hover {
        background: var(--surface-sunk);
        color: var(--text);
    }

    .stepper input,
    .stepper .value {
        width: 44px;
        height: 100%;
        border: 0;
        background: transparent;
        color: var(--text);
        text-align: center;
        font-size: 13.5px;
        font-weight: 600;
        font-variant-numeric: tabular-nums;
    }

    .stepper input:focus { outline: none; }

    /* ---------- Lists ---------- */
    .row {
        display: flex;
        align-items: baseline;
        gap: 10px;
        padding: 7px 14px;
        font-size: 14px;
    }

    .row + .row { border-top: 1px solid var(--border); }

    .row .row-name { flex: 1; min-width: 0; }

    .row .row-value {
        white-space: nowrap;
        font-weight: 600;
        font-size: 13.5px;
        color: var(--accent-text);
        font-variant-numeric: tabular-nums;
    }

    .row .row-note {
        color: var(--text-faint);
        font-size: 12.5px;
    }

    /* ---------- Text ---------- */
    /* One quiet dot-separated line, replacing rows of coloured pills. */
    .metaline {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 6px 0;
        font-size: 13px;
        color: var(--text-muted);
    }

    .metaline > *:not(:last-child)::after {
        content: "·";
        margin: 0 8px;
        color: var(--text-faint);
    }

    .metaline b { font-weight: 600; color: var(--text); }

    .metaline a { color: var(--info); text-decoration: none; }

    .metaline a:hover { text-decoration: underline; }

    .section-label {
        font-size: 11.5px;
        font-weight: 650;
        letter-spacing: .06em;
        text-transform: uppercase;
        color: var(--text-faint);
        margin: 18px 0 8px;
    }

    .chip {
        display: inline-block;
        padding: 1px 8px;
        border-radius: 9999px;
        font-size: 12px;
        font-weight: 500;
        background: var(--surface-sunk);
        color: var(--text-muted);
    }

    /* ---------- Status ---------- */
    /* Shared by the pantry page and the shopping list's pantry sidebar,
       so stock state reads identically in both places. Defined here
       rather than in the pantry task because Task 8 consumes it first. */
    .item-status-dot {
        flex: 0 0 auto;
        width: 7px;
        height: 7px;
        border-radius: 50%;
    }

    .item-status-dot.in-stock { background: var(--ok); }

    .item-status-dot.out-of-stock { background: var(--accent); }
```

Delete the two existing `.item-status-dot.in-stock` / `.item-status-dot.out-of-stock` rules further down `input.css` (the ones using `bg-green-500 dark:bg-green-400` and `bg-red-500 dark:bg-red-400`), so there is only one definition.

- [ ] **Step 2: Build**

Run: `npm run build-css`
Expected: exits 0.

- [ ] **Step 3: Verify the classes emitted**

Run: `grep -c "\.card-head\|\.btn-primary\|\.metaline\|\.section-label" static/css/output.css`
Expected: greater than `0`.

- [ ] **Step 4: Verify no regression**

Run: `npm test`
Expected: same result as the Task 2 baseline. No template uses the new classes yet.

- [ ] **Step 5: Commit**

```bash
git add static/css/input.css
git commit -m "feat(ui): add token-based component layer"
```

---

## Task 4: Restyle shared badge and pill components

Converts the test-visible classes to flat token styling, and removes exactly the dark overrides that would otherwise fight them. `cooking-mode.css` shares these classes, so Cook mode is covered by the same change.

**Files:**
- Modify: `static/css/input.css` (existing `.ingredient-badge`, `.cookware-badge`, `.timer-badge`, `.tag`, `.metadata-*`, `.step-number`, `.recipe-card`, `.nav-pill`)
- Modify: `templates/base.html` (remove two dark-override sections)

- [ ] **Step 1: Replace the badge, tag and pill definitions**

In `static/css/input.css`, delete the existing `.recipe-card`, `.recipe-card::before`, `.ingredient-badge`, `.cookware-badge`, `.timer-badge`, `.step-number`, `.tag`, `.nav-pill`, `.nav-pill:hover`, `.nav-pill.active`, `.metadata-pill`, `.metadata-pill svg`, and all ten `.metadata-*` variant rules.

**Also delete the OLD `.btn-primary` and `.btn-primary:hover` rules** — the gradient ones (`background: linear-gradient(135deg, #ff6b35, #f97316)` with the orange glow `box-shadow`), which sit *later* in the file than the token-based `.btn-primary` added in Task 3.

The two rules currently *partially* override each other, which is worse than one simply winning. CSS resolves per-property, not per-rule: the old rule re-sets only `background`, so the new rule's `border-color` and `color: var(--accent-ink)` stay live — and on `:hover` the old rule sets neither `background` nor `filter`, so the new flat `var(--accent)` fill wins there. The result is a button that renders as a gradient at rest and flips to a flat fill on hover. No page renders `.btn-primary` before Task 6, so this is invisible today, but it must be resolved here rather than carried forward. Verify afterwards that exactly one `.btn-primary` definition remains:

```bash
grep -c "^\s*\.btn-primary\s*{" static/css/input.css   # must print 1
```

Replace all of the above with:

```css
    /* Inline recipe entities: weight + tint, not bordered gradient pills,
       so they stop competing with the prose they sit inside. */
    .ingredient-badge {
        color: var(--accent-text);
        font-weight: 600;
        white-space: nowrap;
    }

    .cookware-badge {
        color: var(--ok);
        font-weight: 600;
        white-space: nowrap;
    }

    .timer-badge {
        padding: 1px 6px;
        border-radius: 5px;
        background: var(--surface-sunk);
        color: var(--text);
        font-weight: 600;
        font-variant-numeric: tabular-nums;
        white-space: nowrap;
    }

    .step-number {
        flex: 0 0 22px;
        height: 22px;
        margin-top: 1px;
        border-radius: 50%;
        display: grid;
        place-items: center;
        background: var(--surface-sunk);
        color: var(--text-muted);
        font-size: 12px;
        font-weight: 650;
        font-variant-numeric: tabular-nums;
    }

    .tag {
        display: inline-block;
        padding: 1px 8px;
        border-radius: 9999px;
        font-size: 12px;
        font-weight: 500;
        background: var(--surface-sunk);
        color: var(--text-muted);
    }

    /* Card in the recipes/menu index. No gradient top border. */
    .recipe-card {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 10px 12px;
        text-decoration: none;
        background: var(--surface);
        border: 1px solid var(--border);
        border-radius: var(--radius-card);
    }

    .recipe-card:hover {
        background: var(--surface-sunk);
        border-color: var(--border-strong);
    }

    .recipe-card .recipe-card-icon {
        flex: 0 0 30px;
        height: 30px;
        display: grid;
        place-items: center;
        border-radius: 8px;
        background: var(--surface-sunk);
        font-size: 15px;
    }

    /* MUST be applied to an <h3>. The client-side sorter in
       recipes.html reads `el.querySelector('h3')?.textContent`
       to sort by name; any other element silently breaks sorting. */
    .recipe-card .recipe-card-title {
        display: block;
        margin: 0;
        font-size: 14px;
        font-weight: 600;
        line-height: 1.3;
        color: var(--text);
    }

    .recipe-card .recipe-card-sub {
        display: block;
        margin-top: 1px;
        font-size: 12px;
        color: var(--text-faint);
    }

    /* Nav items in the app bar. Name kept for the E2E suite. */
    .nav-pill {
        padding: 5px 11px;
        border-radius: 7px;
        font-size: 13.5px;
        font-weight: 500;
        color: var(--text-muted);
        text-decoration: none;
    }

    .nav-pill:hover {
        background: var(--surface-sunk);
        color: var(--text);
    }

    .nav-pill.active {
        background: var(--surface-sunk);
        color: var(--text);
        font-weight: 600;
    }

    /* Kept for custom metadata key/value chips inside .metaline.
       test-helpers.ts selects .metadata-pill by "key: value" text. */
    .metadata-pill {
        display: inline-flex;
        align-items: center;
        padding: 1px 8px;
        border-radius: 9999px;
        font-size: 12px;
        font-weight: 500;
        white-space: nowrap;
        background: var(--surface-sunk);
        color: var(--text-muted);
    }

    .metadata-pill svg {
        width: 14px;
        height: 14px;
        margin-right: 4px;
    }
```

Note: `.recipe-image-placeholder` and `.search-input:focus` stay as they are for now; `.search-input:focus` is replaced in Task 5.

- [ ] **Step 2: Remove the dark overrides for these classes**

In `templates/base.html`, inside the `<style>` block, delete these two sections in full. They sit between the `/* Nav pill dark mode */` comment and the `/* Recipe card specific dark styles */` block, roughly lines 193–272 before edits — locate them by their comment text, not by line number.

Delete the section beginning:

```css
        /* Nav pill dark mode */
        .dark .nav-pill {
```

through the end of `.dark .nav-pill.active { … }`.

Delete the section beginning:

```css
        /* Dark mode tag and badge styles */
```

through the end of the `/* Recipe card specific dark styles */` section, i.e. every rule targeting `.dark .ingredient-badge`, `.dark .cookware-badge`, `.dark .timer-badge`, `.dark .step-number`, `.dark .tag`, `.dark .metadata-pill`, and `.dark .recipe-card`.

**Leave the `@media print` block entirely alone.** It contains its own `.dark .ingredient-badge` rules that make badges readable on paper, and they are still needed.

One print rule does go stale in Task 5: `.bg-white.shadow-lg.rounded-2xl.mb-8 { display: none !important; }` targets the old nav wrapper, which becomes `.appbar`. It is harmless dead CSS because the rule above it already hides `nav` outright — remove it in Task 13 along with the rest of the cleanup.

- [ ] **Step 3: Rebuild and check both themes**

Run: `npm run build-css && cargo build && ./target/debug/cook server ./seed --port 9080`
Open `http://localhost:9080/recipe/Neapolitan Pizza` in light and dark.
Expected: inline ingredients render as orange bold text (no pill outline), cookware as green bold text, step numbers as small grey circles. Both themes legible. No white-on-white or black-on-black.

- [ ] **Step 4: Check Cook mode still styles its entities**

On the same page click **Cook**, then advance past the mise-en-place card to a step card.
Expected: ingredient and cookware names inside step text are tinted and bold, matching the recipe page.

- [ ] **Step 5: Run the tests**

Run: `npm test`
Expected: same as baseline. `recipe-display.spec.ts` and `navigation.spec.ts` assert on the *presence* of `.ingredient-badge` / `.nav-pill`, not their colours, so they must still pass.

- [ ] **Step 6: Commit**

```bash
git add static/css/input.css templates/base.html
git commit -m "refactor(ui): restyle badges and pills onto design tokens"
```

---

## Task 5: Compact app bar

Replaces the ~100px navigation block with a 48px sticky bar. This is the first task with a tablet-specific behaviour change, so it starts with a failing test.

**Files:**
- Create: `tests/e2e/tablet.spec.ts`
- Modify: `templates/base.html:768-800` (body, `.viewport`, `<nav>` and search markup)
- Modify: `static/css/input.css` (app bar components, `.search-input`)

- [ ] **Step 1: Write the failing test**

Create `tests/e2e/tablet.spec.ts`:

```typescript
import { test, expect } from '@playwright/test';

// iPad portrait. The band between md (768) and lg (1024) is where the
// old layout was worst: desktop density at three-quarters the width.
const TABLET = { width: 820, height: 1180 };

test.describe('Tablet layout', () => {
  test.use({ viewport: TABLET });

  test('app bar is compact', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const nav = page.locator('nav').first();
    const box = await nav.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.height).toBeLessThanOrEqual(56);
  });

  test('nav items are visible at tablet width', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    await expect(page.locator('nav a.nav-pill', { hasText: /Recipes/i })).toBeVisible();
  });
});
```

- [ ] **Step 2: Run it and confirm it fails**

Run: `npx playwright test tests/e2e/tablet.spec.ts --project=chromium`
Expected: `app bar is compact` FAILS with a received height around `100`. `nav items are visible` should already PASS (the nav shows at `md` and up).

- [ ] **Step 3: Add app bar components**

Add to the `@layer components` block in `static/css/input.css`:

```css
    .appbar {
        position: sticky;
        top: 0;
        z-index: 10;
        display: flex;
        align-items: center;
        gap: 10px;
        height: 48px;
        padding: 0 16px;
        background: var(--surface);
        border-bottom: 1px solid var(--border);
    }

    .appbar-brand {
        display: flex;
        align-items: center;
        gap: 7px;
        font-size: 14px;
        font-weight: 650;
        letter-spacing: -.01em;
        color: var(--text);
        text-decoration: none;
    }

    .appbar-nav { display: flex; gap: 2px; }

    .icon-btn {
        width: 28px;
        height: 28px;
        display: grid;
        place-items: center;
        border: 0;
        border-radius: 7px;
        background: transparent;
        color: var(--text-muted);
        cursor: pointer;
    }

    .icon-btn:hover {
        background: var(--surface-sunk);
        color: var(--text);
    }

    .icon-btn svg { width: 16px; height: 16px; }

    .search-input {
        width: 100%;
        height: 30px;
        padding: 0 10px 0 30px;
        border-radius: 7px;
        border: 1px solid transparent;
        background: var(--surface-sunk);
        color: var(--text);
        font-size: 13px;
    }

    .search-input::placeholder { color: var(--text-faint); }

    .search-input:focus {
        outline: none;
        border-color: var(--accent);
        box-shadow: none;
    }
```

Delete the old `.search-input:focus` rule (the one setting `border-primary-orange` and an orange ring) from `input.css`.

- [ ] **Step 4: Rewrite the nav markup**

In `templates/base.html`, replace `<body class="bg-gray-50">` with:

```html
<body class="bg-bg text-text">
```

Replace the opening of the nav block — currently:

```html
        <nav class="bg-white shadow-lg rounded-2xl mb-8 relative">
            <div class="px-3 lg:px-6 py-4 relative">
                <div class="flex items-center justify-between flex-wrap gap-y-3">
```

with:

```html
        <nav class="appbar">
            <a href="{{ prefix }}/{% if static_mode %}index.html{% endif %}" class="appbar-brand">
                <img src="{{ prefix }}/static/android-chrome-192x192.png" alt="" class="h-[22px] w-[22px] rounded-md" width="22" height="22">
                <span class="hidden sm:inline">Cook</span>
            </a>
```

Then, for the search container, replace its wrapper classes and input classes:

```html
            <div class="relative z-50 ml-auto w-[180px] md:w-[220px]" id="search-container">
                <div class="absolute inset-y-0 left-0 pl-2.5 flex items-center pointer-events-none">
                    <svg class="h-4 w-4 text-faint" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
                    </svg>
                </div>
                <input type="text"
                       id="search-input"
                       placeholder="{{ tr.t("search-placeholder") }}"
                       class="search-input">
                <div id="search-results"
                     class="absolute top-full mt-1 w-full card hidden max-h-96 overflow-y-auto"
                     style="z-index: 9999;">
                </div>
            </div>
```

Move the `.appbar-nav` group so it sits directly after the brand and before the search container:

```html
            <div class="appbar-nav hidden md:flex">
                {% if features.show_shopping_list || features.show_pantry %}
                <a href="{{ prefix }}/{% if static_mode %}index.html{% endif %}"
                   class="nav-pill {% if active == "recipes" %}active{% endif %}">{{ tr.t("nav-recipes") }}</a>
                {% if features.show_shopping_list && !static_mode %}
                <a href="{{ prefix }}/shopping-list"
                   class="nav-pill {% if active == "shopping" %}active{% endif %}">{{ tr.t("nav-shopping-list") }}</a>
                {% endif %}
                {% if features.show_pantry && !static_mode %}
                <a href="{{ prefix }}/pantry"
                   class="nav-pill {% if active == "pantry" %}active{% endif %}">{{ tr.t("nav-pantry") }}</a>
                {% endif %}
                {% endif %}
            </div>
```

Convert the three trailing controls (preferences link, shortcuts button, theme button) to `.icon-btn`, dropping their `px-3 lg:px-5 rounded-full` / `bg-gray-200 dark:bg-gray-700` utility strings. Keep every `onclick`, `aria-label`, `title`, and the `print:hidden` class exactly as they are. Keep the `md:hidden` overflow-menu block unchanged apart from swapping its trigger button's utility classes for `icon-btn`.

Remove the now-redundant wrapper `<div class="px-3 lg:px-6 py-4 relative">` and `<div class="flex items-center justify-between flex-wrap gap-y-3">`, and their closing tags. `.appbar` is itself the flex row.

- [ ] **Step 5: Update `.viewport` spacing**

In the `<style>` block of `templates/base.html`, change `.viewport` from:

```css
        .viewport {
            width: 100%;
            max-width: 72rem;
            margin: 2rem auto;
            padding: 0 1rem;
        }
```

to:

```css
        .viewport {
            width: 100%;
            max-width: 72rem;
            margin: 0 auto;
            padding: 16px;
        }
```

The nav must move *outside* `.viewport` so the app bar spans the full width and sticks correctly. Move the `<nav class="appbar">…</nav>` block above `<div class="viewport">`.

- [ ] **Step 6: Remove the search dark overrides**

In `templates/base.html`, delete the `/* Search results dark mode */` and `/* Override inline hover styles in search results */` sections in full — every rule matching `.dark #search-results …` and `.dark #search-input …`. The search input and results panel now use tokens.

Also delete the dead `/* Theme toggle button styles */` section (`.theme-toggle`, `.theme-toggle.dark`, `.theme-toggle-handle`, `.theme-toggle.dark .theme-toggle-handle`) — `grep -rn "theme-toggle" templates/ static/js/` returns nothing outside these definitions.

Restyle the one remaining light-mode search rule to use tokens:

```css
        #search-results a.search-selected {
            background: var(--surface-sunk) !important;
        }
```

- [ ] **Step 7: Run the tablet test**

Run: `npm run build-css && cargo build` then `npx playwright test tests/e2e/tablet.spec.ts --project=chromium`
Expected: both tests PASS.

- [ ] **Step 8: Verify search still works**

Start the server, type `pizza` into the search box.
Expected: results dropdown appears, is readable in both themes, and arrow-key selection highlights a row.

- [ ] **Step 9: Run the suite**

Run: `npm test`
Expected: `search.spec.ts` and `navigation.spec.ts` pass. If a navigation test asserts the nav is inside a specific wrapper, update the selector to `nav.appbar`.

- [ ] **Step 10: Commit**

```bash
git add -A
git commit -m "feat(ui): compact 48px app bar"
```

---

## Task 6: Recipe page

The priority page. Mirrors the reference mockup's recipe view.

**Files:**
- Modify: `templates/recipe.html`
- Modify: `static/css/input.css` (recipe layout components)
- Modify: `tests/e2e/tablet.spec.ts`

- [ ] **Step 1: Write the failing tests**

Append to `tests/e2e/tablet.spec.ts`, inside the existing `test.describe('Tablet layout', …)` block:

```typescript
  test('recipe action buttons keep their labels', async ({ page }) => {
    await page.goto('/recipe/Neapolitan Pizza');
    await page.waitForLoadState('networkidle');

    // Below lg these used to collapse to unlabelled coloured circles.
    await expect(page.getByRole('button', { name: /Cook/i })).toContainText(/Cook/i);
    await expect(page.getByRole('link', { name: /Edit/i })).toContainText(/Edit/i);
  });

  test('ingredient rows do not wrap', async ({ page }) => {
    await page.goto('/recipe/Neapolitan Pizza');
    await page.waitForLoadState('networkidle');

    // A single-line row is ~30px tall. Wrapping pushes it past 44px.
    const row = page.locator('.ingredient-list .row', { hasText: 'mozzarella cheese' });
    const box = await row.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.height).toBeLessThan(44);
  });
```

- [ ] **Step 2: Run and confirm failure**

Run: `npx playwright test tests/e2e/tablet.spec.ts --project=chromium`
Expected: `recipe action buttons keep their labels` FAILS (labels are inside `hidden lg:inline` spans). `ingredient rows do not wrap` FAILS with "no element matches" — `.ingredient-list` does not exist yet.

- [ ] **Step 3: Add the recipe layout components**

Add to `@layer components` in `static/css/input.css`:

```css
    /* Fixed-width rail is what guarantees ingredient names and
       quantities never wrap in the 768–1023px tablet band. */
    .recipe-layout {
        display: grid;
        gap: 16px;
        grid-template-columns: 1fr;
    }

    @media (min-width: 700px) {
        .recipe-layout {
            grid-template-columns: 264px minmax(0, 1fr);
            align-items: start;
        }

        .recipe-rail {
            position: sticky;
            top: 64px;
        }
    }

    @media (min-width: 1024px) {
        .recipe-layout { grid-template-columns: 300px minmax(0, 1fr); }
    }

    .ingredient-list { list-style: none; margin: 0; padding: 4px 0; }

    .step-list { list-style: none; margin: 0; padding: 0; }

    .step-list > li {
        display: flex;
        gap: 12px;
        padding: 14px;
    }

    .step-list > li + li { border-top: 1px solid var(--border); }

    .step-body { font-size: 15px; line-height: 1.6; }

    .step-refs {
        margin-top: 7px;
        display: flex;
        flex-wrap: wrap;
        gap: 2px 14px;
        font-size: 12.5px;
        color: var(--text-faint);
    }

    .step-refs b { font-weight: 600; color: var(--text-muted); }

    .recipe-note {
        display: flex;
        gap: 8px;
        padding: 12px 14px;
        border-left: 3px solid var(--border-strong);
        background: var(--surface-sunk);
        color: var(--text-muted);
        font-style: italic;
    }
```

- [ ] **Step 4: Rewrite the recipe head**

In `templates/recipe.html`, replace the title bar block (lines 36–95) with:

```html
        <div class="flex items-start gap-4 mb-1.5">
            <h1 class="text-[24px] leading-tight font-[650] tracking-tight text-text print:text-2xl">
                {{ recipe.name }}
            </h1>
            {% if scale != 1.0 %}
            <div class="hidden print:block text-lg font-normal">{{ tr.t("recipe-scale-label") }}: {{ scale }}x</div>
            {% endif %}
            <div class="ml-auto flex flex-wrap items-center gap-1.5 print:hidden">
                {% if !static_mode %}
                <div class="stepper">
                    <label for="scale" class="sr-only">{{ tr.t("recipe-scale-label") }}</label>
                    <button type="button" aria-label="Decrease scale"
                            onclick="stepScale(-0.5)">&minus;</button>
                    <input type="number"
                           id="scale"
                           value="{{ scale }}"
                           min="0.5"
                           max="200"
                           step="0.5"
                           onchange="window.location.href = `{{ prefix }}/recipe/{{ recipe_path }}?scale=${this.value}`">
                    <button type="button" aria-label="Increase scale"
                            onclick="stepScale(0.5)">+</button>
                </div>
```

The `id="scale"` input, its `min`/`max`/`step` and its `onchange` are preserved verbatim — `addToShoppingList()` reads `document.getElementById('scale').value`, and `recipe-scaling.spec.ts` drives the input directly. The −/+ buttons are additive. Add this helper to the page's existing `<script>` block:

```javascript
function stepScale(delta) {
    const input = document.getElementById('scale');
    const next = Math.min(200, Math.max(0.5, (parseFloat(input.value) || 1) + delta));
    input.value = next;
    input.dispatchEvent(new Event('change'));
}
```

Dispatching `change` rather than calling the handler directly keeps a single source of navigation behaviour.

```html
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
                <button id="start-cooking-btn" onclick="startCookingMode()" class="btn btn-primary print:hidden" title="Start Cooking">
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
```

The `hidden lg:inline` label spans are gone — that is what makes the first new test pass. The `id="scale"` input is preserved because `addToShoppingList()` and `recipe-scaling.spec.ts` both read it.

- [ ] **Step 5: Collapse metadata into one line**

Replace the `<div id="metadata-container" class="flex flex-wrap gap-3">` block and the separate tags row with a single `.metaline`. Keep the tags as `.tag` elements and custom metadata as `.metadata-pill` (both are selected by tests):

```html
            <div id="metadata-container" class="metaline mb-3.5">
                {% match metadata.servings %}
                {% when Some with (servings) %}
                <span><b>{{ servings }}</b> {{ tr.t("recipe-servings-label") }}</span>
                {% when None %}
                {% endmatch %}

                {% match metadata.time %}
                {% when Some with (time) %}
                <span><b>{{ time }}</b></span>
                {% when None %}
                {% endmatch %}

                {% match metadata.difficulty %}
                {% when Some with (difficulty) %}
                <span>{{ difficulty }}</span>
                {% when None %}
                {% endmatch %}

                {% match metadata.course %}
                {% when Some with (course) %}
                <span>{{ course }}</span>
                {% when None %}
                {% endmatch %}

                {% match metadata.prep_time %}
                {% when Some with (prep_time) %}
                <span>{{ tr.t("meta-prep-time") }} <b>{{ prep_time }}</b></span>
                {% when None %}
                {% endmatch %}

                {% match metadata.cook_time %}
                {% when Some with (cook_time) %}
                <span>{{ tr.t("meta-cook-time") }} <b>{{ cook_time }}</b></span>
                {% when None %}
                {% endmatch %}

                {% match metadata.cuisine %}
                {% when Some with (cuisine) %}
                <span>{{ cuisine }}</span>
                {% when None %}
                {% endmatch %}

                {% match metadata.diet %}
                {% when Some with (diet) %}
                <span>{{ diet }}</span>
                {% when None %}
                {% endmatch %}

                {% match metadata.author %}
                {% when Some with (author) %}
                <span>{{ author }}</span>
                {% when None %}
                {% endmatch %}

                {% match metadata.source %}
                {% when Some with (source) %}
                <span>{{ source }}</span>
                {% when None %}
                {% endmatch %}

                {% match metadata.source_url %}
                {% when Some with (source_url) %}
                <span><a href="{{ source_url }}">{{ source_url|hostname }}</a></span>
                {% when None %}
                {% endmatch %}

                {% for (key, value) in metadata.custom %}
                <span><span class="metadata-pill">{{ key }}: {{ value }}</span></span>
                {% endfor %}

                {% if !tags.is_empty() %}
                <span class="flex gap-1.5">
                    {% for tag in tags %}<span class="tag">#{{ tag }}</span>{% endfor %}
                </span>
                {% endif %}
            </div>
```

The emoji prefixes (👥 ⏱️ 📊 🍽️ 🔥 🌍 🥗 👤 📖 🔗) are dropped. The label text and dot separators carry the meaning, and removing them is what lets the row fit on one line at 820px.

Delete the standalone tags row that previously sat above the description, and restyle the description block from its orange gradient to:

```html
            <div class="recipe-note rounded-[10px] mb-3.5">
                <p class="m-0">{{ description }}</p>
            </div>
```

- [ ] **Step 6: Convert the ingredient rail**

Replace `<div class="grid md:grid-cols-3 gap-8 mb-8">` with `<div class="recipe-layout mb-6">`, and `<div class="md:col-span-1">` with `<div class="recipe-rail">`.

Replace the ingredients panel wrapper `<div class="bg-white rounded-2xl shadow-lg p-6">` with a `.card` that has a `.card-head`:

```html
            <div class="card">
                <div class="card-head">
                    <h2>{{ tr.t("recipe-ingredients") }}</h2>
                    <span class="count">{{ ingredients.len() }}</span>
                </div>
```

Both the sectioned and the flat ingredient loops use `<ul class="ingredient-list">`, and each `<li>` becomes:

```html
                    <li class="row">
                        <span class="row-name">
                            {% match ingredient.reference_path %}
                            {% when Some with (path) %}
                                {% if !static_mode %}
                                <input type="checkbox" checked
                                       class="ref-checkbox align-middle mr-1.5 accent-[var(--accent)]"
                                       data-ref-path="{{ path }}"
                                       title="{{ tr.t("shopping-include-in-list") }}">
                                {% endif %}
                                <a href="{{ prefix }}/recipe/{{ path }}{% if static_mode %}.html{% endif %}" class="text-info hover:underline">{{ ingredient.name }}</a>
                            {% when None %}
                                {{ ingredient.name }}
                            {% endmatch %}
                            {% match ingredient.note %}
                            {% when Some with (note) %}<span class="row-note" aria-label="{{ tr.t("recipe-preparation") }}: {{ note }}">({{ note }})</span>{% when None %}
                            {% endmatch %}
                        </span>
                        <span class="row-value">{% match ingredient.quantity %}{% when Some with (quantity) %}{{ quantity }}{% when None %}{% endmatch %}{% match ingredient.unit %}{% when Some with (unit) %} {{ unit }}{% when None %}{% endmatch %}</span>
                    </li>
```

Apply the same replacement to **both** the sectioned loop and the flat fallback loop. The chain-link SVG next to referenced ingredients is dropped — the `--info` colour already marks it as a link, and the icon was a wrap trigger at rail width.

Move cookware into its own sibling `.card` (not a heading inside the ingredients card), so the two panels stack:

```html
            {% if cookware.len() > 0 %}
            <div class="card mt-3">
                <div class="card-head">
                    <h2>{{ tr.t("recipe-cookware") }}</h2>
                    <span class="count">{{ cookware.len() }}</span>
                </div>
                <ul class="ingredient-list">
                    {% for item in cookware %}
                    <li class="row"><span class="row-name">{{ item.name }}</span></li>
                    {% endfor %}
                </ul>
            </div>
            {% endif %}
```

- [ ] **Step 7: Convert the method column**

Replace `<div class="md:col-span-2">` + `<div class="bg-white rounded-2xl shadow-lg p-6">` with a single `.card` carrying a head:

```html
        <div class="card">
            <div class="card-head">
                <h2>{{ tr.t("recipe-method") }}</h2>
            </div>
```

If the translation key `recipe-method` does not exist, use the literal `Method` and note it for a follow-up i18n pass — do not invent a key that has no entry in the locale files.

Change each `<ol class="space-y-4 …">` to `<ol class="step-list">`. Each step `<li>` becomes:

```html
                                <li>
                                    <div class="step-number">{{ step.number }}</div>
                                    <div class="flex-1 min-w-0">
                                        {% match step.image_path %}
                                        {% when Some with (img) %}
                                        <img class="image-step mb-2" src="{{ img }}" />
                                        {% when None %}
                                        {% endmatch %}
                                        <div class="step-body">
                                        {% for step_item in step.items %}{% match step_item %}{% when crate::web::templates::StepItem::Text with (text) %}{{ text }}{% when crate::web::templates::StepItem::Ingredient with { name, reference_path } %}{% match reference_path %}{% when Some with (path) %}<a href="{{ prefix }}/recipe/{{ path }}{% if static_mode %}.html{% endif %}" class="ingredient-badge hover:underline" title="View recipe: {{ name }}">{{ name }}</a>{% when None %}<span class="ingredient-badge">{{ name }}</span>{% endmatch %}{% when crate::web::templates::StepItem::Cookware with (name) %}<span class="cookware-badge">{{ name }}</span>{% when crate::web::templates::StepItem::Timer with (name) %}<span class="timer-badge">{{ name }}</span>{% when crate::web::templates::StepItem::Quantity with (qty) %}<span class="font-bold text-accent-text">{{ qty }}</span>{% when crate::web::templates::StepItem::LineBreak %}<br>{% endmatch %}{% endfor %}
                                        </div>
                                        {% if step.ingredients.len() > 0 %}
                                        <div class="step-refs">
                                            {% for ing in step.ingredients %}
                                            <span><b>{{ ing.name }}</b>{% match ing.quantity %}{% when Some with (q) %} {{ q }}{% when None %}{% endmatch %}{% match ing.unit %}{% when Some with (u) %} {{ u }}{% when None %}{% endmatch %}{% match ing.note %}{% when Some with (note) %} <span class="italic" aria-label="{{ tr.t("recipe-preparation") }}: {{ note }}">({{ note }})</span>{% when None %}{% endmatch %}</span>
                                            {% endfor %}
                                        </div>
                                        {% endif %}
                                    </div>
                                </li>
```

The nested `bg-gradient-to-r from-gray-50 to-orange-50 rounded-xl p-4` wrapper and the inner `flex flex-col gap-4` / `flex gap-4` divs are gone — `.step-list > li` is itself the flex row, and separation comes from a hairline instead of a card inside a card. The `⏱️` prefix is dropped from timers; `.timer-badge`'s tinted background already marks it.

Note blocks become:

```html
                                <li class="recipe-note">
                                    <p class="m-0" style="white-space: pre-line">{{ note }}</p>
                                </li>
```

Section headings become:

```html
                        <h3 class="section-label px-3.5">{{ name }}</h3>
```

- [ ] **Step 8: Restyle the JS-injected error banner**

In the `showRecipeError()` function near the bottom of `templates/recipe.html`, change the banner class from `mb-4 bg-red-50 border border-red-200 rounded-xl p-4` to `mb-4 card p-3.5 border-l-[3px]` and set `border-left-color: var(--accent)` via an inline style. Also update the "added" success state in `addToShoppingList()`: it currently swaps `from-purple-500`/`to-pink-500` gradient classes that no longer exist. Replace that class juggling with a single toggle:

```javascript
            button.classList.add('btn-primary');
            setTimeout(() => {
                button.innerHTML = originalContent;
                button.classList.remove('btn-primary');
            }, 2000);
```

- [ ] **Step 9: Rebuild and run the tablet tests**

Run: `npm run build-css && cargo build` then `npx playwright test tests/e2e/tablet.spec.ts --project=chromium`
Expected: all four tablet tests PASS.

- [ ] **Step 10: Visual check**

Start the server and open `http://localhost:9080/recipe/Neapolitan Pizza` at 820px, 1024px and 1440px, in light and dark.
Expected at 820px: breadcrumb, title, four labelled buttons, one metadata line, then the ingredient rail and five-ish steps — matching the mockup's recipe view. `mozzarella cheese  100 g` on one line.

- [ ] **Step 11: Run the suite**

Run: `npm test`
Expected: `recipe-display.spec.ts`, `recipe-scaling.spec.ts`, `cooking-mode.spec.ts` and `shopping-list.spec.ts` all pass. If `recipe-display.spec.ts` asserts on the emoji-prefixed metadata text you removed, update those assertions to match the new plain text.

- [ ] **Step 12: Commit**

```bash
git add -A
git commit -m "feat(ui): compact recipe page layout"
```

---

## Task 7: Recipes index

**Files:**
- Modify: `templates/recipes.html`
- Modify: `tests/e2e/tablet.spec.ts`

- [ ] **Step 1: Write the failing test**

Append inside the `test.describe('Tablet layout', …)` block in `tests/e2e/tablet.spec.ts`:

```typescript
  test('recipe cards are dense', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Hero cards were ~190px tall for one word and a count.
    const card = page.locator('.recipe-card').first();
    const box = await card.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.height).toBeLessThan(80);
  });
```

- [ ] **Step 2: Run and confirm failure**

Run: `npx playwright test tests/e2e/tablet.spec.ts --project=chromium -g "dense"`
Expected: FAIL, received height around `190`.

- [ ] **Step 3: Convert the grid and cards**

In `templates/recipes.html` line 68, replace:

```html
    <div id="recipes-grid" class="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
```

with:

```html
    <div id="recipes-grid" class="grid gap-2.5 [grid-template-columns:repeat(auto-fill,minmax(230px,1fr))]">
```

Replace the directory card (currently a `bg-white rounded-2xl shadow-lg … flex flex-col` anchor wrapping a 64px gradient circle) with:

```html
        <a href="{{ prefix }}/directory/{{ item.path }}{% if static_mode %}.html{% endif %}" data-type="directory" class="recipe-card">
            <span class="recipe-card-icon">📁</span>
            <span class="min-w-0">
                <h3 class="recipe-card-title">{{ item.name }}</h3>
                {% match item.count %}
                {% when Some with (count) %}
                <span class="recipe-card-sub">{{ tr.tn("recipes-count", count) }}</span>
                {% when None %}
                {% endmatch %}
            </span>
        </a>
```

and the recipe/menu card with:

```html
        <a href="{{ prefix }}/{% if static_mode %}{% if item.is_menu %}menu{% else %}recipe{% endif %}{% else %}recipe{% endif %}/{{ item.path }}{% if static_mode %}.html{% endif %}"
           data-type="recipe"
           {% if let Some(ts) = item.modified_at %}data-modified="{{ ts }}"{% endif %}
           {% if let Some(ts) = item.created_at %}data-created="{{ ts }}"{% endif %}
           class="recipe-card">
            {% match item.image_path %}
            {% when Some with (img) %}
            <img src="{{ img }}" alt="" class="recipe-card-icon object-cover" width="30" height="30">
            {% when None %}
            <span class="recipe-card-icon">{% if item.is_menu %}📋{% else %}🍽{% endif %}</span>
            {% endmatch %}
            <span class="min-w-0">
                <h3 class="recipe-card-title">{{ item.name }}</h3>
                <span class="recipe-card-sub">
                    {%- if item.is_menu %}{{ tr.t("recipe-type-menu") }}{% if !item.tags.is_empty() %} · {% endif %}{% endif -%}
                    {%- for tag in item.tags.iter().take(3) %}{{ tag }}{% if !loop.last %} · {% endif %}{% endfor -%}
                </span>
            </span>
        </a>
```

Four things here are load-bearing and must not be changed:

- **`<h3>` for the title.** The sorter reads `el.querySelector('h3')?.textContent`.
- **`data-type="directory"` / `data-type="recipe"`.** The sorter partitions on these and hides the sort controls when fewer than two recipe cards exist.
- **`data-modified` / `data-created`.** The sorter reads these, and removes the "Created" option when any card lacks `data-created`.
- **`a.recipe-card[href^="/recipe/"]` stays valid.** `navigation.spec.ts:24` selects exactly that.

The 192px image header (`h-48`) collapses to the 30px icon slot. Tags move into the subtitle as a dot-joined list, and the separate `.tag` elements and the `+N` overflow counter are dropped, so every card is one line tall.

**Do not add `.section-label` grouping to this page.** The sorter calls `grid.appendChild()` on every child to reorder them, which would strip any interleaved heading elements out of position. Directories already sort ahead of recipes in `applySort()`. Grouping headings are used on the pantry page instead, where there is no client-side sorter.

- [ ] **Step 4: Compact the page head and toolbar**

Replace the gradient `All Recipes` heading with `class="text-[24px] leading-tight font-[650] tracking-tight text-text"`, and give the New Recipe link `class="btn btn-primary"`. Change the sort `<select>` and direction button to use `.btn` sizing so the toolbar row matches the app bar's control height.

- [ ] **Step 5: Rebuild and test**

Run: `npm run build-css && cargo build` then `npx playwright test tests/e2e/tablet.spec.ts --project=chromium`
Expected: all five tablet tests PASS.

- [ ] **Step 6: Verify sorting still works**

Open `http://localhost:9080/`, change **Sort by** to each option and click the direction arrow.
Expected: cards reorder, directories stay ahead of recipes, the arrow flips between ↑ and ↓. If nothing reorders, the title is no longer an `<h3>`.

- [ ] **Step 7: Visual check and suite**

Open `http://localhost:9080/` at 820px in both themes — expect grouped, single-line rows, roughly three columns.
Run: `npm test`
Expected: `navigation.spec.ts` passes.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat(ui): dense recipes index"
```

---

## Task 8: Shopping list

**Important:** unlike every other page in this plan, the shopping list body is rendered **client-side from JavaScript template literals** inside `templates/shopping_list.html` (around lines 465–520), not by Askama. You are editing JS strings, not Askama markup. The Askama part is only the page shell.

**Files:**
- Modify: `templates/shopping_list.html`

- [ ] **Step 1: Convert the Askama shell panels**

Replace the three `bg-gradient-to-r` wrappers (the Selected Recipes panel and the two list-section panels) with `.card` + `.card-head`. The heading inside each `.card-head` uses a plain `<h2>` that the component styles automatically — remove `text-xl font-bold text-orange-600`-style utilities.

- [ ] **Step 2: Convert the JS-rendered aisle groups**

Replace the category block template literal (currently `<div class="mb-6 bg-white rounded-lg p-4 shadow-xs">` with an `<h3 class="font-semibold text-lg mb-3 text-orange-600">`) with:

```javascript
        html += data.categories.map(category => `
            <div class="card mb-2.5">
                <div class="card-head plain"><h3>${escHtml(category.category)}</h3></div>
                <ul>
                    ${category.items.map((item, idx) => {
                        const itemId = `item-${item.name.replace(/\s+/g, '-')}`;
                        return `
                        <li class="row">
                            <input type="checkbox"
                                id="${escHtml(itemId)}"
                                class="w-4 h-4 accent-[var(--accent)]"
                                data-action="toggle-item"
                                data-item-id="${escHtml(itemId)}"
                                data-ingredient-name="${escHtml(item.name)}">
                            <label for="${escHtml(itemId)}" class="row-name cursor-pointer">
                                <span class="item-name">${escHtml(item.name)}</span>
                            </label>
                            <span class="row-value">${escHtml(formatQuantities(item.quantities))}</span>
                        </li>
                    `}).join('')}
                </ul>
            </div>
        `).join('');
```

Four things must survive verbatim, because other code reads them:

- **`<li>` stays the direct parent of both the checkbox and `.item-name`.** Line ~765 does `nameElement.closest('li').querySelector('input[type="checkbox"]')`. The old markup nested them in an extra `<div class="flex items-center flex-1">`; removing that div is safe, but the checkbox must stay inside the same `<li>`.
- **`id` / `for` pairing on checkbox and label** — clicking the label is how items get ticked.
- **`data-action="toggle-item"`, `data-item-id`, `data-ingredient-name"`** — the delegated `onchange` handler dispatches on these.
- **`.item-name`** — read by the copy-to-clipboard builder and by the tests.

Aisle names come from `aisle.conf` and are user-authored, so they use the `.plain` variant rather than the uppercase micro-label treatment.

- [ ] **Step 3: Convert the JS-rendered pantry sidebar**

Replace the pantry item literal (`<li class="flex items-center justify-between py-1">` with its green check SVG and `text-green-800` / `text-green-700` spans) with:

```javascript
                    return `
                    <li class="row">
                        <span class="item-status-dot in-stock"></span>
                        <span class="row-name">${escHtml(itemName)}</span>
                        <span class="row-value">${itemQuantities ? escHtml(formatQuantities(itemQuantities)) : ''}</span>
                    </li>
                    `;
```

The green tick SVG is replaced by the shared `.item-status-dot` from Task 9, so pantry state reads identically on both pages.

- [ ] **Step 4: Buttons and copy-options**

Convert every action button to `.btn`, with at most one `.btn-primary` on the page. Keep `id="copy-option-aisles"` and `id="copy-option-amounts"` and their `onchange="onCopyOptionChange()"` handlers exactly as they are — `shopping-list-copy.spec.ts` drives them directly.

- [ ] **Step 5: Rebuild and test**

Run: `npm run build-css && cargo build` then `npm test`
Expected: all three shopping-list specs pass.

- [ ] **Step 6: Visual check**

Add a recipe to the list, open `/shopping-list` at 820px in both themes.
Expected: grouped card sections, single-line rows, right-aligned quantities.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat(ui): compact shopping list"
```

---

## Task 9: Pantry

**Files:**
- Modify: `templates/pantry.html`
- Modify: `static/css/input.css` (replace the `.pantry-item` rules)

- [ ] **Step 1: Replace the pantry styles**

In `static/css/input.css`, delete the `.pantry-item`, `.pantry-item:hover`, `.pantry-item.out-of-stock`, `.pantry-item.out-of-stock:hover`, `.pantry-item .text-gray-900`, `.pantry-item .text-gray-600`, `.pantry-item .text-gray-500` and `.pantry-item.out-of-stock .quantity-display` rules. (The two `.item-status-dot.*` rules were already removed in Task 3.) Replace with:

```css
    .pantry-item {
        display: flex;
        align-items: baseline;
        gap: 10px;
        padding: 7px 14px;
        font-size: 14px;
    }

    .pantry-item + .pantry-item { border-top: 1px solid var(--border); }

    .pantry-item:hover { background: var(--surface-sunk); }

    .pantry-item .quantity-display {
        white-space: nowrap;
        font-weight: 600;
        font-size: 13.5px;
        color: var(--accent-text);
        font-variant-numeric: tabular-nums;
    }

    .pantry-item .pantry-dates {
        font-size: 12.5px;
        color: var(--text-faint);
    }

    .pantry-item.out-of-stock .quantity-display { color: var(--text-faint); }
```

`.item-status-dot` is **not** redefined here — Task 3 already defines it, because the shopping list's pantry sidebar (Task 8) consumes it first.

Note: `.item-status-dot.out-of-stock` uses `var(--danger)`, not `var(--accent)` — the accent is reserved for the single primary action per view. If the out-of-stock row needs a tinted background, use `var(--danger-soft)`.

The seven `.pantry-item .text-gray-*` rules existed only to force dark-mode text colours onto Tailwind utilities. Tokens make them unnecessary — but that means you must also remove those `text-gray-*` utilities from the pantry markup in the next step, or the text will stay grey in dark mode.

- [ ] **Step 2: Collapse the item markup**

Each pantry item currently renders a four-line `Qty: / Bought: / Expires: / Low at:` stack, where every field prints `-` when empty. Collapse the display half to one row plus a secondary date line that omits empty fields:

```html
                    <div class="pantry-item group"
                         data-section="{{ section.name }}"
                         data-name="{{ item.name }}"
                         data-quantity="{% if let Some(quantity) = item.quantity %}{{ quantity }}{% endif %}"
                         data-low="{% if let Some(low) = item.low %}{{ low }}{% endif %}">
                        <span class="item-status-dot in-stock"></span>
                        <span class="row-name min-w-0">
                            <h3 class="m-0 font-medium text-[14px]">{{ item.name }}</h3>
                            <span class="item-display pantry-dates">
                                {%- if let Some(bought) = item.bought %}{{ tr.t("pantry-item-bought") }} <span class="item-bought">{{ bought }}</span>{% endif -%}
                                {%- if let Some(expire) = item.expire %} · {{ tr.t("pantry-item-expire") }} <span class="item-expire">{{ expire }}</span>{% endif -%}
                                {%- if let Some(low) = item.low %} · {{ tr.t("pantry-item-low") }} <span class="item-low">{{ low }}</span>{% endif -%}
                            </span>
                        </span>
                        <span class="quantity-display">
                            <span class="item-quantity">{% if let Some(quantity) = item.quantity %}{{ quantity }}{% else %}-{% endif %}</span>
                            <svg class="out-of-stock-icon w-3.5 h-3.5 ml-1 inline text-accent hidden" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path>
                            </svg>
                        </span>
                    </div>
```

Every one of these is a JavaScript hook — read `templates/pantry.html`'s inline script before editing and keep them all: `.item-display`, `.item-edit`, `.item-quantity`, `.item-bought`, `.item-expire`, `.item-low`, `.quantity-display`, `.out-of-stock-icon`, `.item-status-dot`, and the four `data-*` attributes. The `.item-edit` form block is left structurally as-is; only restyle its inputs (`border rounded-sm` → `border-line rounded-[6px] bg-surface text-text`).

Drop every `text-gray-900` / `text-gray-600` / `text-gray-500` utility from this file — the deleted `.pantry-item .text-gray-*` rules were the only thing making them legible in dark mode.

- [ ] **Step 2b: Group with section labels**

Unlike the recipes index, the pantry has no client-side sorter reparenting its children, so `.section-label` headings are safe here. Use one per storage location.

- [ ] **Step 3: Convert the section grid**

Replace `<div class="grid gap-3 md:grid-cols-2 lg:grid-cols-3">` (line 59) with a `.card` per storage location containing a `class="card-head plain"` head (location name + item count) and the items as `.pantry-item` rows. Wrap the locations in `<div class="grid gap-2.5 [grid-template-columns:repeat(auto-fill,minmax(280px,1fr))]">`. Storage location names are user-authored, so they use the `.plain` variant rather than the uppercase micro-label treatment.

- [ ] **Step 4: Head and modal**

Compact the `Pantry Inventory` heading to match the other pages, convert the `max-w-md` modal at line 166 to `.card`, and convert its buttons to `.btn`.

- [ ] **Step 5: Rebuild and test**

Run: `npm run build-css && cargo build` then `npm test`
Expected: `pantry.spec.ts` (10 tests) passes.

- [ ] **Step 6: Visual check**

Open `/pantry` at 820px in both themes.
Expected: compact single-line items with a status dot and right-aligned quantity.

Note: quantities render as `250%g` / `2%l`. `%` is the quantity/unit separator in `pantry.conf` — the edit form's own placeholder reads `Quantity (e.g., 500%g)` — so this is the raw stored value being displayed unformatted, not a broken format string. Rendering it as `250 g` is a worthwhile follow-up but is explicitly out of scope here. Do not fix it in this task and do not let it block you.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat(ui): compact pantry inventory"
```

---

## Task 10: Preferences

This page has 11 gradients and a test that asserts on gradient class names.

**Files:**
- Modify: `templates/preferences.html`
- Modify: `tests/e2e/preferences.spec.ts:231-233`, `:245`

- [ ] **Step 1: Replace the gradient assertions with a state attribute**

`preferences.spec.ts` asserts `toHaveClass(/from-orange-500/)` to detect an active toggle. That couples the test to a gradient this plan removes. Add `data-active="true"` / `data-active="false"` to each feature toggle button in `templates/preferences.html`, then update the assertions:

```typescript
    // Both enabled → toggles report active state
    await expect(shoppingBtn).toHaveAttribute('data-active', 'true');
    await expect(pantryBtn).toHaveAttribute('data-active', 'true');
```

and at line 245:

```typescript
    await expect(shoppingBtn).toHaveAttribute('data-active', 'false');
```

- [ ] **Step 2: Run and confirm failure**

Run: `npx playwright test tests/e2e/preferences.spec.ts --project=chromium`
Expected: the two state tests FAIL — `data-active` does not exist yet.

- [ ] **Step 3: Add the attribute and convert the toggles**

In `templates/preferences.html`, give each feature toggle button `data-active="{% if enabled %}true{% else %}false{% endif %}"` (matching the template's actual condition variable) and replace its gradient utility string with:

```html
class="btn {% if enabled %}btn-primary{% endif %}"
```

- [ ] **Step 4: Convert the remaining sections**

Replace the other gradient panels with `.card` + `.card-head`, and all remaining buttons with `.btn`.

- [ ] **Step 5: Rebuild and test**

Run: `npm run build-css && cargo build` then `npx playwright test tests/e2e/preferences.spec.ts --project=chromium`
Expected: all 13 tests PASS.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(ui): token-based preferences page"
```

---

## Task 11: Menu page

**Files:**
- Modify: `templates/menu.html`

- [ ] **Step 1: Convert**

Replace the six `bg-gradient-to-r` panels with `.card` + `.card-head`. Match `recipe.html`'s head structure: 24px title, `.metaline` for any metadata, `.btn` for actions. Change the `max-w-4xl mx-auto` wrapper at line 27 to plain full-width — `.viewport` already caps page width at 72rem.

- [ ] **Step 2: Rebuild and check**

Run: `npm run build-css && cargo build && ./target/debug/cook server ./seed --port 9080`
Open `http://localhost:9080/menu/Weekly Plan` at 820px in both themes.
Expected: consistent with the recipe page; no gradients.

- [ ] **Step 3: Run the suite**

Run: `npm test`
Expected: same as baseline.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(ui): token-based menu page"
```

---

## Task 12: Editor, new-recipe, API docs and error pages

**Files:**
- Modify: `templates/edit.html`, `templates/new.html`, `templates/api_docs.html`, `templates/error.html`
- Modify: `static/css/input.css` (CodeMirror dark rules)

- [ ] **Step 1: Convert the four templates**

Replace panel wrappers with `.card`, buttons with `.btn` (one `.btn-primary` per page), and headings with the 24px title style. In `edit.html:31` the modal `bg-white dark:bg-gray-800 rounded-2xl shadow-xl p-6 max-w-md mx-4` becomes `card p-5 max-w-md mx-4`.

- [ ] **Step 2: Point CodeMirror at the tokens**

In `static/css/input.css`, replace the eight `.dark .cm-*` rules with token-driven equivalents that work in both themes:

```css
    .cm-editor { background: var(--surface); color: var(--text); }

    .cm-gutters {
        background: var(--surface-sunk);
        border-color: var(--border);
        color: var(--text-faint);
    }

    .cm-activeLineGutter { background: var(--surface-sunk); }

    .cm-activeLine { background: color-mix(in srgb, var(--surface-sunk) 60%, transparent); }

    .cm-content { caret-color: var(--text); }

    .cm-cursor { border-left-color: var(--text); }

    .cm-selectionBackground { background: color-mix(in srgb, var(--info) 30%, transparent) !important; }
```

- [ ] **Step 3: Rebuild and check the editor**

Run: `npm run build-css && npm run build-js && cargo build && ./target/debug/cook server ./seed --port 9080`
Open `http://localhost:9080/edit/Neapolitan Pizza` in both themes.
Expected: editor text legible, cursor visible, gutter readable, selection visible in both themes.

- [ ] **Step 4: Run the suite**

Run: `npm test`
Expected: `api-docs.spec.ts` passes; overall same as baseline.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(ui): token-based editor, docs and error pages"
```

---

## Task 13: Delete the remaining dark-override block

Only now is it safe: every page consumes tokens, so nothing depends on `.dark .bg-white` and friends.

**Files:**
- Modify: `templates/base.html` (`<style>` block)
- Modify: `tailwind.config.js`

- [ ] **Step 1: Confirm nothing still relies on the overrides**

Run: `grep -rn "bg-white\|text-gray-[0-9]\|bg-gray-50\|border-gray-[0-9]\|bg-gradient-to-" templates/*.html | grep -v "print:" | grep -v "^templates/base.html:[0-9]*: *\." `
Expected: no output, or only matches inside `base.html`'s `@media print` block. **If any page still uses these utilities, go back and finish migrating it — deleting the block now would break that page's dark mode.**

- [ ] **Step 2: Delete the block**

In `templates/base.html`, delete everything in the `<style>` block from the `/* Dark mode styles */` comment up to (but **not** including) `@media print {`.

Keep:
- the `.viewport` rule above it
- the entire `@media print { … }` block below it
- the `#search-results a.search-selected` rule you retokenised in Task 5

The `.dark { color-scheme: dark; }` declaration moves to `input.css` in Task 1, so it is not lost.

- [ ] **Step 2b: Remove the stale print selector**

Inside the retained `@media print` block, delete:

```css
            /* Hide entire navigation bar and its contents */
            .bg-white.shadow-lg.rounded-2xl.mb-8 {
                display: none !important;
            }
```

It targeted the old nav wrapper, which no longer exists. The `nav, #search-container, …` rule directly above it already hides the app bar.

- [ ] **Step 3: Remove the legacy palette**

In `tailwind.config.js`, delete the `colors` block (`primary-orange`, `primary-green`, `light-orange`, `light-blue`, `light-green`, `light-yellow`). Leave `darkMode`, `content` and the gradient keyframes.

Run: `grep -rn "primary-orange\|primary-green\|light-orange\|light-blue\|light-green\|light-yellow" templates/ static/css/input.css`
Expected: no output. If anything matches, convert it to a token first.

- [ ] **Step 4: Rebuild and check every page in dark mode**

Run: `npm run build-css && cargo build && ./target/debug/cook server ./seed --port 9080`
Visit each of `/`, `/recipe/Neapolitan Pizza`, `/shopping-list`, `/pantry`, `/preferences`, `/menu/Weekly Plan`, `/edit/Neapolitan Pizza`, `/api-docs` in dark mode.
Expected: every page fully legible. Any white-on-white or black-on-black means a page still depended on an override — fix that page rather than restoring the block.

- [ ] **Step 5: Check printing**

Open `/recipe/Neapolitan Pizza` in dark mode and use the browser's print preview.
Expected: dark background forced to white, text dark, nav hidden, badges readable. The `@media print` block must be untouched.

- [ ] **Step 6: Run the suite**

Run: `npm test`
Expected: same as baseline.

- [ ] **Step 3b: Split `input.css`**

`input.css` now holds the theme contract (tokens, `@theme` registration) and the whole component vocabulary in one file, and the component half is edited constantly while the token half is meant to stay stable. Split at the `@layer components` boundary:

- `static/css/input.css` keeps `@import "tailwindcss"`, `@config`, the `:root` / `.dark` token blocks and `@theme inline` — the rarely-touched theme contract.
- Move the entire `@layer components { … }` block into a new `static/css/components.css`.
- Pull it back in with `@import "./components.css";` placed immediately after the `@theme inline` block. Tailwind v4 processes `@layer components` contributions from imported partials correctly.

Verify with `npm run build-css` (exit 0) and confirm the compiled `output.css` still contains `.card`, `.btn`, `.row` and `.metaline`.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "refactor(ui): remove dark-mode utility override block"
```

---

## Task 14: Final verification

**Files:** none modified unless a check fails.

- [ ] **Step 1: Accessibility contrast**

Run: `npx playwright test tests/e2e/accessibility.spec.ts --project=chromium`
Expected: 12 tests PASS, including `should have sufficient color contrast` (axe `color-contrast` is enabled at line 134).

If contrast fails, darken `--accent-text` in light mode and lighten `--text-muted` in dark mode until it passes. These four pairs must reach AA in both themes: `--text` on `--surface`, `--text-muted` on `--surface`, `--accent-text` on `--surface`, and `--accent-ink` on `--accent`.

- [ ] **Step 2: Full E2E suite**

Run: `npm test`
Expected: 127 passed, 2 flaky, 5 skipped — matching the Task 2 baseline exactly.

- [ ] **Step 3: Rust checks**

```bash
cargo fmt
cargo clippy
cargo test
```

Expected: all three clean, no warnings. Required by `CLAUDE.md` before any PR.

- [ ] **Step 4: Manual matrix**

With the server running, check every page at **820px**, **1024px** and **1440px**, in **light** and **dark**:

`/` · `/recipe/Neapolitan Pizza` · `/shopping-list` · `/pantry` · `/preferences` · `/menu/Weekly Plan` · `/edit/Neapolitan Pizza` · `/api-docs`

Expected: no horizontal scrollbars, no wrapped ingredient rows, no unlabelled icon-only buttons, one accent colour throughout, consistent card and control heights across pages.

- [ ] **Step 5: Confirm the gradients are gone**

Run: `grep -rc "bg-gradient-to-" templates/*.html`
Expected: `0` for every template except `base.html`, whose remaining matches must all be inside `@media print`.

- [ ] **Step 6: Commit any fixes and open the PR**

```bash
git add -A
git commit -m "fix(ui): final contrast and layout adjustments"
git push -u origin design/web-ui-refresh
gh pr create --title "feat(ui): design refresh with semantic tokens" --body "Implements docs/superpowers/specs/2026-08-15-web-ui-refresh-design.md"
```

---

## Deliberately out of scope

Do not do these as part of this plan:

- **Pantry raw quantity display (`250%g`).** `%` is `pantry.conf`'s quantity/unit separator, so this is the stored value shown unformatted rather than a defect. Rendering it as `250 g` is a separate change.
- **Cook mode layout.** Its dead vertical space, missing next/previous affordances and duplicated mise-en-place entries are a known follow-up. This plan only updates the entity classes it shares with the recipe page (Task 4).
- **i18n keys.** If `recipe-method`, `recipes-group-folders`, `recipes-group-menus` or `recipes-group-recipes` do not exist, use English literals and note them. Adding locale entries is a separate change.
- **Any backend, routing, template-data or API change.** This is presentation-layer only.
