# Shopping List
shopping-title = Liste de Courses
shopping-empty = Votre liste de courses est vide
shopping-no-items = Aucun article dans la liste de courses. Ajoutez des recettes pour commencer.
shopping-no-recipes = Aucune recette sélectionnée. Ajoutez des recettes depuis la page des recettes.
shopping-item-count =
    { $count ->
        [one] { $count } article
       *[other] { $count } articles
    }
shopping-from-recipe = De { $recipeName }
shopping-clear-confirm = Êtes-vous sûr de vouloir effacer tous les articles ?
shopping-added = { $itemName } ajouté à la liste de courses
shopping-removed = { $itemName } retiré de la liste de courses
shopping-cleared = Liste de courses effacée
shopping-export = Exporter
shopping-import = Importer
shopping-mark-done = Marquer comme fait
shopping-mark-undone = Marquer comme non fait
shopping-selected-recipes = Recettes sélectionnées
shopping-clear-all = Tout effacer
shopping-in-pantry = Dans votre garde-manger
shopping-pantry-unused = Le fichier du garde-manger n'a pas pu être lu : rien de votre garde-manger n'a été retiré de cette liste.
shopping-pantry-open = Ouvrir le garde-manger pour le corriger
shopping-remove = Retirer
shopping-add-recipes-first = Veuillez d'abord ajouter quelques recettes
shopping-failed-to-generate = Échec de la génération de la liste de courses
shopping-warnings = Certaines recettes sont à vérifier, il manque peut-être quelque chose à la liste :
shopping-warning-not-a-number = { $recipe } : impossible d'adapter la recette { $reference } à { $amount }, qui n'est pas un nombre. Elle figure sur la liste telle quelle, pas sous forme de ses ingrédients.
shopping-warning-no-servings = { $recipe } : impossible d'adapter la recette { $reference } à { $amount } : elle n'indique pas un nombre entier de portions, par exemple `servings: 4`. Elle figure sur la liste telle quelle, pas sous forme de ses ingrédients.
shopping-warning-no-yield = { $recipe } : impossible d'adapter la recette { $reference } à { $amount } : elle n'indique pas combien elle produit. Ajoutez un rendement au format nombre%unité, par exemple `yield: 500%{ $unit }`. Elle figure sur la liste telle quelle, pas sous forme de ses ingrédients.
shopping-warning-yield-unit = { $recipe } : impossible d'adapter la recette { $reference } à { $amount } : son rendement est en { $unit }, demandez-la donc en { $unit }. Elle figure sur la liste telle quelle, pas sous forme de ses ingrédients.
shopping-warning-non-positive-base = { $recipe } : impossible d'adapter la recette { $reference } à { $amount } : elle déclare `{ $declared }`, il n'y a donc rien à partir de quoi adapter. Elle figure sur la liste telle quelle, pas sous forme de ses ingrédients.
shopping-failed-to-add = Échec de l'ajout à la liste de courses
shopping-error = Erreur
shopping-print = Imprimer
shopping-include-in-list = Inclure dans la liste de courses
shopping-optional = (optionnel)
shopping-copy = Copier
shopping-copied = Copié !
shopping-copy-failed = Échec de la copie
shopping-copy-options = Options de copie
shopping-copy-include-aisles = Inclure les noms de rayons
shopping-copy-include-amounts = Inclure les quantités
