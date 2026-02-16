// Cooking Mode for CookCLI
(function() {
    'use strict';

    // ─── Constants ───────────────────────────────────────────────

    const WHEEL_DEBOUNCE_MS = 300;
    const SWIPE_THRESHOLD = 50;
    const SWIPE_CLAMP_MAX = 150;
    const SWIPE_SCALE_FACTOR = 0.001;

    // ─── State ───────────────────────────────────────────────────

    const state = {
        cards: [],
        cardEls: [],
        currentIndex: 0,
        wakeLock: null,
        overlay: null,
        touchStartY: 0,
        touchCurrentY: 0,
        isSwiping: false,
        scrolling: false,
        canScrollCard: false,
        startScrollTop: 0,
        wheelTimeout: null,
        triggerElement: null
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

    function captureStepHTML() {
        const stepElements = [];
        const sectionEls = document.querySelectorAll('.md\\:col-span-2 ol');
        sectionEls.forEach(function(ol) {
            ol.querySelectorAll(':scope > li').forEach(function(li) {
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

        data.sections.forEach(function(section, sectionIndex) {
            const sectionName = section.name || (data.sections.length > 1 ? 'Main' : null);

            if (section.ingredients.length > 0 || sectionName) {
                cards.push({
                    type: 'section',
                    sectionIndex: sectionIndex,
                    name: sectionName || data.name,
                    ingredients: section.ingredients
                });
            }

            section.steps.forEach(function(step) {
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

        cards.push({
            type: 'done',
            sectionIndex: data.sections.length - 1,
            name: data.name
        });

        return cards;
    }

    // ─── Rendering ────────────────────────────────────────────────

    function escapeHTML(str) {
        const div = document.createElement('div');
        div.textContent = str;
        return div.innerHTML;
    }

    function renderCard(card) {
        const div = document.createElement('div');
        div.className = 'cooking-card hidden-card';

        if (card.type === 'section') {
            div.classList.add('cooking-card-section');
            let ingredientsHTML = '';
            if (card.ingredients.length > 0) {
                const items = card.ingredients.map(function(ing) {
                    const qty = [ing.quantity, ing.unit].filter(Boolean).join(' ');
                    const note = ing.note ? '<span class="cooking-mise-note">(' + escapeHTML(ing.note) + ')</span>' : '';
                    return '<div class="cooking-mise-item">' +
                        '<span class="cooking-mise-name">' + escapeHTML(ing.name) + ' ' + note + '</span>' +
                        (qty ? '<span class="cooking-mise-qty">' + escapeHTML(qty) + '</span>' : '') +
                        '</div>';
                }).join('');
                ingredientsHTML = '<div class="cooking-mise-grid">' + items + '</div>';
            }
            div.innerHTML = '<h2>' + escapeHTML(card.name) + '</h2>' + ingredientsHTML;
            div.querySelectorAll('.cooking-mise-item').forEach(function(item) {
                item.addEventListener('click', function() {
                    item.classList.toggle('checked');
                });
            });
        }
        else if (card.type === 'step') {
            div.classList.add('cooking-card-step');
            const imageHTML = card.image
                ? '<img class="cooking-step-image" src="' + escapeHTML(card.image) + '" alt="Step ' + card.number + '" />'
                : '';
            let ingredientsHTML = '';
            if (card.ingredients.length > 0) {
                ingredientsHTML = '<div class="cooking-step-ingredients">' +
                    card.ingredients.map(function(ing) {
                        const qty = [ing.quantity, ing.unit].filter(Boolean).join(' ');
                        const note = ing.note ? ' (' + escapeHTML(ing.note) + ')' : '';
                        return '<span>' + escapeHTML(ing.name) + (qty ? ': ' + escapeHTML(qty) : '') + note + '</span>';
                    }).join('') + '</div>';
            }
            div.innerHTML =
                imageHTML +
                '<div class="cooking-step-text"><span class="cooking-step-number">' + card.number + '</span>' + card.html + '</div>' +
                ingredientsHTML;
        }
        else if (card.type === 'done') {
            div.classList.add('cooking-card-done');
            div.innerHTML = '<h2>Bon Appetit!</h2>' +
                '<button class="cooking-done-close-btn">Close</button>';
            div.querySelector('.cooking-done-close-btn').addEventListener('click', closeCookingMode);
        }

        return div;
    }

    function renderOverlay(data, cards) {
        const overlay = document.createElement('div');
        overlay.className = 'cooking-overlay';
        overlay.id = 'cooking-overlay';
        overlay.setAttribute('role', 'dialog');
        overlay.setAttribute('aria-modal', 'true');
        overlay.setAttribute('aria-label', 'Cooking mode: ' + data.name);

        // ARIA live region for step announcements
        const liveRegion = document.createElement('div');
        liveRegion.setAttribute('aria-live', 'polite');
        liveRegion.setAttribute('aria-atomic', 'true');
        liveRegion.className = 'sr-only';
        liveRegion.id = 'cooking-status';
        overlay.appendChild(liveRegion);

        // Header
        const header = document.createElement('div');
        header.className = 'cooking-header';

        const title = document.createElement('div');
        title.className = 'cooking-header-title';
        title.textContent = data.name;

        const sectionsNav = document.createElement('div');
        sectionsNav.className = 'cooking-header-sections';
        sectionsNav.id = 'cooking-sections-nav';

        data.sections.forEach(function(section, i) {
            const pill = document.createElement('button');
            pill.className = 'cooking-section-pill';
            pill.textContent = section.name || (data.sections.length > 1 ? 'Main' : 'Steps');
            pill.dataset.sectionIndex = i;
            pill.addEventListener('click', function() { navigateToSection(i); });
            sectionsNav.appendChild(pill);
        });

        const closeBtn = document.createElement('button');
        closeBtn.className = 'cooking-close-btn';
        closeBtn.setAttribute('aria-label', 'Close cooking mode');
        closeBtn.innerHTML = '&times;';
        closeBtn.addEventListener('click', closeCookingMode);

        header.appendChild(title);
        if (data.sections.length > 1 || (data.sections[0] && data.sections[0].name)) {
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

        cards.forEach(function(card) {
            container.appendChild(renderCard(card));
        });

        // Click on prev/next card to navigate
        container.addEventListener('click', function(e) {
            const card = e.target.closest('.cooking-card');
            if (!card) return;
            if (card.classList.contains('prev')) {
                navigateTo(state.currentIndex - 1);
            } else if (card.classList.contains('next')) {
                navigateTo(state.currentIndex + 1);
            }
        });

        carousel.appendChild(container);

        // Progress bar
        const progress = document.createElement('div');
        progress.className = 'cooking-progress';
        progress.innerHTML = '<div class="cooking-progress-bar" id="cooking-progress-bar"></div>';

        overlay.appendChild(header);
        overlay.appendChild(carousel);
        overlay.appendChild(progress);

        // Cache card element references
        state.cardEls = Array.from(container.querySelectorAll('.cooking-card'));

        return overlay;
    }

    // ─── Navigation ───────────────────────────────────────────────

    function updateCards() {
        const cardEls = state.cardEls;
        if (cardEls.length === 0) return;

        // Only update cards that need changing: previous current +-1 and new current +-1
        const indicesToUpdate = new Set();
        for (let i = 0; i < cardEls.length; i++) {
            const el = cardEls[i];
            if (el.classList.contains('current') || el.classList.contains('prev') || el.classList.contains('next')) {
                indicesToUpdate.add(i);
            }
        }
        indicesToUpdate.add(state.currentIndex);
        if (state.currentIndex > 0) indicesToUpdate.add(state.currentIndex - 1);
        if (state.currentIndex < cardEls.length - 1) indicesToUpdate.add(state.currentIndex + 1);

        indicesToUpdate.forEach(function(i) {
            const el = cardEls[i];
            el.classList.remove('current', 'prev', 'next', 'hidden-card', 'swiping');
            el.style.transform = '';
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
            for (let j = 0; j < pills.length; j++) {
                const isActive = parseInt(pills[j].dataset.sectionIndex) === currentCard.sectionIndex;
                pills[j].classList.toggle('active', isActive);
            }
        }

        // Update ARIA live region
        updateAriaStatus();
    }

    function updateAriaStatus() {
        const statusEl = document.getElementById('cooking-status');
        if (!statusEl) return;

        const currentCard = state.cards[state.currentIndex];
        if (!currentCard) return;

        let message;
        if (currentCard.type === 'section') {
            message = 'Ingredients for ' + currentCard.name;
        } else if (currentCard.type === 'step') {
            message = 'Step ' + currentCard.number + ' of ' + state.cards.filter(function(c) { return c.type === 'step'; }).length;
        } else if (currentCard.type === 'done') {
            message = 'Recipe complete. Bon Appetit!';
        }

        statusEl.textContent = message;
    }

    function navigateTo(index) {
        if (index < 0 || index >= state.cards.length) return;
        state.currentIndex = index;
        updateCards();
    }

    function navigateToSection(sectionIndex) {
        for (let i = 0; i < state.cards.length; i++) {
            if (state.cards[i].sectionIndex === sectionIndex) {
                navigateTo(i);
                return;
            }
        }
    }

    // ─── Touch Handling ───────────────────────────────────────────

    function getCurrentCardEl() {
        return state.cardEls[state.currentIndex] || null;
    }

    function isAtScrollTop(el) {
        return el.scrollTop <= 0;
    }

    function isAtScrollBottom(el) {
        return el.scrollTop + el.clientHeight >= el.scrollHeight - 1;
    }

    function cardCanScroll(el) {
        return el.scrollHeight > el.clientHeight + 1;
    }

    function onTouchStart(e) {
        if (e.touches.length !== 1) return;
        state.touchStartY = e.touches[0].clientY;
        state.touchCurrentY = state.touchStartY;
        state.isSwiping = false;
        state.scrolling = false;

        const currentEl = getCurrentCardEl();
        if (currentEl) {
            state.startScrollTop = currentEl.scrollTop;
            state.canScrollCard = cardCanScroll(currentEl);
        } else {
            state.canScrollCard = false;
        }
    }

    function onTouchMove(e) {
        if (e.touches.length !== 1) return;
        state.touchCurrentY = e.touches[0].clientY;
        const delta = state.touchCurrentY - state.touchStartY;
        const currentEl = getCurrentCardEl();
        if (!currentEl) return;

        // If card has scrollable content, let it scroll until at boundary
        if (state.canScrollCard && !state.isSwiping) {
            const swipingUp = delta < 0;
            const swipingDown = delta > 0;
            const atTop = isAtScrollTop(currentEl);
            const atBottom = isAtScrollBottom(currentEl);

            // Allow native scroll if not at boundary, or scrolling into content
            if ((swipingUp && !atBottom) || (swipingDown && !atTop)) {
                state.scrolling = true;
                return; // let browser handle scroll
            }

            // At boundary and swiping away from content — start card swipe
            if (!state.scrolling) {
                state.isSwiping = true;
                state.touchStartY = state.touchCurrentY; // reset start to boundary
            } else {
                // Was scrolling, now at boundary — wait for a new gesture
                return;
            }
        } else {
            state.isSwiping = true;
        }

        if (state.isSwiping) {
            const swipeDelta = state.touchCurrentY - state.touchStartY;
            const clampedDelta = Math.max(-SWIPE_CLAMP_MAX, Math.min(SWIPE_CLAMP_MAX, swipeDelta));
            currentEl.classList.add('swiping');
            currentEl.style.transform = 'translateY(' + clampedDelta + 'px) scale(' + (1 - Math.abs(clampedDelta) * SWIPE_SCALE_FACTOR) + ')';
        }
    }

    function onTouchEnd() {
        try {
            const currentEl = getCurrentCardEl();

            if (state.isSwiping) {
                const delta = state.touchCurrentY - state.touchStartY;

                if (currentEl) {
                    currentEl.classList.remove('swiping');
                    currentEl.style.transform = '';
                }

                if (delta < -SWIPE_THRESHOLD && state.currentIndex < state.cards.length - 1) {
                    navigateTo(state.currentIndex + 1);
                } else if (delta > SWIPE_THRESHOLD && state.currentIndex > 0) {
                    navigateTo(state.currentIndex - 1);
                } else {
                    updateCards();
                }
            }
        } finally {
            state.isSwiping = false;
            state.scrolling = false;
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

    // ─── Wheel Handling ─────────────────────────────────────────────

    function onWheel(e) {
        if (!state.overlay) return;
        e.preventDefault();
        if (state.wheelTimeout) return; // debounce
        if (e.deltaY > 0) {
            navigateTo(state.currentIndex + 1);
        } else if (e.deltaY < 0) {
            navigateTo(state.currentIndex - 1);
        }
        state.wheelTimeout = setTimeout(function() { state.wheelTimeout = null; }, WHEEL_DEBOUNCE_MS);
    }

    // ─── Wake Lock ────────────────────────────────────────────────

    function acquireWakeLock() {
        if (!('wakeLock' in navigator)) return;
        navigator.wakeLock.request('screen').then(function(lock) {
            state.wakeLock = lock;
            lock.addEventListener('release', function() {
                state.wakeLock = null;
            });
        }).catch(function(err) {
            console.warn('Wake lock request failed:', err);
        });
    }

    function releaseWakeLock() {
        if (state.wakeLock) {
            state.wakeLock.release().catch(function() {});
            state.wakeLock = null;
        }
    }

    function onVisibilityChange() {
        if (state.overlay && document.visibilityState === 'visible' && !state.wakeLock) {
            acquireWakeLock();
        }
    }

    function onBeforeUnload() {
        releaseWakeLock();
    }

    // ─── Open / Close ─────────────────────────────────────────────

    function startCookingMode() {
        const data = loadRecipeData();
        if (!data) return;

        // Save trigger element for focus restoration
        state.triggerElement = document.activeElement;

        const stepHTMLs = captureStepHTML();
        state.cards = buildCards(data, stepHTMLs);
        if (state.cards.length === 0) return;

        state.currentIndex = 0;

        state.overlay = renderOverlay(data, state.cards);
        document.body.appendChild(state.overlay);
        document.body.style.overflow = 'hidden';

        updateCards();

        // Add event listeners
        const carousel = document.getElementById('cooking-carousel');
        carousel.addEventListener('touchstart', onTouchStart, { passive: true });
        carousel.addEventListener('touchmove', onTouchMove, { passive: true });
        carousel.addEventListener('touchend', onTouchEnd, { passive: true });
        carousel.addEventListener('wheel', onWheel, { passive: false });

        document.addEventListener('keydown', onKeyDown);
        window.addEventListener('beforeunload', onBeforeUnload);

        acquireWakeLock();
        document.addEventListener('visibilitychange', onVisibilityChange);

        // Move focus to overlay for accessibility
        state.overlay.setAttribute('tabindex', '-1');
        state.overlay.focus();
    }

    function closeCookingMode() {
        if (!state.overlay) return;

        // Remove carousel event listeners
        const carousel = document.getElementById('cooking-carousel');
        if (carousel) {
            carousel.removeEventListener('touchstart', onTouchStart);
            carousel.removeEventListener('touchmove', onTouchMove);
            carousel.removeEventListener('touchend', onTouchEnd);
            carousel.removeEventListener('wheel', onWheel);
        }

        state.overlay.remove();
        state.overlay = null;
        document.body.style.overflow = '';

        document.removeEventListener('keydown', onKeyDown);
        window.removeEventListener('beforeunload', onBeforeUnload);

        releaseWakeLock();
        document.removeEventListener('visibilitychange', onVisibilityChange);

        // Clear wheel debounce timeout
        if (state.wheelTimeout) {
            clearTimeout(state.wheelTimeout);
            state.wheelTimeout = null;
        }

        // Restore focus to trigger element
        if (state.triggerElement && state.triggerElement.focus) {
            state.triggerElement.focus();
            state.triggerElement = null;
        }

        state.cards = [];
        state.cardEls = [];
        state.currentIndex = 0;
    }

    // ─── Global API ───────────────────────────────────────────────

    window.startCookingMode = startCookingMode;
    window.closeCookingMode = closeCookingMode;

})();
