<script>
  export let recipe;
</script>

{#if recipe}

  {#if recipe.ingredients}

    <h3>Ingredients</h3>
    {#each recipe.ingredients as ingredient}
      <div>{ingredient.name}, {ingredient.amount}</div>
    {/each}

  {/if}

  {#if recipe.cookware}

    <h3>Cookware</h3>
    {#each recipe.cookware as cookware}
      <div>{cookware.name}</div>
    {/each}

  {/if}

  {#if recipe.steps}

    <h3>Steps</h3>
    {#each recipe.steps as step, index}
      <div>{step.description}</div>
    {/each}

  {/if}
{/if}
