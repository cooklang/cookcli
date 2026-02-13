// Cooking Mode for CookCLI
(function() {
    'use strict';

    let state = {
        cards: [],
        currentIndex: 0,
        wakeLock: null,
        overlay: null,
        touchStartY: 0,
        touchCurrentY: 0,
        isSwiping: false
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
        var cards = [];
        var stepHTMLIndex = 0;

        data.sections.forEach(function(section, sectionIndex) {
            var sectionName = section.name || (data.sections.length > 1 ? 'Main' : null);

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
        var div = document.createElement('div');
        div.textContent = str;
        return div.innerHTML;
    }

    function renderCard(card) {
        var div = document.createElement('div');
        div.className = 'cooking-card hidden-card';

        if (card.type === 'section') {
            div.classList.add('cooking-card-section');
            var ingredientsHTML = '';
            if (card.ingredients.length > 0) {
                var items = card.ingredients.map(function(ing) {
                    var qty = [ing.quantity, ing.unit].filter(Boolean).join(' ');
                    var note = ing.note ? '<span class="cooking-mise-note">(' + escapeHTML(ing.note) + ')</span>' : '';
                    return '<div class="cooking-mise-item">' +
                        '<span class="cooking-mise-name">' + escapeHTML(ing.name) + ' ' + note + '</span>' +
                        (qty ? '<span class="cooking-mise-qty">' + escapeHTML(qty) + '</span>' : '') +
                        '</div>';
                }).join('');
                ingredientsHTML = '<div class="cooking-mise-grid">' + items + '</div>';
            }
            div.innerHTML = '<h2>' + escapeHTML(card.name) + '</h2>' + ingredientsHTML;
        }
        else if (card.type === 'step') {
            div.classList.add('cooking-card-step');
            var imageHTML = card.image
                ? '<img class="cooking-step-image" src="' + escapeHTML(card.image) + '" alt="Step ' + card.number + '" />'
                : '';
            var ingredientsHTML = '';
            if (card.ingredients.length > 0) {
                ingredientsHTML = '<div class="cooking-step-ingredients">' +
                    card.ingredients.map(function(ing) {
                        var qty = [ing.quantity, ing.unit].filter(Boolean).join(' ');
                        var note = ing.note ? ' (' + escapeHTML(ing.note) + ')' : '';
                        return '<span>' + escapeHTML(ing.name) + (qty ? ': ' + escapeHTML(qty) : '') + note + '</span>';
                    }).join('') + '</div>';
            }
            div.innerHTML =
                '<div class="cooking-step-number">' + card.number + '</div>' +
                imageHTML +
                '<div class="cooking-step-text">' + card.html + '</div>' +
                ingredientsHTML;
        }
        else if (card.type === 'done') {
            div.classList.add('cooking-card-done');
            div.innerHTML = '<h2>Bon Appetit!</h2><p>' + escapeHTML(card.name) + '</p>';
        }

        return div;
    }

    function renderOverlay(data, cards) {
        var overlay = document.createElement('div');
        overlay.className = 'cooking-overlay';
        overlay.id = 'cooking-overlay';

        // Header
        var header = document.createElement('div');
        header.className = 'cooking-header';

        var title = document.createElement('div');
        title.className = 'cooking-header-title';
        title.textContent = data.name;

        var sectionsNav = document.createElement('div');
        sectionsNav.className = 'cooking-header-sections';
        sectionsNav.id = 'cooking-sections-nav';

        data.sections.forEach(function(section, i) {
            var pill = document.createElement('button');
            pill.className = 'cooking-section-pill';
            pill.textContent = section.name || (data.sections.length > 1 ? 'Main' : 'Steps');
            pill.dataset.sectionIndex = i;
            pill.addEventListener('click', function() { navigateToSection(i); });
            sectionsNav.appendChild(pill);
        });

        var closeBtn = document.createElement('button');
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
        var carousel = document.createElement('div');
        carousel.className = 'cooking-carousel';
        carousel.id = 'cooking-carousel';

        var container = document.createElement('div');
        container.className = 'cooking-card-container';
        container.id = 'cooking-card-container';

        cards.forEach(function(card) {
            container.appendChild(renderCard(card));
        });

        carousel.appendChild(container);

        // Progress bar
        var progress = document.createElement('div');
        progress.className = 'cooking-progress';
        progress.innerHTML = '<div class="cooking-progress-bar" id="cooking-progress-bar"></div>';

        overlay.appendChild(header);
        overlay.appendChild(carousel);
        overlay.appendChild(progress);

        return overlay;
    }

    // ─── Navigation ───────────────────────────────────────────────

    function updateCards() {
        var container = document.getElementById('cooking-card-container');
        if (!container) return;
        var cardEls = container.querySelectorAll('.cooking-card');

        for (var i = 0; i < cardEls.length; i++) {
            var el = cardEls[i];
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
        }

        // Update progress bar
        var progressBar = document.getElementById('cooking-progress-bar');
        if (progressBar) {
            var pct = state.cards.length > 1
                ? (state.currentIndex / (state.cards.length - 1)) * 100
                : 100;
            progressBar.style.width = pct + '%';
        }

        // Update section pills
        var currentCard = state.cards[state.currentIndex];
        if (currentCard) {
            var pills = document.querySelectorAll('.cooking-section-pill');
            for (var j = 0; j < pills.length; j++) {
                var isActive = parseInt(pills[j].dataset.sectionIndex) === currentCard.sectionIndex;
                pills[j].classList.toggle('active', isActive);
            }
        }
    }

    function navigateTo(index) {
        if (index < 0 || index >= state.cards.length) return;
        state.currentIndex = index;
        updateCards();
    }

    function navigateToSection(sectionIndex) {
        var cardIndex = -1;
        for (var i = 0; i < state.cards.length; i++) {
            if (state.cards[i].sectionIndex === sectionIndex) {
                cardIndex = i;
                break;
            }
        }
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

        var container = document.getElementById('cooking-card-container');
        if (!container) return;
        var currentEl = container.querySelectorAll('.cooking-card')[state.currentIndex];
        if (currentEl) currentEl.classList.add('swiping');
    }

    function onTouchMove(e) {
        if (!state.isSwiping || e.touches.length !== 1) return;
        state.touchCurrentY = e.touches[0].clientY;
        var delta = state.touchCurrentY - state.touchStartY;

        var container = document.getElementById('cooking-card-container');
        if (!container) return;
        var currentEl = container.querySelectorAll('.cooking-card')[state.currentIndex];

        if (currentEl) {
            var clampedDelta = Math.max(-150, Math.min(150, delta));
            currentEl.style.transform = 'translateY(' + clampedDelta + 'px) scale(' + (1 - Math.abs(clampedDelta) * 0.001) + ')';
        }
    }

    function onTouchEnd() {
        if (!state.isSwiping) return;
        state.isSwiping = false;

        var delta = state.touchCurrentY - state.touchStartY;
        var threshold = 50;

        var container = document.getElementById('cooking-card-container');
        if (container) {
            var currentEl = container.querySelectorAll('.cooking-card')[state.currentIndex];
            if (currentEl) {
                currentEl.classList.remove('swiping');
                currentEl.style.transform = '';
            }
        }

        if (delta < -threshold && state.currentIndex < state.cards.length - 1) {
            navigateTo(state.currentIndex + 1);
        } else if (delta > threshold && state.currentIndex > 0) {
            navigateTo(state.currentIndex - 1);
        } else {
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

    // ─── Open / Close ─────────────────────────────────────────────

    function startCookingMode() {
        var data = loadRecipeData();
        if (!data) return;

        var stepHTMLs = captureStepHTML();
        state.cards = buildCards(data, stepHTMLs);
        if (state.cards.length === 0) return;

        state.currentIndex = 0;

        state.overlay = renderOverlay(data, state.cards);
        document.body.appendChild(state.overlay);
        document.body.style.overflow = 'hidden';

        updateCards();

        var carousel = document.getElementById('cooking-carousel');
        carousel.addEventListener('touchstart', onTouchStart, { passive: true });
        carousel.addEventListener('touchmove', onTouchMove, { passive: true });
        carousel.addEventListener('touchend', onTouchEnd, { passive: true });

        document.addEventListener('keydown', onKeyDown);

        acquireWakeLock();
        document.addEventListener('visibilitychange', onVisibilityChange);
    }

    function closeCookingMode() {
        if (!state.overlay) return;

        state.overlay.remove();
        state.overlay = null;
        document.body.style.overflow = '';

        document.removeEventListener('keydown', onKeyDown);

        releaseWakeLock();
        document.removeEventListener('visibilitychange', onVisibilityChange);

        state.cards = [];
        state.currentIndex = 0;
    }

    // ─── Global API ───────────────────────────────────────────────

    window.startCookingMode = startCookingMode;
    window.closeCookingMode = closeCookingMode;

})();
