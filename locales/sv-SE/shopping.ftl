# Shopping List
shopping-title = Handlingslista
shopping-empty = Din handlingslista är tom
shopping-no-items = Inga produkter i din handlingslista. Lägg till recept för att börja.
shopping-no-recipes = Inga recept markerade. Lägg till recept från recept sidan.
shopping-item-count =
    { $count ->
        [one] { $count } produkt
       *[other] { $count } produkter
    }
shopping-from-recipe = Från { $recipeName }
shopping-clear-confirm = Är du säker på att du vill ta bort alla produkter?
shopping-added = La till { $itemName } till handlingslistan
shopping-removed = Tog bort { $itemName } från handlingslistan
shopping-cleared = Handlingslista rensad
shopping-export = Exportera
shopping-import = Importera
shopping-mark-done = Markera som klar
shopping-mark-undone = Markera som ej klar
shopping-selected-recipes = Valda recept
shopping-clear-all = Rensa alla
shopping-in-pantry = I ditt skafferi
shopping-pantry-unused = Skafferifilen kunde inte läsas, så inget i ditt skafferi har dragits av från den här listan.
shopping-pantry-open = Öppna skafferiet för att rätta det
shopping-remove = Radera
shopping-add-recipes-first = Vänligen lägg till recept först
shopping-failed-to-generate = Misslyckades att generera handlingslista
shopping-warnings = Några recept behöver ses över, listan kan sakna något:
shopping-warning-not-a-number = { $recipe }: receptet { $reference } kunde inte räknas om till { $amount }, som inte är ett tal. Det står på listan som det är skrivet, inte som sina ingredienser.
shopping-warning-no-servings = { $recipe }: receptet { $reference } kunde inte räknas om till { $amount }: det anger inget helt antal portioner, som `servings: 4`. Det står på listan som det är skrivet, inte som sina ingredienser.
shopping-warning-no-yield = { $recipe }: receptet { $reference } kunde inte räknas om till { $amount }: det anger inte hur mycket det ger. Lägg till ett utbyte skrivet tal%enhet, som `yield: 500%{ $unit }`. Det står på listan som det är skrivet, inte som sina ingredienser.
shopping-warning-yield-unit = { $recipe }: receptet { $reference } kunde inte räknas om till { $amount }: dess utbyte anges i { $unit }, så be om det i { $unit }. Det står på listan som det är skrivet, inte som sina ingredienser.
shopping-failed-to-add = Misslyckades att lägga till handlingslista
shopping-error = Fel
shopping-print = Skriv ut
shopping-include-in-list = Ta med i handlingslistan
shopping-copy = Kopiera
shopping-copied = Kopierad!
shopping-copy-failed = Kopiering misslyckades
shopping-copy-options = Kopieringsalternativ
shopping-copy-include-aisles = Inkludera hyllnamn
shopping-copy-include-amounts = Inkludera mängder
