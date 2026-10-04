# Navigation
nav-recipes = Recept
nav-shopping-list = Handlingslista
nav-pantry = Skafferi
nav-preferences = Egenskaper

# Search
search-placeholder = Sök recept...
search-no-results = Inga recept hittades

# Common Actions
action-add = Lägg till
action-remove = Ta bort
action-edit = Redigera
action-save = Spara
action-cancel = Avbryt
action-back = Tillbaka
action-delete = Radera
action-clear = Rensa
action-print = Skriv ut
action-preview = Förhandsgranska
action-done = Klar

# Common Labels
label-scale = Skala
label-servings = Portioner
label-time = Tid
label-difficulty = Svårighetsgrad
label-name = Namn
label-description = Beskrivning

# Editor
editor-unsaved-changes = Osparade ändringar
editor-saved = Sparad
editor-saving = Sparar...
editor-save-failed = Spara misslyckades
editor-placeholder = Ange ditt recept här...

# Editor toolbar
editor-toolbar-label = Cooklang-formatering
editor-toolbar-inline = Element i texten
editor-toolbar-block = Radelement
editor-toolbar-ingredient = Ingrediens
editor-toolbar-ingredient-title = Infoga en ingrediens (@), eller gör markeringen till en ingrediens
editor-toolbar-cookware = Köksredskap
editor-toolbar-cookware-title = Infoga ett köksredskap (#), eller gör markeringen till ett köksredskap
editor-toolbar-timer = Timer
editor-toolbar-timer-title = Infoga en timer i minuter (~), eller namnge den efter markeringen
editor-toolbar-section = Avsnitt
editor-toolbar-section-title = Börja ett nytt avsnitt (== Avsnitt ==), namngivet efter markeringen
editor-toolbar-section-default = Avsnitt
editor-toolbar-note = Anteckning
editor-toolbar-note-title = Gör de aktuella raderna till en anteckning (>), eller tillbaka till steg
editor-toolbar-comment = Kommentar
editor-toolbar-comment-title = Kommentera bort de aktuella raderna (--), eller markeringen inom en rad
editor-toolbar-metadata = Metadata
editor-toolbar-metadata-title = Lägg till en metadatarad i frontmatter (---)
editor-toolbar-menu = Menyelement
editor-toolbar-other = Övriga element
editor-toolbar-day = Dag
editor-toolbar-day-title = Börja en ny dag (== Dag ==), daterad om ett datum är valt
editor-toolbar-day-default = Dag
editor-toolbar-day-date = Datum för nästa dag
editor-toolbar-day-date-title = Valfritt datum för nästa dag, som i == Lördag (2026-03-07) ==
editor-toolbar-meal = Måltid
editor-toolbar-meal-title = Börja en måltid (Frukost: \) med en första punkt
editor-toolbar-meal-breakfast = Frukost
editor-toolbar-meal-lunch = Lunch
editor-toolbar-meal-dinner = Middag
editor-toolbar-meal-snacks = Mellanmål
editor-toolbar-add-recipe = Lägg till recept
editor-toolbar-add-recipe-title = Lägg till ett recept i den aktuella måltiden (- @./Recept{"{}"})
editor-toolbar-recipe-reference = Receptreferens
editor-toolbar-recipe-reference-title = Referera till ett annat recept (@./Recept{"{}"})

# Recipe picker
recipe-picker-title = Välj ett recept
recipe-picker-search = Sök recept
recipe-picker-results = Recept
recipe-picker-servings = Portioner
recipe-picker-servings-hint = Lämna tomt för att använda receptets egna portioner.
recipe-picker-insert = Infoga
recipe-picker-no-results = Inga recept hittades
recipe-picker-load-failed = Kunde inte läsa in recepten

# LSP Status
lsp-connected = LSP Ansluten
lsp-disconnected = Frånkopplad
lsp-error = LSP Fel

# New Recipe
new-recipe = Nytt Recept
new-recipe-path = Recept sökväg
new-recipe-filename = Recept namn
new-recipe-placeholder = Middag/Italienskt/Pasta Carbonara
new-recipe-hint = Använd mapp/recept-namn format
new-recipe-create = Skapa Recept

# New Menu
new-menu = Ny Meny
new-menu-path = Meny sökväg
new-menu-placeholder = Planer/Vecka 12
new-menu-hint = Använd mapp/meny-namn format
new-menu-create = Skapa Meny
new-plan = Ny Måltidsplan
new-plan-path = Sökväg för planen
new-plan-placeholder = Planer/Oktober
new-plan-hint = Använd formatet mapp/plannamn
new-plan-create = Skapa Måltidsplan
new-plan-start = Första dagen
new-plan-today = Idag
new-plan-this-week = Den här veckan
new-plan-next-week = Nästa vecka
new-plan-days = Antal dagar
new-plan-one-week = 1 vecka
new-plan-two-weeks = 2 veckor
new-plan-meals = Måltider

# Delete Recipe
delete-recipe = Radera Recept
delete-recipe-confirm = Är du säker att du vill radera detta recept?
delete-recipe-warning = Detta kan inte ångras.

# Rename Recipe
action-rename = Byt namn
rename-title = Byt namn på fil
rename-label = Nytt namn
rename-hint = Filen stannar i sin mapp. Dess bilder byter namn med den, och recept och menyer som använder den uppdateras med det nya namnet.
rename-failed = Kunde inte byta namn: %s
rename-skipped = Namnet är bytt, men vissa referenser lämnades oförändrade: %s
rename-write-failed = Namnet är bytt, men dessa filer kunde inte uppdateras: %s
rename-shopping-list = Inköpslistan använder fortfarande det gamla namnet; lägg till det igen från dess nya sida.

# Title Picture
picture-button = Bild
picture-title = Titelbild
picture-none = Ingen bild än. Välj en, eller släpp den här.
picture-choose = Välj bild
picture-replace = Byt bild
picture-remove = Ta bort
picture-remove-confirm = Ta bort den här bilden? Filen raderas och kan inte återställas.
picture-hint = JPEG, PNG eller WebP. Sparas som JPEG; stora bilder skalas ned till 2048 px.
picture-from-metadata = Receptets bild anges av fältet image i dess metadata. Ta bort den raden för att använda en uppladdad bild.
picture-uploading = Laddar upp...
picture-removing = Tar bort...
picture-load-failed = Det gick inte att läsa in bilden
picture-upload-failed = Uppladdningen misslyckades
picture-remove-failed = Det gick inte att ta bort bilden
picture-too-large = Bilden är för stor. Gränsen är 10 MB.
picture-heif = HEIC- och AVIF-foton kan inte läsas. Ladda upp från telefonens egen webbläsare, som konverterar dem till JPEG, eller ställ in iPhone-kameran på Mest kompatibelt (Inställningar > Kamera > Format).
picture-for = Bild för
picture-step-title = Stegbild
picture-target-step = Steg { $step }: { $text }
picture-target-section = Avsnitt { $section }
picture-step-note = En stegbild hör till stegets position, inte dess text: lägger du till eller tar bort ett tidigare steg i det här avsnittet hamnar den på ett annat steg.
step-picture-add = Lägg till en bild på det här steget
step-picture-change = Byt bild för det här steget
# Sign-in (users are managed on the server with `cook server user`)
sign-in = Logga in
sign-out = Logga ut
sign-in-intro = Logga in för att lägga till, redigera eller radera recept och för att ändra skafferiet eller handlingslistan.
sign-in-username = Användarnamn
sign-in-password = Lösenord
sign-in-failed = Fel användarnamn eller lösenord.
signed-in-as = Inloggad som
role-forbidden = Ditt konto kan inte göra detta. Be den som driver servern om en annan roll.

# Errors
error-title = Något gick snett
error-back-home = Tillbaka till recept

# Icon button labels (aria-label / title)
aria-toggle-theme = Byt tema
aria-keyboard-shortcuts = Tangentbordsgenvägar
aria-more-options = Fler alternativ
aria-preferences = Inställningar
aria-dismiss = Stäng
aria-decrease-scale = Minska skala
aria-increase-scale = Öka skala
aria-decrease-servings = Minska portioner
aria-increase-servings = Öka portioner
aria-close = Stäng
