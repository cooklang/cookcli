# Recipe Listing
recipes-title = Toutes les Recettes
recipes-directory-title = { $name }
recipes-empty = Aucune recette trouvée dans ce répertoire.
recipes-count =
    { $count ->
        [one] { $count } recette
       *[other] { $count } recettes
    }

# Recipe Display
recipe-ingredients = Ingrédients
recipe-steps = Instructions
recipe-notes = Notes
recipe-cookware = Ustensiles
recipe-optional = (optionnel)
recipe-timers = Minuteries
recipe-tags = Étiquettes
recipe-add-to-shopping = Ajouter à la liste de courses
recipe-add-all-to-shopping = Tout ajouter à la liste de courses
recipe-scale-label = Échelle
recipe-servings-label = portions
recipe-written-for =
    { $count ->
        [one] Prévue pour { $count } portion
       *[other] Prévue pour { $count } portions
    }
recipe-written-for-hint = Les quantités sont recalculées à partir de la recette d'origine. Les temps de cuisson, la taille des moules et l'assaisonnement peuvent demander un ajustement. Sélectionner pour revenir aux portions d'origine.
recipe-scaled-from-original = ×{ $factor } de la recette d'origine
recipe-scaled-from-original-hint = Les quantités sont recalculées à partir de la recette d'origine. Les temps de cuisson, la taille des moules et l'assaisonnement peuvent demander un ajustement. Sélectionner pour revenir à la recette d'origine.
recipe-print = Imprimer la recette
recipe-added = Ajouté !
recipe-main-section = Principal
recipe-preparation = préparation

# Recipe Metadata
meta-course = Plat
meta-cuisine = Cuisine
meta-diet = Régime
meta-author = Auteur
meta-source = Source
meta-prep-time = Temps de Préparation
meta-cook-time = Temps de Cuisson
meta-total-time = Temps Total
meta-servings = Portions
meta-difficulty = Difficulté
meta-description = Description
meta-created-by = Créé par
meta-created-at = Créé le
meta-modified-by = Modifié par
meta-modified-at = Modifié le
meta-file-title = Fichier
meta-translated-from = Traduit de

# Nutrition (métadonnées personnalisées `nutrition:`)
nutrition-per-serving = par portion
nutrition-proteins = protéines
nutrition-lipids = lipides
nutrition-saturated-fat = graisses saturées
nutrition-unsaturated-fat = graisses insaturées
nutrition-monounsaturated-fat = graisses mono-insaturées
nutrition-polyunsaturated-fat = graisses poly-insaturées
nutrition-trans-fat = graisses trans
nutrition-starch = amidon
nutrition-carbohydrates = glucides
nutrition-sugars = sucres
nutrition-fibers = fibres
nutrition-salt = sel
nutrition-title = Nutrition
nutrition-per-100g = pour 100 g
nutrition-whole-recipe = recette entière
nutrition-energy = énergie
nutrition-of-which = dont
nutrition-for = pour
nutrition-full-table = Tableau complet
nutrition-legend = Appréciation santé, pour 100 g :
nutrition-rating-good = bon
nutrition-rating-medium = moyen
nutrition-rating-bad = pas bon
nutrition-legend-good = bon pour la santé
nutrition-legend-medium = moyen
nutrition-legend-bad = pas bon pour la santé
nutrition-legend-unrated = sans seuil

# Recipe Types
recipe-type-menu = Menu
recipe-type-plan = Planning de repas
plan-outside = Hors de ce planning
plan-nothing-planned = Rien de prévu
plan-add-to = Ajouter à { $meal }, { $day }
plan-line-actions = Modifier
plan-move-to = Déplacer vers…
plan-copy-to = Copier vers…
plan-remove = Retirer
plan-move-title = Déplacer vers un autre jour
plan-copy-title = Copier vers un autre jour
plan-move = Déplacer
plan-copy = Copier
plan-target-day = Jour
plan-target-meal = Repas
plan-changed = Le planning a changé depuis le chargement de la page. Rechargez-la pour le voir, puis réessayez.
plan-save-failed = Impossible d'enregistrer le planning.
plan-saved = Planning enregistré
plan-reload = Recharger
plan-pick-label = Jours pour la liste de courses
plan-pick-all = Tous
plan-pick-week = 7 prochains jours
plan-pick-none = Aucun
plan-add-days = Ajouter les jours à la liste de courses

# Today's Menu Banner
todays-menu-title = Menu du Jour
todays-menu-from = De
todays-menu-view = Voir le Menu

# Sort Controls
sort-by = Trier par :
sort-name = Nom
sort-modified = Modifié
sort-created = Créé
sort-direction-toggle = Inverser l'ordre
random-recipe = Recette au hasard

# Standards: the layout, intakes and ratings come from the standard in use.
nutrition-more-info = Plus d'infos
nutrition-less-info = Moins d'infos
nutrition-collapse-all = Tout replier
nutrition-expand-all = Tout déplier
nutrition-tree-lines = Traits d'arbre
nutrition-fold = Replier ou déplier
nutrition-legend-claims = Allégations de l'UE, pour 100 g :
nutrition-legend-dv = Part de la valeur quotidienne, par portion :
nutrition-well-present = Vitamines et minéraux bien présents
nutrition-well-source = source de
nutrition-well-high = riche en
nutrition-well-rule-100g = au moins { $source } % de l'apport de référence pour 100 g (« source de »), { $high } % ou plus (« riche en »)
nutrition-well-rule-serving = au moins { $source } % de la valeur quotidienne par portion (« source de »), { $high } % ou plus (« riche en »)
nutrition-well-rule-serving-high = { $high } % ou plus de la valeur quotidienne par portion (« riche en »)
nutrition-standard-note = Norme : { $name } ({ $id }.yaml)
nutrition-carbs-without-fibre = Les glucides sont donnés hors fibres (la recette les compte dedans).
nutrition-group-macronutrients = Macronutriments
nutrition-group-other = Autres composants
nutrition-group-minerals = Minéraux
nutrition-group-macro-minerals = Macro-minéraux
nutrition-group-trace-elements = Oligo-éléments
nutrition-group-vitamins = Vitamines
nutrition-group-fat-soluble = Liposolubles
nutrition-group-water-soluble = Hydrosolubles
nutrition-group-other-data = Autres données
nutrition-cholesterol = cholestérol
nutrition-sodium = sodium
nutrition-added-sugars = sucres ajoutés
nutrition-polyols = polyols
nutrition-omega-3 = oméga-3
nutrition-omega-6 = oméga-6
nutrition-calcium = calcium
nutrition-phosphorus = phosphore
nutrition-magnesium = magnésium
nutrition-potassium = potassium
nutrition-chloride = chlorure
nutrition-iron = fer
nutrition-zinc = zinc
nutrition-copper = cuivre
nutrition-manganese = manganèse
nutrition-fluoride = fluorure
nutrition-selenium = sélénium
nutrition-chromium = chrome
nutrition-molybdenum = molybdène
nutrition-iodine = iode
nutrition-choline = choline
nutrition-vitamin-a = vitamine A
nutrition-vitamin-d = vitamine D
nutrition-vitamin-e = vitamine E
nutrition-vitamin-k = vitamine K
nutrition-vitamin-c = vitamine C
nutrition-vitamin-b1 = vitamine B1
nutrition-vitamin-b2 = vitamine B2
nutrition-vitamin-b3 = vitamine B3
nutrition-vitamin-b5 = vitamine B5
nutrition-vitamin-b6 = vitamine B6
nutrition-vitamin-b8 = vitamine B8
nutrition-vitamin-b9 = vitamine B9
nutrition-vitamin-b12 = vitamine B12

# Explanation bubbles: what each text says comes from the source its -source message names.
nutrition-help-what = C'est quoi
nutrition-help-use = À quoi ça sert
nutrition-help-where = Où le trouve-t-on
nutrition-help-source = Source : { $source }
nutrition-help-source-todo = Source : à documenter
nutrition-help-reference = Apport de référence ({ $standard }) : { $amount } par jour.
nutrition-help-reference-none = Pas d’apport de référence dans la norme { $standard }.
nutrition-help-recipe = Cette recette : { $amount } par portion.
nutrition-help-recipe-pct = Cette recette : { $amount } par portion, soit { $pct } % de l’apport de référence.
nutrition-help-rating-traffic = Feux tricolores britanniques, pour 100 g : bon jusqu’à { $low } { $unit }, moyen jusqu’à { $high } { $unit }, pas bon au-delà.
nutrition-help-rating-dv-limit = À limiter : { $low } % de la valeur quotidienne ou moins est faible, { $high } % ou plus est élevé.
nutrition-help-rating-dv-encourage = À privilégier : { $high } % de la valeur quotidienne ou plus est élevé.
nutrition-help-rating-dv-none = Pas de valeur quotidienne pour ce nutriment : la FDA ne le juge pas.
nutrition-help-rating-claim-max = Allégation de l’UE « { $name } » : jusqu’à { $amount } { $unit } pour 100 g.
nutrition-help-rating-claim-min = Allégation de l’UE « { $name } » : à partir de { $amount } { $unit } pour 100 g.
nutrition-claim-low-fat = faible en graisses
nutrition-claim-low-saturated-fat = faible en graisses saturées
nutrition-claim-low-sugars = faible en sucres
nutrition-claim-low-sodium = faible en sel
nutrition-claim-source-of-fibre = source de fibres
nutrition-claim-high-fibre = riche en fibres
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
nutrition-help-energy-what = Ce que l’aliment apporte au corps, en kilocalories (kcal) ou en kilojoules (kJ).
nutrition-help-energy-use = Elle se calcule avec des facteurs fixes : 9 kcal par gramme de lipides, 4 de glucides et de protéines, 7 d’alcool, 2 de fibres.
nutrition-help-lipids-what = Les matières grasses d’un aliment.
nutrition-help-lipids-use = Les lipides apportent des acides gras essentiels, aident à absorber les vitamines A, D et E, et donnent 9 kcal par gramme contre 4 pour les glucides et les protéines.
nutrition-help-saturated-fat-what = Des graisses qui viennent surtout de sources animales, et de l’huile de palme et de coco.
nutrition-help-saturated-fat-use = Trop en fait monter le « mauvais » cholestérol (LDL) et le risque de maladie cardiaque et d’AVC. OMS : moins de 10 % de l’énergie.
nutrition-help-saturated-fat-where = Viande grasse, saucisses et tourtes, beurre, ghee et saindoux, fromage, crème, chocolat, biscuits et gâteaux, huile de palme et de coco.
nutrition-help-unsaturated-fat-what = Des graisses présentes dans le poisson, l’avocat et les noix, et dans les huiles de tournesol, de soja, de colza et d’olive.
nutrition-help-unsaturated-fat-use = Préférables aux graisses saturées (OMS).
nutrition-help-monounsaturated-fat-what = Un type de graisse insaturée.
nutrition-help-monounsaturated-fat-use = Aide à protéger le cœur : maintient le « bon » cholestérol (HDL) tout en faisant baisser le « mauvais » (LDL).
nutrition-help-monounsaturated-fat-where = Huile d’olive et de colza, avocat, amandes, noix du Brésil, cacahuètes.
nutrition-help-polyunsaturated-fat-what = Un type de graisse insaturée. Il y en a deux grands types : les oméga-3 et les oméga-6.
nutrition-help-polyunsaturated-fat-use = Le corps ne sait pas en fabriquer certaines formes : une petite quantité doit venir de l’alimentation.
nutrition-help-omega-3-what = Une graisse poly-insaturée, indispensable dans l’alimentation.
nutrition-help-omega-3-use = On conseille au moins 2 portions de poisson par semaine, dont une de poisson gras.
nutrition-help-omega-3-where = Poissons gras (hareng, truite, sardine, saumon, maquereau), huiles de lin et de colza, noix.
nutrition-help-omega-6-what = Une graisse poly-insaturée, indispensable dans l’alimentation.
nutrition-help-omega-6-use = La plupart des gens en ont assez dans leur alimentation.
nutrition-help-omega-6-where = Huiles de colza, de maïs et de tournesol, noix, amandes, noix de cajou.
nutrition-help-trans-fat-what = Des acides gras insaturés, d’origine industrielle ou naturelle.
nutrition-help-trans-fat-use = Comme les graisses saturées, elles font monter le cholestérol. OMS : moins de 1 % de l’énergie.
nutrition-help-trans-fat-where = Margarines, matières grasses végétales et fritures ; naturellement dans la viande et le lait des ruminants.
nutrition-help-cholesterol-what = Une substance grasse surtout fabriquée par le foie.
nutrition-help-cholesterol-use = Les graisses saturées et trans font monter le « mauvais » cholestérol (LDL) dans le sang.
nutrition-help-carbohydrates-what = Les sucres, les amidons et, selon la norme, les fibres.
nutrition-help-carbohydrates-use = La principale source d’énergie du corps (OMS) : 4 kcal par gramme.
nutrition-help-sugars-what = Les sucres libres sont tous ceux qu’on ajoute aux aliments et aux boissons, plus ceux du miel, des sirops et des jus ; les sucres du lait, des fruits et des légumes n’en font pas partie.
nutrition-help-sugars-use = Trop de sucre fait prendre du poids et favorise les caries. OMS : sucres libres sous 10 % de l’énergie, environ 50 g.
nutrition-help-sugars-where = Bonbons, gâteaux, biscuits, chocolat, boissons gazeuses, céréales du petit-déjeuner.
nutrition-help-starch-what = Le glucide des féculents, notre principale source de glucides.
nutrition-help-starch-use = Les féculents devraient faire un peu plus d’un tiers de l’assiette.
nutrition-help-starch-where = Pommes de terre, pain, riz, pâtes, céréales.
nutrition-help-fibers-what = Des substances des parois des cellules des légumes, des fruits, des légumineuses et des céréales.
nutrition-help-fibers-use = Elles aident la digestion et préviennent la constipation. OMS : au moins 25 g par jour (30 g au Royaume-Uni).
nutrition-help-fibers-where = Pain et pâtes complets, riz brun, pommes de terre avec la peau, légumineuses, légumes, fruits, noix et graines.
nutrition-help-proteins-what = Les briques du corps, comme les muscles.
nutrition-help-proteins-use = 10 à 15 % de l’énergie suffisent en général chez l’adulte (OMS).
nutrition-help-salt-what = Le sel de table, du chlorure de sodium. Sel = sodium × 2,5 : 1 g de sel contient 400 mg de sodium.
nutrition-help-salt-use = Trop de sel fait monter la tension et le risque de crise cardiaque et d’AVC. OMS : moins de 5 g par jour.
nutrition-help-salt-where = Pain, céréales du petit-déjeuner, charcuterie, plats préparés, sauces, fromage.
nutrition-help-sodium-what = Le minéral du sel de table : 1 g de sel contient 400 mg de sodium.
nutrition-help-sodium-use = OMS : moins de 2 000 mg de sodium par jour.
