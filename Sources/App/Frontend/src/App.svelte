<script>
    import { onMount } from 'svelte';
    import Folder from './Folder.svelte';
    import File from './File.svelte';
    import {fetchFileTree, fetchRecipe} from './data.js';
    import { bind } from 'svelte-simple-modal';
    import Modal from 'svelte-simple-modal';
    import Recipe from './Recipe.svelte';

    let modal

    let path = [];
    let selected;
    let fileTree;
    let currentTree;

    function navigate(path) {
        currentTree = fileTree;

        path.forEach((chunk) => {
            currentTree = currentTree[chunk]["children"];
        });
    }

    onMount(async () => {
        fileTree = await fetchFileTree();
        currentTree = fileTree
    });

    function up() {
        path = path.slice(0, -1);

        navigate(path);
    }

    function diveIn(chunk) {
        path = [...path, chunk];

        navigate(path);
    }

    async function showRecipe(path) {
        let recipe = await fetchRecipe(path)
        modal = bind(Recipe, { recipe: recipe })
    }
</script>

<div class="container">
    <Modal show={modal} />

    {#if path.length > 0}
        <div on:click={up}>...</div>
    {/if}

    {#if currentTree}

        {#each Object.entries(currentTree) as [name, file] (name)}
            {#if file.type === 'directory'}
                <div on:click={() => diveIn(name) }>[{name}]</div>
            {/if}

            {#if file.type === 'file'}
                <div on:click={() => showRecipe([...path, name]) }>{name}</div>
            {/if}
        {/each}

    {/if}
</div>

<style>
    .container {
        width: 600px;
        margin: 100px auto;
    }
</style>
