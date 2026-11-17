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

# Standards: the layout, intakes and ratings come from the standard in use.
nutrition-more-info = More info
nutrition-less-info = Less info
nutrition-collapse-all = Collapse all
nutrition-expand-all = Expand all
nutrition-tree-lines = Tree lines
nutrition-fold = Fold or unfold
nutrition-legend-claims = EU claims, per 100 g:
nutrition-legend-dv = Share of the daily value, per serving:
nutrition-well-present = Vitamins and minerals well present
nutrition-well-source = source of
nutrition-well-high = rich in
nutrition-well-rule-100g = at least { $source } % of the reference intake per 100 g ("source of"), { $high } % or more ("rich in")
nutrition-well-rule-serving = at least { $source } % of the daily value per serving ("source of"), { $high } % or more ("rich in")
nutrition-well-rule-serving-high = { $high } % or more of the daily value per serving ("rich in")
nutrition-standard-note = Standard: { $name } ({ $id }.yaml)
nutrition-carbs-without-fibre = Carbohydrates are shown without fibre (the recipe counts it in them).
nutrition-group-macronutrients = Macronutrients
nutrition-group-other = Other components
nutrition-group-minerals = Minerals
nutrition-group-macro-minerals = Major minerals
nutrition-group-trace-elements = Trace elements
nutrition-group-vitamins = Vitamins
nutrition-group-fat-soluble = Fat-soluble
nutrition-group-water-soluble = Water-soluble
nutrition-group-other-data = Other data
nutrition-cholesterol = cholesterol
nutrition-sodium = sodium
nutrition-added-sugars = added sugars
nutrition-polyols = polyols
nutrition-omega-3 = omega-3
nutrition-omega-6 = omega-6
nutrition-calcium = calcium
nutrition-phosphorus = phosphorus
nutrition-magnesium = magnesium
nutrition-potassium = potassium
nutrition-chloride = chloride
nutrition-iron = iron
nutrition-zinc = zinc
nutrition-copper = copper
nutrition-manganese = manganese
nutrition-fluoride = fluoride
nutrition-selenium = selenium
nutrition-chromium = chromium
nutrition-molybdenum = molybdenum
nutrition-iodine = iodine
nutrition-choline = choline
nutrition-vitamin-a = vitamin A
nutrition-vitamin-d = vitamin D
nutrition-vitamin-e = vitamin E
nutrition-vitamin-k = vitamin K
nutrition-vitamin-c = vitamin C
nutrition-vitamin-b1 = vitamin B1
nutrition-vitamin-b2 = vitamin B2
nutrition-vitamin-b3 = vitamin B3
nutrition-vitamin-b5 = vitamin B5
nutrition-vitamin-b6 = vitamin B6
nutrition-vitamin-b8 = vitamin B8
nutrition-vitamin-b9 = vitamin B9
nutrition-vitamin-b12 = vitamin B12

# Explanation bubbles: what each text says comes from the source its -source message names.
nutrition-help-what = What it is
nutrition-help-use = What it is for
nutrition-help-where = Where to find it
nutrition-help-source = Source: { $source }
nutrition-help-source-todo = Source: to be documented
nutrition-help-reference = Reference intake ({ $standard }): { $amount } per day.
nutrition-help-reference-none = No reference intake in the { $standard } standard.
nutrition-help-recipe = This recipe: { $amount } per serving.
nutrition-help-recipe-pct = This recipe: { $amount } per serving, { $pct } % of the reference intake.
nutrition-help-rating-traffic = UK traffic lights, per 100 g: good up to { $low } { $unit }, medium up to { $high } { $unit }, not good above.
nutrition-help-rating-dv-limit = To limit: { $low } % of the daily value or less is low, { $high } % or more is high.
nutrition-help-rating-dv-encourage = To get more of: { $high } % of the daily value or more is high.
nutrition-help-rating-dv-none = No daily value for this nutrient: the FDA does not rate it.
nutrition-help-rating-claim-max = EU claim “{ $name }”: up to { $amount } { $unit } per 100 g.
nutrition-help-rating-claim-min = EU claim “{ $name }”: from { $amount } { $unit } per 100 g.
nutrition-claim-low-fat = low in fat
nutrition-claim-low-saturated-fat = low in saturated fat
nutrition-claim-low-sugars = low in sugars
nutrition-claim-low-sodium = low in salt
nutrition-claim-source-of-fibre = source of fibre
nutrition-claim-high-fibre = high in fibre
nutrition-help-energy-source = Regulation (EU) 1169/2011, Annex XIV; NHS, Fat: the facts (2023)
nutrition-help-lipids-source = NHS, Fat: the facts (2023)
nutrition-help-saturated-fat-source = WHO, Healthy diet (2026); NHS, Fat: the facts (2023)
nutrition-help-unsaturated-fat-source = WHO, Healthy diet (2026)
nutrition-help-monounsaturated-fat-source = NHS, Fat: the facts (2023)
nutrition-help-polyunsaturated-fat-source = NHS, Fat: the facts (2023)
nutrition-help-omega-3-source = NHS, Fat: the facts (2023)
nutrition-help-omega-6-source = NHS, Fat: the facts (2023)
nutrition-help-trans-fat-source = WHO, Trans fat (2024); NHS, Fat: the facts (2023)
nutrition-help-cholesterol-source = NHS, Fat: the facts (2023)
nutrition-help-carbohydrates-source = WHO, Healthy diet (2026); Regulation (EU) 1169/2011, Annex I
nutrition-help-sugars-source = WHO, Healthy diet (2026); NHS, Sugar: the facts (2023)
nutrition-help-starch-source = NHS, Starchy foods and carbohydrates (2023)
nutrition-help-fibers-source = WHO, Healthy diet (2026); NHS, How to get more fibre into your diet (2026)
nutrition-help-proteins-source = WHO, Healthy diet (2026)
nutrition-help-salt-source = WHO, Sodium reduction (2026); NHS, Salt in your diet (2023); Regulation (EU) 1169/2011, Annex I
nutrition-help-sodium-source = WHO, Sodium reduction (2026)
nutrition-help-energy-what = What the food gives the body, in kilocalories (kcal) or kilojoules (kJ).
nutrition-help-energy-use = It is counted with fixed factors: 9 kcal per gram of fat, 4 of carbohydrate and of protein, 7 of alcohol, 2 of fibre.
nutrition-help-lipids-what = The fats and oils of a food.
nutrition-help-lipids-use = Fat is a source of essential fatty acids, helps absorb vitamins A, D and E, and gives 9 kcal per gram, against 4 for carbohydrate and protein.
nutrition-help-saturated-fat-what = Fats that come mostly from animal sources, and from palm and coconut oil.
nutrition-help-saturated-fat-use = Too much raises the “bad” LDL cholesterol and the risk of heart disease and stroke. WHO: less than 10 % of the energy.
nutrition-help-saturated-fat-where = Fatty meat, sausages and pies, butter, ghee and lard, cheese, cream, chocolate, biscuits and cakes, palm and coconut oil.
nutrition-help-unsaturated-fat-what = Fats found in fish, avocado and nuts, and in sunflower, soybean, rapeseed and olive oils.
nutrition-help-unsaturated-fat-use = Preferable to saturated fat (WHO).
nutrition-help-monounsaturated-fat-what = A kind of unsaturated fat.
nutrition-help-monounsaturated-fat-use = Helps protect the heart by keeping “good” HDL cholesterol while lowering “bad” LDL.
nutrition-help-monounsaturated-fat-where = Olive and rapeseed oils, avocados, almonds, Brazil nuts, peanuts.
nutrition-help-polyunsaturated-fat-what = A kind of unsaturated fat. There are two main types: omega-3 and omega-6.
nutrition-help-polyunsaturated-fat-use = The body cannot make them in some forms, so a small amount has to come from food.
nutrition-help-omega-3-what = A polyunsaturated fat, essential in the diet.
nutrition-help-omega-3-use = At least 2 portions of fish a week are advised, one of them oily fish.
nutrition-help-omega-3-where = Oily fish (herring, trout, sardines, salmon, mackerel), flaxseed and rapeseed oils, walnuts.
nutrition-help-omega-6-what = A polyunsaturated fat, essential in the diet.
nutrition-help-omega-6-use = Most people get enough of it.
nutrition-help-omega-6-where = Rapeseed, corn and sunflower oils, walnuts, almonds, cashews.
nutrition-help-trans-fat-what = Unsaturated fatty acids of industrial or natural origin.
nutrition-help-trans-fat-use = Like saturated fats they raise cholesterol. WHO: less than 1 % of the energy.
nutrition-help-trans-fat-where = Margarine, shortening and fried foods; naturally in meat and dairy from ruminants.
nutrition-help-cholesterol-what = A fatty substance mostly made by the body in the liver.
nutrition-help-cholesterol-use = Saturated and trans fats raise the “bad” LDL cholesterol in the blood.
nutrition-help-carbohydrates-what = Sugars, starches and, depending on the standard, fibre.
nutrition-help-carbohydrates-use = The main source of energy of the body (WHO): 4 kcal per gram.
nutrition-help-sugars-what = Free sugars are all those added to foods and drinks, plus those of honey, syrups and juices; the sugars of milk, fruit and vegetables do not count.
nutrition-help-sugars-use = Too much makes you gain weight and causes tooth decay. WHO: free sugars under 10 % of the energy, about 50 g.
nutrition-help-sugars-where = Sweets, cakes, biscuits, chocolate, fizzy drinks, breakfast cereals.
nutrition-help-starch-what = The carbohydrate of starchy foods, our main source of carbohydrate.
nutrition-help-starch-use = Starchy foods should make up just over a third of what you eat.
nutrition-help-starch-where = Potatoes, bread, rice, pasta, cereals.
nutrition-help-fibers-what = Substances found in the cell walls of vegetables, fruit, pulses and cereal grains.
nutrition-help-fibers-use = They help digestion and prevent constipation. WHO: at least 25 g a day (30 g in the UK).
nutrition-help-fibers-where = Wholemeal bread and pasta, brown rice, potatoes with their skins, pulses, vegetables, fruit, nuts and seeds.
nutrition-help-proteins-what = The building blocks of the body, such as muscle.
nutrition-help-proteins-use = 10 to 15 % of the energy is generally enough for adults (WHO).
nutrition-help-salt-what = Table salt, sodium chloride. Salt = sodium × 2.5: 1 g of salt holds 400 mg of sodium.
nutrition-help-salt-use = Too much raises blood pressure and the risk of heart attack and stroke. WHO: less than 5 g a day.
nutrition-help-salt-where = Bread, breakfast cereals, meat products, ready meals, sauces, cheese.
nutrition-help-sodium-what = The mineral of table salt: 1 g of salt holds 400 mg of sodium.
nutrition-help-sodium-use = WHO: less than 2000 mg of sodium a day.
