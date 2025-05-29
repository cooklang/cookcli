<script>
    import {Input} from "sveltestrap";
    import {onMount} from "svelte";
    import {useNavigate} from "svelte-navigator";

    let searchQuery = "";
    let searchResults = [];
    let showSearchResults = false;
    const navigate = useNavigate();

    async function handleSearch() {
        if (searchQuery.length < 2) {
            searchResults = [];
            showSearchResults = false;
            return;
        }

        const response = await fetch(`/api/search?q=${encodeURIComponent(searchQuery)}`);
        const data = await response.json();
        searchResults = data;
        showSearchResults = true;
    }

    function closeSearchResults() {
        showSearchResults = false;
    }

    async function handleResultClick(path) {
        searchQuery = "";
        searchResults = [];
        showSearchResults = false;
        navigate(`/recipe/${path.replace(/\.cook$/, '')}`);
    }
</script>

<div class="d-flex align-items-center mx-3 position-relative">
    <Input
        type="search"
        placeholder="Search recipes..."
        class="form-control"
        bind:value={searchQuery}
        on:input={handleSearch}
    />
    {#if showSearchResults && searchResults.length > 0}
        <div class="search-results position-absolute top-100 start-0 mt-1 bg-white border rounded shadow-sm" style="width: 100%; z-index: 1000;">
            {#each searchResults as result}
                <button
                    class="d-block w-100 text-start p-2 text-decoration-none text-dark hover-bg-light border-0 bg-transparent"
                    on:click={() => handleResultClick(result.path)}
                >
                    {result.name}
                </button>
            {/each}
        </div>
    {/if}
</div>

<style>
    .search-results button:hover {
        background-color: #f8f9fa;
    }

    .search-results {
        max-height: 300px;
        overflow-y: auto;
    }
</style>
