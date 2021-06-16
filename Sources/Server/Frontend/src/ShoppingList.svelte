<script>
    import { getContext, onMount } from 'svelte';
    import { fetchShoppingList } from './data.js';
    import { ListGroup, ListGroupItem } from 'sveltestrap';

    let shoppingList = []

    onMount(async () => {
        shoppingList = await fetchShoppingList(["Root Vegetable Tray Bake.cook"])
    })
</script>


<ListGroup>
{#each shoppingList as item }
    <ListGroupItem>{item.name} {item.amount}</ListGroupItem>
{/each}
</ListGroup>
