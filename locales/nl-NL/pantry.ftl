# Pantry
pantry-title = Voorraadkast
pantry-intro = Wat er in huis is, per bewaarplek. De boodschappenlijst laat weg wat je al hebt.
pantry-unreadable = Het voorraadbestand kon niet worden gelezen
pantry-empty = Uw voorraadkast is leeg
pantry-filters = Tonen
pantry-filter-all = Alles
pantry-filter-low = Bijna op
pantry-filter-out = Niet op voorraad
pantry-filter-expiring = Verloopt binnenkort
pantry-filter-empty = Geen artikel past bij dit filter.
pantry-stock-ok = Op voorraad
pantry-expired = Verlopen
pantry-expires-today = Verloopt vandaag
pantry-expires-in =
    { $count ->
        [one] Over { $count } dag
       *[other] Over { $count } dagen
    }
pantry-section = { $name }
pantry-no-config = Geen voorraadkast-configuratie gevonden
pantry-create-config = Maak een pantry.conf-bestand om uw voorraad bij te houden
pantry-configure = Voorraadkast configureren →
pantry-manage = Voorraadkast beheren

# Pantry Item Fields
pantry-item-name = Artikelnaam
pantry-item-quantity = Hoeveelheid
pantry-item-bought = Gekocht:
pantry-item-bought-date = Aankoopdatum
pantry-item-expire = Verloopt:
pantry-item-expire-date = Vervaldatum
pantry-item-low = Laag bij:
pantry-item-low-threshold = Lage voorraaddrempel
pantry-placeholder-quantity = bijv. 500%g of 2%L
pantry-placeholder-low = bijv. 100%g of 2

# Pantry Actions
pantry-add-item = Artikel toevoegen
pantry-add = Toevoegen
pantry-edit-item = Artikel bewerken
pantry-remove-item = Artikel verwijderen
pantry-remove-confirm = { $name } verwijderen uit { $section }?
pantry-confirm-remove-template = %s uit %s verwijderen?
pantry-mark-low = Markeer als laag
pantry-restock = Aanvullen
pantry-save = Opslaan
pantry-cancel = Annuleren
pantry-section-label = Sectie
pantry-section-required = Geef de sectie een naam
pantry-general-quantity-only = Artikelen vóór de eerste sectie kunnen alleen een hoeveelheid hebben. Verplaats het naar een sectie om data of een minimum op te geven.
pantry-section-freezer = Vriezer
pantry-section-fridge = Koelkast
pantry-section-pantry = Voorraadkast
pantry-section-spices = Kruiden
pantry-section-general = Algemeen
pantry-tab-items = Artikelen
pantry-tab-text = Tekst
pantry-text-intro = Bewerk het bestand direct: een [sectie]-regel, daarna één artikel per regel, geschreven als naam = "hoeveelheid" of naam = {"{"} quantity = "…", bought = "…", expire = "…", low = "…" {"}"}. Regels die met # beginnen zijn opmerkingen.
pantry-text-save = Bestand opslaan
pantry-text-saved = Opgeslagen
pantry-text-conflict = Het voorraadbestand is elders gewijzigd terwijl je aan het bewerken was. Je tekst blijft bewaard; sla opnieuw op om die wijziging te vervangen.
pantry-rename-section = Sectie hernoemen
pantry-section-name = Naam van de sectie
pantry-failed-rename = Sectie hernoemen mislukt
pantry-failed-save-file = Bestand opslaan mislukt
pantry-optional = (optioneel)
pantry-failed-add = Artikel kon niet worden toegevoegd
pantry-item-name-required = Geef het artikel een naam
pantry-item-exists = %s staat al in %s
pantry-section-name-taken = Er is al een sectie of artikel met de naam %s
pantry-failed-update = Artikel kon niet worden bijgewerkt
pantry-failed-remove = Artikel kon niet worden verwijderd
