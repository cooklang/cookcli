<script>
    import {Link} from "svelte-navigator";
    import {TabContent, TabPane, ListGroup, ListGroupItem} from "sveltestrap";
    import {fetchRecipe} from "./backend.js";
    import Breadcrumbs from "./Breadcrumbs.svelte";

    export let recipePath;

    // breadcrumbs = ["Breakfasts","Jamie","Easy Pancakes"]
    $: maybeRecipe = fetchRecipe(recipePath);
</script>

<Breadcrumbs path={recipePath} />

{#await maybeRecipe}
    <p>Loading recipe...</p>
{:then recipe}
    <TabContent>

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
