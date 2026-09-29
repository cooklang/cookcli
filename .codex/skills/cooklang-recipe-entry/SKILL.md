---
name: cooklang-recipe-entry
description: Add or revise Cooklang .cook recipes from user-provided sources, preserving quantities and cooking method while validating the finished file.
---

# Cooklang recipe entry

Create or update a `.cook` recipe using Cooklang markup: `@` for ingredients and quantities, `#` for cookware, and `~` for cooking timers. Include useful metadata such as title and servings when known.

Before editing, identify the source's servings, ingredient quantities, cooking sequence, temperatures, durations, and finish/serving instructions. Ask about only information that is genuinely missing or ambiguous; do not invent quantities or alter the method.

Before delivery, perform a source-to-recipe verification pass. Compare every ingredient and amount, step order, temperature, duration, and finishing instruction against the user-provided original. Correct any mismatch. Then run `cook recipe <file>` when CookCLI is available and report any validation limitation clearly.
