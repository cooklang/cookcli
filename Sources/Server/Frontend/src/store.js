import { writable } from 'svelte/store';

export const fileTree = writable();


function createShoppingListPaths() {
    const storedShoppingList = JSON.parse(localStorage.getItem("shoppingList") || "[]");

    const { subscribe, set, update } = writable(storedShoppingList);

    subscribe(value => {
        localStorage.setItem("shoppingList", JSON.stringify(value));
    });

    return {
        subscribe,
        add: (item) => update(prev => [...prev, item]),
        remove: (item) => update(prev => prev.filter((i) => i == item)),
        reset: () => set([])
    };
}

export const shoppingListPaths = createShoppingListPaths();
