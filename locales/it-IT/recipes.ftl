# Recipe Listing
recipes-title = Tutte le ricette
recipes-directory-title = { $name }
recipes-empty = Nessuna ricetta trovata in questa cartella.
recipes-count =
    { $count ->
        [one] { $count } ricetta
       *[other] { $count } ricette
    }

# Recipe Display
recipe-ingredients = Ingredienti
recipe-steps = Preparazione
recipe-notes = Note
recipe-cookware = Utensili
recipe-optional = (facoltativo)
recipe-timers = Timer
recipe-tags = Tag
recipe-add-to-shopping = Aggiungi alla lista della spesa
recipe-add-all-to-shopping = Aggiungi tutto alla lista della spesa
recipe-scale-label = Scala
recipe-servings-label = porzioni
recipe-written-for =
    { $count ->
        [one] Scritta per { $count } porzione
       *[other] Scritta per { $count } porzioni
    }
recipe-written-for-hint = Le quantità sono ricalcolate dalla ricetta originale. Tempi di cottura, dimensioni delle teglie e condimento potrebbero richiedere modifiche. Seleziona per tornare alle porzioni originali.
recipe-scaled-from-original = ×{ $factor } della ricetta originale
recipe-scaled-from-original-hint = Le quantità sono ricalcolate dalla ricetta originale. Tempi di cottura, dimensioni delle teglie e condimento potrebbero richiedere modifiche. Seleziona per tornare alla ricetta originale.
recipe-print = Stampa ricetta
recipe-added = Aggiunto!
recipe-main-section = Principale
recipe-preparation = preparazione

# Recipe Metadata
meta-course = Portata
meta-cuisine = Cucina
meta-diet = Dieta
meta-author = Autore
meta-source = Fonte
meta-prep-time = Tempo di preparazione
meta-cook-time = Tempo di cottura
meta-total-time = Tempo totale
meta-servings = Porzioni
meta-difficulty = Difficoltà
meta-description = Descrizione

# Recipe Types
recipe-type-menu = Menù
recipe-type-plan = Piano pasti
plan-outside = Fuori da questo piano
plan-nothing-planned = Niente in programma
plan-add-to = Aggiungi a { $meal }, { $day }
plan-line-actions = Modifica
plan-move-to = Sposta in…
plan-copy-to = Copia in…
plan-remove = Rimuovi
plan-move-title = Sposta in un altro giorno
plan-copy-title = Copia in un altro giorno
plan-move = Sposta
plan-copy = Copia
plan-target-day = Giorno
plan-target-meal = Pasto
plan-changed = Il piano è cambiato da quando la pagina è stata caricata. Ricaricala per vederlo, poi riprova.
plan-save-failed = Impossibile salvare il piano.
plan-saved = Piano salvato
plan-reload = Ricarica
plan-pick-label = Giorni per la lista della spesa
plan-pick-all = Tutti
plan-pick-week = Prossimi 7 giorni
plan-pick-none = Nessuno
plan-add-days = Aggiungi i giorni alla lista della spesa

# Today's Menu Banner
todays-menu-title = Il menù di oggi
todays-menu-from = Da
todays-menu-view = Vedi menù

# Sort Controls
sort-by = Ordina per:
sort-name = Nome
sort-modified = Data di modifica
sort-created = Data di creazione
sort-direction-toggle = Inverti l'ordine
random-recipe = Ricetta a caso

# Nutrition panel and file information
meta-created-by = Creato da
meta-created-at = Creato il
meta-modified-by = Modificato da
meta-modified-at = Modificato il
meta-file-title = File
meta-translated-from = Tradotto da
nutrition-per-serving = per porzione
nutrition-proteins = proteine
nutrition-lipids = grassi
nutrition-saturated-fat = grassi saturi
nutrition-unsaturated-fat = grassi insaturi
nutrition-monounsaturated-fat = grassi monoinsaturi
nutrition-polyunsaturated-fat = grassi polinsaturi
nutrition-trans-fat = grassi trans
nutrition-starch = amido
nutrition-carbohydrates = carboidrati
nutrition-sugars = zuccheri
nutrition-fibers = fibre
nutrition-salt = sale
nutrition-title = Valori nutrizionali
nutrition-per-100g = per 100 g
nutrition-whole-recipe = ricetta intera
nutrition-energy = energia
nutrition-of-which = di cui
nutrition-for = per
nutrition-full-table = Tabella completa
nutrition-legend = Valutazione per la salute, per 100 g:
nutrition-rating-good = buono
nutrition-rating-medium = medio
nutrition-rating-bad = scarso
nutrition-legend-good = buono per la salute
nutrition-legend-medium = medio
nutrition-legend-bad = non buono per la salute
nutrition-legend-unrated = nessuna soglia

# Standards: the layout, intakes and ratings come from the standard in use.
nutrition-more-info = Più informazioni
nutrition-less-info = Meno informazioni
nutrition-collapse-all = Comprimi tutto
nutrition-expand-all = Espandi tutto
nutrition-tree-lines = Linee ad albero
nutrition-fold = Comprimi o espandi
nutrition-legend-claims = Indicazioni UE, per 100 g:
nutrition-legend-dv = Percentuale del valore giornaliero, per porzione:
nutrition-well-present = Vitamine e minerali ben presenti
nutrition-well-source = fonte di
nutrition-well-high = ricco di
nutrition-well-rule-100g = almeno il { $source } % dell'assunzione di riferimento per 100 g («fonte di»), { $high } % o più («ricco di»)
nutrition-well-rule-serving = almeno il { $source } % del valore giornaliero per porzione («fonte di»), { $high } % o più («ricco di»)
nutrition-well-rule-serving-high = { $high } % o più del valore giornaliero per porzione («ricco di»)
nutrition-standard-note = Norma: { $name } ({ $id }.yaml)
nutrition-carbs-without-fibre = I carboidrati sono mostrati senza le fibre (la ricetta le include).
nutrition-group-macronutrients = Macronutrienti
nutrition-group-other = Altri componenti
nutrition-group-minerals = Minerali
nutrition-group-macro-minerals = Macrominerali
nutrition-group-trace-elements = Oligoelementi
nutrition-group-vitamins = Vitamine
nutrition-group-fat-soluble = Liposolubili
nutrition-group-water-soluble = Idrosolubili
nutrition-group-other-data = Altri dati
nutrition-cholesterol = colesterolo
nutrition-sodium = sodio
nutrition-added-sugars = zuccheri aggiunti
nutrition-polyols = polioli
nutrition-omega-3 = omega-3
nutrition-omega-6 = omega-6
nutrition-calcium = calcio
nutrition-phosphorus = fosforo
nutrition-magnesium = magnesio
nutrition-potassium = potassio
nutrition-chloride = cloruro
nutrition-iron = ferro
nutrition-zinc = zinco
nutrition-copper = rame
nutrition-manganese = manganese
nutrition-fluoride = fluoruro
nutrition-selenium = selenio
nutrition-chromium = cromo
nutrition-molybdenum = molibdeno
nutrition-iodine = iodio
nutrition-choline = colina
nutrition-vitamin-a = vitamina A
nutrition-vitamin-d = vitamina D
nutrition-vitamin-e = vitamina E
nutrition-vitamin-k = vitamina K
nutrition-vitamin-c = vitamina C
nutrition-vitamin-b1 = vitamina B1
nutrition-vitamin-b2 = vitamina B2
nutrition-vitamin-b3 = vitamina B3
nutrition-vitamin-b5 = vitamina B5
nutrition-vitamin-b6 = vitamina B6
nutrition-vitamin-b8 = vitamina B8
nutrition-vitamin-b9 = vitamina B9
nutrition-vitamin-b12 = vitamina B12

# Explanation bubbles: what each text says comes from the source its -source message names.
nutrition-help-what = Che cos’è
nutrition-help-use = A cosa serve
nutrition-help-where = Dove si trova
nutrition-help-source = Fonte: { $source }
nutrition-help-source-todo = Fonte: da documentare
nutrition-help-reference = Assunzione di riferimento ({ $standard }): { $amount } al giorno.
nutrition-help-reference-none = Nessuna assunzione di riferimento nello standard { $standard }.
nutrition-help-recipe = Questa ricetta: { $amount } per porzione.
nutrition-help-recipe-pct = Questa ricetta: { $amount } per porzione, il { $pct } % dell’assunzione di riferimento.
nutrition-help-rating-traffic = Semaforo britannico, per 100 g: buono fino a { $low } { $unit }, medio fino a { $high } { $unit }, oltre non è buono.
nutrition-help-rating-dv-limit = Da limitare: { $low } % del valore giornaliero o meno è basso, { $high } % o più è alto.
nutrition-help-rating-dv-encourage = Da favorire: { $high } % del valore giornaliero o più è alto.
nutrition-help-rating-dv-none = Nessun valore giornaliero per questo nutriente: la FDA non lo valuta.
nutrition-help-rating-claim-max = Indicazione UE «{ $name }»: fino a { $amount } { $unit } per 100 g.
nutrition-help-rating-claim-min = Indicazione UE «{ $name }»: da { $amount } { $unit } per 100 g.
nutrition-claim-low-fat = a basso contenuto di grassi
nutrition-claim-low-saturated-fat = a basso contenuto di grassi saturi
nutrition-claim-low-sugars = a basso contenuto di zuccheri
nutrition-claim-low-sodium = a basso contenuto di sale
nutrition-claim-source-of-fibre = fonte di fibre
nutrition-claim-high-fibre = ad alto contenuto di fibre
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
nutrition-help-energy-what = Ciò che l’alimento apporta al corpo, in chilocalorie (kcal) o chilojoule (kJ).
nutrition-help-energy-use = Si calcola con fattori fissi: 9 kcal per grammo di grassi, 4 di carboidrati e di proteine, 7 di alcol, 2 di fibre.
nutrition-help-lipids-what = I grassi e gli oli di un alimento.
nutrition-help-lipids-use = I grassi apportano acidi grassi essenziali, aiutano ad assorbire le vitamine A, D ed E e forniscono 9 kcal per grammo, contro 4 di carboidrati e proteine.
nutrition-help-saturated-fat-what = Grassi che provengono soprattutto da fonti animali, e dall’olio di palma e di cocco.
nutrition-help-saturated-fat-use = Troppi fanno salire il colesterolo «cattivo» (LDL) e il rischio di malattie cardiache e ictus. OMS: meno del 10 % dell’energia.
nutrition-help-saturated-fat-where = Carne grassa, salsicce e torte salate, burro, ghi e strutto, formaggio, panna, cioccolato, biscotti e dolci, olio di palma e di cocco.
nutrition-help-unsaturated-fat-what = Grassi presenti nel pesce, nell’avocado e nella frutta secca, e negli oli di girasole, soia, colza e oliva.
nutrition-help-unsaturated-fat-use = Preferibili ai grassi saturi (OMS).
nutrition-help-monounsaturated-fat-what = Un tipo di grasso insaturo.
nutrition-help-monounsaturated-fat-use = Aiuta a proteggere il cuore: mantiene il colesterolo «buono» (HDL) e abbassa quello «cattivo» (LDL).
nutrition-help-monounsaturated-fat-where = Olio d’oliva e di colza, avocado, mandorle, noci del Brasile, arachidi.
nutrition-help-polyunsaturated-fat-what = Un tipo di grasso insaturo. Ce ne sono due tipi principali: omega-3 e omega-6.
nutrition-help-polyunsaturated-fat-use = Il corpo non può produrne alcune forme, quindi una piccola quantità deve venire dal cibo.
nutrition-help-omega-3-what = Un grasso polinsaturo, essenziale nella dieta.
nutrition-help-omega-3-use = Si consigliano almeno 2 porzioni di pesce a settimana, una delle quali di pesce azzurro.
nutrition-help-omega-3-where = Pesce azzurro (aringa, trota, sardine, salmone, sgombro), oli di lino e di colza, noci.
nutrition-help-omega-6-what = Un grasso polinsaturo, essenziale nella dieta.
nutrition-help-omega-6-use = La maggior parte delle persone ne assume abbastanza.
nutrition-help-omega-6-where = Oli di colza, di mais e di girasole, noci, mandorle, anacardi.
nutrition-help-trans-fat-what = Acidi grassi insaturi di origine industriale o naturale.
nutrition-help-trans-fat-use = Come i grassi saturi, fanno salire il colesterolo. OMS: meno dell’1 % dell’energia.
nutrition-help-trans-fat-where = Margarina, grassi vegetali e fritti; naturalmente nella carne e nei latticini dei ruminanti.
nutrition-help-cholesterol-what = Una sostanza grassa prodotta soprattutto dal fegato.
nutrition-help-cholesterol-use = I grassi saturi e trans fanno salire il colesterolo «cattivo» (LDL) nel sangue.
nutrition-help-carbohydrates-what = Zuccheri, amidi e, secondo lo standard, fibre.
nutrition-help-carbohydrates-use = La principale fonte di energia del corpo (OMS): 4 kcal per grammo.
nutrition-help-sugars-what = Gli zuccheri liberi sono tutti quelli aggiunti ad alimenti e bevande, più quelli di miele, sciroppi e succhi; quelli di latte, frutta e verdura non contano.
nutrition-help-sugars-use = Troppi fanno ingrassare e causano carie. OMS: zuccheri liberi sotto il 10 % dell’energia, circa 50 g.
nutrition-help-sugars-where = Caramelle, dolci, biscotti, cioccolato, bevande gassate, cereali per la colazione.
nutrition-help-starch-what = Il carboidrato degli alimenti amidacei, la nostra principale fonte di carboidrati.
nutrition-help-starch-use = Gli alimenti amidacei dovrebbero costituire poco più di un terzo di ciò che si mangia.
nutrition-help-starch-where = Patate, pane, riso, pasta, cereali.
nutrition-help-fibers-what = Sostanze presenti nelle pareti cellulari di verdure, frutta, legumi e cereali.
nutrition-help-fibers-use = Aiutano la digestione e prevengono la stitichezza. OMS: almeno 25 g al giorno (30 g nel Regno Unito).
nutrition-help-fibers-where = Pane e pasta integrali, riso integrale, patate con la buccia, legumi, verdure, frutta, frutta secca e semi.
nutrition-help-proteins-what = I mattoni del corpo, come i muscoli.
nutrition-help-proteins-use = Il 10-15 % dell’energia basta in genere per gli adulti (OMS).
nutrition-help-salt-what = Il sale da cucina, cloruro di sodio. Sale = sodio × 2,5: 1 g di sale contiene 400 mg di sodio.
nutrition-help-salt-use = Troppo fa salire la pressione e il rischio di infarto e ictus. OMS: meno di 5 g al giorno.
nutrition-help-salt-where = Pane, cereali per la colazione, prodotti a base di carne, piatti pronti, salse, formaggio.
nutrition-help-sodium-what = Il minerale del sale da cucina: 1 g di sale contiene 400 mg di sodio.
nutrition-help-sodium-use = OMS: meno di 2000 mg di sodio al giorno.
