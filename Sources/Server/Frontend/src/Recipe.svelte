<script>
    import {Link} from "svelte-navigator";
    import {TabContent, TabPane, ListGroup, ListGroupItem, Button, Toast, Col, Container, Row} from "sveltestrap";
    import {fetchRecipe} from "./backend.js";
    import Breadcrumbs from "./Breadcrumbs.svelte";
    import Ingredients from "./Ingredients.svelte";

    import {shoppingListPaths} from "./store.js";

    export let recipePath;

    // breadcrumbs = ["Breakfasts","Jamie","Easy Pancakes"]
    $: maybeRecipe = fetchRecipe(recipePath);

    let isAddedToShoppingListToastOpen = false;
    let buttonDisabled = false;

    async function onAddToShoppingList() {
        buttonDisabled = true;
        await new Promise(r => setTimeout(r, 500));
        shoppingListPaths.add(recipePath);
        isAddedToShoppingListToastOpen = true
        buttonDisabled = false;
    }
</script>

<Container>
  <Row>
    <Col><Breadcrumbs path={recipePath} /></Col>
    <Col class="text-end"><Button disabled={buttonDisabled} color="warning" outline on:click={onAddToShoppingList}>Add to shopping list</Button></Col>
  </Row>
</Container>


{#await maybeRecipe}
    <div class="mt-5 mx-auto" style="width: 250px;">Loading recipe...</div>
{:then recipe}
    <TabContent>

        {#if recipe.ingredients.length > 0 }
        <TabPane tabId="ingredients" tab="Ingredients" active>
            <Ingredients ingredients={recipe.ingredients} />
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
