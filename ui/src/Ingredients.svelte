<script>
    import {ListGroup, ListGroupItem} from "sveltestrap";
    import {Link} from "svelte-navigator";
    import {formatGroupedQuantity} from "./quantity";

    import {showIngredientNotes} from "./store.js"

    export let ingredients;

    function sorted(a, b) {
        return a.name.localeCompare(b.name);
    }

    function getRecipePath(reference) {
        if (!reference) return null;
        return `/recipe/${reference.components.join('/')}/${reference.name}`;
    }
</script>

<ListGroup>
{#each ingredients.sort(sorted) as ingredient}
    <ListGroupItem class="list-group-item d-flex justify-content-between align-items-center border-0">
        <div>
            {#if ingredient.reference}
                <Link to={getRecipePath(ingredient.reference)}>{ingredient.name}</Link>
            {:else}
                {ingredient.name}
            {/if}
            {#if $showIngredientNotes && ingredient.note}
                <br><small><i>{ingredient.note}</i></small>
            {/if}
        </div>
        <span class="text-muted">{formatGroupedQuantity(ingredient.quantities)}</span>
    </ListGroupItem>
{/each}
</ListGroup>
