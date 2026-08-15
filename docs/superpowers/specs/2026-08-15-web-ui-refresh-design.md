# Web UI Design Refresh

**Date:** 2026-08-15
**Status:** Approved, ready for planning
**Reference mockup:** [`2026-08-15-web-ui-refresh-mockup.html`](./2026-08-15-web-ui-refresh-mockup.html) — open directly in a browser; the `⇄` button switches between the recipe and list views, `◐` toggles light/dark.

## Goal

Refresh the CookCLI web server UI so it is visually consistent, more concise, and pleasant to use on a tablet. The priority use case is **cooking from a recipe on a tablet propped in the kitchen**, but the refresh applies to all pages.

## Problem

Measured at 820px viewport width (iPad portrait) and by inventorying the templates:

### Density

- Content starts ~370px down the recipe page: a ~100px nav bar, then a title block, a tag row, and one-to-two rows of metadata pills.
- Only ~2.5 method steps and half the ingredient list fit above the fold.
- The recipes index shows ~4 cards per screen; each folder card is ~190px tall to display one word and a count.

### Layout breaks in the tablet band

Templates use only the `md` (768px) and `lg` (1024px) breakpoints. Tablet portrait falls in "md but not lg", so it receives desktop-density layout at three-quarters of the width:

- `recipe.html` applies `grid md:grid-cols-3` at 820px, leaving the ingredient rail ~190px wide. `mozzarella cheese` / `100 grams` wraps onto two lines.
- Action buttons hide their labels below `lg` (`<span class="hidden lg:inline">`), so Edit / Add to Shopping List / Cook render as three unlabeled coloured circles.

### No design system

| Primitive | Distinct values in use |
| --- | --- |
| Border radius | 5 (`rounded-sm`, `-lg`, `-xl`, `-2xl`, `-full`) |
| Shadow | 4 (`shadow-xs`, `-md`, `-lg`, `-xl`) |
| Type size | 7 (`text-xs` … `text-4xl`) |
| Gradients | ~40 instances across 9 templates |

Accent colour is not controlled: orange, yellow, green, blue, cyan, purple, and pink gradients all appear as primary-weight UI.

### Three competing sources of styling truth

`static/css/custom-styles.css` defines `.recipe-card`, `.ingredient-badge`, `.cookware-badge`, `.timer-badge`, `.btn-primary`, `.search-input`, `.recipe-image-placeholder`, `.step-number`, `.tag`, `.nav-pill` and `.metadata-pill` — every one of which `input.css` *also* defines, with different values. It is linked after `output.css` in `base.html`, so it silently wins. `static/css/styles.css` (444 lines) is referenced by nothing at all.

Both files are deleted; `input.css` becomes the single source of truth.

### Dark mode is unmaintainable

`templates/base.html` contains roughly 450 lines of blanket overrides of the form:

```css
.dark .bg-white     { … }
.dark .text-gray-700 { … }
.dark .bg-gradient-to-r.from-orange-50.to-yellow-50 { … }
```

Dark mode patches Tailwind utility classes one at a time. Every new component requires another manual override, which is the root cause of the inconsistency — it is structurally impossible to stay consistent under this approach.

## Approach

Three options were considered:

| Option | Description | Trade-off |
| --- | --- | --- |
| **A. Token-based system refresh** | Semantic CSS custom properties defined once for light and dark; a small component layer; compact spacing and type scales; a real tablet tier. Delete the `.dark .*` override block. | Touches every page. The only option that fixes consistency at the root. |
| B. Compaction only | Keep the current look, tighten padding, fix breakpoints. | Fast and low-risk, but gradient soup and the dark-override pile remain. |
| C. Full redesign | New palette, type, sidebar navigation, new layout patterns. | Highest risk; discards working features (Cook mode, search, shortcuts). |

**Option A is chosen.**

### Visual identity

The refreshed identity is **restrained**: one accent colour, no gradients, hairline separators, and tinted text in place of bordered pills. This reads as a calm reference tool, maximises legibility at arm's length in a kitchen, and is the easiest to keep consistent as pages are added.

This is a deliberate move away from the current playful identity (gradients, emoji circles, rainbow card borders).

## Design

### 1. Token layer

Added to `static/css/input.css`. Eleven semantic names, defined once per theme:

```css
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
}

.dark {
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
}
```

Semantic tokens are registered with Tailwind v4's `@theme inline` so they generate real utilities that resolve to the custom properties and therefore flip automatically under `.dark`:

```css
@theme inline {
  --color-bg:           var(--bg);           /* bg-bg          */
  --color-surface:      var(--surface);      /* bg-surface     */
  --color-sunk:         var(--surface-sunk); /* bg-sunk        */
  --color-line:         var(--border);       /* border-line    */
  --color-line-strong:  var(--border-strong);/* border-line-strong */
  --color-text:         var(--text);         /* text-text      */
  --color-muted:        var(--text-muted);   /* text-muted     */
  --color-faint:        var(--text-faint);   /* text-faint     */
  --color-accent:       var(--accent);       /* bg-accent      */
  --color-accent-text:  var(--accent-text);  /* text-accent-text */
  --color-accent-soft:  var(--accent-soft);  /* bg-accent-soft */
  --color-accent-ink:   var(--accent-ink);   /* text-accent-ink */
}
```

Border tokens are named `line` rather than `border` so the generated utility reads `border-line`, not `border-border`.

The existing `@config "../../tailwind.config.js"` directive stays; the legacy `primary-orange` / `light-*` colours in `tailwind.config.js` are removed once no template references them.

Three further status tokens cover pantry stock and shopping-list state: `--ok`, `--ok-soft`, `--info` (fifteen tokens in total).

**Collapsed scales:**

| Scale | Values |
| --- | --- |
| Radius | `6px` (controls), `10px` (cards), `999px` (chips) |
| Shadow | One level, `0 1px 2px rgba(0,0,0,.04)`; `none` in dark |
| Type | `12 / 13 / 15 / 17 / 24px` |
| Spacing | 4px grid |

Prose line-height is `1.6`. The current `leading-8` (2rem on 16px text) is the main reason steps consume so much vertical space.

### 2. Component layer

Seven classes in `@layer components`, all consuming tokens. They replace ad-hoc utility strings so a page's markup states intent rather than appearance.

| Class | Purpose |
| --- | --- |
| `.card` / `.card-head` | Bordered surface with an uppercase micro-label header and optional right-aligned count |
| `.btn`, `.btn-primary` | 32px control. One primary (accent-filled) per view; everything else is a neutral bordered button |
| `.row` | Hairline-separated list item: flexible name, right-aligned tabular-figure value |
| `.chip` | Small neutral pill for tags |
| `.metaline` | Single line of dot-separated metadata |
| `.stepper` | Segmented −/value/+ control for recipe scaling |
| `.section-label` | Uppercase micro-label for grouping list sections |

Inline recipe entities become weight plus tint rather than bordered gradient pills, so they stop competing with the prose:

- `.i-ing` — `--accent-text`, weight 600
- `.i-cook` — `--ok`, weight 600
- `.i-time` — weight 600 on `--surface-sunk`, tabular figures

The existing `.ingredient-badge`, `.cookware-badge`, `.timer-badge`, `.nav-pill`, `.metadata-*`, `.tag`, `.recipe-card`, `.step-number`, and `.btn-primary` gradient definitions in `input.css` are replaced by the above. `cooking-mode.css` references `.ingredient-badge` / `.cookware-badge` / `.timer-badge` and must be updated in the same change so Cook mode does not lose its inline entity styling.

### 3. Responsive tiers

- Recipe layout switches at **700px** to `grid-template-columns: 264px minmax(0,1fr)`, widening the rail to `300px` at 1024px. A fixed rail is what guarantees ingredient names and quantities never wrap.
- The ingredient rail is `position: sticky; top: 64px` at ≥700px, so it stays visible while scrolling the method.
- Action buttons keep their text labels at all widths — the `hidden lg:inline` label spans are removed.
- Card grids use `repeat(auto-fill, minmax(230px, 1fr))` so column count follows available width rather than hard breakpoints.

### 4. Application chrome

`base.html` navigation compacts from ~100px to a 48px sticky bar: brand mark, three text nav items, a normal-width search input (replacing the ~500px purple-ringed field), and three icon buttons (theme, shortcuts, preferences). The existing mobile "more menu" behaviour below `md` is preserved.

### 5. Page-by-page changes

| Page | Change |
| --- | --- |
| `base.html` | 48px sticky app bar; **delete the ~450-line `.dark .*` override block** |
| `recipe.html` | Compact head (breadcrumb, 24px title, action row); six colour-coded metadata pills → one `.metaline`; ingredient rail using `.card` + `.row`; steps unboxed into a single `.card` with hairline separators and 22px muted step numbers; inline entities retinted |
| `recipes.html` | 190px hero cards → ~60px `.row`-style cards grouped under Folders / Menus / Recipes `.section-label`s |
| `shopping_list.html` | Aisle groups become `.card` + `.row`; three gradients removed |
| `pantry.html` | Four-line stat blocks → single row with right-aligned quantity and secondary dates; status dot uses `--ok` / status tokens |
| `preferences.html` | Eleven gradients → plain `.card` sections |
| `menu.html` | Six gradients → `.card`; head aligned with `recipe.html` |
| `edit.html`, `new.html` | Inherit `.btn` / `.card`; CodeMirror dark theme reads tokens instead of the hardcoded `.dark .cm-*` rules in `input.css` |
| `api_docs.html`, `error.html` | Inherit `.card` / type scale |

### 6. Sequencing and risk

The token and component layers land **first** and are purely additive — no existing page changes behaviour. Pages then migrate one at a time, each independently reviewable and revertable.

**The `.dark .*` override block in `base.html` is deleted last**, only once no remaining page depends on it. Deleting it earlier would break every unmigrated page's dark mode.

### 7. Verification

- `npm test` (Playwright E2E, `tests/e2e/`) runs after each page migration. Tests use mostly semantic selectors, but any that assert on gradient or colour utility classes will need updating alongside the page they cover.
- `tests/e2e/accessibility.spec.ts` covers WCAG 2.0 AA contrast and will catch regressions in the new palette. All token pairs (`--text` on `--surface`, `--text-muted` on `--surface`, `--accent-text` on `--surface`, `--accent-ink` on `--accent`) must pass AA in both themes.
- Each migrated page is visually checked at **820px** (tablet portrait), **1024px**, and **1440px**, in both light and dark.
- `cargo fmt`, `cargo clippy`, and `cargo test` must pass before the PR, per `CLAUDE.md`.

## Out of scope

- **Cook mode** (`static/css/cooking-mode.css`, `static/js/cooking-mode.js`) keeps its current appearance, apart from the inline entity classes it shares with the main stylesheet. Its own compaction — dead vertical space, missing next/previous affordances, duplicate ingredient entries in the mise-en-place grid — is a natural follow-up, tracked separately.
- **Pantry raw quantity display.** Quantities render as `250%g`, `2%l`, `200%g`. `%` is the quantity/unit separator used by `pantry.conf` — the pantry edit form's own placeholder reads `Quantity (e.g., 500%g)` — so this is the stored value being displayed unformatted rather than a defect. Rendering it as `250 g` is worthwhile but belongs in its own change.
- No backend, routing, template-data, or API changes. This is a presentation-layer refresh only.
