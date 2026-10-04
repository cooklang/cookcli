# Navigation
nav-recipes = Recettes
nav-shopping-list = Liste de Courses
nav-pantry = Garde-Manger
nav-preferences = Préférences

# Search
search-placeholder = Rechercher des recettes...
search-no-results = Aucune recette trouvée

# Common Actions
action-add = Ajouter
action-remove = Retirer
action-edit = Modifier
action-save = Enregistrer
action-cancel = Annuler
action-back = Retour
action-delete = Supprimer
action-clear = Effacer
action-print = Imprimer
action-preview = Aperçu
action-done = Terminé

# Common Labels
label-scale = Échelle
label-servings = Portions
label-time = Temps
label-difficulty = Difficulté
label-name = Nom
label-description = Description

# Editor
editor-unsaved-changes = Modifications non enregistrées
editor-saved = Enregistré
editor-saving = Enregistrement...
editor-save-failed = Échec de l'enregistrement
editor-placeholder = Entrez votre recette ici...

# Editor toolbar
editor-toolbar-label = Mise en forme Cooklang
editor-toolbar-inline = Éléments en ligne
editor-toolbar-block = Éléments de ligne
editor-toolbar-ingredient = Ingrédient
editor-toolbar-ingredient-title = Insérer un ingrédient (@), ou transformer la sélection en ingrédient
editor-toolbar-cookware = Ustensile
editor-toolbar-cookware-title = Insérer un ustensile (#), ou transformer la sélection en ustensile
editor-toolbar-timer = Minuteur
editor-toolbar-timer-title = Insérer un minuteur (~) en minutes, ou le nommer d'après la sélection
editor-toolbar-section = Section
editor-toolbar-section-title = Commencer une nouvelle section (== Section ==), nommée d'après la sélection
editor-toolbar-section-default = Section
editor-toolbar-note = Note
editor-toolbar-note-title = Transformer les lignes courantes en note (>), ou les rétablir en étapes
editor-toolbar-comment = Commentaire
editor-toolbar-comment-title = Commenter les lignes courantes (--), ou la sélection dans une ligne
editor-toolbar-metadata = Métadonnées
editor-toolbar-metadata-title = Ajouter une ligne de métadonnées dans l'en-tête (---)
editor-toolbar-menu = Éléments de menu
editor-toolbar-other = Autres éléments
editor-toolbar-day = Jour
editor-toolbar-day-title = Commencer un nouveau jour (== Jour ==), daté si une date est choisie
editor-toolbar-day-default = Jour
editor-toolbar-day-date = Date du prochain jour
editor-toolbar-day-date-title = Date facultative du prochain jour, comme dans == Samedi (2026-03-07) ==
editor-toolbar-meal = Repas
editor-toolbar-meal-title = Commencer un repas (Petit-déjeuner : \) avec une première puce
editor-toolbar-meal-breakfast = Petit-déjeuner
editor-toolbar-meal-lunch = Déjeuner
editor-toolbar-meal-dinner = Dîner
editor-toolbar-meal-snacks = Collations
editor-toolbar-add-recipe = Ajouter une recette
editor-toolbar-add-recipe-title = Ajouter une recette au repas en cours (- @./Recette{"{}"})
editor-toolbar-recipe-reference = Référence de recette
editor-toolbar-recipe-reference-title = Faire référence à une autre recette (@./Recette{"{}"})

# Recipe picker
recipe-picker-title = Choisir une recette
recipe-picker-search = Rechercher des recettes
recipe-picker-results = Recettes
recipe-picker-servings = Portions
recipe-picker-servings-hint = Laisser vide pour garder les portions de la recette.
recipe-picker-insert = Insérer
recipe-picker-no-results = Aucune recette trouvée
recipe-picker-load-failed = Impossible de charger les recettes

# LSP Status
lsp-connected = LSP connecté
lsp-disconnected = Déconnecté
lsp-error = Erreur LSP

# New Recipe
new-recipe = Nouvelle Recette
new-recipe-path = Chemin de la recette
new-recipe-filename = Nom de la recette
new-recipe-placeholder = Diner/Italien/Pates Carbonara
new-recipe-hint = Format: dossier/nom-recette
new-recipe-create = Creer la Recette

# New Menu
new-menu = Nouveau Menu
new-menu-path = Chemin du menu
new-menu-placeholder = Plannings/Semaine 12
new-menu-hint = Format : dossier/nom-du-menu
new-menu-create = Créer le Menu
new-plan = Nouveau Planning de repas
new-plan-path = Chemin du planning
new-plan-placeholder = Plannings/Octobre
new-plan-hint = Format : dossier/nom-du-planning
new-plan-create = Créer le Planning
new-plan-start = Premier jour
new-plan-today = Aujourd'hui
new-plan-this-week = Cette semaine
new-plan-next-week = La semaine prochaine
new-plan-days = Nombre de jours
new-plan-one-week = 1 semaine
new-plan-two-weeks = 2 semaines
new-plan-meals = Repas

# Delete Recipe
delete-recipe = Supprimer la Recette
delete-recipe-confirm = Êtes-vous sûr de vouloir supprimer cette recette?
delete-recipe-warning = Cette action est irréversible.

# Rename Recipe
action-rename = Renommer
rename-title = Renommer le fichier
rename-label = Nouveau nom
rename-hint = Le fichier reste dans son dossier. Ses photos sont renommées avec lui, et les recettes et menus qui l’utilisent sont mis à jour avec le nouveau nom.
rename-failed = Impossible de renommer : %s
rename-skipped = Renommé, mais certaines références n’ont pas été modifiées : %s
rename-write-failed = Renommé, mais ces fichiers n’ont pas pu être mis à jour : %s
rename-shopping-list = La liste de courses utilise encore l’ancien nom ; ajoutez-le à nouveau depuis sa nouvelle page.

# Title Picture
picture-button = Photo
picture-title = Photo de la recette
picture-none = Pas encore de photo. Choisissez-en une, ou déposez-la ici.
picture-choose = Choisir une photo
picture-replace = Remplacer la photo
picture-remove = Retirer
picture-remove-confirm = Retirer cette photo ? Le fichier sera supprimé définitivement.
picture-hint = JPEG, PNG ou WebP. Enregistrée en JPEG ; les grandes photos sont réduites à 2048 px.
picture-from-metadata = La photo de cette recette est définie par le champ image de ses métadonnées. Supprimez cette ligne pour utiliser une photo envoyée.
picture-uploading = Envoi en cours...
picture-removing = Suppression...
picture-load-failed = Impossible de charger la photo
picture-upload-failed = Échec de l'envoi
picture-remove-failed = Impossible de retirer la photo
picture-too-large = Cette photo est trop volumineuse. La limite est de 10 Mo.
picture-heif = Les photos HEIC et AVIF ne peuvent pas être lues. Envoyez-la depuis le navigateur du téléphone, qui la convertit en JPEG, ou réglez l'appareil photo de l'iPhone sur « Le plus compatible » (Réglages > Appareil photo > Formats).
picture-for = Photo pour
picture-step-title = Photo de l'étape
picture-target-step = Étape { $step } : { $text }
picture-target-section = Section { $section }
picture-step-note = La photo d'une étape suit sa position, pas son texte : ajouter ou retirer une étape plus haut dans cette section la fait passer sur une autre étape.
step-picture-add = Ajouter une photo à cette étape
step-picture-change = Changer la photo de cette étape
# Sign-in (users are managed on the server with `cook server user`)
sign-in = Se connecter
sign-out = Se déconnecter
sign-in-intro = Connectez-vous pour ajouter, modifier ou supprimer des recettes, et pour changer le garde-manger ou la liste de courses.
sign-in-username = Nom d'utilisateur
sign-in-password = Mot de passe
sign-in-failed = Nom d'utilisateur ou mot de passe incorrect.
signed-in-as = Connecté en tant que
role-forbidden = Votre compte ne permet pas de faire cela. Demandez un autre rôle à la personne qui gère ce serveur.

# Errors
error-title = Une erreur s'est produite
error-back-home = Retour aux recettes

# Icon button labels (aria-label / title)
aria-toggle-theme = Changer de thème
aria-keyboard-shortcuts = Raccourcis clavier
aria-more-options = Plus d'options
aria-preferences = Préférences
aria-dismiss = Fermer
aria-decrease-scale = Réduire l'échelle
aria-increase-scale = Augmenter l'échelle
aria-decrease-servings = Réduire les portions
aria-increase-servings = Augmenter les portions
aria-close = Fermer
