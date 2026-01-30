/**
 * Keyboard shortcuts for Cook CLI web interface
 *
 * Global shortcuts (available on all pages):
 * - /          Focus search
 * - g h        Go to home/recipes
 * - g s        Go to shopping list
 * - g p        Go to pantry
 * - g x        Go to preferences
 * - ?          Show keyboard shortcuts help
 * - Escape     Close modals/dropdowns
 * - t          Toggle theme (dark/light)
 *
 * Recipe page shortcuts:
 * - e          Edit recipe
 * - a          Add to shopping list
 * - p          Print recipe
 * - +/-        Increase/decrease scale
 * - [/]        Decrease/increase scale by 0.5
 */

(function() {
    'use strict';

    // Track pending key sequences (for multi-key shortcuts like "g h")
    let pendingKey = null;
    let pendingTimeout = null;

    // Check if user is typing in an input field
    function isTyping(event) {
        const target = event.target;
        const tagName = target.tagName.toLowerCase();

        // Check for input elements
        if (tagName === 'input' || tagName === 'textarea' || tagName === 'select') {
            return true;
        }

        // Check for contenteditable elements
        if (target.isContentEditable) {
            return true;
        }

        // Check for CodeMirror editor
        if (target.closest('.cm-editor')) {
            return true;
        }

        return false;
    }

    // Clear pending key sequence
    function clearPendingKey() {
        pendingKey = null;
        if (pendingTimeout) {
            clearTimeout(pendingTimeout);
            pendingTimeout = null;
        }
    }

    // Show keyboard shortcuts modal
    window.showShortcutsHelp = function() {
        const existingModal = document.getElementById('keyboard-shortcuts-modal');
        if (existingModal) {
            existingModal.classList.remove('hidden');
            return;
        }

        const modal = document.createElement('div');
        modal.id = 'keyboard-shortcuts-modal';
        modal.className = 'fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50';
        modal.innerHTML = `
            <div class="bg-white dark:bg-gray-800 rounded-2xl shadow-xl max-w-2xl w-full mx-4 max-h-[80vh] overflow-hidden">
                <div class="p-6 border-b border-gray-200 dark:border-gray-700 flex justify-between items-center">
                    <h2 class="text-xl font-bold text-gray-900 dark:text-white">Keyboard Shortcuts</h2>
                    <button onclick="closeShortcutsHelp()" class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">
                        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                        </svg>
                    </button>
                </div>
                <div class="p-6 overflow-y-auto max-h-[60vh]">
                    <div class="grid md:grid-cols-2 gap-6">
                        <div>
                            <h3 class="font-semibold text-gray-900 dark:text-white mb-3">Navigation</h3>
                            <div class="space-y-2">
                                <div class="flex justify-between items-center">
                                    <span class="text-gray-600 dark:text-gray-400">Focus search</span>
                                    <kbd class="px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-sm font-mono">/</kbd>
                                </div>
                                <div class="flex justify-between items-center">
                                    <span class="text-gray-600 dark:text-gray-400">Go to recipes</span>
                                    <span><kbd class="px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-sm font-mono">g</kbd> <kbd class="px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-sm font-mono">h</kbd></span>
                                </div>
                                <div class="flex justify-between items-center">
                                    <span class="text-gray-600 dark:text-gray-400">Go to shopping list</span>
                                    <span><kbd class="px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-sm font-mono">g</kbd> <kbd class="px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-sm font-mono">s</kbd></span>
                                </div>
                                <div class="flex justify-between items-center">
                                    <span class="text-gray-600 dark:text-gray-400">Go to pantry</span>
                                    <span><kbd class="px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-sm font-mono">g</kbd> <kbd class="px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-sm font-mono">p</kbd></span>
                                </div>
                                <div class="flex justify-between items-center">
                                    <span class="text-gray-600 dark:text-gray-400">Go to preferences</span>
                                    <span><kbd class="px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-sm font-mono">g</kbd> <kbd class="px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-sm font-mono">x</kbd></span>
                                </div>
                            </div>
                        </div>
                        <div>
                            <h3 class="font-semibold text-gray-900 dark:text-white mb-3">General</h3>
                            <div class="space-y-2">
                                <div class="flex justify-between items-center">
                                    <span class="text-gray-600 dark:text-gray-400">Toggle theme</span>
                                    <kbd class="px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-sm font-mono">t</kbd>
                                </div>
                                <div class="flex justify-between items-center">
                                    <span class="text-gray-600 dark:text-gray-400">Show shortcuts</span>
                                    <kbd class="px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-sm font-mono">?</kbd>
                                </div>
                                <div class="flex justify-between items-center">
                                    <span class="text-gray-600 dark:text-gray-400">Close modal</span>
                                    <kbd class="px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-sm font-mono">Esc</kbd>
                                </div>
                            </div>
                        </div>
                        <div>
                            <h3 class="font-semibold text-gray-900 dark:text-white mb-3">Recipe Page</h3>
                            <div class="space-y-2">
                                <div class="flex justify-between items-center">
                                    <span class="text-gray-600 dark:text-gray-400">Edit recipe</span>
                                    <kbd class="px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-sm font-mono">e</kbd>
                                </div>
                                <div class="flex justify-between items-center">
                                    <span class="text-gray-600 dark:text-gray-400">Add to shopping list</span>
                                    <kbd class="px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-sm font-mono">a</kbd>
                                </div>
                                <div class="flex justify-between items-center">
                                    <span class="text-gray-600 dark:text-gray-400">Print recipe</span>
                                    <kbd class="px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-sm font-mono">p</kbd>
                                </div>
                                <div class="flex justify-between items-center">
                                    <span class="text-gray-600 dark:text-gray-400">Increase scale</span>
                                    <kbd class="px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-sm font-mono">+</kbd>
                                </div>
                                <div class="flex justify-between items-center">
                                    <span class="text-gray-600 dark:text-gray-400">Decrease scale</span>
                                    <kbd class="px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-sm font-mono">-</kbd>
                                </div>
                            </div>
                        </div>
                        <div>
                            <h3 class="font-semibold text-gray-900 dark:text-white mb-3">Shopping List</h3>
                            <div class="space-y-2">
                                <div class="flex justify-between items-center">
                                    <span class="text-gray-600 dark:text-gray-400">Clear all items</span>
                                    <kbd class="px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-sm font-mono">c</kbd>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
                <div class="p-4 border-t border-gray-200 dark:border-gray-700 text-center text-sm text-gray-500 dark:text-gray-400">
                    Press <kbd class="px-1 py-0.5 bg-gray-100 dark:bg-gray-700 rounded text-xs font-mono">Esc</kbd> to close
                </div>
            </div>
        `;

        document.body.appendChild(modal);

        // Close on backdrop click
        modal.addEventListener('click', function(e) {
            if (e.target === modal) {
                closeShortcutsHelp();
            }
        });
    }

    // Close keyboard shortcuts modal
    window.closeShortcutsHelp = function() {
        const modal = document.getElementById('keyboard-shortcuts-modal');
        if (modal) {
            modal.classList.add('hidden');
        }
    };

    // Handle keyboard events
    function handleKeydown(event) {
        // Don't handle if user is typing
        if (isTyping(event)) {
            // But still allow Escape to blur inputs
            if (event.key === 'Escape') {
                event.target.blur();
            }
            return;
        }

        // Don't handle if modifier keys are pressed (except Shift for ? and +)
        if (event.ctrlKey || event.metaKey || event.altKey) {
            return;
        }

        const key = event.key;

        // Handle pending key sequences (like "g h")
        if (pendingKey === 'g') {
            clearPendingKey();
            switch (key) {
                case 'h':
                case 'r':
                    event.preventDefault();
                    window.location.href = '/';
                    return;
                case 's':
                    event.preventDefault();
                    window.location.href = '/shopping-list';
                    return;
                case 'p':
                    event.preventDefault();
                    window.location.href = '/pantry';
                    return;
                case 'x':
                    event.preventDefault();
                    window.location.href = '/preferences';
                    return;
            }
            // If no valid second key, fall through to handle as new key
        }

        // Global shortcuts
        switch (key) {
            case '/':
                event.preventDefault();
                const searchInput = document.getElementById('search-input');
                if (searchInput) {
                    searchInput.focus();
                    searchInput.select();
                }
                return;

            case 'g':
                event.preventDefault();
                pendingKey = 'g';
                // Clear pending after 1.5 seconds
                pendingTimeout = setTimeout(clearPendingKey, 1500);
                return;

            case '?':
                event.preventDefault();
                showShortcutsHelp();
                return;

            case 'Escape':
                event.preventDefault();
                // Close shortcuts modal if open
                const shortcutsModal = document.getElementById('keyboard-shortcuts-modal');
                if (shortcutsModal && !shortcutsModal.classList.contains('hidden')) {
                    closeShortcutsHelp();
                    return;
                }
                // Close search results if open
                const searchResults = document.getElementById('search-results');
                if (searchResults && !searchResults.classList.contains('hidden')) {
                    searchResults.classList.add('hidden');
                    return;
                }
                // Close any other modals
                const modals = document.querySelectorAll('[data-modal]');
                modals.forEach(modal => modal.classList.add('hidden'));
                return;

            case 't':
                event.preventDefault();
                if (typeof toggleTheme === 'function') {
                    toggleTheme();
                }
                return;
        }

        // Page-specific shortcuts
        const path = window.location.pathname;

        // Recipe page shortcuts
        if (path.startsWith('/recipe/')) {
            handleRecipeShortcuts(event, key);
        }
        // Shopping list page shortcuts
        else if (path === '/shopping-list') {
            handleShoppingListShortcuts(event, key);
        }
    }

    // Recipe page specific shortcuts
    function handleRecipeShortcuts(event, key) {
        switch (key) {
            case 'e':
                event.preventDefault();
                // Find and click the edit link
                const editLink = document.querySelector('a[href^="/edit/"]');
                if (editLink) {
                    editLink.click();
                }
                return;

            case 'a':
                event.preventDefault();
                // Find and click the add to shopping list button
                const addButton = document.querySelector('button[onclick^="addToShoppingList"]');
                if (addButton) {
                    addButton.click();
                }
                return;

            case 'p':
                event.preventDefault();
                window.print();
                return;

            case '+':
            case '=': // = is on the same key as + without shift
                event.preventDefault();
                adjustScale(0.5);
                return;

            case '-':
            case '_':
                event.preventDefault();
                adjustScale(-0.5);
                return;

            case ']':
                event.preventDefault();
                adjustScale(1);
                return;

            case '[':
                event.preventDefault();
                adjustScale(-1);
                return;
        }
    }

    // Adjust recipe scale
    function adjustScale(delta) {
        const scaleInput = document.getElementById('scale');
        if (!scaleInput) return;

        let newValue = parseFloat(scaleInput.value) + delta;
        const min = parseFloat(scaleInput.min) || 0.5;
        const max = parseFloat(scaleInput.max) || 200;

        // Clamp to valid range
        newValue = Math.max(min, Math.min(max, newValue));

        // Round to avoid floating point issues
        newValue = Math.round(newValue * 10) / 10;

        if (newValue !== parseFloat(scaleInput.value)) {
            scaleInput.value = newValue;
            // Trigger the onchange event
            scaleInput.dispatchEvent(new Event('change'));
        }
    }

    // Shopping list page specific shortcuts
    function handleShoppingListShortcuts(event, key) {
        switch (key) {
            case 'c':
                event.preventDefault();
                // Clear the list (if the function exists)
                if (typeof clearList === 'function') {
                    if (confirm('Clear all items from shopping list?')) {
                        clearList();
                    }
                }
                return;
        }
    }

    // Initialize keyboard shortcuts
    document.addEventListener('keydown', handleKeydown);

    // Add visual indicator for keyboard navigation
    document.addEventListener('DOMContentLoaded', function() {
        // Add a small hint in the search placeholder about the shortcut
        const searchInput = document.getElementById('search-input');
        if (searchInput) {
            const currentPlaceholder = searchInput.getAttribute('placeholder');
            if (currentPlaceholder && !currentPlaceholder.includes('/')) {
                // Don't modify placeholder - keep it clean
            }
        }
    });

})();
