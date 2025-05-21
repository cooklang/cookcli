<script>
    import {Link} from "svelte-navigator";
    import {ListGroup, ListGroupItem} from "sveltestrap";

    import Breadcrumbs from "./Breadcrumbs.svelte";
    import DirectoryIcon from "./DirectoryIcon.svelte";

    import {fileTree} from "./store.js";

    export let recipesPath;

    function getFilePath(prepath, name) {
        return `/recipe/${prepath}/${name}`;
    }

    function getDirPath(prepath, name) {
        return `/directory/${prepath}/${name}`;
    }

    function calculateCurrentSubtree(path, fullTree) {
        console.log('Calculating subtree for path:', path);
        console.log('Full tree:', fullTree);

        if (!fullTree) {
            console.log('No full tree available');
            return null;
        }

        if (path === "") {
            console.log('Root path, returning children:', fullTree.children);
            return fullTree.children;
        }

        let currentTree = fullTree;
        const pathParts = path.split("/");
        console.log('Path parts:', pathParts);

        for (const chunk of pathParts) {
            if (!currentTree.children || !currentTree.children[chunk]) {
                console.log('Path not found:', chunk);
                return null;
            }
            currentTree = currentTree.children[chunk];
        }

        console.log('Final current tree:', currentTree);
        return currentTree.children;
    }

    function sorted(a, b) {
        return a[1].name.localeCompare(b[1].name);
    }

    $: {
        console.log('RecipesPath changed:', recipesPath);
        console.log('FileTree:', $fileTree);
    }

    $: currentTree = calculateCurrentSubtree(recipesPath, $fileTree);
    $: console.log('Current tree:', currentTree);

</script>

<ListGroup>

    <Breadcrumbs path={recipesPath} />

    {#if currentTree}
        {#each Object.entries(currentTree).sort(sorted) as [_, item] (item.name)}
            {#if Object.keys(item.children).length > 0}
            <ListGroupItem>
                <DirectoryIcon /> <Link to={getDirPath(recipesPath, item.name)}>{item.name}</Link>
            </ListGroupItem>
            {/if}
        {/each}

        {#each Object.entries(currentTree).sort(sorted) as [_, item] (item.name)}
            {#if Object.keys(item.children).length === 0}
            <ListGroupItem>
                {#if item.recipe?.metadata?.map?.image}
                    <img height="42px" alt={item.name} src={"/" + item.path} />
                {/if}
                <Link to={getFilePath(recipesPath, item.name)}>
                    {item.name}
                </Link>
            </ListGroupItem>
            {/if}
        {/each}
    {:else}
        <ListGroupItem>Loading...</ListGroupItem>
    {/if}

</ListGroup>
