import { writable } from 'svelte/store';

export const fileTree = writable();
export const localMenu = writable({});

export function convertPathsIntoTree(paths) {
    let result = {};
    let level = {result};

    paths.forEach(path => {
        let chunks = path.split('/');

        chunks.reduce((r, name, index) => {
          if(!r[name]) {
            r[name] = { result: {} };
            r.result[name] = {
                type: index + 1 == chunks.length ? "file" : "directory",
                children: r[name].result
            };
        }

        return r[name];
    }, level);
    });

    return result;
}

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

// Preference for showing quantities next to ingredients, default is false
const showQuantitiesNextToIngredientsValue = localStorage.getItem("showQuantitiesNextToIngredients") || "false";
export const showQuantitiesNextToIngredients = writable(JSON.parse(showQuantitiesNextToIngredientsValue));

// Subscribe method to update local storage
showQuantitiesNextToIngredients.subscribe((val) => {
    localStorage.setItem("showQuantitiesNextToIngredients", JSON.stringify(val));
})
