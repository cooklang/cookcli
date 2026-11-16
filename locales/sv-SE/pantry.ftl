# Pantry
pantry-title = Skafferi Invetering
pantry-intro = Det som finns hemma, efter var det förvaras. Inköpslistan hoppar över det du redan har.
pantry-unreadable = Skafferifilen kunde inte läsas
pantry-empty = Ditt skafferi är tomt
pantry-filters = Visa
pantry-filter-all = Alla
pantry-filter-low = Snart slut
pantry-filter-out = Slut i lager
pantry-filter-expiring = Går snart ut
pantry-filter-empty = Ingen produkt matchar det här filtret.
pantry-stock-ok = I lager
pantry-expired = Utgånget
pantry-expires-today = Går ut i dag
pantry-expires-in =
    { $count ->
        [one] Om { $count } dag
       *[other] Om { $count } dagar
    }
pantry-section = { $name }
pantry-no-config = Ingen skaffer configuration hittades
pantry-create-config = Skapa en pantry.conf fil för att spåra din Invetering
pantry-configure = Konfigurera skafferi →
pantry-manage = Hantera skafferi

# Pantry Item Fields
pantry-item-name = Produkt namn
pantry-item-quantity = Antal
pantry-item-bought = Köpt:
pantry-item-bought-date = Köpt Datum
pantry-item-expire = Går ut:
pantry-item-expire-date = Utgångsdatum
pantry-item-low = Lite vid:
pantry-item-low-threshold = Litet Lager Tröskel
pantry-placeholder-quantity = t.ex. 500%g eller 2%L
pantry-placeholder-low = t.ex. 100%g eller 2

# Pantry Actions
pantry-add-item = Lägg till produkt
pantry-add = Lägg till
pantry-edit-item = Redigera produkt
pantry-remove-item = Radera produkt
pantry-remove-confirm = Radera { $name } från { $section }?
pantry-confirm-remove-template = Radera %s från %s?
pantry-mark-low = Markera som lite
pantry-restock = Återfyll
pantry-save = Spara
pantry-cancel = Avbryt
pantry-section-label = Sektion
pantry-section-required = Ge sektionen ett namn
pantry-general-quantity-only = Produkter före den första sektionen kan bara ha ett antal. Flytta den till en sektion för att ge den datum eller en lägstanivå.
pantry-section-freezer = Frys
pantry-section-fridge = Kyl
pantry-section-pantry = Skafferi
pantry-section-spices = Kryddor
pantry-section-general = Allmänt
pantry-tab-items = Varor
pantry-tab-text = Text
pantry-text-intro = Redigera filen direkt: en [sektion]-rad, sedan en vara per rad, skriven namn = "mängd" eller namn = {"{"} quantity = "…", bought = "…", expire = "…", low = "…" {"}"}. Rader som börjar med # är kommentarer.
pantry-text-save = Spara fil
pantry-text-saved = Sparat
pantry-text-conflict = Skafferifilen ändrades någon annanstans medan du redigerade. Din text finns kvar; spara igen för att ersätta den ändringen.
pantry-rename-section = Byt namn på sektion
pantry-section-name = Sektionens namn
pantry-failed-rename = Det gick inte att byta namn på sektionen
pantry-failed-save-file = Det gick inte att spara filen
pantry-optional = (valfritt)
pantry-failed-add = Kunde inte lägga till produkt
pantry-item-name-required = Ge produkten ett namn
pantry-item-exists = %s finns redan i %s
pantry-section-name-taken = Det finns redan en sektion eller produkt som heter %s
pantry-failed-update = Kunde inte uppdatera produkt
pantry-failed-remove = Kunde inte radera produkt
