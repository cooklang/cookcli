<script>
    import {onMount} from "svelte";
    import {Router, Route, links} from "svelte-navigator";
    import {Navbar, NavbarBrand, Nav, NavItem, NavLink} from "sveltestrap";
    import {setup} from 'svelte-match-media';

    import Recipes from "./Recipes.svelte";
    import Recipe from "./Recipe.svelte";
    import Logo from "./Logo.svelte";
    import ShoppingList from "./ShoppingList.svelte";
    import Preferences from "./Preferences.svelte";
    import Search from "./Search.svelte";

    import {fetchRecipes} from "./backend.js";
    import {fileTree, convertPathsIntoTree} from "./store.js";

    onMount(async () => {
        let response = await fetchRecipes();
        fileTree.set(convertPathsIntoTree(response));
    });

    setup({
      print: 'print',
      screen: 'screen'
    });
</script>

<div class="viewport" use:links>
    <Router>
        <Navbar color="light" light expand="md">
            <NavbarBrand href="/"><Logo /> Cook</NavbarBrand>
            <Search />
            <Nav navbar>
                <!-- TODO select active links -->
                <NavItem>
                    <NavLink href="/">Recipes</NavLink>
                </NavItem>

                <NavItem>
                    <NavLink href="/shopping-list">Shopping list</NavLink>
                </NavItem>

                <NavItem>
                    <NavLink href="/preferences">Preferences</NavLink>
                </NavItem>
            </Nav>
        </Navbar>

        <div class="py-3">
            <Route path="shopping-list" component="{ShoppingList}" />

            <Route path="preferences" component="{Preferences}" />

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
        width: 100%;
        max-width: 800px;
        margin: 50px auto;
    }

    .search-results a:hover {
        background-color: #f8f9fa;
    }

    .search-results {
        max-height: 300px;
        overflow-y: auto;
    }
</style>
