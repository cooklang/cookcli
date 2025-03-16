<script>
    import {ListGroup, ListGroupItem, TabContent, TabPane, Button} from "sveltestrap";

    import {showQuantitiesNextToIngredients, showIngredientNotes} from "./store.js";
</script>

<TabContent>
    <TabPane tabId="preferences" tab="Preferences" active>
        <!-- add checkbox input to toggle showQuantitiesNextToIngredients store -->
        <label class="my-3" for="show-quantities-next-to-ingredients">
            <input type="checkbox" id="show-quantities-next-to-ingredients" bind:checked={$showQuantitiesNextToIngredients}>
            <span>Show quantities next to ingredients in recipes</span>
        </label>
        <br>
        <label class="my-3" for="show-ingredient-notes">
            <input type="checkbox" id="show-ingredient-notes" bind:checked={$showIngredientNotes}>
            <span>Show notes next to ingredients</span>
        </label>
    </TabPane>


</TabContent>
