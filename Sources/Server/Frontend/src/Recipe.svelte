<script>
    import { onMount } from 'svelte';
    import { TabContent, TabPane, Breadcrumb, BreadcrumbItem, ListGroup, ListGroupItem } from 'sveltestrap';
    import { fetchRecipe } from './data.js';

    let recipe
    let path
    onMount(async () => {
        path = window.location.pathname.split("/")
        recipe = await fetchRecipe(window.location.pathname)
    })
</script>

{#if recipe}
<Breadcrumb>
    {#each path as component}
    <BreadcrumbItem>
        <a href="">{component}</a>
    </BreadcrumbItem>
    {/each}
</Breadcrumb>

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
{/if}
