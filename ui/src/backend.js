export const fetchRecipes = (async () => {
    const response = await fetch(`/api/recipes`);
    return await response.json();
})


export const fetchRecipe = (async (path, scale) => {
    const response = await fetch(`/api/recipes/${path}?scale=${scale}`);
    const json = await response.json();
    return json.recipe;
})


export const fetchShoppingList = (async (recipes) => {
    const response = await fetch(`/api/shopping_list`,{
        method: "POST",
        headers: {
            "Accept": "application/json",
            "Content-Type": "application/json"
        },
        body: JSON.stringify(recipes)
    });

    return await response.json();
})
