<script>
    import {Link} from "svelte-navigator";
    import {TabContent, TabPane, ListGroup, ListGroupItem, Button, Toast, Col, Container, Row} from "sveltestrap";
    import {fetchRecipe} from "./backend.js";
    import Breadcrumbs from "./Breadcrumbs.svelte";

    import {shoppingListPaths} from "./store.js";

    export let recipePath;

    // breadcrumbs = ["Breakfasts","Jamie","Easy Pancakes"]
    $: maybeRecipe = fetchRecipe(recipePath);

    let isAddedToShoppingListToastOpen = false;

    function onAddToShoppingList() {
        shoppingListPaths.add(recipePath);
        isAddedToShoppingListToastOpen = true
    }
</script>

<Container>
  <Row>
    <Col><Breadcrumbs path={recipePath} /></Col>
    <Col class="text-end"><Button color="warning" outline on:click={onAddToShoppingList}>Add to shopping list</Button></Col>
  </Row>
</Container>

{#await maybeRecipe}
    <p>Loading recipe...</p>
{:then recipe}
    <TabContent pills vertical>

        {#if recipe.ingredients.length > 0 }
        <TabPane tabId="ingredients" tab="Ingredients" active>
            <ListGroup>
            {#each recipe.ingredients as ingredient}
                <ListGroupItem class="list-group-item d-flex justify-content-between align-items-center border-0">
                    {ingredient.name}
                    <span class="text-muted">{ingredient.amount}</span>
                </ListGroupItem>
            {/each}
            </ListGroup>
        </TabPane>
        {/if}

        {#if recipe.cookware.length > 0 }
        <TabPane tabId="cookware" tab="Cookware">
            <ListGroup>
                {#each recipe.cookware as cookware}
                <ListGroupItem class="list-group-item border-0">{cookware.name}</ListGroupItem>
                {/each}
            </ListGroup>
        </TabPane>
        {/if}

        {#if recipe.steps.length > 0 }
        <TabPane tabId="steps" tab="Steps">
            {#each recipe.steps as step, index}
            <div class="card border-0">
                <div class="card-body">
                    <h6 class="card-title">Step {index + 1}</h6>
                    <p class="card-text">{step.description}</p>
                </div>
            </div>
            {/each}
        </TabPane>
        {/if}

    </TabContent>
{:catch error}
    <p>Something went wrong: {error.message}</p>
{/await}

<div class="position-fixed bottom-0 end-0 p-3" style="z-index: 11">
<Toast
  autohide
  body
  isOpen={isAddedToShoppingListToastOpen}
  on:close={() => isAddedToShoppingListToastOpen = false}>
  Added {recipePath} to a shopping list...
</Toast>
</div>
