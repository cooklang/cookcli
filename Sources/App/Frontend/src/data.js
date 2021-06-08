
export const fetchFileTree = (async () => {
    const response = await fetch('http://localhost:9080/api/v1/file_tree')
    return await response.json()
})


export const fetchRecipe = (async (path) => {
    const response = await fetch(`http://localhost:9080/api/v1/recipe/${path.map(e => encodeURIComponent(e)).join("/")}`)
    return await response.json()
})
