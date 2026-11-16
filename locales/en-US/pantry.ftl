# Pantry
pantry-title = Pantry Inventory
pantry-intro = What is at home, by where it is kept. The shopping list leaves out what you already have.
pantry-unreadable = The pantry file could not be read
pantry-empty = Your pantry is empty
pantry-filters = Show
pantry-filter-all = All
pantry-filter-low = Running low
pantry-filter-out = Out of stock
pantry-filter-expiring = Expiring soon
pantry-filter-empty = No item matches this filter.
pantry-stock-ok = In stock
pantry-expired = Expired
pantry-expires-today = Expires today
pantry-expires-in =
    { $count ->
        [one] In { $count } day
       *[other] In { $count } days
    }
pantry-section = { $name }
pantry-no-config = No pantry configuration found
pantry-create-config = Create a pantry.conf file to track your inventory
pantry-configure = Configure pantry →
pantry-manage = Manage pantry

# Pantry Item Fields
pantry-item-name = Item Name
pantry-item-quantity = Quantity
pantry-item-bought = Bought:
pantry-item-bought-date = Bought Date
pantry-item-expire = Expires:
pantry-item-expire-date = Expiry Date
pantry-item-low = Low at:
pantry-item-low-threshold = Low Stock Threshold
pantry-placeholder-quantity = e.g. 500%g or 2%L
pantry-placeholder-low = e.g. 100%g or 2

# Pantry Actions
pantry-add-item = Add Item
pantry-add = Add
pantry-edit-item = Edit item
pantry-remove-item = Remove item
pantry-remove-confirm = Remove { $name } from { $section }?
pantry-confirm-remove-template = Remove %s from %s?
pantry-mark-low = Mark as Low
pantry-restock = Restock
pantry-save = Save
pantry-cancel = Cancel
pantry-section-label = Section
pantry-section-required = Give the section a name
pantry-general-quantity-only = Items above the first section can only have a quantity. Move one into a section to give it dates or a low mark.
pantry-section-freezer = Freezer
pantry-section-fridge = Fridge
pantry-section-pantry = Pantry
pantry-section-spices = Spices
pantry-section-general = General
pantry-tab-items = Items
pantry-tab-text = Text
pantry-text-intro = Edit the file directly: a [section] line, then one item per line, written name = "quantity" or name = {"{"} quantity = "…", bought = "…", expire = "…", low = "…" {"}"}. Lines starting with # are comments.
pantry-text-save = Save file
pantry-text-saved = Saved
pantry-text-conflict = The pantry file was changed elsewhere while you were editing. Your text is kept; save again to replace that change with it.
pantry-rename-section = Rename section
pantry-section-name = Section name
pantry-failed-rename = Failed to rename section
pantry-failed-save-file = Could not save the file
pantry-optional = (optional)
pantry-failed-add = Failed to add item
pantry-item-name-required = Give the item a name
pantry-item-exists = %s is already in %s
pantry-section-name-taken = There is already a section or item called %s
pantry-failed-update = Failed to update item
pantry-failed-remove = Failed to remove item
