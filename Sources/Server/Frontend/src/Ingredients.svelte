<script>
    import {ListGroup, ListGroupItem} from "sveltestrap";

    export let ingredients;

    function sorted(a, b) {
        return  a.name.localeCompare(b.name);
    }
</script>

<ListGroup>
{#each ingredients.sort(sorted) as ingredient}
    <ListGroupItem class="list-group-item d-flex justify-content-between align-items-center border-0">
        {ingredient.name}
        <span class="text-muted">{ingredient.amount}</span>
    </ListGroupItem>
{/each}
</ListGroup>
