<script>
    import {ListGroup, ListGroupItem} from "sveltestrap";
    import {formatGroupedQuantity} from "./quantity";

    export let ingredients;

    function sorted(a, b) {
        return  a.name.localeCompare(b.name);
    }


</script>

<ListGroup>
{#each ingredients.sort(sorted) as ingredient}
    <ListGroupItem class="list-group-item d-flex justify-content-between align-items-center border-0">
        {ingredient.name}
        <span class="text-muted">{formatGroupedQuantity(ingredient.quantity)}</span>
    </ListGroupItem>
{/each}
</ListGroup>
