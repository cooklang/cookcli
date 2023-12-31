<script>
    import {ListGroup, ListGroupItem, TabContent, TabPane, Button} from "sveltestrap";

    import {showQuantitiesNextToIngredients} from "./store.js";
</script>

<TabContent>
    <TabPane tabId="preferences" tab="Preferences" active>
    <!-- add checkbox input to toggle showQuantitiesNextToIngredients store -->
    <label class="my-3" for="show-quantities-next-to-ingredients">
        <input type="checkbox" id="show-quantities-next-to-ingredients" bind:checked={$showQuantitiesNextToIngredients}>
        <span>Show quantities next to ingredients in recipes</span>
    </label>
</TabPane>


</TabContent>
