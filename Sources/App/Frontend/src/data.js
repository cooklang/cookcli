import { writable } from 'svelte/store';

export const modal = writable(null);


export const fileTree = {
    "Healthy Recipes 2": {
        type: "directory",
        children: {
            "Risotto": { type: "file" },
        }
    },
    "Breakfasts": {
        type: "directory",
        children: {
            "Jamie": {
                type: "directory",
                children: {
                    "Mexican Style Burrito": { type: "file" },
                    "Two chesees omelette": { type: "file" },
                }
            },
            "Irish Breakfast": { type: "file", image: "/path/to/image.png", metadata: {} },
            "Shakshuka": { type: "file" },
            "Oats": { type: "file" }
        }
    },
    "Sicilian style lamb chops 2": { type: "file" }
}

export const recipe = {
    ingredients: [
        { name: "salt", amount: "1 tbsp" }
    ],
    cookware: [
        { name: "oven" }
    ],
    steps: [
        { description: "hello" },
        { description: "hello" },
        { description: "hello" },
        { description: "hello" },
    ]
};
