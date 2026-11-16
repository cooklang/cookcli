# Recipe Listing
recipes-title = All Recipes
recipes-directory-title = { $name }
recipes-empty = No recipes found in this directory.
recipes-count =
    { $count ->
        [one] { $count } recipe
       *[other] { $count } recipes
    }

# Recipe Display
recipe-ingredients = Ingredients
recipe-steps = Instructions
recipe-notes = Notes
recipe-cookware = Cookware
recipe-optional = (optional)
recipe-timers = Timers
recipe-tags = Tags
recipe-add-to-shopping = Add to Shopping List
recipe-add-all-to-shopping = Add All to Shopping List
recipe-scale-label = Scale
recipe-servings-label = servings
recipe-written-for =
    { $count ->
        [one] Written for { $count } serving
       *[other] Written for { $count } servings
    }
recipe-written-for-hint = Quantities are scaled from the original recipe. Cooking times, pan sizes and seasoning may need adjusting. Select to go back to the original servings.
recipe-scaled-from-original = ×{ $factor } of the original recipe
recipe-scaled-from-original-hint = Quantities are scaled from the original recipe. Cooking times, pan sizes and seasoning may need adjusting. Select to go back to the original recipe.
recipe-print = Print Recipe
recipe-added = Added!
recipe-main-section = Main
recipe-preparation = preparation

# Recipe Metadata
meta-course = Course
meta-cuisine = Cuisine
meta-diet = Diet
meta-author = Author
meta-source = Source
meta-prep-time = Prep Time
meta-cook-time = Cook Time
meta-total-time = Total Time
meta-servings = Servings
meta-difficulty = Difficulty
meta-description = Description
meta-created-by = Created by
meta-created-at = Created on
meta-modified-by = Modified by
meta-modified-at = Modified on
meta-file-title = File
meta-translated-from = Translated from

# Nutrition (custom `nutrition:` metadata)
nutrition-per-serving = per serving
nutrition-proteins = proteins
nutrition-lipids = fat
nutrition-saturated-fat = saturated fat
nutrition-unsaturated-fat = unsaturated fat
nutrition-monounsaturated-fat = monounsaturated fat
nutrition-polyunsaturated-fat = polyunsaturated fat
nutrition-trans-fat = trans fat
nutrition-starch = starch
nutrition-carbohydrates = carbohydrates
nutrition-sugars = sugars
nutrition-fibers = fiber
nutrition-salt = salt
nutrition-title = Nutrition
nutrition-per-100g = per 100 g
nutrition-whole-recipe = whole recipe
nutrition-energy = energy
nutrition-of-which = of which
nutrition-for = for
nutrition-full-table = Full table
nutrition-legend = Health rating, per 100 g:
nutrition-rating-good = good
nutrition-rating-medium = medium
nutrition-rating-bad = poor
nutrition-legend-good = good for health
nutrition-legend-medium = medium
nutrition-legend-bad = not good for health
nutrition-legend-unrated = no threshold

# Recipe Types
recipe-type-menu = Menu
recipe-type-plan = Meal Plan
plan-outside = Outside this plan
plan-nothing-planned = Nothing planned
plan-add-to = Add to { $meal }, { $day }
plan-line-actions = Change
plan-move-to = Move to…
plan-copy-to = Copy to…
plan-remove = Remove
plan-move-title = Move to another day
plan-copy-title = Copy to another day
plan-move = Move
plan-copy = Copy
plan-target-day = Day
plan-target-meal = Meal
plan-changed = The plan changed since this page was loaded. Reload to see it, then try again.
plan-save-failed = Could not save the plan.
plan-saved = Plan saved
plan-reload = Reload
plan-pick-label = Days for the shopping list
plan-pick-all = All
plan-pick-week = Next 7 days
plan-pick-none = None
plan-add-days = Add days to shopping list

# Today's Menu Banner
todays-menu-title = Today's Menu
todays-menu-from = From
todays-menu-view = View Menu

# Sort Controls
sort-by = Sort by:
sort-name = Name
sort-modified = Modified
sort-created = Created
sort-direction-toggle = Toggle sort direction
random-recipe = Random recipe
