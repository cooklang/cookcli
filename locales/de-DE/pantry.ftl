# Pantry
pantry-title = Vorratskammer
pantry-intro = Was zu Hause ist, nach Aufbewahrungsort. Die Einkaufsliste lässt weg, was du schon hast.
pantry-unreadable = Die Vorratsdatei konnte nicht gelesen werden
pantry-empty = Ihre Vorratskammer ist leer
pantry-filters = Anzeigen
pantry-filter-all = Alle
pantry-filter-low = Wird knapp
pantry-filter-out = Nicht vorrätig
pantry-filter-expiring = Läuft bald ab
pantry-filter-empty = Kein Artikel passt zu diesem Filter.
pantry-stock-ok = Vorrätig
pantry-expired = Abgelaufen
pantry-expires-today = Läuft heute ab
pantry-expires-in =
    { $count ->
        [one] In { $count } Tag
       *[other] In { $count } Tagen
    }
pantry-section = { $name }
pantry-no-config = Keine Vorratskammer-Konfiguration gefunden
pantry-create-config = Erstellen Sie eine pantry.conf-Datei, um Ihren Bestand zu verfolgen
pantry-configure = Vorratskammer konfigurieren →
pantry-manage = Vorrat verwalten

# Pantry Item Fields
pantry-item-name = Artikelname
pantry-item-quantity = Menge
pantry-item-bought = Gekauft:
pantry-item-bought-date = Kaufdatum
pantry-item-expire = Verfällt:
pantry-item-expire-date = Verfallsdatum
pantry-item-low = Niedrig bei:
pantry-item-low-threshold = Mindestbestandsschwelle
pantry-placeholder-quantity = z. B. 500%g oder 2%L
pantry-placeholder-low = z. B. 100%g oder 2

# Pantry Actions
pantry-add-item = Artikel hinzufügen
pantry-add = Hinzufügen
pantry-edit-item = Artikel bearbeiten
pantry-remove-item = Artikel entfernen
pantry-remove-confirm = { $name } aus { $section } entfernen?
pantry-confirm-remove-template = %s aus %s entfernen?
pantry-mark-low = Als niedrig markieren
pantry-restock = Nachfüllen
pantry-save = Speichern
pantry-cancel = Abbrechen
pantry-section-label = Bereich
pantry-section-required = Gib dem Bereich einen Namen
pantry-general-quantity-only = Artikel vor dem ersten Bereich können nur eine Menge haben. Verschiebe ihn in einen Bereich, um Daten oder eine Mindestmenge anzugeben.
pantry-section-freezer = Gefrierschrank
pantry-section-fridge = Kühlschrank
pantry-section-pantry = Vorratskammer
pantry-section-spices = Gewürze
pantry-section-general = Allgemein
pantry-tab-items = Artikel
pantry-tab-text = Text
pantry-text-intro = Bearbeite die Datei direkt: eine [Abschnitt]-Zeile, dann ein Artikel pro Zeile, geschrieben name = "menge" oder name = {"{"} quantity = "…", bought = "…", expire = "…", low = "…" {"}"}. Zeilen, die mit # beginnen, sind Kommentare.
pantry-text-save = Datei speichern
pantry-text-saved = Gespeichert
pantry-text-conflict = Die Vorratsdatei wurde während deiner Bearbeitung anderswo geändert. Dein Text bleibt erhalten; speichere erneut, um diese Änderung damit zu ersetzen.
pantry-rename-section = Abschnitt umbenennen
pantry-section-name = Name des Abschnitts
pantry-failed-rename = Abschnitt konnte nicht umbenannt werden
pantry-failed-save-file = Datei konnte nicht gespeichert werden
pantry-optional = (optional)
pantry-failed-add = Artikel konnte nicht hinzugefügt werden
pantry-item-name-required = Gib dem Artikel einen Namen
pantry-item-exists = %s ist schon in %s
pantry-section-name-taken = Es gibt schon einen Bereich oder Artikel namens %s
pantry-failed-update = Artikel konnte nicht aktualisiert werden
pantry-failed-remove = Artikel konnte nicht entfernt werden
