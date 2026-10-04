# Navigation
nav-recipes = Recipes
nav-shopping-list = Shopping List
nav-pantry = Pantry
nav-preferences = Preferences

# Search
search-placeholder = Search recipes...
search-no-results = No recipes found

# Common Actions
action-add = Add
action-remove = Remove
action-edit = Edit
action-save = Save
action-cancel = Cancel
action-back = Back
action-delete = Delete
action-clear = Clear
action-print = Print
action-preview = Preview
action-done = Done

# Common Labels
label-scale = Scale
label-servings = Servings
label-time = Time
label-difficulty = Difficulty
label-name = Name
label-description = Description

# Editor
editor-unsaved-changes = Unsaved changes
editor-saved = Saved
editor-saving = Saving...
editor-save-failed = Save failed
editor-placeholder = Enter your recipe here...

# Editor toolbar
editor-toolbar-label = Cooklang formatting
editor-toolbar-inline = Inline elements
editor-toolbar-block = Line elements
editor-toolbar-ingredient = Ingredient
editor-toolbar-ingredient-title = Insert an ingredient (@), or turn the selection into one
editor-toolbar-cookware = Cookware
editor-toolbar-cookware-title = Insert cookware (#), or turn the selection into cookware
editor-toolbar-timer = Timer
editor-toolbar-timer-title = Insert a timer (~) in minutes, or name it after the selection
editor-toolbar-section = Section
editor-toolbar-section-title = Start a new section (== Section ==), named after the selection
editor-toolbar-section-default = Section
editor-toolbar-note = Note
editor-toolbar-note-title = Turn the current lines into a note (>), or back into steps
editor-toolbar-comment = Comment
editor-toolbar-comment-title = Comment out the current lines (--), or the selection within a line
editor-toolbar-metadata = Metadata
editor-toolbar-metadata-title = Add a metadata line to the frontmatter (---)
editor-toolbar-menu = Menu elements
editor-toolbar-other = Other elements
editor-toolbar-day = Day
editor-toolbar-day-title = Start a new day (== Day ==), dated when a date is picked
editor-toolbar-day-default = Day
editor-toolbar-day-date = Date of the next day
editor-toolbar-day-date-title = Optional date for the next day, as in == Saturday (2026-03-07) ==
editor-toolbar-meal = Meal
editor-toolbar-meal-title = Start a meal (Breakfast: \) with a first bullet
editor-toolbar-meal-breakfast = Breakfast
editor-toolbar-meal-lunch = Lunch
editor-toolbar-meal-dinner = Dinner
editor-toolbar-meal-snacks = Snacks
editor-toolbar-add-recipe = Add recipe
editor-toolbar-add-recipe-title = Add a recipe to the current meal (- @./Recipe{"{}"})
editor-toolbar-recipe-reference = Recipe reference
editor-toolbar-recipe-reference-title = Reference another recipe (@./Recipe{"{}"})

# Recipe picker
recipe-picker-title = Choose a recipe
recipe-picker-search = Search recipes
recipe-picker-results = Recipes
recipe-picker-servings = Servings
recipe-picker-servings-hint = Leave empty to use the recipe's own servings.
recipe-picker-insert = Insert
recipe-picker-no-results = No recipes found
recipe-picker-load-failed = Could not load the recipes

# LSP Status
lsp-connected = LSP Connected
lsp-disconnected = Disconnected
lsp-error = LSP Error

# New Recipe
new-recipe = New Recipe
new-recipe-path = Recipe path
new-recipe-filename = Recipe name
new-recipe-placeholder = Dinner/Italian/Pasta Carbonara
new-recipe-hint = Use folder/recipe-name format
new-recipe-create = Create Recipe

# New Menu
new-menu = New Menu
new-menu-path = Menu path
new-menu-placeholder = Plans/Week 12
new-menu-hint = Use folder/menu-name format
new-menu-create = Create Menu
new-plan = New Meal Plan
new-plan-path = Meal plan path
new-plan-placeholder = Plans/October
new-plan-hint = Use folder/plan-name format
new-plan-create = Create Meal Plan
new-plan-start = First day
new-plan-today = Today
new-plan-this-week = This week
new-plan-next-week = Next week
new-plan-days = Number of days
new-plan-one-week = 1 week
new-plan-two-weeks = 2 weeks
new-plan-meals = Meals

# Delete Recipe
delete-recipe = Delete Recipe
delete-recipe-confirm = Are you sure you want to delete this recipe?
delete-recipe-warning = This action cannot be undone.

# Rename Recipe
action-rename = Rename
rename-title = Rename file
rename-label = New name
rename-hint = The file stays in its folder. Its pictures are renamed with it, and the recipes and menus that use it are updated to the new name.
rename-failed = Could not rename: %s
rename-skipped = Renamed, but some references were left unchanged: %s
rename-write-failed = Renamed, but these files could not be updated: %s
rename-shopping-list = The shopping list still uses the old name; add it again from its new page.

# Title Picture
picture-button = Picture
picture-title = Title picture
picture-none = No picture yet. Choose one, or drop it here.
picture-choose = Choose picture
picture-replace = Replace picture
picture-remove = Remove
picture-remove-confirm = Remove this picture? The file is deleted and cannot be restored.
picture-hint = JPEG, PNG or WebP. Saved as JPEG; large pictures are scaled down to 2048 px.
picture-from-metadata = This recipe's picture is set by the image field in its metadata. Remove that line to use an uploaded picture.
picture-uploading = Uploading...
picture-removing = Removing...
picture-load-failed = Could not load the picture
picture-upload-failed = Upload failed
picture-remove-failed = Could not remove the picture
picture-too-large = This picture is too large. The limit is 10 MB.
picture-heif = HEIC and AVIF photos can't be read. Upload from the phone's own browser, which converts them to JPEG, or set the iPhone camera to Most Compatible (Settings > Camera > Formats).
picture-for = Picture for
picture-step-title = Step picture
picture-target-step = Step { $step }: { $text }
picture-target-section = Section { $section }
picture-step-note = A step picture goes with the step's position, not its text: adding or removing an earlier step in this section moves it onto another step.
step-picture-add = Add a picture to this step
step-picture-change = Change this step's picture
# Sign-in (users are managed on the server with `cook server user`)
sign-in = Sign in
sign-out = Sign out
sign-in-intro = Sign in to add, edit or delete recipes and to change the pantry or the shopping list.
sign-in-username = Username
sign-in-password = Password
sign-in-failed = Wrong username or password.
signed-in-as = Signed in as
role-forbidden = Your account can't do this. Ask whoever runs this server for a different role.

# Errors
error-title = Something went wrong
error-back-home = Back to recipes

# Icon button labels (aria-label / title)
aria-toggle-theme = Toggle theme
aria-keyboard-shortcuts = Keyboard shortcuts
aria-more-options = More options
aria-preferences = Preferences
aria-dismiss = Dismiss
aria-decrease-scale = Decrease scale
aria-increase-scale = Increase scale
aria-decrease-servings = Decrease servings
aria-increase-servings = Increase servings
aria-close = Close
