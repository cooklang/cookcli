<script>
    import { localMenu } from "./store.js";
    import { onMount, setContext} from "svelte";
    import { writable } from "svelte/store";

    let calandarMonth = new Date();
    let selectedDate = new Date();
    let formattedSelectedDate = "";
    let selectedRecipe = "";
    let recipes = ["Gathering recipes..."];

    setContext('localMenu', localMenu);

    function formatedDate() {
        const format = {weekday: 'long', year: 'numeric', month: 'long', day: 'numeric'};

        return selectedDate.toLocaleDateString('en-UK', format);
    }

    function setSelectedDate() {
        return new Date(
            calandarMonth.getFullYear(), 
            calandarMonth.getMonth(),
            calandarMonth.getDate()
        );
    }

    function closeOverlay() {
        document.querySelector('.overlay').style.display = 'none';
    }
    
    async function getAllRecipes() {
        let database = [];
        const response = await fetch('/api/recipes');
        const data = await response.json();
        data.forEach((path) => {
            const dir = path.split('/');
            const name = dir[dir.length - 1];
            if (!database.includes(name)) {
                database.push(name);
            }
        });
        return database;
    }

    function addToMenu() {
        if (!localMenu[selectedDate]) {
            localMenu[selectedDate] = [];
        }
        // We don't want to add the same recipe twice
        if (!localMenu[selectedDate].includes(selectedRecipe)) {
            localMenu[selectedDate] = [...localMenu[selectedDate], selectedRecipe];
        }
    }

    function removeFromMenu(event){
        const recipe = event.target.innerText;
        localMenu[selectedDate] = localMenu[selectedDate].filter((r) => r !== recipe);
    }

    $: localMenu = writable(localMenu);
    $: formattedSelectedDate = formatedDate();
    $: selectedDate = setSelectedDate();

    onMount(async () => {
        recipes = await getAllRecipes();
    });
</script>

<div class="overlay" >
    <div class="content">
        <button id="close" on:click={closeOverlay}>Close</button>
        <fieldset>
            <legend>
                {formattedSelectedDate}
            </legend>
            <input 
                list="recipe" 
                type="search" 
                placeholder="search" 
                bind:value={selectedRecipe}
            />
            <datalist id="recipe" role="combobox" >
            {#each recipes as recipe}
                <option value={recipe}>{recipe}</option>
            {/each}
            </datalist>
            <button on:click={addToMenu}>Add</button>
            <hr>
            <div>
                <ul>
                {#each localMenu[selectedDate] as recipe}
                        <li on:click={removeFromMenu}>
                            {recipe}
                        </li>
                {/each}
                </ul>
            </div>
        </fieldset>
    </div>
</div>

<style>

.calandar .overlay .content {
    background-color: white;
    /* width: 80vw; */
    margin: 2em;
    padding: 2em;
    border-radius: 3px;
    cursor: default;
}

.calandar .overlay .content button {
    background-color: none;
    border: none;
    cursor: pointer;
    size: 3em;

}

fieldset {
  border-radius: 5px;
}

legend {
  color: var(--accent-1l); 
  font-size: 1.8em;
}

input {
  font-size: 1em;
  width: 100%;
  font-size: 1em;
  padding: 5px;
  margin: 5px;
  border: 1px solid var(--black);
  outline: none;
  border-radius: 5px;
}

fieldset button {
  font-size: 1em;
  padding: 5px;
  border: none;
  margin: 5px;
  border-radius: 5px;
  cursor: pointer;
  background-color: var(--accent-1l);
  transition: var(--fade-out);
}

fieldset button:hover {
    background-color: var(--accent-1d);
    transition: var(--fade-in);
}

fieldset ul {
  list-style-type: none;
  padding: 0;
  margin: 0;
}

li:after {
    pointer-events: none;
}

li:before {
    content: "🗑️";
    color: var(--accent-1l);
    display: inline-block;
    width: 1.5em;
    margin: 0.25em;
    cursor: pointer;
    
}

</style>

