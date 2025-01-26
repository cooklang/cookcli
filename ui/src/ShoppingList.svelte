<script>
    import {getContext, onMount} from "svelte";
    import {fetchShoppingList} from "./backend.js";
    import {ListGroup, ListGroupItem, TabContent, TabPane, Button} from "sveltestrap";
    import Ingredients from "./Ingredients.svelte";

    import {shoppingListPaths} from "./store.js";

    $: maybeShoppingList = fetchShoppingList($shoppingListPaths);

    function onDelete(path) {
        shoppingListPaths.remove(path)
    }
</script>

<TabContent>
    <TabPane tabId="aisle" tab="Aisle" active>
    {#await maybeShoppingList}
        <div class="mt-5 mx-auto" style="width: 250px;">Loading shopping list...</div>
    {:then shoppingList}
        {#each shoppingList.categories as {category, items}}
            <h5 class="pt-4">{category}</h5>
            <Ingredients ingredients={items} />
        {/each}
        {#if shoppingList.categories.length === 0}
            <div class="mt-5 mx-auto" style="width: 250px;">
              Nothing added to a shopping list.
            </div>
        {/if}
    {/await}
    </TabPane>
    <TabPane tabId="recipe" tab="Recipe">
        <ListGroup>
        {#each $shoppingListPaths as path}
            <ListGroupItem class="list-group-item d-flex justify-content-between align-items-center border-0">
                {path}
                <Button outline dark on:click={() => onDelete(path)} size="sm">Delete</Button>
            </ListGroupItem>
        {:else}
            <div class="mt-5 mx-auto" style="width: 250px;">
              No recipes added.
            </div>
        {/each}
    </ListGroup>
    </TabPane>
</TabContent>
