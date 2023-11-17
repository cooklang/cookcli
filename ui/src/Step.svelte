<script>
export let step, ingredients, cookware, timers;
import {formatQuantity} from "./quantity";

function formatText(item) {
    return item.value;
}

function formatIngredient(ingredient) {
    return ingredient.name;
}

function formatCookware(cookware) {
    return cookware.name;
}

function formatTimer(timer) {
    if (!timer.quantity) {
        return timer.name;
    } else {
        return formatQuantity(timer.quantity);
    }
}

function itemsToString(items, ingredients, cookware, timers) {
    return items.map((item) => {
        switch(item.type) {
            case "ingredient":
                return formatIngredient(ingredients[item.index]);
            case "cookware":
                return formatCookware(cookware[item.index]);
            case "text":
                return formatText(item);
            case "timer":
                return formatTimer(timers[item.index]);
            default:
                throw new Error(`Unrecognizable item type ${item.type}`);
            }
    }).join("");
}
</script>

<div class="card border-0">
    <div class="card-body">
        <h6 class="card-title">Step {step.number}</h6>
        <p class="card-text">{itemsToString(step.items, ingredients, cookware, timers)}</p>
    </div>
</div>


