# Shopping List
shopping-title = Boodschappenlijst
shopping-empty = Uw boodschappenlijst is leeg
shopping-no-items = Geen artikelen in de boodschappenlijst. Voeg recepten toe om te beginnen.
shopping-no-recipes = Geen recepten geselecteerd. Voeg recepten toe vanaf de receptpagina.
shopping-item-count =
    { $count ->
        [one] { $count } artikel
       *[other] { $count } artikelen
    }
shopping-from-recipe = Van { $recipeName }
shopping-clear-confirm = Weet u zeker dat u alle artikelen wilt verwijderen?
shopping-added = { $itemName } toegevoegd aan boodschappenlijst
shopping-removed = { $itemName } verwijderd van boodschappenlijst
shopping-cleared = Boodschappenlijst gewist
shopping-export = Exporteren
shopping-import = Importeren
shopping-mark-done = Markeer als gedaan
shopping-mark-undone = Markeer als niet gedaan
shopping-selected-recipes = Geselecteerde recepten
shopping-clear-all = Alles wissen
shopping-in-pantry = In uw voorraadkast
shopping-pantry-unused = Het voorraadbestand kon niet worden gelezen, dus er is niets uit uw voorraadkast van deze lijst afgetrokken.
shopping-pantry-open = Open de voorraadkast om het te herstellen
shopping-remove = Verwijderen
shopping-add-recipes-first = Voeg eerst enkele recepten toe
shopping-failed-to-generate = Boodschappenlijst kon niet worden gegenereerd
shopping-warnings = Sommige recepten verdienen een blik; er ontbreekt misschien iets op de lijst:
shopping-warning-not-a-number = { $recipe }: het recept { $reference } kon niet worden omgerekend naar { $amount }, want dat is geen getal. Het staat op de lijst zoals het geschreven is, niet als zijn ingrediënten.
shopping-warning-no-servings = { $recipe }: het recept { $reference } kon niet worden omgerekend naar { $amount }: het noemt geen heel aantal porties, zoals `servings: 4`. Het staat op de lijst zoals het geschreven is, niet als zijn ingrediënten.
shopping-warning-no-yield = { $recipe }: het recept { $reference } kon niet worden omgerekend naar { $amount }: het zegt niet hoeveel het oplevert. Voeg een opbrengst toe als getal%eenheid, zoals `yield: 500%{ $unit }`. Het staat op de lijst zoals het geschreven is, niet als zijn ingrediënten.
shopping-warning-yield-unit = { $recipe }: het recept { $reference } kon niet worden omgerekend naar { $amount }: de opbrengst staat in { $unit }, vraag het dus in { $unit }. Het staat op de lijst zoals het geschreven is, niet als zijn ingrediënten.
shopping-warning-non-positive-base = { $recipe }: het recept { $reference } kon niet worden omgerekend naar { $amount }: het declareert `{ $declared }`, er is dus niets om vanuit om te rekenen. Het staat op de lijst zoals het geschreven is, niet als zijn ingrediënten.
shopping-failed-to-add = Toevoegen aan boodschappenlijst mislukt
shopping-error = Fout
shopping-print = Afdrukken
shopping-include-in-list = Opnemen in boodschappenlijst
shopping-optional = (optioneel)
shopping-copy = Kopiëren
shopping-copied = Gekopieerd!
shopping-copy-failed = Kopiëren mislukt
shopping-copy-options = Kopieeropties
shopping-copy-include-aisles = Schapnamen opnemen
shopping-copy-include-amounts = Hoeveelheden opnemen
