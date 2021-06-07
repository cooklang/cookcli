<script>
    import Folder from './Folder.svelte';
    import File from './File.svelte';
    import {fileTree, recipe} from './data.js';
    import { bind } from 'svelte-simple-modal';
    import Modal from 'svelte-simple-modal';
    import Recipe from './Recipe.svelte';

    let modal

    let path = [];
    let selected;
    let currentTree = fileTree;

    function navigate(path) {
        currentTree = fileTree;

        path.forEach((chunk) => {
            currentTree = currentTree[chunk]["children"];
        });
    }

    function up() {
        path = path.slice(0, -1);

        navigate(path);
    }

    function diveIn(chunk) {
        path = [...path, chunk];

        navigate(path);
    }

    function showRecipe(path) {
        modal = bind(Recipe, { recipe: recipe })
    }
</script>

<div class="container">
    <Modal show={modal} />

    {#if path.length > 0}
        <div on:click={up}>...</div>
    {/if}

    {#each Object.entries(currentTree) as [name, file] (name)}
        {#if file.type === 'directory'}
            <div on:click={() => diveIn(name) }>[{name}]</div>
        {/if}

        {#if file.type === 'file'}
            <div on:click={() => showRecipe([...path, file]) }>{name}</div>
        {/if}
    {/each}
</div>

<style>
    .container {
        width: 600px;
        margin: 100px auto;
    }
</style>
