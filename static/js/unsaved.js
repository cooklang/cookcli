// Asks before the page is left -- tab closed, browser Back, a link, a reload --
// while something is not saved yet. What counts:
//
// - a change on its way: any fetch other than GET or HEAD, until it answers;
// - an element with `data-dirty`, which a page sets while it holds what it has
//   not sent yet (the Text tabs, a list of names waiting for Save);
// - an element with `data-unsaved` (a form, an inline editor, one field) that
//   is on screen with fields changed since it was first focused or clicked. A
//   form submitted, or an editor removed or hidden, no longer counts;
// - whatever a page adds with `CookUnsaved.watch(() => boolean)`.
//
// A page that moves on by itself once its change is saved goes through
// `CookUnsaved.leave(url)` or `CookUnsaved.reload()`, so the browser does not
// ask as well. A form posted the ordinary way is let through.
(function () {
    const checks = new Set();
    // Values of a `data-unsaved` element's fields when it was first touched.
    const touched = new WeakMap();
    let writes = 0;
    let leaving = false;

    const send = window.fetch;
    window.fetch = function (resource, options) {
        const method = String((options && options.method)
            || (resource instanceof Request ? resource.method : 'GET')).toUpperCase();
        if (method === 'GET' || method === 'HEAD') return send.apply(this, arguments);
        writes++;
        return send.apply(this, arguments).finally(() => { writes--; });
    };

    const FIELDS = 'input, textarea, select';
    function values(box) {
        const fields = box.matches(FIELDS) ? [box] : [...box.querySelectorAll(FIELDS)];
        return fields.map(field => field.type === 'checkbox' || field.type === 'radio'
            ? String(field.checked)
            : field.value).join('\u0000');
    }

    function touch(event) {
        const box = event.target.closest && event.target.closest('[data-unsaved]');
        if (box && !touched.has(box)) touched.set(box, values(box));
    }
    // Focus comes before typing; a click on a checkbox does not focus it in
    // every browser.
    document.addEventListener('focusin', touch, true);
    document.addEventListener('pointerdown', touch, true);

    // What a form submits is handed over: if that fails the page says so.
    document.addEventListener('submit', event => {
        const box = event.target.closest('[data-unsaved]');
        if (box) touched.set(box, values(box));
    }, true);
    // Not stopped by a script: the browser posts the form and leaves.
    window.addEventListener('submit', event => {
        if (!event.defaultPrevented) leaving = true;
    });

    function unsaved() {
        if (writes > 0 || document.querySelector('[data-dirty]')) return true;
        for (const check of checks) {
            if (check()) return true;
        }
        for (const box of document.querySelectorAll('[data-unsaved]')) {
            const before = touched.get(box);
            if (before === undefined) continue;
            // Closed: whatever it held was saved or given up.
            if (box.getClientRects().length === 0) {
                touched.delete(box);
                continue;
            }
            if (values(box) !== before) return true;
        }
        return false;
    }

    window.addEventListener('beforeunload', event => {
        if (leaving || !unsaved()) return;
        event.preventDefault();
        // Older browsers ask only when this is set; none show the text.
        event.returnValue = '';
    });
    // Back to this page from the browser's cache, after leaving it.
    window.addEventListener('pageshow', event => {
        if (event.persisted) leaving = false;
    });

    window.CookUnsaved = {
        watch(check) {
            checks.add(check);
        },
        unsaved,
        leave(url) {
            leaving = true;
            window.location.href = url;
        },
        reload() {
            leaving = true;
            window.location.reload();
        },
    };
})();
