<script>
    import {onMount} from "svelte";
    import {Router, Route, links} from "svelte-navigator";
    import {Navbar, NavbarBrand, Nav, NavItem, NavLink} from "sveltestrap";

    import Recipes from "./Recipes.svelte";
    import Recipe from "./Recipe.svelte";
    import ShoppingList from "./ShoppingList.svelte";

    import {fetchFileTree} from "./backend.js";
    import {fileTree} from "./store.js";

    onMount(async () => {
        let fullTree = await fetchFileTree();

        fileTree.set(fullTree["children"]);
    });
</script>

<div class="viewport" use:links>
    <Router>
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

            <Route path="recipe/*recipePath" let:params>
                <Recipe recipePath={params.recipePath} />
            </Route>

            <Route path="directory/*recipesPath" let:params>
                <Recipes recipesPath={params.recipesPath} />
            </Route>

            <Route path="/">
                <Recipes recipesPath="" />
            </Route>
        </div>
    </Router>
</div>

<style>
    .viewport {
        width: 800px;
        margin: 50px auto;
    }
</style>
