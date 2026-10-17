# Shopping List
shopping-title = Shopping List
shopping-empty = Your shopping list is empty
shopping-no-items = No items in shopping list. Add recipes to get started.
shopping-no-recipes = No recipes selected. Add recipes from the recipe page.
shopping-item-count =
    { $count ->
        [one] { $count } item
       *[other] { $count } items
    }
shopping-from-recipe = From { $recipeName }
shopping-clear-confirm = Are you sure you want to clear all items?
shopping-added = Added { $itemName } to shopping list
shopping-removed = Removed { $itemName } from shopping list
shopping-cleared = Shopping list cleared
shopping-export = Export
shopping-import = Import
shopping-mark-done = Mark as done
shopping-mark-undone = Mark as undone
shopping-selected-recipes = Selected Recipes
shopping-clear-all = Clear All
shopping-in-pantry = In Your Pantry
shopping-pantry-unused = The pantry file could not be read, so nothing in your pantry was taken off this list.
shopping-pantry-open = Open the pantry to fix it
shopping-remove = Remove
shopping-add-recipes-first = Please add some recipes first
shopping-failed-to-generate = Failed to generate shopping list
shopping-warnings = Some recipes need a look, so the list may be missing something:
shopping-warning-not-a-number = { $recipe }: the recipe { $reference } could not be scaled to { $amount }, which is not a number. It is on the list as written, not as its ingredients.
shopping-warning-no-servings = { $recipe }: the recipe { $reference } could not be scaled to { $amount }: it has no whole number of servings, such as `servings: 4`. It is on the list as written, not as its ingredients.
shopping-warning-no-yield = { $recipe }: the recipe { $reference } could not be scaled to { $amount }: it does not say how much it makes. Add a yield, written number%unit, such as `yield: 500%{ $unit }`. It is on the list as written, not as its ingredients.
shopping-warning-yield-unit = { $recipe }: the recipe { $reference } could not be scaled to { $amount }: its yield is in { $unit }, so ask for it in { $unit }. It is on the list as written, not as its ingredients.
shopping-warning-non-positive-base = { $recipe }: the recipe { $reference } could not be scaled to { $amount }: it declares `{ $declared }`, so there is nothing to scale from. It is on the list as written, not as its ingredients.
shopping-failed-to-add = Failed to add to shopping list
shopping-error = Error
shopping-print = Print
shopping-include-in-list = Include in shopping list
shopping-optional = (optional)
shopping-copy = Copy
shopping-copied = Copied!
shopping-copy-failed = Copy failed
shopping-copy-options = Copy options
shopping-copy-include-aisles = Include aisle names
shopping-copy-include-amounts = Include amounts
