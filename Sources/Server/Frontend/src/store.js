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
        remove: (item) => update(prev => {
            let i = prev.indexOf(item);

            prev.splice(i, 1);

            return [...prev];
        }),
        reset: () => set([])
    };
}

export const shoppingListPaths = createShoppingListPaths();
