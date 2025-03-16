<script>
    import {ListGroup, ListGroupItem} from "sveltestrap";
    import {formatGroupedQuantity} from "./quantity";

    import {showIngredientNotes} from "./store.js"

    export let ingredients;

    function sorted(a, b) {
        return  a.name.localeCompare(b.name);
    }


</script>

<ListGroup>
{#each ingredients.sort(sorted) as ingredient}
    <ListGroupItem class="list-group-item d-flex justify-content-between align-items-center border-0">
        <div>
            {ingredient.name}
            {#if $showIngredientNotes && ingredient.note}
                <br><small><i>{ingredient.note}</i></small>
            {/if}
        </div>
        <span class="text-muted">{formatGroupedQuantity(ingredient.quantities)}</span>
    </ListGroupItem>
{/each}
</ListGroup>
