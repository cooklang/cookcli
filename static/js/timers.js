// Recipe timers for CookCLI web interface
//
// Timers can only be started, viewed, and cancelled from inside cooking mode
// (static/js/cooking-mode.js) — the header strip lives in `#cooking-timer-strip`
// (created by cooking-mode.js) and the enlarged popup is mounted inside
// `#cooking-overlay`, so both disappear automatically when cooking mode closes.
// The underlying countdown/persistence/alerts keep running in the background
// regardless of whether cooking mode is open, so a chime/notification still
// fires even if you've stepped away or exited back to the plain recipe view.
// State is persisted to localStorage, scoped per recipe path, so it survives
// reloads and stays isolated between recipes.
(function() {
    'use strict';

    const container = document.querySelector('[data-recipe-path]');
    if (!container) return;

    const STORAGE_KEY = 'cook.timers.' + container.getAttribute('data-recipe-path');

    // key -> { startedAt, duration, display, section, step }
    let timers = loadTimers();
    // key -> 'running' | 'done', in-memory only, used to fire the completion
    // alert exactly once per timer (not persisted, so reloading after a timer
    // already finished doesn't replay the chime/notification).
    const lastStatus = new Map();

    let tickInterval = null;
    let audioCtx = null;
    let notifyPermissionRequested = false;

    // ─── Storage ─────────────────────────────────────────────────

    function loadTimers() {
        try {
            const raw = localStorage.getItem(STORAGE_KEY);
            if (!raw) return new Map();
            return new Map(Object.entries(JSON.parse(raw)));
        } catch (e) {
            return new Map();
        }
    }

    function saveTimers() {
        try {
            if (timers.size === 0) {
                localStorage.removeItem(STORAGE_KEY);
                return;
            }
            const obj = {};
            timers.forEach(function(entry, key) { obj[key] = entry; });
            localStorage.setItem(STORAGE_KEY, JSON.stringify(obj));
        } catch (e) {
            // localStorage unavailable (e.g. Safari private mode) - timers just won't persist.
        }
    }

    // ─── Time helpers ────────────────────────────────────────────

    function remainingSeconds(entry) {
        return entry.duration - (Date.now() - entry.startedAt) / 1000;
    }

    function statusOf(entry) {
        return remainingSeconds(entry) <= 0 ? 'done' : 'running';
    }

    function formatTime(totalSeconds) {
        const s = Math.max(0, Math.round(totalSeconds));
        const m = Math.floor(s / 60);
        const sec = s % 60;
        return m + ':' + (sec < 10 ? '0' : '') + sec;
    }

    function escapeHtml(s) {
        return String(s)
            .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
    }

    // ─── Completion alert (visual handled via CSS classes below) ──

    function getAudioContext() {
        if (!audioCtx) {
            const Ctx = window.AudioContext || window.webkitAudioContext;
            if (!Ctx) return null;
            audioCtx = new Ctx();
        }
        return audioCtx;
    }

    function playChime() {
        try {
            const ctx = getAudioContext();
            if (!ctx) return;
            if (ctx.state === 'suspended') ctx.resume();
            [0, 0.25, 0.5].forEach(function(offset) {
                const osc = ctx.createOscillator();
                const gain = ctx.createGain();
                osc.type = 'sine';
                osc.frequency.value = 880;
                gain.gain.setValueAtTime(0.0001, ctx.currentTime + offset);
                gain.gain.exponentialRampToValueAtTime(0.2, ctx.currentTime + offset + 0.01);
                gain.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + offset + 0.15);
                osc.connect(gain);
                gain.connect(ctx.destination);
                osc.start(ctx.currentTime + offset);
                osc.stop(ctx.currentTime + offset + 0.16);
            });
        } catch (e) {
            // ignore - sound is a nice-to-have, not required for the timer to work.
        }
    }

    function requestNotifyPermissionOnce() {
        if (notifyPermissionRequested) return;
        notifyPermissionRequested = true;
        try {
            if (window.Notification && Notification.permission === 'default') {
                Notification.requestPermission();
            }
        } catch (e) {
            // ignore
        }
    }

    function notifyDone(entry) {
        try {
            if (!window.Notification || Notification.permission !== 'granted') return;
            let body = entry.display;
            if (entry.section) {
                body += ' — ' + entry.section + ', Step ' + entry.step;
            } else if (entry.step) {
                body += ' — Step ' + entry.step;
            }
            new Notification('Timer done', { body: body });
        } catch (e) {
            // ignore
        }
    }

    // ─── DOM rendering ───────────────────────────────────────────
    // Scoped to `.cooking-overlay` throughout: badges in the plain recipe view
    // are left exactly as the server rendered them, never touched by JS.

    function badgesForKey(key) {
        return document.querySelectorAll('.cooking-overlay [data-timer-key="' + key + '"]');
    }

    function updateBadgeDOM(key, entry) {
        const status = statusOf(entry);
        const label = '⏱️ ' + formatTime(remainingSeconds(entry));
        badgesForKey(key).forEach(function(badge) {
            badge.classList.toggle('timer-running', status === 'running');
            badge.classList.toggle('timer-done', status === 'done');
            badge.textContent = label;
        });
    }

    function resetBadgeDOM(key) {
        badgesForKey(key).forEach(function(badge) {
            badge.classList.remove('timer-running', 'timer-done');
            badge.textContent = '⏱️ ' + (badge.getAttribute('data-display') || '');
        });
    }

    function renderStrip() {
        // Only present while cooking mode is open; created by cooking-mode.js.
        const strip = document.getElementById('cooking-timer-strip');
        if (!strip) return;
        if (timers.size === 0) {
            strip.classList.add('hidden');
            strip.classList.remove('flex');
            strip.innerHTML = '';
            return;
        }
        strip.classList.remove('hidden');
        strip.classList.add('flex');
        const html = [];
        timers.forEach(function(entry, key) {
            const status = statusOf(entry);
            const cls = 'timer-chip' + (status === 'done' ? ' timer-done' : '');
            html.push(
                '<button type="button" class="' + cls + '" data-timer-chip="' + key + '" ' +
                'aria-label="' + escapeHtml(entry.display) + '">⏱ ' +
                formatTime(remainingSeconds(entry)) + '</button>'
            );
        });
        strip.innerHTML = html.join('');
    }

    function updateModalIfOpen() {
        const modal = document.getElementById('timer-detail-modal');
        if (!modal || modal.classList.contains('hidden')) return;
        const key = modal.getAttribute('data-active-key');
        const entry = timers.get(key);
        if (!entry) {
            closeModal();
            return;
        }
        const timeEl = modal.querySelector('#timer-modal-time');
        timeEl.textContent = formatTime(remainingSeconds(entry));
        timeEl.classList.toggle('timer-done', statusOf(entry) === 'done');
    }

    // ─── Ticking ─────────────────────────────────────────────────

    function refreshDOM() {
        timers.forEach(function(entry, key) { updateBadgeDOM(key, entry); });
        renderStrip();
        updateModalIfOpen();
    }

    function tick() {
        timers.forEach(function(entry, key) {
            const status = statusOf(entry);
            if (status === 'done' && lastStatus.get(key) === 'running') {
                playChime();
                notifyDone(entry);
            }
            lastStatus.set(key, status);
        });
        refreshDOM();
    }

    function ensureTicking() {
        if (tickInterval) return;
        tickInterval = setInterval(tick, 1000);
    }

    function stopTickingIfIdle() {
        if (timers.size === 0 && tickInterval) {
            clearInterval(tickInterval);
            tickInterval = null;
        }
    }

    // ─── Timer lifecycle ─────────────────────────────────────────

    function startTimer(badge) {
        const key = badge.getAttribute('data-timer-key');
        if (timers.has(key)) {
            openDetail(key);
            return;
        }
        const duration = parseInt(badge.getAttribute('data-seconds'), 10);
        if (!duration || duration <= 0) return;

        requestNotifyPermissionOnce();
        getAudioContext();

        const entry = {
            startedAt: Date.now(),
            duration: duration,
            display: badge.getAttribute('data-display') || '',
            section: badge.getAttribute('data-section') || '',
            step: badge.getAttribute('data-step') || ''
        };
        timers.set(key, entry);
        lastStatus.set(key, 'running');
        saveTimers();
        updateBadgeDOM(key, entry);
        renderStrip();
        ensureTicking();
    }

    function cancelTimer(key) {
        timers.delete(key);
        lastStatus.delete(key);
        saveTimers();
        resetBadgeDOM(key);
        renderStrip();
        stopTickingIfIdle();
    }

    // ─── Enlarged single-timer view ──────────────────────────────

    function ensureModal() {
        // Mounted inside #cooking-overlay (not document.body) so it's torn down
        // for free when cooking mode closes, and stacks above the overlay's own
        // header/progress bar via the overlay's stacking context.
        const overlay = document.getElementById('cooking-overlay');
        if (!overlay) return null;

        let modal = document.getElementById('timer-detail-modal');
        if (modal) return modal;

        modal = document.createElement('div');
        modal.id = 'timer-detail-modal';
        modal.className = 'fixed inset-0 z-50 hidden items-center justify-center bg-black/50';
        modal.innerHTML =
            '<div class="card shadow-[var(--shadow-overlay)] max-w-sm w-full mx-4 p-6 text-center">' +
                '<div class="timer-modal-time mb-2" id="timer-modal-time"></div>' +
                '<div class="text-muted mb-1" id="timer-modal-context"></div>' +
                '<div class="text-faint text-sm mb-6" id="timer-modal-original"></div>' +
                '<div class="flex justify-center gap-2">' +
                    '<button type="button" class="btn" id="timer-modal-close">Close</button>' +
                    '<button type="button" class="btn btn-danger" id="timer-modal-cancel">Cancel timer</button>' +
                '</div>' +
            '</div>';
        overlay.appendChild(modal);

        modal.addEventListener('click', function(e) {
            if (e.target === modal) closeModal();
        });
        modal.querySelector('#timer-modal-close').addEventListener('click', closeModal);
        modal.querySelector('#timer-modal-cancel').addEventListener('click', function() {
            const key = modal.getAttribute('data-active-key');
            closeModal();
            if (key) cancelTimer(key);
        });

        return modal;
    }

    function closeModal() {
        const modal = document.getElementById('timer-detail-modal');
        if (modal) {
            modal.classList.add('hidden');
            modal.classList.remove('flex');
        }
    }

    function openDetail(key) {
        const entry = timers.get(key);
        if (!entry) return;
        const modal = ensureModal();
        if (!modal) return;
        modal.setAttribute('data-active-key', key);

        let context = entry.section || '';
        if (entry.step) context += (context ? ' — ' : '') + 'Step ' + entry.step;
        modal.querySelector('#timer-modal-context').textContent = context;
        modal.querySelector('#timer-modal-original').textContent = 'Originally ' + entry.display;

        modal.classList.remove('hidden');
        modal.classList.add('flex');
        updateModalIfOpen();
    }

    // ─── Event delegation ────────────────────────────────────────
    // Delegated on document (not bound to individual badges) so this also
    // works for the badges cooking-mode.js clones via innerHTML. Starting/
    // reopening a timer only works for badges inside cooking mode's overlay —
    // the same badges in the plain recipe view are inert.

    document.addEventListener('click', function(e) {
        const chip = e.target.closest('[data-timer-chip]');
        if (chip) {
            openDetail(chip.getAttribute('data-timer-chip'));
            return;
        }
        const badge = e.target.closest('[data-timer]');
        if (badge && badge.closest('.cooking-overlay')) {
            startTimer(badge);
        }
    });

    document.addEventListener('keydown', function(e) {
        if (e.key === 'Escape') {
            // Close the timer popup first without also closing cooking mode.
            // This listener is registered at page load, before cooking-mode.js
            // adds its own Escape handler on entering cooking mode, so it runs
            // first and can stop the event from reaching that handler.
            const modal = document.getElementById('timer-detail-modal');
            if (modal && !modal.classList.contains('hidden')) {
                closeModal();
                e.stopImmediatePropagation();
            }
            return;
        }
        if (e.key !== 'Enter' && e.key !== ' ') return;
        const badge = e.target.closest && e.target.closest('[data-timer]');
        if (badge && badge.closest('.cooking-overlay')) {
            e.preventDefault();
            startTimer(badge);
        }
    });

    // Keep timers in sync across multiple tabs open to the same recipe.
    window.addEventListener('storage', function(e) {
        if (e.key !== STORAGE_KEY) return;
        timers = loadTimers();
        timers.forEach(function(entry, key) { lastStatus.set(key, statusOf(entry)); });
        refreshDOM();
        if (timers.size > 0) {
            ensureTicking();
        } else {
            stopTickingIfIdle();
        }
    });

    // cooking-mode.js dispatches this right after building its overlay/header,
    // so a still-running timer shows up immediately instead of waiting for the
    // next 1s tick (the header strip and card badges are freshly (re)created
    // each time cooking mode opens, since cooking-mode.js rebuilds its overlay
    // from a fresh snapshot of the plain recipe view every time).
    document.addEventListener('cookingmode:opened', refreshDOM);

    // ─── Init ────────────────────────────────────────────────────

    timers.forEach(function(entry, key) { lastStatus.set(key, statusOf(entry)); });
    if (timers.size > 0) ensureTicking();
})();
