
<script>
  import MenuOverlay from './MenuOverlay.svelte';

    import { localMenu } from "./store.js";
    import { onMount, setContext} from "svelte";
    import { writable } from "svelte/store";

    const days = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
    let calandarMonth = new Date();
    let dates = getMonthDates();
    let selectedDate = new Date();

    setContext('localMenu', localMenu);

    function monthDateToEnglish(month){
        const monthNames = ["January", "February", "March", "April", "May", "June",
            "July", "August", "September", "October", "November", "December"];
        return monthNames[month];
    }

    function formatedDate() {
        const format = {weekday: 'long', year: 'numeric', month: 'long', day: 'numeric'};

        return selectedDate.toLocaleDateString('en-UK', format);
    }

    function getMonthDates() {
        const year = calandarMonth.getFullYear();
        const month = calandarMonth.getMonth();
        const firstDay = new Date(year, month, 1);
        const weekday = firstDay.getDay();
        let dates = [];
        const daysInMonth = new Date(year, month + 1, 0).getDate(); 
        for (let i = 0; i < weekday; i++) {
            dates.push(null);
        }
        for (let i = 1; i <= daysInMonth; i++) {
            dates.push(new Date(year, month, i));
        }
        while (dates.length < 35) {
            dates.push(null);
        }
        return dates;
    }

    function setSelectedDate() {
        return new Date(
            calandarMonth.getFullYear(), 
            calandarMonth.getMonth(),
            calandarMonth.getDate()
        );
    }

    function setMonth(month) {
        calandarMonth.setMonth(month);
        calandarMonth = new Date(calandarMonth);
        dates = getMonthDates();
    }

    function nextMonth() {
        setMonth(calandarMonth.getMonth() + 1);
    }
    function previousMonth() {
        setMonth(calandarMonth.getMonth() - 1);
    }

    function handleDateClick(event) {
        selectedDate = new Date(
            calandarMonth.getFullYear(), 
            calandarMonth.getMonth(),
            event.target.id
        );
        formattedSelectedDate = formatedDate();
        if (!localMenu[selectedDate]) {
            localMenu[selectedDate] = [];
        }
        openOveraly();
    }

    function openOveraly() {
        document.querySelector('.overlay').style.display = 'flex';
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

    $: localMenu = writable(localMenu);
    $: selectedDate = setSelectedDate();

    onMount(async () => {
        recipes = await getAllRecipes();
    });

</script>

<div class="calandar">
    <div class="header">
        <h1>{monthDateToEnglish(calandarMonth.getMonth()).toLocaleUpperCase()} {calandarMonth.getFullYear()}</h1>
    </div>
    <div class="header">
        <button on:click={() => previousMonth()}> &#10999; </button>
        <button on:click={() => setMonth(new Date().getMonth())}> &#11044; </button>
        <button on:click={() => nextMonth()}> &#11000; </button>
    </div>
    <div class="grid">
        {#each days as day}
            <div id="day">{day}</div>
        {/each}
    </div>
    <div class="grid">
        {#each dates as date}
            {#if date == null}
                <div class="date"></div>
            {:else}
                <div on:click={handleDateClick} class="date" id={date.getDate()}>
                    {date.getDate()}
                    {#if localMenu[date]}
                    {#each localMenu[date] as recipe}
                        <div class="item" on:click={handleDateClick} id={date.getDate()}>{recipe}</div>
                    {/each} 
                    {/if}
                </div>
            {/if}
        {/each}
    </div>
    <!-- THE OVERLAY TO BUILD A MENU FOR A DAY -->
    <MenuOverlay />
</div>

<style>

.calandar {
    --bg-light: rgba(248, 249, 250, 1);
    --black: rgba(0, 0, 0, 1);
    --accent-1l: rgba(228, 105, 60, 1);
    --accent-1d: rgb(212, 93, 50);
    --accent-2: rgba(255, 193, 7, 1);
    --accent-3: lightblue;

    --fade-in: ease-in 0.4s;
    --fade-out: ease-out 0.2s;

    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
}

.calandar .header {
    width: 100%;
    background-color: var(--bg-light);
    display: flex;
    flex-direction: row;
    justify-content: center;
    letter-spacing: 0.4vw;
}

.calandar .header button {
    background-color: rgba(0, 0, 0, 0);
    transition: var(--fade-out);
    padding: 5px;
    margin: 5px;
    width: 100%;
    border: 0px;
}

.calandar .header button:hover {
    background-color: rgba(0, 0, 0, 0.2);
    border: 0px;
    transition: var(--fade-in);
} 

.calandar .grid {
    width: 100%;
    display: flex;
    flex-direction: row;
    align-items: left;
    flex-wrap: wrap;
    justify-content: space-around;
}

.calandar .grid #day {
    background-color: var(--accent-1l);
    text-align: center;
    display: flex;
    justify-content: center;
    align-items: center;
    flex-basis: calc(100% / 7);
}

.calandar .grid .date {
    background-color: var(--bg-light);
    cursor: pointer;
    border: 2px solid rgba(0, 0, 0, 0.35);
    flex-basis: calc(100% / 7);
    min-height: calc(100vw / 7);
    min-width: calc(1vw / 7);
    transition: var(--fade-out);
}

.calandar .grid .date:hover {
    background-color: var(--accent-2);
    transition: var(--fade-in);
}

.calandar .grid .date .item {
    background-color: var(--accent-3);
    cursor: default;
    border-radius: 3px;
    font-size: 0.75em;
    margin-bottom: 0.1em;
    padding: 0.1em;
    align-items: center;
    justify-content: center;

}

</style>

