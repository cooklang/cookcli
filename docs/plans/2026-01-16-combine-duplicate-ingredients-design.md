# Design: Combine Duplicate Ingredients in Recipe Display

## Problem

When a recipe uses the same ingredient multiple times with different quantities, the recipe view displays multiple lines for that ingredient:

```
Ingredients:
  sea salt       pinch
  sea salt       1 tbsp
```

This creates visual clutter and makes it harder to see the total amount needed at a glance.

## Solution

Merge duplicate ingredients by name in the recipe display, similar to how the shopping list works.

### Behavior

- **Same units**: Add quantities together
  - Example: "flour: 100 g" + "flour: 200 g" → "flour: 300 g"

- **Different units**: Display comma-separated
  - Example: "sea salt: pinch" + "sea salt: 1 tbsp" → "sea salt: pinch, 1 tbsp"
  - Example: "milk: 1 cup" + "milk: 250 ml" → "milk: 1 cup, 250 ml"

### Implementation Details

The `ingredients()` function in `src/util/cooklang_to_human.rs:264` needs to be modified:

**Current flow:**
1. Call `recipe.group_ingredients(converter)` - returns `Vec<GroupedIngredient>`
2. Iterate and display each entry

**New flow:**
1. Call `recipe.group_ingredients(converter)` - returns `Vec<GroupedIngredient>`
2. Group entries by `ingredient.display_name()` using `BTreeMap<String, Vec<GroupedIngredient>>`
3. For each group with the same display name:
   - Create an empty `GroupedQuantity`
   - Merge all quantities using `GroupedQuantity::merge(&other, converter)`
   - Keep track of additional metadata (optional flag, reference, note) from entries
4. Display the merged results

**Key insight:** The `GroupedQuantity::merge()` method (used by shopping lists) already handles:
- Adding quantities with compatible units
- Keeping quantities with incompatible units separate
- Proper unit conversion via the `Converter`

### Edge Cases

1. **Optional modifier**: If any occurrence is non-optional, the merged ingredient should be non-optional
2. **Recipe references**: If any occurrence is a recipe reference, display the reference marker
3. **Notes**: Combine notes from all occurrences (comma-separated if multiple)
4. **Order**: Maintain alphabetical order by display name (via BTreeMap)

### Files to Modify

- `src/util/cooklang_to_human.rs` - Update `ingredients()` function (lines 264-316)

### Testing

Manual test with the provided example:
```bash
cargo run -- recipe read './Breakfast/Easy Pancakes'
```

Expected output:
```
Ingredients:
  eggs           3
  flour          125 g
  milk           250 ml
  olive oil      drizzle
  sea salt       pinch, 1 tbsp
```

### Notes

- This aligns with shopping list behavior where duplicate ingredients are automatically combined
- The implementation reuses existing `GroupedQuantity` infrastructure
- No changes needed to the cooklang-rs library - all logic is in the display layer
