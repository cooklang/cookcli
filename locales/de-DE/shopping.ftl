# Shopping List
shopping-title = Einkaufsliste
shopping-empty = Ihre Einkaufsliste ist leer
shopping-no-items = Keine Artikel in der Einkaufsliste. Fügen Sie Rezepte hinzu, um zu beginnen.
shopping-no-recipes = Keine Rezepte ausgewählt. Fügen Sie Rezepte von der Rezeptseite hinzu.
shopping-item-count =
    { $count ->
        [one] { $count } Artikel
       *[other] { $count } Artikel
    }
shopping-from-recipe = Aus { $recipeName }
shopping-clear-confirm = Möchten Sie wirklich alle Artikel löschen?
shopping-added = { $itemName } zur Einkaufsliste hinzugefügt
shopping-removed = { $itemName } von der Einkaufsliste entfernt
shopping-cleared = Einkaufsliste geleert
shopping-export = Exportieren
shopping-import = Importieren
shopping-mark-done = Als erledigt markieren
shopping-mark-undone = Als unerledigt markieren
shopping-selected-recipes = Ausgewählte Rezepte
shopping-clear-all = Alles löschen
shopping-in-pantry = In Ihrer Vorratskammer
shopping-pantry-unused = Die Vorratsdatei konnte nicht gelesen werden, daher wurde nichts aus Ihrer Vorratskammer von dieser Liste abgezogen.
shopping-pantry-open = Vorratskammer öffnen, um es zu beheben
shopping-remove = Entfernen
shopping-add-recipes-first = Bitte fügen Sie zuerst einige Rezepte hinzu
shopping-failed-to-generate = Einkaufsliste konnte nicht erstellt werden
shopping-warnings = Einige Rezepte sollten Sie prüfen, der Liste fehlt vielleicht etwas:
shopping-warning-not-a-number = { $recipe }: Das Rezept { $reference } ließ sich nicht auf { $amount } umrechnen, denn das ist keine Zahl. Es steht so auf der Liste, wie es geschrieben ist, nicht als seine Zutaten.
shopping-warning-no-servings = { $recipe }: Das Rezept { $reference } ließ sich nicht auf { $amount } umrechnen: Es nennt keine ganze Zahl an Portionen, etwa `servings: 4`. Es steht so auf der Liste, wie es geschrieben ist, nicht als seine Zutaten.
shopping-warning-no-yield = { $recipe }: Das Rezept { $reference } ließ sich nicht auf { $amount } umrechnen: Es gibt nicht an, wie viel es ergibt. Ergänzen Sie eine Ausbeute im Format Zahl%Einheit, etwa `yield: 500%{ $unit }`. Es steht so auf der Liste, wie es geschrieben ist, nicht als seine Zutaten.
shopping-warning-yield-unit = { $recipe }: Das Rezept { $reference } ließ sich nicht auf { $amount } umrechnen: Seine Ausbeute ist in { $unit } angegeben, fordern Sie es also in { $unit } an. Es steht so auf der Liste, wie es geschrieben ist, nicht als seine Zutaten.
shopping-warning-non-positive-base = { $recipe }: Das Rezept { $reference } ließ sich nicht auf { $amount } umrechnen: Es deklariert `{ $declared }`, es gibt also nichts, von dem aus umgerechnet werden könnte. Es steht so auf der Liste, wie es geschrieben ist, nicht als seine Zutaten.
shopping-failed-to-add = Hinzufügen zur Einkaufsliste fehlgeschlagen
shopping-error = Fehler
shopping-print = Drucken
shopping-include-in-list = In die Einkaufsliste aufnehmen
shopping-copy = Kopieren
shopping-copied = Kopiert!
shopping-copy-failed = Kopieren fehlgeschlagen
shopping-copy-options = Kopieroptionen
shopping-copy-include-aisles = Regalnamen einschließen
shopping-copy-include-amounts = Mengen einschließen
