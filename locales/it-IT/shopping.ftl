# Shopping List
shopping-title = Lista della spesa
shopping-empty = La tua lista della spesa è vuota
shopping-no-items = Nessun articolo nella lista della spesa. Aggiungi delle ricette per iniziare.
shopping-no-recipes = Nessuna ricetta selezionata. Aggiungi le ricette dalla pagina della ricetta.
shopping-item-count =
    { $count ->
        [one] { $count } articolo
       *[other] { $count } articoli
    }
shopping-from-recipe = Da { $recipeName }
shopping-clear-confirm = Vuoi davvero rimuovere tutti gli articoli?
shopping-added = { $itemName } aggiunto alla lista della spesa
shopping-removed = { $itemName } rimosso dalla lista della spesa
shopping-cleared = Lista della spesa svuotata
shopping-export = Esporta
shopping-import = Importa
shopping-mark-done = Segna come fatto
shopping-mark-undone = Segna come da fare
shopping-selected-recipes = Ricette selezionate
shopping-clear-all = Svuota tutto
shopping-in-pantry = Già in dispensa
shopping-pantry-unused = Impossibile leggere il file della dispensa, quindi nulla della tua dispensa è stato tolto da questa lista.
shopping-pantry-open = Apri la dispensa per correggerlo
shopping-remove = Rimuovi
shopping-add-recipes-first = Aggiungi prima qualche ricetta
shopping-failed-to-generate = Impossibile generare la lista della spesa
shopping-warnings = Alcune ricette vanno controllate: alla lista potrebbe mancare qualcosa:
shopping-warning-not-a-number = { $recipe }: impossibile adattare la ricetta { $reference } a { $amount }, che non è un numero. È nella lista così come è scritta, non come i suoi ingredienti.
shopping-warning-no-servings = { $recipe }: impossibile adattare la ricetta { $reference } a { $amount }: non indica un numero intero di porzioni, come `servings: 4`. È nella lista così come è scritta, non come i suoi ingredienti.
shopping-warning-no-yield = { $recipe }: impossibile adattare la ricetta { $reference } a { $amount }: non dice quanto rende. Aggiungi una resa nel formato numero%unità, come `yield: 500%{ $unit }`. È nella lista così come è scritta, non come i suoi ingredienti.
shopping-warning-yield-unit = { $recipe }: impossibile adattare la ricetta { $reference } a { $amount }: la sua resa è in { $unit }, quindi richiedila in { $unit }. È nella lista così come è scritta, non come i suoi ingredienti.
shopping-failed-to-add = Impossibile aggiungere alla lista della spesa
shopping-error = Errore
shopping-print = Stampa
shopping-include-in-list = Includi nella lista della spesa
shopping-optional = (facoltativo)
shopping-copy = Copia
shopping-copied = Copiato!
shopping-copy-failed = Copia non riuscita
shopping-copy-options = Opzioni di copia
shopping-copy-include-aisles = Includi i nomi dei reparti
shopping-copy-include-amounts = Includi le quantità
