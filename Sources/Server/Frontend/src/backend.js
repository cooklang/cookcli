const HOST = "http://localhost:9080";

export const fetchFileTree = (async () => {
    const response = await fetch(`${HOST}/api/v1/file_tree`);
    return await response.json();
})


export const fetchRecipe = (async (path) => {
    const response = await fetch(`${HOST}/api/v1/recipe/${path}`);
    return await response.json();
})


export const fetchShoppingList = (async (recipes) => {
    const response = await fetch(`${HOST}/api/v1/shopping-list`,{
        method: "POST",
        headers: {
            "Accept": "application/json",
            "Content-Type": "application/json"
        },
        body: JSON.stringify(recipes)
    });

    return await response.json();
})
