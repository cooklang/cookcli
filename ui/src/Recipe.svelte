<script>
    import {Link} from "svelte-navigator";
    import {media} from 'svelte-match-media'

    import {TabContent, TabPane, ListGroup, ListGroupItem, Button, Toast, Col, Container, Row} from "sveltestrap";
    import {fetchRecipe} from "./backend.js";
    import Breadcrumbs from "./Breadcrumbs.svelte";
    import Ingredients from "./Ingredients.svelte";
    import Step from "./Step.svelte";

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

    function namedGroupIngredients(grouped, flat) {
        grouped.forEach((ingredient) => {
            ingredient.name = flat[ingredient.index].name;
            ingredient.note = flat[ingredient.index].note;
        });

        return grouped;
    }
</script>

<Container>
  <Row>
    <Col><Breadcrumbs path={recipePath} /></Col>
    {#if $media.screen}
    <Col class="text-end"><Button disabled={buttonDisabled} color="warning" outline on:click={onAddToShoppingList}>Add to shopping list</Button></Col>
    {/if}
  </Row>
</Container>


{#await maybeRecipe}
    <div class="mt-5 mx-auto" style="width: 250px;">Loading recipe...</div>
{:then recipe}
    {#if $media.screen}
    <TabContent>
        {#if recipe.ingredients && recipe.ingredients.length > 0}
        <TabPane tabId="ingredients" tab="Ingredients" active>
            <Ingredients ingredients={namedGroupIngredients(recipe.grouped_ingredients, recipe.ingredients)} />
        </TabPane>
        {/if}

        {#if recipe.cookware && recipe.cookware.length > 0}
        <TabPane tabId="cookware" tab="Cookware">
            <ListGroup>
                {#each recipe.cookware as cookware}
                <ListGroupItem class="list-group-item border-0">{cookware.name}</ListGroupItem>
                {/each}
            </ListGroup>
        </TabPane>
        {/if}

        {#if recipe.sections && recipe.sections.length > 0}
        <TabPane tabId="steps" tab="Steps">
            {#each recipe.sections as section}
                {#each section.content as item}
                    {#if item.type === 'step'}
                    <Step
                        step={item.value}
                        ingredients={recipe.ingredients}
                        cookware={recipe.cookware}
                        timers={recipe.timers}
                    />
                    {/if}
                {/each}
            {/each}
        </TabPane>
        {/if}
    </TabContent>
    {/if}

    {#if $media.print}
    <div class="ingredients-box">
        {#if recipe.ingredients && recipe.ingredients.length > 0}
        <Ingredients ingredients={recipe.ingredients} />
        {/if}

        {#if recipe.cookware && recipe.cookware.length > 0}
        <div class="card border-0">
            <div class="card-body">
                <p class="card-text">{recipe.cookware.map((c) => c.name).join(', ')}</p>
            </div>
        </div>
        {/if}
    </div>

    {#if recipe.sections && recipe.sections.length > 0}
        {#each recipe.sections as section}
            {#each section.content as item}
                {#if item.type === 'step'}
                <div class="card border-0">
                    <div class="card-body">
                        <h6 class="card-title">Step {item.value.number}</h6>
                        <p class="card-text"><Step step={item.value} ingredients={recipe.ingredients} cookware={recipe.cookware} timers={recipe.timers} /></p>
                    </div>
                </div>
                {/if}
            {/each}
        {/each}
    {/if}
    {/if}
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

<style>
    .ingredients-box {
        width: 20rem;
        border: 1px solid black;
        border-radius: 0px;
        padding: 1.5rem 1rem;

        float: left;
        margin-right: 2rem;
        margin-left: 1rem;
    }
</style>
