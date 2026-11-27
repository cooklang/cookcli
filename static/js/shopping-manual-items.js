// The shopping list's add field: type "Milk 2 l" or paste a list, edit an
// item from "Other items", and see new rows highlighted. Loaded by
// templates/shopping_list.html, which calls `CookManualItems.init(config)`
// once and `CookManualItems.afterRender()` after every render of the list.
//
// config: {
//   prefix,                      // URL prefix of the server
//   strings: { add, save, remove, restore, itemChanged },
//   refresh(),                   // the page's requestRefresh('list'), a promise
//   showError(message), hideError(),
// }
(function () {
    'use strict';

    const HIGHLIGHT_MS = 2000;
    const RECOGNIZE_DELAY_MS = 150;
    // The text field look of the pantry page; `.search-input` leaves room
    // for a search icon.
    const INPUT_CLASS = 'flex-1 min-w-0 h-10 pointer-coarse:h-11 px-3 rounded-[var(--radius-control)] border border-line-strong '
        + 'bg-surface text-text focus:outline-none focus:border-accent';

    let config = null;
    // `{ name, quantity, text }` of the item the field is editing, or null;
    // `text` is what the field was filled with.
    let editing = null;
    // Lowercase row name -> time (ms) its highlight ends.
    const highlights = new Map();
    // Lowercase row name to scroll to on the next render.
    let pendingScroll = null;

    function el(id) {
        return document.getElementById(id);
    }

    async function post(path, body) {
        const response = await fetch(config.prefix + '/api/shopping_list/' + path, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(body),
        });
        const data = await response.json().catch(() => ({}));
        return { response, data };
    }

    function chipText(entry) {
        if (!entry) return '';
        return [entry.chip, entry.category].filter(Boolean).join(' · ');
    }

    function setChip(chip, entry) {
        const text = chipText(entry);
        chip.textContent = text;
        chip.title = text;
        chip.classList.toggle('hidden', text === '');
    }

    // Ask the server how `input`'s line reads, 150 ms after the last change,
    // and show it in `chip`. An answer for older text is dropped.
    function recognizeInto(input, chip) {
        const state = input._recognize || (input._recognize = { timer: null, seq: 0 });
        clearTimeout(state.timer);
        const seq = ++state.seq;
        const line = input.value;
        if (line.trim() === '') {
            setChip(chip, null);
            return;
        }
        state.timer = setTimeout(async () => {
            let entry = null;
            try {
                const { response, data } = await post('recognize', { lines: [line] });
                if (response.ok && Array.isArray(data)) entry = data[0];
            } catch (_) {
                // No chip; adding still works.
            }
            if (seq === state.seq) setChip(chip, entry);
        }, RECOGNIZE_DELAY_MS);
    }

    function updateSubmit() {
        el('manual-add-submit').disabled = el('manual-add-input').value.trim() === '';
    }

    function flash(rows) {
        const until = Date.now() + HIGHLIGHT_MS;
        rows.filter(Boolean).forEach(row => highlights.set(row.toLowerCase(), until));
        if (rows[0]) pendingScroll = rows[0].toLowerCase();
    }

    async function addLines(lines) {
        let result;
        try {
            result = await post('add_items', { lines });
        } catch (error) {
            config.showError('Failed to add items');
            return false;
        }
        const { response, data } = result;
        if (!response.ok) {
            config.showError(data.error || 'Failed to add items');
            return false;
        }
        config.hideError();
        flash((data.added || []).map(item => item.row));
        await config.refresh();
        return true;
    }

    async function saveEdit(line) {
        let result;
        try {
            result = await post('edit_extra_item', { name: editing.name, quantity: editing.quantity, line });
        } catch (error) {
            config.showError('Failed to save the item');
            return false;
        }
        const { response, data } = result;
        if (response.status === 404) {
            config.showError(config.strings.itemChanged);
            await config.refresh();
            return true;
        }
        if (!response.ok) {
            config.showError(data.error || 'Failed to save the item');
            return false;
        }
        config.hideError();
        flash([data.row]);
        await config.refresh();
        return true;
    }

    function clearField() {
        const input = el('manual-add-input');
        input.value = '';
        if (input._recognize) {
            clearTimeout(input._recognize.timer);
            input._recognize.seq++;
        }
        setChip(el('manual-add-chip'), null);
        updateSubmit();
    }

    function stopEditing() {
        editing = null;
        el('manual-add-submit').textContent = config.strings.add;
        el('manual-add-cancel').classList.add('hidden');
        clearField();
    }

    function startEditing(item) {
        closePaste();
        const text = item.editText || item.name;
        editing = { name: item.name, quantity: item.quantity, text };
        const input = el('manual-add-input');
        input.value = text;
        el('manual-add-submit').textContent = config.strings.save;
        el('manual-add-cancel').classList.remove('hidden');
        updateSubmit();
        recognizeInto(input, el('manual-add-chip'));
        input.focus();
        input.select();
    }

    async function submit(event) {
        event.preventDefault();
        const input = el('manual-add-input');
        const line = input.value.trim();
        if (line === '') return;
        el('manual-add-submit').disabled = true;
        const ok = editing ? await saveEdit(line) : await addLines([line]);
        if (ok) {
            if (editing) stopEditing();
            else clearField();
        } else {
            updateSubmit();
        }
        input.focus();
    }

    // Whether leaving the page would lose something typed: text in the field
    // (changed, when editing) or a paste preview not yet added.
    function holdsUnsaved() {
        const text = el('manual-add-input').value.trim();
        if (editing) return text !== editing.text.trim();
        return text !== '' || !el('manual-paste').classList.contains('hidden');
    }

    // -- Paste preview ------------------------------------------------------

    function pastedLines(text) {
        return text.split(/\r\n|\r|\n|\u2028|\u2029/).filter(line => line.trim() !== '');
    }

    function updatePasteAdd() {
        const kept = el('manual-paste-rows').querySelectorAll('li:not([data-removed])');
        el('manual-paste-add').disabled = kept.length === 0;
    }

    function pasteRow(entry) {
        const li = document.createElement('li');
        li.className = 'flex items-center gap-2';

        const toggle = document.createElement('button');
        toggle.type = 'button';
        toggle.className = 'icon-btn shrink-0';
        toggle.textContent = '×';
        toggle.setAttribute('aria-label', config.strings.remove);

        const input = document.createElement('input');
        input.type = 'text';
        input.className = INPUT_CLASS;
        input.value = entry.edit_text;
        input.maxLength = 500;
        input.setAttribute('aria-label', entry.edit_text);

        const chip = document.createElement('span');
        chip.className = 'tag shrink-0 max-w-[40%] truncate';
        setChip(chip, entry);

        input.addEventListener('input', () => recognizeInto(input, chip));
        toggle.addEventListener('click', () => {
            const removed = li.toggleAttribute('data-removed');
            input.disabled = removed;
            input.classList.toggle('line-through', removed);
            input.classList.toggle('text-faint', removed);
            toggle.textContent = removed ? '↺' : '×';
            toggle.setAttribute('aria-label', removed ? config.strings.restore : config.strings.remove);
            updatePasteAdd();
        });

        li.append(toggle, input, chip);
        return li;
    }

    function closePaste() {
        el('manual-paste').classList.add('hidden');
        el('manual-paste-rows').replaceChildren();
    }

    async function openPaste(text) {
        let entries = [];
        try {
            const { response, data } = await post('recognize', { text });
            if (response.ok && Array.isArray(data)) entries = data;
        } catch (_) {
            // Fall through to the field below.
        }
        if (entries.length === 0) {
            const input = el('manual-add-input');
            input.value = text.replace(/\s+/g, ' ').trim();
            recognizeInto(input, el('manual-add-chip'));
            updateSubmit();
            return;
        }
        el('manual-paste-rows').replaceChildren(...entries.map(pasteRow));
        el('manual-paste').classList.remove('hidden');
        updatePasteAdd();
    }

    async function addPasted() {
        const lines = Array.from(el('manual-paste-rows').querySelectorAll('li:not([data-removed]) input'))
            .map(input => input.value.trim())
            .filter(line => line !== '');
        if (lines.length === 0) return;
        el('manual-paste-add').disabled = true;
        if (await addLines(lines)) {
            closePaste();
            clearField();
        } else {
            updatePasteAdd();
        }
    }

    function onPaste(event) {
        if (editing) return;
        const data = event.clipboardData || window.clipboardData;
        const text = data ? data.getData('text') : '';
        if (pastedLines(text).length < 2) return;
        event.preventDefault();
        openPaste(text);
    }

    // -- After each render of the list --------------------------------------

    function afterRender() {
        const now = Date.now();
        let scrollTarget = null;
        document.querySelectorAll('#list-content [data-ingredient-name]').forEach(box => {
            const key = box.dataset.ingredientName.toLowerCase();
            const until = highlights.get(key);
            const row = box.closest('li');
            if (!row || !until) return;
            if (until <= now) {
                highlights.delete(key);
                return;
            }
            row.classList.add('row-flashable', 'row-flash');
            setTimeout(() => row.classList.remove('row-flash'), until - now);
            if (key === pendingScroll) scrollTarget = row;
        });
        if (scrollTarget) {
            const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
            scrollTarget.scrollIntoView({ block: 'nearest', behavior: reduce ? 'auto' : 'smooth' });
            pendingScroll = null;
        }
    }

    function init(options) {
        if (!el('manual-add-form')) return null;
        config = options;
        const input = el('manual-add-input');
        el('manual-add-form').addEventListener('submit', submit);
        input.addEventListener('input', () => {
            updateSubmit();
            recognizeInto(input, el('manual-add-chip'));
        });
        input.addEventListener('paste', onPaste);
        input.addEventListener('keydown', event => {
            if (event.key !== 'Escape') return;
            if (editing) stopEditing();
            else if (!el('manual-paste').classList.contains('hidden')) closePaste();
        });
        el('manual-add-cancel').addEventListener('click', () => {
            stopEditing();
            input.focus();
        });
        el('manual-paste-cancel').addEventListener('click', () => {
            closePaste();
            input.focus();
        });
        el('manual-paste-add').addEventListener('click', addPasted);
        if (window.CookUnsaved) window.CookUnsaved.watch(holdsUnsaved);
        updateSubmit();
        return api;
    }

    const api = { init, afterRender, edit: startEditing };
    window.CookManualItems = api;
})();
