<script>
    import {Link} from "svelte-navigator";
    import {Breadcrumb, BreadcrumbItem} from "sveltestrap";

    export let path;

    // breadcrumbs = ["Breakfasts","Jamie","Easy Pancakes"]
    $: breadcrumbs = path.split("/").reduce((acc, curr) => [...acc, curr], []);
</script>


{#if breadcrumbs}
<Breadcrumb>
    <BreadcrumbItem>
        <Link to="/">Home</Link>
    </BreadcrumbItem>

    {#each breadcrumbs.slice(0, -1) as component, index}
    <BreadcrumbItem>
        <Link to={"/directory/" + breadcrumbs.slice(0, index + 1).join("/")}>{component}</Link>
    </BreadcrumbItem>
    {/each}

    <BreadcrumbItem>
       {breadcrumbs[breadcrumbs.length-1]}
    </BreadcrumbItem>
</Breadcrumb>
{/if}
