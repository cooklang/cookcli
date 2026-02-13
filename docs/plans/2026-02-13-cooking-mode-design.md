# Cooking Mode Design

## Overview

A full-screen cooking mode overlay for the CookCLI web UI, optimized for tablet use in the kitchen. It presents recipe steps one at a time in a vertical carousel with large fonts, section-based navigation, and wake lock to prevent screen sleep.

## Architecture

### Data Approach: Embedded JSON

The recipe template embeds a `<script id="cooking-mode-data" type="application/json">` tag containing structured recipe data. A new `cooking-mode.js` reads this JSON and drives the overlay UI. No extra network requests needed.

JSON structure:

```json
{
  "name": "Recipe Name",
  "scale": 2.0,
  "image": "/api/static/recipe.jpg",
  "sections": [
    {
      "name": "Sauce",
      "ingredients": [
        { "name": "tomatoes", "quantity": "4", "unit": "whole", "note": "diced" }
      ],
      "steps": [
        {
          "number": 1,
          "html": "<rendered step HTML with badges>",
          "ingredients": [{ "name": "olive oil", "quantity": "2", "unit": "tbsp" }],
          "image": "/api/static/step1.jpg"
        }
      ]
    }
  ]
}
```

### Implementation: JS Overlay

- Full-screen overlay (`position: fixed; inset: 0; z-index: 9999`) on top of the recipe page
- Same URL, no route change
- Recipe page stays loaded underneath; close returns instantly

## UI Flow: Continuous Section Carousel

The carousel is one continuous vertical stream of cards:

1. **Section 1 Header Card** — Section name (large), mise en place ingredients for this section
2. **Section 1 Step 1** — Step content with image/ingredients
3. **Section 1 Step 2** — ...
4. **Section 1 Step N** — Last step of section 1
5. **Section 2 Header Card** — Section name, mise en place for section 2
6. **Section 2 Step 1** — ...
7. ... continues for all sections
8. **Done Card** — Completion screen

For single-section recipes with no section name: skip the section header, show mise en place as the first card, then steps.

### Card Types

**Section Header Card:**
- Section name large + centered
- Ingredients listed in a grid (name, quantity, unit, note)
- Acts as a "chapter" divider

**Step Card:**
- Step number badge (top-left)
- Step image (if present, top area)
- Step text with ingredient/cookware/timer badges (large font)
- Required ingredients listed below (smaller, muted)

**Done Card:**
- Recipe name
- Simple completion message

### Carousel Behavior

- One card visible at a time (centered, large font)
- Previous card: visible but faded (~30% opacity, ~85% scale) at top
- Next card: visible but faded at bottom
- Swipe up → next card, swipe down → previous card
- 300ms ease-out transitions

### Header Bar (persistent)

- Recipe name (left)
- Section navigation pills (center) — tap to jump to section header
- Close button X (right)
- Thin progress bar at very bottom of screen

## Entry Point

"Start Cooking" button on the recipe page, alongside existing Edit/Add to Shopping/Print buttons. Only visible if recipe has steps.

## Visual Design

### Always-Dark Theme

Cooking mode uses a dark theme regardless of the user's theme setting:
- Background: deep dark (#1a1a2e or similar)
- Card background: subtle dark (#16213e or similar)
- Text: white/light gray
- Badges: keep existing gradient colors (pop on dark)

### Typography

- Step text: ~24-28px on tablet
- Section headers: ~32-36px, bold
- Mise en place ingredients: ~20px
- Readable from arm's length

### Responsive

- Optimized for tablet (768-1024px)
- Works on phone (slightly smaller text, single column)
- Desktop: constrain max-width to ~800px, center content

## Interactions

### Wake Lock

- Screen Wake Lock API: `navigator.wakeLock.request('screen')`
- Acquire on cooking mode entry
- Release on cooking mode exit
- Re-acquire on visibility change (tab becomes visible again)
- Graceful fallback: if API unavailable, do nothing

### Swipe Navigation

- Custom touch event handling (no library)
- `touchstart` → record Y position
- `touchmove` → translate card to follow finger
- `touchend` → if delta > ~50px threshold, commit; else snap back
- Keyboard: Arrow Up/Down for prev/next
- Debounce rapid swipes

### Section Navigation

- Clicking a section pill jumps to that section's header card
- Smooth animated transition

### Close

- X button in header
- Escape key
- Releases wake lock, removes overlay

### No Persistence

- Current step position is ephemeral
- Closing and reopening starts from the beginning
- Mise en place checkmarks are visual-only, not persisted

## Files to Create/Modify

### New Files
- `static/js/cooking-mode.js` — all cooking mode logic (carousel, swipe, wake lock)
- `static/css/cooking-mode.css` — cooking mode styles (separate from Tailwind build)

### Modified Files
- `templates/recipe.html` — add JSON data blob, Start Cooking button, include new JS/CSS
- `src/server/templates.rs` — add serialization for cooking mode JSON data
- `src/server/ui.rs` — build cooking mode data structure from recipe data
