# Recipe Listing
recipes-title = Alle Recepten
recipes-directory-title = { $name }
recipes-empty = Geen recepten gevonden in deze map.
recipes-count =
    { $count ->
        [one] { $count } recept
       *[other] { $count } recepten
    }

# Recipe Display
recipe-ingredients = Ingrediënten
recipe-steps = Bereidingswijze
recipe-notes = Notities
recipe-cookware = Keukengerei
recipe-optional = (optioneel)
recipe-timers = Timers
recipe-tags = Tags
recipe-add-to-shopping = Toevoegen aan boodschappenlijst
recipe-add-all-to-shopping = Alles toevoegen aan boodschappenlijst
recipe-scale-label = Schaal
recipe-servings-label = porties
recipe-written-for =
    { $count ->
        [one] Geschreven voor { $count } portie
       *[other] Geschreven voor { $count } porties
    }
recipe-written-for-hint = De hoeveelheden zijn omgerekend vanuit het originele recept. Bereidingstijden, vormgroottes en kruiden moeten mogelijk worden aangepast. Selecteer om terug te gaan naar de oorspronkelijke porties.
recipe-scaled-from-original = ×{ $factor } van het originele recept
recipe-scaled-from-original-hint = De hoeveelheden zijn omgerekend vanuit het originele recept. Bereidingstijden, vormgroottes en kruiden moeten mogelijk worden aangepast. Selecteer om terug te gaan naar het originele recept.
recipe-print = Recept afdrukken
recipe-added = Toegevoegd!
recipe-main-section = Hoofdgedeelte
recipe-preparation = voorbereiding

# Recipe Metadata
meta-course = Gang
meta-cuisine = Keuken
meta-diet = Dieet
meta-author = Auteur
meta-source = Bron
meta-prep-time = Voorbereidingstijd
meta-cook-time = Bereidingstijd
meta-total-time = Totale tijd
meta-servings = Porties
meta-difficulty = Moeilijkheidsgraad
meta-description = Beschrijving
meta-created-by = Aangemaakt door
meta-created-at = Aangemaakt op
meta-modified-by = Gewijzigd door
meta-modified-at = Gewijzigd op
meta-file-title = Bestand
meta-translated-from = Vertaald uit

# Nutrition (aangepaste `nutrition:`-metadata)
nutrition-per-serving = per portie
nutrition-proteins = eiwitten
nutrition-lipids = vetten
nutrition-saturated-fat = verzadigd vet
nutrition-unsaturated-fat = onverzadigd vet
nutrition-monounsaturated-fat = enkelvoudig onverzadigd vet
nutrition-polyunsaturated-fat = meervoudig onverzadigd vet
nutrition-trans-fat = transvet
nutrition-starch = zetmeel
nutrition-carbohydrates = koolhydraten
nutrition-sugars = suikers
nutrition-fibers = vezels
nutrition-salt = zout
nutrition-title = Voedingswaarde
nutrition-per-100g = per 100 g
nutrition-whole-recipe = heel recept
nutrition-energy = energie
nutrition-of-which = waarvan
nutrition-for = voor
nutrition-full-table = Volledige tabel
nutrition-legend = Gezondheidsbeoordeling per 100 g:
nutrition-rating-good = goed
nutrition-rating-medium = matig
nutrition-rating-bad = slecht
nutrition-legend-good = goed voor de gezondheid
nutrition-legend-medium = matig
nutrition-legend-bad = niet goed voor de gezondheid
nutrition-legend-unrated = geen drempel

# Recipe Types
recipe-type-menu = Menu
recipe-type-plan = Maaltijdplan
plan-outside = Buiten dit plan
plan-nothing-planned = Niets gepland
plan-add-to = Toevoegen aan { $meal }, { $day }
plan-line-actions = Wijzigen
plan-move-to = Verplaatsen naar…
plan-copy-to = Kopiëren naar…
plan-remove = Verwijderen
plan-move-title = Naar een andere dag verplaatsen
plan-copy-title = Naar een andere dag kopiëren
plan-move = Verplaatsen
plan-copy = Kopiëren
plan-target-day = Dag
plan-target-meal = Maaltijd
plan-changed = Het plan is gewijzigd sinds deze pagina is geladen. Laad opnieuw om het te zien en probeer het nog eens.
plan-save-failed = Het plan kon niet worden opgeslagen.
plan-saved = Plan opgeslagen
plan-reload = Opnieuw laden
plan-pick-label = Dagen voor de boodschappenlijst
plan-pick-all = Alle
plan-pick-week = Komende 7 dagen
plan-pick-none = Geen
plan-add-days = Dagen aan boodschappenlijst toevoegen

# Today's Menu Banner
todays-menu-title = Menu van Vandaag
todays-menu-from = Van
todays-menu-view = Menu bekijken

# Sort Controls
sort-by = Sorteren op:
sort-name = Naam
sort-modified = Gewijzigd
sort-created = Aangemaakt
sort-direction-toggle = Sorteervolgorde omdraaien
random-recipe = Willekeurig recept

# Standards: the layout, intakes and ratings come from the standard in use.
nutrition-more-info = Meer info
nutrition-less-info = Minder info
nutrition-collapse-all = Alles inklappen
nutrition-expand-all = Alles uitklappen
nutrition-tree-lines = Boomlijnen
nutrition-fold = In- of uitklappen
nutrition-legend-claims = EU-claims, per 100 g:
nutrition-legend-dv = Aandeel van de dagwaarde, per portie:
nutrition-well-present = Goed aanwezige vitamines en mineralen
nutrition-well-source = bron van
nutrition-well-high = rijk aan
nutrition-well-rule-100g = minstens { $source } % van de referentie-inname per 100 g ("bron van"), { $high } % of meer ("rijk aan")
nutrition-well-rule-serving = minstens { $source } % van de dagwaarde per portie ("bron van"), { $high } % of meer ("rijk aan")
nutrition-well-rule-serving-high = { $high } % of meer van de dagwaarde per portie ("rijk aan")
nutrition-standard-note = Standaard: { $name } ({ $id }.yaml)
nutrition-carbs-without-fibre = Koolhydraten worden zonder vezels getoond (het recept rekent ze erbij).
nutrition-group-macronutrients = Macronutriënten
nutrition-group-other = Overige bestanddelen
nutrition-group-minerals = Mineralen
nutrition-group-macro-minerals = Macromineralen
nutrition-group-trace-elements = Sporenelementen
nutrition-group-vitamins = Vitamines
nutrition-group-fat-soluble = Vetoplosbare
nutrition-group-water-soluble = Wateroplosbare
nutrition-group-other-data = Overige gegevens
nutrition-cholesterol = cholesterol
nutrition-sodium = natrium
nutrition-added-sugars = toegevoegde suikers
nutrition-polyols = polyolen
nutrition-omega-3 = omega-3
nutrition-omega-6 = omega-6
nutrition-calcium = calcium
nutrition-phosphorus = fosfor
nutrition-magnesium = magnesium
nutrition-potassium = kalium
nutrition-chloride = chloride
nutrition-iron = iJzer
nutrition-zinc = zink
nutrition-copper = koper
nutrition-manganese = mangaan
nutrition-fluoride = fluoride
nutrition-selenium = seleen
nutrition-chromium = chroom
nutrition-molybdenum = molybdeen
nutrition-iodine = jodium
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
nutrition-help-what = Wat het is
nutrition-help-use = Waarvoor het dient
nutrition-help-where = Waar je het vindt
nutrition-help-source = Bron: { $source }
nutrition-help-source-todo = Bron: nog te documenteren
nutrition-help-reference = Referentie-inname ({ $standard }): { $amount } per dag.
nutrition-help-reference-none = Geen referentie-inname in de standaard { $standard }.
nutrition-help-recipe = Dit recept: { $amount } per portie.
nutrition-help-recipe-pct = Dit recept: { $amount } per portie, { $pct } % van de referentie-inname.
nutrition-help-rating-traffic = Brits stoplicht, per 100 g: goed tot { $low } { $unit }, matig tot { $high } { $unit }, daarboven niet goed.
nutrition-help-rating-dv-limit = Te beperken: { $low } % van de dagwaarde of minder is laag, { $high } % of meer is hoog.
nutrition-help-rating-dv-encourage = Meer van te nemen: { $high } % van de dagwaarde of meer is hoog.
nutrition-help-rating-dv-none = Geen dagwaarde voor deze voedingsstof: de FDA beoordeelt hem niet.
nutrition-help-rating-claim-max = EU-claim “{ $name }”: tot { $amount } { $unit } per 100 g.
nutrition-help-rating-claim-min = EU-claim “{ $name }”: vanaf { $amount } { $unit } per 100 g.
nutrition-claim-low-fat = vetarm
nutrition-claim-low-saturated-fat = arm aan verzadigd vet
nutrition-claim-low-sugars = suikerarm
nutrition-claim-low-sodium = zoutarm
nutrition-claim-source-of-fibre = bron van vezels
nutrition-claim-high-fibre = rijk aan vezels
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
nutrition-help-energy-what = Wat het voedingsmiddel het lichaam geeft, in kilocalorieën (kcal) of kilojoules (kJ).
nutrition-help-energy-use = Ze wordt berekend met vaste factoren: 9 kcal per gram vet, 4 voor koolhydraten en eiwit, 7 voor alcohol, 2 voor vezels.
nutrition-help-lipids-what = De vetten en oliën van een voedingsmiddel.
nutrition-help-lipids-use = Vet levert essentiële vetzuren, helpt bij de opname van vitamine A, D en E en geeft 9 kcal per gram, tegenover 4 voor koolhydraten en eiwit.
nutrition-help-saturated-fat-what = Vetten die vooral uit dierlijke bronnen komen, en uit palm- en kokosolie.
nutrition-help-saturated-fat-use = Te veel verhoogt het “slechte” LDL-cholesterol en het risico op hartziekten en beroerte. WHO: minder dan 10 % van de energie.
nutrition-help-saturated-fat-where = Vet vlees, worst en pasteien, boter, ghee en reuzel, kaas, room, chocolade, koekjes en gebak, palm- en kokosolie.
nutrition-help-unsaturated-fat-what = Vetten in vis, avocado en noten, en in zonnebloem-, soja-, raap- en olijfolie.
nutrition-help-unsaturated-fat-use = Beter dan verzadigd vet (WHO).
nutrition-help-monounsaturated-fat-what = Een soort onverzadigd vet.
nutrition-help-monounsaturated-fat-use = Helpt het hart te beschermen: houdt het “goede” HDL-cholesterol op peil en verlaagt het “slechte” LDL.
nutrition-help-monounsaturated-fat-where = Olijf- en raapolie, avocado’s, amandelen, paranoten, pinda’s.
nutrition-help-polyunsaturated-fat-what = Een soort onverzadigd vet. Er zijn twee hoofdtypen: omega-3 en omega-6.
nutrition-help-polyunsaturated-fat-use = Het lichaam kan sommige vormen niet zelf maken, dus een kleine hoeveelheid moet uit voeding komen.
nutrition-help-omega-3-what = Een meervoudig onverzadigd vet, onmisbaar in de voeding.
nutrition-help-omega-3-use = Minstens 2 porties vis per week worden aangeraden, waarvan één vette vis.
nutrition-help-omega-3-where = Vette vis (haring, forel, sardines, zalm, makreel), lijnzaad- en raapolie, walnoten.
nutrition-help-omega-6-what = Een meervoudig onverzadigd vet, onmisbaar in de voeding.
nutrition-help-omega-6-use = De meeste mensen krijgen er genoeg van binnen.
nutrition-help-omega-6-where = Raap-, maïs- en zonnebloemolie, walnoten, amandelen, cashewnoten.
nutrition-help-trans-fat-what = Onverzadigde vetzuren van industriële of natuurlijke oorsprong.
nutrition-help-trans-fat-use = Net als verzadigd vet verhogen ze het cholesterol. WHO: minder dan 1 % van de energie.
nutrition-help-trans-fat-where = Margarine, bak- en braadvet en gefrituurd voedsel; van nature in vlees en zuivel van herkauwers.
nutrition-help-cholesterol-what = Een vetachtige stof die het lichaam vooral in de lever maakt.
nutrition-help-cholesterol-use = Verzadigd vet en transvet verhogen het “slechte” LDL-cholesterol in het bloed.
nutrition-help-carbohydrates-what = Suikers, zetmeel en, afhankelijk van de standaard, vezels.
nutrition-help-carbohydrates-use = De belangrijkste energiebron van het lichaam (WHO): 4 kcal per gram.
nutrition-help-sugars-what = Vrije suikers zijn alle suikers die aan voeding en dranken worden toegevoegd, plus die in honing, siropen en sappen; de suikers in melk, fruit en groente tellen niet mee.
nutrition-help-sugars-use = Te veel doet je aankomen en veroorzaakt gaatjes. WHO: vrije suikers onder 10 % van de energie, ongeveer 50 g.
nutrition-help-sugars-where = Snoep, gebak, koekjes, chocolade, frisdrank, ontbijtgranen.
nutrition-help-starch-what = Het koolhydraat van zetmeelrijke voeding, onze belangrijkste bron van koolhydraten.
nutrition-help-starch-use = Zetmeelrijke voeding hoort iets meer dan een derde van je eten te zijn.
nutrition-help-starch-where = Aardappelen, brood, rijst, pasta, granen.
nutrition-help-fibers-what = Stoffen in de celwanden van groenten, fruit, peulvruchten en graankorrels.
nutrition-help-fibers-use = Ze helpen de spijsvertering en voorkomen verstopping. WHO: minstens 25 g per dag (30 g in het Verenigd Koninkrijk).
nutrition-help-fibers-where = Volkorenbrood en -pasta, bruine rijst, aardappelen met schil, peulvruchten, groenten, fruit, noten en zaden.
nutrition-help-proteins-what = De bouwstenen van het lichaam, zoals spieren.
nutrition-help-proteins-use = 10 tot 15 % van de energie is bij volwassenen meestal genoeg (WHO).
nutrition-help-salt-what = Keukenzout, natriumchloride. Zout = natrium × 2,5: 1 g zout bevat 400 mg natrium.
nutrition-help-salt-use = Te veel verhoogt de bloeddruk en het risico op een hartaanval en beroerte. WHO: minder dan 5 g per dag.
nutrition-help-salt-where = Brood, ontbijtgranen, vleesproducten, kant-en-klaarmaaltijden, sauzen, kaas.
nutrition-help-sodium-what = Het mineraal in keukenzout: 1 g zout bevat 400 mg natrium.
nutrition-help-sodium-use = WHO: minder dan 2000 mg natrium per dag.
