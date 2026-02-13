# Cooking Mode Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a full-screen cooking mode overlay to the recipe web UI, optimized for tablet use in the kitchen, with a vertical carousel of section headers and steps.

**Architecture:** Embed recipe data as JSON in the recipe template. A new `cooking-mode.js` reads this data and renders a full-screen overlay with a vertical card carousel (section headers with mise en place + individual steps). Wake Lock API keeps the screen on. Custom touch handling for swipe navigation.

**Tech Stack:** Vanilla JS, CSS (hand-written, not Tailwind-compiled), Askama templates, serde_json for JSON serialization

---

### Task 1: Add JSON data blob to recipe template

**Files:**
- Modify: `templates/recipe.html:336-411` (add JSON script tag before existing `<script>`)

**Step 1: Add the embedded JSON data block**

In `templates/recipe.html`, add a `<script>` tag right before the existing `<script>` block (before line 336). This uses the Askama `|json` filter on the already-Serialize data:

```html
<script id="cooking-mode-data" type="application/json">
{
    "name": "{{ recipe.name }}",
    "scale": {{ scale }},
    "image": {% match image_path %}{% when Some with (img) %}"{{ img }}"{% when None %}null{% endmatch %},
    "sections": [
        {% for section in sections %}
        {
            "name": {% match section.name %}{% when Some with (name) %}"{{ name }}"{% when None %}null{% endmatch %},
            "stepOffset": {{ section.step_offset }},
            "ingredients": [
                {% for ing in section.ingredients %}
                {
                    "name": "{{ ing.name }}",
                    "quantity": {% match ing.quantity %}{% when Some with (q) %}"{{ q }}"{% when None %}null{% endmatch %},
                    "unit": {% match ing.unit %}{% when Some with (u) %}"{{ u }}"{% when None %}null{% endmatch %},
                    "note": {% match ing.note %}{% when Some with (n) %}"{{ n }}"{% when None %}null{% endmatch %}
                }{% if !loop.last %},{% endif %}
                {% endfor %}
            ],
            "steps": [
                {% for item in section.items %}
                {% match item %}
                {% when crate::server::templates::RecipeSectionItem::Step with (step) %}
                {
                    "type": "step",
                    "number": {{ step.number }},
                    "globalNumber": {{ section.step_offset + step.number }},
                    "image": {% match step.image_path %}{% when Some with (img) %}"{{ img }}"{% when None %}null{% endmatch %},
                    "ingredients": [
                        {% for ing in step.ingredients %}
                        {
                            "name": "{{ ing.name }}",
                            "quantity": {% match ing.quantity %}{% when Some with (q) %}"{{ q }}"{% when None %}null{% endmatch %},
                            "unit": {% match ing.unit %}{% when Some with (u) %}"{{ u }}"{% when None %}null{% endmatch %},
                            "note": {% match ing.note %}{% when Some with (n) %}"{{ n }}"{% when None %}null{% endmatch %}
                        }{% if !loop.last %},{% endif %}
                        {% endfor %}
                    ]
                }{% if !loop.last %},{% endif %}
                {% when crate::server::templates::RecipeSectionItem::Note with (note) %}
                {% endmatch %}
                {% endfor %}
            ]
        }{% if !loop.last %},{% endif %}
        {% endfor %}
    ]
}
</script>
```

Note: The step HTML content will be captured from the DOM at runtime by the JS (reading the rendered step elements), rather than duplicating the complex Askama step rendering logic in JSON. The JSON provides structured data (ingredients, images, step numbers), while the rendered HTML provides the rich step text.

**Step 2: Verify the template compiles**

Run: `cargo build -p cookcli 2>&1 | head -20`
Expected: Build succeeds (or only unrelated warnings)

**Step 3: Commit**

```bash
git add templates/recipe.html
git commit -m "feat(cooking-mode): embed recipe JSON data in template"
```

---

### Task 2: Add "Start Cooking" button to recipe page

**Files:**
- Modify: `templates/recipe.html:41-77` (add button to action buttons row)

**Step 1: Add the Start Cooking button**

In `templates/recipe.html`, add a new button after the print button (after line 76, before the closing `</div>` on line 77):

```html
                <button id="start-cooking-btn"
                        onclick="startCookingMode()"
                        class="px-3 lg:px-4 py-2 text-sm lg:text-base bg-gradient-to-r from-green-500 to-emerald-500 text-white rounded-lg hover:from-green-600 hover:to-emerald-600 transition-all shadow-md flex items-center gap-1.5 lg:gap-2 whitespace-nowrap print:hidden"
                        title="Start Cooking">
                    <svg class="w-4 h-4 lg:w-5 lg:h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"></path>
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                    </svg>
                    <span class="hidden lg:inline">Cook</span>
                </button>
```

**Step 2: Verify build**

Run: `cargo build -p cookcli 2>&1 | head -20`
Expected: Build succeeds

**Step 3: Commit**

```bash
git add templates/recipe.html
git commit -m "feat(cooking-mode): add Start Cooking button to recipe page"
```

---

### Task 3: Create cooking-mode.css

**Files:**
- Create: `static/css/cooking-mode.css`

**Step 1: Write the cooking mode stylesheet**

Create `static/css/cooking-mode.css` with all styles for the overlay. This is a standalone CSS file (not processed by Tailwind):

```css
/* Cooking Mode Overlay */
.cooking-overlay {
    position: fixed;
    inset: 0;
    z-index: 9999;
    background: #1a1a2e;
    color: #e0e0e0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    touch-action: none;
    user-select: none;
    -webkit-user-select: none;
}

/* Header bar */
.cooking-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 20px;
    background: rgba(0, 0, 0, 0.3);
    flex-shrink: 0;
    gap: 12px;
    z-index: 2;
}

.cooking-header-title {
    font-size: 16px;
    font-weight: 600;
    color: #fff;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex-shrink: 1;
    min-width: 0;
}

.cooking-header-sections {
    display: flex;
    gap: 8px;
    overflow-x: auto;
    flex-shrink: 0;
    -webkit-overflow-scrolling: touch;
    scrollbar-width: none;
}

.cooking-header-sections::-webkit-scrollbar {
    display: none;
}

.cooking-section-pill {
    padding: 6px 14px;
    border-radius: 20px;
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    cursor: pointer;
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.7);
    border: 1px solid rgba(255, 255, 255, 0.15);
    transition: all 0.2s;
}

.cooking-section-pill:hover {
    background: rgba(255, 255, 255, 0.15);
}

.cooking-section-pill.active {
    background: linear-gradient(135deg, #f97316, #eab308);
    color: #fff;
    border-color: transparent;
}

.cooking-close-btn {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.1);
    border: none;
    color: #fff;
    font-size: 20px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: background 0.2s;
}

.cooking-close-btn:hover {
    background: rgba(255, 255, 255, 0.2);
}

/* Carousel container */
.cooking-carousel {
    flex: 1;
    position: relative;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
}

/* Card positioning */
.cooking-card-container {
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
}

.cooking-card {
    position: absolute;
    width: calc(100% - 40px);
    max-width: 800px;
    max-height: 70vh;
    overflow-y: auto;
    border-radius: 20px;
    padding: 32px;
    transition: transform 0.3s ease-out, opacity 0.3s ease-out;
    scrollbar-width: thin;
    scrollbar-color: rgba(255,255,255,0.2) transparent;
}

.cooking-card::-webkit-scrollbar {
    width: 4px;
}

.cooking-card::-webkit-scrollbar-thumb {
    background: rgba(255,255,255,0.2);
    border-radius: 2px;
}

.cooking-card.current {
    opacity: 1;
    transform: translateY(0) scale(1);
    z-index: 1;
}

.cooking-card.prev {
    opacity: 0.25;
    transform: translateY(-75%) scale(0.85);
    z-index: 0;
    pointer-events: none;
}

.cooking-card.next {
    opacity: 0.25;
    transform: translateY(75%) scale(0.85);
    z-index: 0;
    pointer-events: none;
}

.cooking-card.hidden-card {
    opacity: 0;
    transform: translateY(100%) scale(0.8);
    pointer-events: none;
}

/* Swipe animation */
.cooking-card.swiping {
    transition: none;
}

/* Section header card */
.cooking-card-section {
    background: linear-gradient(135deg, #1e293b, #0f172a);
    border: 1px solid rgba(255, 255, 255, 0.1);
    text-align: center;
}

.cooking-card-section h2 {
    font-size: 32px;
    font-weight: 700;
    margin-bottom: 24px;
    background: linear-gradient(135deg, #f97316, #eab308);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
}

.cooking-mise-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 12px;
    text-align: left;
}

.cooking-mise-item {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    padding: 8px 12px;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 10px;
    gap: 8px;
}

.cooking-mise-name {
    font-size: 18px;
    color: #e0e0e0;
}

.cooking-mise-note {
    font-size: 14px;
    color: rgba(255, 255, 255, 0.5);
    font-style: italic;
}

.cooking-mise-qty {
    font-size: 16px;
    font-weight: 600;
    color: #f97316;
    white-space: nowrap;
}

/* Step card */
.cooking-card-step {
    background: linear-gradient(135deg, #1e293b, #162032);
    border: 1px solid rgba(255, 255, 255, 0.1);
}

.cooking-step-number {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: linear-gradient(135deg, #f97316, #eab308);
    color: #fff;
    font-weight: 700;
    font-size: 16px;
    margin-bottom: 16px;
}

.cooking-step-image {
    width: 100%;
    max-height: 300px;
    object-fit: cover;
    border-radius: 12px;
    margin-bottom: 16px;
}

.cooking-step-text {
    font-size: 24px;
    line-height: 1.6;
    color: #f0f0f0;
}

/* Override badge styles within cooking mode for readability */
.cooking-step-text .ingredient-badge {
    font-size: 22px;
    padding: 2px 10px;
}

.cooking-step-text .cookware-badge {
    font-size: 22px;
    padding: 2px 10px;
}

.cooking-step-text .timer-badge {
    font-size: 22px;
    padding: 2px 10px;
}

.cooking-step-ingredients {
    margin-top: 20px;
    padding-top: 16px;
    border-top: 1px solid rgba(255, 255, 255, 0.1);
    font-size: 15px;
    color: rgba(255, 255, 255, 0.5);
}

.cooking-step-ingredients span {
    margin-right: 12px;
}

/* Done card */
.cooking-card-done {
    background: linear-gradient(135deg, #1e293b, #0f172a);
    border: 1px solid rgba(255, 255, 255, 0.1);
    text-align: center;
}

.cooking-card-done h2 {
    font-size: 36px;
    margin-bottom: 12px;
}

.cooking-card-done p {
    font-size: 20px;
    color: rgba(255, 255, 255, 0.6);
}

/* Progress bar */
.cooking-progress {
    height: 3px;
    background: rgba(255, 255, 255, 0.1);
    flex-shrink: 0;
}

.cooking-progress-bar {
    height: 100%;
    background: linear-gradient(90deg, #f97316, #eab308);
    transition: width 0.3s ease-out;
}

/* Responsive */
@media (max-width: 640px) {
    .cooking-card {
        padding: 24px 20px;
        max-height: 75vh;
    }

    .cooking-card-section h2 {
        font-size: 26px;
    }

    .cooking-step-text {
        font-size: 20px;
    }

    .cooking-mise-grid {
        grid-template-columns: 1fr;
    }

    .cooking-header-title {
        font-size: 14px;
    }
}

@media (min-width: 1024px) {
    .cooking-step-text {
        font-size: 28px;
    }
}
```

**Step 2: Commit**

```bash
git add static/css/cooking-mode.css
git commit -m "feat(cooking-mode): add cooking mode stylesheet"
```

---

### Task 4: Create cooking-mode.js — core carousel and data loading

**Files:**
- Create: `static/js/cooking-mode.js`

**Step 1: Write the cooking mode JavaScript**

Create `static/js/cooking-mode.js`. This is the main file containing all cooking mode logic. Build it in one go as it's all interconnected:

```javascript
// Cooking Mode for CookCLI
(function() {
    'use strict';

    let state = {
        cards: [],        // Array of card objects
        currentIndex: 0,
        wakeLock: null,
        overlay: null,
        touchStartY: 0,
        touchCurrentY: 0,
        isSwiping: false,
        totalSteps: 0
    };

    // ─── Data Loading ─────────────────────────────────────────────

    function loadRecipeData() {
        const el = document.getElementById('cooking-mode-data');
        if (!el) return null;
        try {
            return JSON.parse(el.textContent);
        } catch (e) {
            console.error('Failed to parse cooking mode data:', e);
            return null;
        }
    }

    // Capture rendered step HTML from the recipe page DOM before overlay
    function captureStepHTML() {
        const stepElements = [];
        const sectionEls = document.querySelectorAll('.md\\:col-span-2 ol');
        sectionEls.forEach(ol => {
            ol.querySelectorAll(':scope > li').forEach(li => {
                // Get the step text div (the one with leading-8 class)
                const textDiv = li.querySelector('.leading-8');
                if (textDiv) {
                    stepElements.push(textDiv.innerHTML);
                }
            });
        });
        return stepElements;
    }

    // ─── Card Building ────────────────────────────────────────────

    function buildCards(data, stepHTMLs) {
        const cards = [];
        let stepHTMLIndex = 0;

        data.sections.forEach((section, sectionIndex) => {
            // Section header card (mise en place)
            // Skip if single unnamed section — but still show mise en place as first card
            const sectionName = section.name || (data.sections.length > 1 ? 'Main' : null);

            if (section.ingredients.length > 0 || sectionName) {
                cards.push({
                    type: 'section',
                    sectionIndex: sectionIndex,
                    name: sectionName || data.name,
                    ingredients: section.ingredients
                });
            }

            // Step cards
            section.steps.forEach(step => {
                cards.push({
                    type: 'step',
                    sectionIndex: sectionIndex,
                    sectionName: sectionName,
                    number: step.globalNumber,
                    image: step.image,
                    html: stepHTMLs[stepHTMLIndex] || '',
                    ingredients: step.ingredients
                });
                stepHTMLIndex++;
            });
        });

        // Done card
        cards.push({
            type: 'done',
            sectionIndex: data.sections.length - 1,
            name: data.name
        });

        return cards;
    }

    // ─── Rendering ────────────────────────────────────────────────

    function renderCard(card) {
        const div = document.createElement('div');
        div.className = 'cooking-card hidden-card';

        if (card.type === 'section') {
            div.classList.add('cooking-card-section');
            let ingredientsHTML = '';
            if (card.ingredients.length > 0) {
                const items = card.ingredients.map(ing => {
                    const qty = [ing.quantity, ing.unit].filter(Boolean).join(' ');
                    const note = ing.note ? `<span class="cooking-mise-note">(${ing.note})</span>` : '';
                    return `<div class="cooking-mise-item">
                        <span class="cooking-mise-name">${ing.name} ${note}</span>
                        ${qty ? `<span class="cooking-mise-qty">${qty}</span>` : ''}
                    </div>`;
                }).join('');
                ingredientsHTML = `<div class="cooking-mise-grid">${items}</div>`;
            }
            div.innerHTML = `<h2>${card.name}</h2>${ingredientsHTML}`;
        }
        else if (card.type === 'step') {
            div.classList.add('cooking-card-step');
            const imageHTML = card.image
                ? `<img class="cooking-step-image" src="${card.image}" alt="Step ${card.number}" />`
                : '';
            const ingredientsHTML = card.ingredients.length > 0
                ? `<div class="cooking-step-ingredients">${card.ingredients.map(ing => {
                    const qty = [ing.quantity, ing.unit].filter(Boolean).join(' ');
                    const note = ing.note ? ` (${ing.note})` : '';
                    return `<span>${ing.name}${qty ? ': ' + qty : ''}${note}</span>`;
                  }).join('')}</div>`
                : '';
            div.innerHTML = `
                <div class="cooking-step-number">${card.number}</div>
                ${imageHTML}
                <div class="cooking-step-text">${card.html}</div>
                ${ingredientsHTML}
            `;
        }
        else if (card.type === 'done') {
            div.classList.add('cooking-card-done');
            div.innerHTML = `<h2>Bon Appetit!</h2><p>${card.name}</p>`;
        }

        return div;
    }

    function renderOverlay(data, cards) {
        const overlay = document.createElement('div');
        overlay.className = 'cooking-overlay';
        overlay.id = 'cooking-overlay';

        // Header
        const header = document.createElement('div');
        header.className = 'cooking-header';

        const title = document.createElement('div');
        title.className = 'cooking-header-title';
        title.textContent = data.name;

        const sectionsNav = document.createElement('div');
        sectionsNav.className = 'cooking-header-sections';
        sectionsNav.id = 'cooking-sections-nav';

        // Build section pills
        data.sections.forEach((section, i) => {
            const pill = document.createElement('button');
            pill.className = 'cooking-section-pill';
            pill.textContent = section.name || (data.sections.length > 1 ? 'Main' : 'Steps');
            pill.dataset.sectionIndex = i;
            pill.addEventListener('click', () => navigateToSection(i));
            sectionsNav.appendChild(pill);
        });

        const closeBtn = document.createElement('button');
        closeBtn.className = 'cooking-close-btn';
        closeBtn.innerHTML = '&times;';
        closeBtn.addEventListener('click', closeCookingMode);

        header.appendChild(title);
        if (data.sections.length > 1 || data.sections[0]?.name) {
            header.appendChild(sectionsNav);
        }
        header.appendChild(closeBtn);

        // Carousel
        const carousel = document.createElement('div');
        carousel.className = 'cooking-carousel';
        carousel.id = 'cooking-carousel';

        const container = document.createElement('div');
        container.className = 'cooking-card-container';
        container.id = 'cooking-card-container';

        cards.forEach(card => {
            container.appendChild(renderCard(card));
        });

        carousel.appendChild(container);

        // Progress bar
        const progress = document.createElement('div');
        progress.className = 'cooking-progress';
        progress.innerHTML = '<div class="cooking-progress-bar" id="cooking-progress-bar"></div>';

        overlay.appendChild(header);
        overlay.appendChild(carousel);
        overlay.appendChild(progress);

        return overlay;
    }

    // ─── Navigation ───────────────────────────────────────────────

    function updateCards() {
        const container = document.getElementById('cooking-card-container');
        if (!container) return;
        const cardEls = container.querySelectorAll('.cooking-card');

        cardEls.forEach((el, i) => {
            el.classList.remove('current', 'prev', 'next', 'hidden-card', 'swiping');
            if (i === state.currentIndex) {
                el.classList.add('current');
            } else if (i === state.currentIndex - 1) {
                el.classList.add('prev');
            } else if (i === state.currentIndex + 1) {
                el.classList.add('next');
            } else {
                el.classList.add('hidden-card');
            }
        });

        // Update progress bar
        const progressBar = document.getElementById('cooking-progress-bar');
        if (progressBar) {
            const pct = state.cards.length > 1
                ? (state.currentIndex / (state.cards.length - 1)) * 100
                : 100;
            progressBar.style.width = pct + '%';
        }

        // Update section pills
        const currentCard = state.cards[state.currentIndex];
        if (currentCard) {
            const pills = document.querySelectorAll('.cooking-section-pill');
            pills.forEach(pill => {
                pill.classList.toggle('active',
                    parseInt(pill.dataset.sectionIndex) === currentCard.sectionIndex);
            });
        }
    }

    function navigateTo(index) {
        if (index < 0 || index >= state.cards.length) return;
        state.currentIndex = index;
        updateCards();
    }

    function navigateToSection(sectionIndex) {
        const cardIndex = state.cards.findIndex(c => c.sectionIndex === sectionIndex);
        if (cardIndex !== -1) {
            navigateTo(cardIndex);
        }
    }

    // ─── Touch Handling ───────────────────────────────────────────

    function onTouchStart(e) {
        if (e.touches.length !== 1) return;
        state.touchStartY = e.touches[0].clientY;
        state.touchCurrentY = state.touchStartY;
        state.isSwiping = true;

        const container = document.getElementById('cooking-card-container');
        const currentEl = container?.querySelectorAll('.cooking-card')[state.currentIndex];
        if (currentEl) currentEl.classList.add('swiping');
    }

    function onTouchMove(e) {
        if (!state.isSwiping || e.touches.length !== 1) return;
        state.touchCurrentY = e.touches[0].clientY;
        const delta = state.touchCurrentY - state.touchStartY;

        const container = document.getElementById('cooking-card-container');
        if (!container) return;
        const cardEls = container.querySelectorAll('.cooking-card');
        const currentEl = cardEls[state.currentIndex];

        if (currentEl) {
            // Limit the drag distance
            const clampedDelta = Math.max(-150, Math.min(150, delta));
            currentEl.style.transform = `translateY(${clampedDelta}px) scale(${1 - Math.abs(clampedDelta) * 0.001})`;
        }
    }

    function onTouchEnd(e) {
        if (!state.isSwiping) return;
        state.isSwiping = false;

        const delta = state.touchCurrentY - state.touchStartY;
        const threshold = 50;

        const container = document.getElementById('cooking-card-container');
        const currentEl = container?.querySelectorAll('.cooking-card')[state.currentIndex];
        if (currentEl) {
            currentEl.classList.remove('swiping');
            currentEl.style.transform = '';
        }

        if (delta < -threshold && state.currentIndex < state.cards.length - 1) {
            // Swipe up → next
            navigateTo(state.currentIndex + 1);
        } else if (delta > threshold && state.currentIndex > 0) {
            // Swipe down → prev
            navigateTo(state.currentIndex - 1);
        } else {
            // Snap back
            updateCards();
        }
    }

    // ─── Keyboard Handling ────────────────────────────────────────

    function onKeyDown(e) {
        if (!state.overlay) return;

        switch (e.key) {
            case 'ArrowDown':
            case 'ArrowRight':
                e.preventDefault();
                navigateTo(state.currentIndex + 1);
                break;
            case 'ArrowUp':
            case 'ArrowLeft':
                e.preventDefault();
                navigateTo(state.currentIndex - 1);
                break;
            case 'Escape':
                e.preventDefault();
                closeCookingMode();
                break;
        }
    }

    // ─── Wake Lock ────────────────────────────────────────────────

    async function acquireWakeLock() {
        if (!('wakeLock' in navigator)) return;
        try {
            state.wakeLock = await navigator.wakeLock.request('screen');
            state.wakeLock.addEventListener('release', () => {
                state.wakeLock = null;
            });
        } catch (err) {
            // Wake lock request failed (e.g., low battery)
            console.warn('Wake lock request failed:', err);
        }
    }

    async function releaseWakeLock() {
        if (state.wakeLock) {
            try {
                await state.wakeLock.release();
            } catch (err) {
                // Already released
            }
            state.wakeLock = null;
        }
    }

    function onVisibilityChange() {
        if (state.overlay && document.visibilityState === 'visible' && !state.wakeLock) {
            acquireWakeLock();
        }
    }

    // ─── Open / Close ─────────────────────────────────────────────

    function startCookingMode() {
        const data = loadRecipeData();
        if (!data) return;

        // Capture step HTML from the rendered page
        const stepHTMLs = captureStepHTML();

        // Build cards
        state.cards = buildCards(data, stepHTMLs);
        if (state.cards.length === 0) return;

        state.currentIndex = 0;

        // Render overlay
        state.overlay = renderOverlay(data, state.cards);
        document.body.appendChild(state.overlay);
        document.body.style.overflow = 'hidden';

        // Initial card positions
        updateCards();

        // Touch events
        const carousel = document.getElementById('cooking-carousel');
        carousel.addEventListener('touchstart', onTouchStart, { passive: true });
        carousel.addEventListener('touchmove', onTouchMove, { passive: true });
        carousel.addEventListener('touchend', onTouchEnd, { passive: true });

        // Keyboard events
        document.addEventListener('keydown', onKeyDown);

        // Wake lock
        acquireWakeLock();
        document.addEventListener('visibilitychange', onVisibilityChange);
    }

    function closeCookingMode() {
        if (!state.overlay) return;

        // Remove overlay
        state.overlay.remove();
        state.overlay = null;
        document.body.style.overflow = '';

        // Remove keyboard handler
        document.removeEventListener('keydown', onKeyDown);

        // Release wake lock
        releaseWakeLock();
        document.removeEventListener('visibilitychange', onVisibilityChange);

        // Reset state
        state.cards = [];
        state.currentIndex = 0;
    }

    // ─── Global API ───────────────────────────────────────────────

    window.startCookingMode = startCookingMode;
    window.closeCookingMode = closeCookingMode;

})();
```

**Step 2: Commit**

```bash
git add static/js/cooking-mode.js
git commit -m "feat(cooking-mode): add cooking mode JS with carousel, swipe, and wake lock"
```

---

### Task 5: Include cooking mode assets in recipe template

**Files:**
- Modify: `templates/recipe.html` (add CSS link and JS include)

**Step 1: Add CSS link and JS include to recipe template**

In `templates/recipe.html`, add a `{% block scripts %}` block at the end (after the existing `</script>` closing tag, before `{% endblock %}`):

```html
{% block scripts %}
<link rel="stylesheet" href="/static/css/cooking-mode.css">
<script src="/static/js/cooking-mode.js"></script>
{% endblock %}
```

**Step 2: Verify build**

Run: `cargo build -p cookcli 2>&1 | head -20`
Expected: Build succeeds

**Step 3: Commit**

```bash
git add templates/recipe.html
git commit -m "feat(cooking-mode): include cooking mode CSS and JS in recipe template"
```

---

### Task 6: Add keyboard shortcut for cooking mode

**Files:**
- Modify: `static/js/keyboard-shortcuts.js:303-349` (add 'c' shortcut to recipe page shortcuts)

**Step 1: Add the 'c' keyboard shortcut**

In `static/js/keyboard-shortcuts.js`, inside the `handleRecipeShortcuts` function switch statement, add a case for 'c' before the existing 'e' case (around line 304):

```javascript
            case 'c':
                event.preventDefault();
                if (typeof startCookingMode === 'function') {
                    startCookingMode();
                }
                return;
```

Also add 'c' to the help text. Find the recipe shortcuts help section and add: `C - Start cooking mode`

**Step 2: Commit**

```bash
git add static/js/keyboard-shortcuts.js
git commit -m "feat(cooking-mode): add 'c' keyboard shortcut for cooking mode"
```

---

### Task 7: Manual testing and polish

**Step 1: Start the dev server**

Run: `cargo run -- server ./seed`

**Step 2: Test in browser**

Open a recipe page and verify:
1. "Start Cooking" button appears in the action buttons row
2. Clicking it opens the full-screen dark overlay
3. The first card shows section header with ingredients (mise en place)
4. Swiping up/down navigates between cards
5. Arrow keys navigate between cards
6. Section pills in header work (if multi-section recipe)
7. Progress bar updates as you navigate
8. X button closes the overlay
9. Escape key closes the overlay
10. Screen stays awake during cooking mode (check via browser DevTools > Application > Wake Lock)

**Step 3: Test with different recipes**
- Single-section recipe (no section name)
- Multi-section recipe
- Recipe with step images
- Recipe with no images
- Recipe with scaling applied

**Step 4: Test responsive behavior**
- Tablet size (768-1024px)
- Phone size (<640px)
- Desktop (>1024px)

**Step 5: Fix any issues found during testing**

Address any visual glitches, navigation bugs, or responsive issues.

**Step 6: Commit any fixes**

```bash
git add -A
git commit -m "fix(cooking-mode): polish from manual testing"
```
