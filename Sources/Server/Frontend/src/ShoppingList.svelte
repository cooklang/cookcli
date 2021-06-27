<script>
    import {getContext, onMount} from "svelte";
    import {fetchShoppingList} from "./backend.js";
    import {ListGroup, ListGroupItem} from "sveltestrap";

    import {shoppingListPaths} from "./store.js";

    $: maybeShoppingList = fetchShoppingList($shoppingListPaths);
</script>


{#await maybeShoppingList}
    <p>Loading shopping list...</p>
{:then shoppingList}
    <ListGroup>
    {#each shoppingList as item }
        <ListGroupItem>{item.name} {item.amount}</ListGroupItem>
    {:else}
    <p>Nothing added to a shopping list.</p>
    {/each}
    </ListGroup>
{/await}
