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
        if (!fullTree) return fullTree;
        if (path == "") return fullTree;

        let currentTree = fullTree;

        path.split("/").forEach((chunk) => {
            currentTree = currentTree[chunk]["children"];
        });

        return currentTree;
    }

    $: currentTree = calculateCurrentSubtree(recipesPath, $fileTree);

</script>

<ListGroup>

    <Breadcrumbs path={recipesPath} />

    {#if currentTree}
    {#each Object.entries(currentTree) as [name, file] (name)}
        {#if file.type === "directory"}
        <ListGroupItem>
            <DirectoryIcon /> <Link to={getDirPath(recipesPath, name)}>{name}</Link>
        </ListGroupItem>
        {/if}
    {/each}

    {#each Object.entries(currentTree) as [name, file] (name)}
        {#if file.type === "file"}
        <ListGroupItem>
            {#if file.image}
                <img height="42px" alt={name} src={"/" + recipesPath + "/" + file.image} />
            {/if}
            <Link to={getFilePath(recipesPath, name)}>
                {name}
            </Link>
        </ListGroupItem>
        {/if}
    {/each}
    {/if}

</ListGroup>
