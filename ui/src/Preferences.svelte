<script>
    import {ListGroup, ListGroupItem, TabContent, TabPane, Button} from "sveltestrap";

    import {showUnitsNextToIngredients} from "./store.js";
</script>

<TabContent>
    <TabPane tabId="preferences" tab="Preferences" active>
    <!-- add checkbox input to toggle showUnitsNextToIngredients store -->
    <label class="my-3" for="show-units-next-to-ingredients">
        <input type="checkbox" id="show-units-next-to-ingredients" bind:checked={$showUnitsNextToIngredients}>
        <span>Show units next to ingredients in recipes</span>
    </label>
</TabPane>


</TabContent>
