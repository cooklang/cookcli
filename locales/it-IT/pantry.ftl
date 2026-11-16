# Pantry
pantry-title = Inventario della dispensa
pantry-intro = Quello che hai in casa, per luogo di conservazione. La lista della spesa lascia fuori quello che hai già.
pantry-unreadable = Impossibile leggere il file della dispensa
pantry-empty = La tua dispensa è vuota
pantry-filters = Mostra
pantry-filter-all = Tutto
pantry-filter-low = In esaurimento
pantry-filter-out = Esaurito
pantry-filter-expiring = Scade presto
pantry-filter-empty = Nessun articolo corrisponde a questo filtro.
pantry-stock-ok = Disponibile
pantry-expired = Scaduto
pantry-expires-today = Scade oggi
pantry-expires-in =
    { $count ->
        [one] Tra { $count } giorno
       *[other] Tra { $count } giorni
    }
pantry-section = { $name }
pantry-no-config = Nessuna configurazione della dispensa trovata
pantry-create-config = Crea un file pantry.conf per tenere traccia del tuo inventario
pantry-configure = Configura la dispensa →
pantry-manage = Gestisci dispensa

# Pantry Item Fields
pantry-item-name = Nome dell'articolo
pantry-item-quantity = Quantità
pantry-item-bought = Acquistato:
pantry-item-bought-date = Data di acquisto
pantry-item-expire = Scade:
pantry-item-expire-date = Data di scadenza
pantry-item-low = In esaurimento a:
pantry-item-low-threshold = Soglia di scorta minima
pantry-placeholder-quantity = es. 500%g o 2%L
pantry-placeholder-low = es. 100%g o 2

# Pantry Actions
pantry-add-item = Aggiungi articolo
pantry-add = Aggiungi
pantry-edit-item = Modifica articolo
pantry-remove-item = Rimuovi articolo
pantry-remove-confirm = Rimuovere { $name } da { $section }?
pantry-confirm-remove-template = Rimuovere %s da %s?
pantry-mark-low = Segna in esaurimento
pantry-restock = Rifornisci
pantry-save = Salva
pantry-cancel = Annulla
pantry-section-label = Sezione
pantry-section-required = Dai un nome alla sezione
pantry-general-quantity-only = Gli articoli prima della prima sezione possono avere solo una quantità. Spostalo in una sezione per dargli date o una soglia minima.
pantry-section-freezer = Congelatore
pantry-section-fridge = Frigorifero
pantry-section-pantry = Dispensa
pantry-section-spices = Spezie
pantry-section-general = Generale
pantry-tab-items = Articoli
pantry-tab-text = Testo
pantry-text-intro = Modifica direttamente il file: una riga [sezione], poi un articolo per riga, scritto nome = "quantità" oppure nome = {"{"} quantity = "…", bought = "…", expire = "…", low = "…" {"}"}. Le righe che iniziano con # sono commenti.
pantry-text-save = Salva file
pantry-text-saved = Salvato
pantry-text-conflict = Il file della dispensa è stato modificato altrove mentre lo stavi modificando. Il tuo testo è conservato; salva di nuovo per sostituire quella modifica.
pantry-rename-section = Rinomina sezione
pantry-section-name = Nome della sezione
pantry-failed-rename = Impossibile rinominare la sezione
pantry-failed-save-file = Impossibile salvare il file
pantry-optional = (facoltativo)
pantry-failed-add = Impossibile aggiungere l'articolo
pantry-item-name-required = Dai un nome all'articolo
pantry-item-exists = %s è già in %s
pantry-section-name-taken = Esiste già una sezione o un articolo chiamato %s
pantry-failed-update = Impossibile aggiornare l'articolo
pantry-failed-remove = Impossibile rimuovere l'articolo
