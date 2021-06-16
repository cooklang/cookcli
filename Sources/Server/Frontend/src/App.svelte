<script>
    import { onMount, setContext } from 'svelte';
    import { Router, Link, Route, links } from "svelte-routing";

    import {fetchFileTree} from './data.js';

    import Recipes from './Recipes.svelte';
    import Recipe from './Recipe.svelte';
    import ShoppingList from './ShoppingList.svelte';

    import {
        Navbar,
        NavbarBrand,
        Nav,
        NavItem,
        NavLink
    } from 'sveltestrap';

    export let url = "";
</script>

<div class="viewport" use:links>
    <Router url="{url}">
        <Navbar color="light" light expand="md">
            <NavbarBrand href="/">Cook</NavbarBrand>
            <Nav navbar>
                <NavItem>
                    <NavLink href="/">Recipes</NavLink>
                </NavItem>
                <NavItem>
                    <NavLink href="/shopping-list">Shopping list</NavLink>
                </NavItem>
            </Nav>
        </Navbar>

        <div class="py-3">
            <Route path="shopping-list" component="{ShoppingList}" />
            <!-- <Route path=":id" component="{Recipe}" /> -->
            <Route path="/*"><Recipes /></Route>
        </div>
    </Router>
</div>

<style>
.viewport {
    width: 800px;
    margin: 50px auto;
}
</style>
