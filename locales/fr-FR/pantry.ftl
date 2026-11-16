# Pantry
pantry-title = Garde-Manger
pantry-intro = Ce que vous avez à la maison, par endroit de rangement. La liste de courses laisse de côté ce que vous avez déjà.
pantry-unreadable = Le fichier du garde-manger n'a pas pu être lu
pantry-empty = Votre garde-manger est vide
pantry-filters = Afficher
pantry-filter-all = Tout
pantry-filter-low = Bientôt épuisé
pantry-filter-out = En rupture de stock
pantry-filter-expiring = Expire bientôt
pantry-filter-empty = Aucun article ne correspond à ce filtre.
pantry-stock-ok = En stock
pantry-expired = Expiré
pantry-expires-today = Expire aujourd'hui
pantry-expires-in =
    { $count ->
        [one] Dans { $count } jour
       *[other] Dans { $count } jours
    }
pantry-section = { $name }
pantry-no-config = Aucune configuration de garde-manger trouvée
pantry-create-config = Créez un fichier pantry.conf pour suivre votre inventaire
pantry-configure = Configurer le garde-manger →
pantry-manage = Gérer le garde-manger

# Pantry Item Fields
pantry-item-name = Nom de l'article
pantry-item-quantity = Quantité
pantry-item-bought = Acheté :
pantry-item-bought-date = Date d'achat
pantry-item-expire = Expire :
pantry-item-expire-date = Date d'expiration
pantry-item-low = Bas à :
pantry-item-low-threshold = Seuil de stock bas
pantry-placeholder-quantity = ex. 500%g ou 2%L
pantry-placeholder-low = ex. 100%g ou 2

# Pantry Actions
pantry-add-item = Ajouter un article
pantry-add = Ajouter
pantry-edit-item = Modifier l'article
pantry-remove-item = Retirer l'article
pantry-remove-confirm = Retirer { $name } de { $section } ?
pantry-confirm-remove-template = Retirer %s de %s ?
pantry-mark-low = Marquer comme bas
pantry-restock = Réapprovisionner
pantry-save = Enregistrer
pantry-cancel = Annuler
pantry-section-label = Section
pantry-section-required = Donnez un nom à la section
pantry-general-quantity-only = Les articles placés avant la première section ne peuvent avoir qu'une quantité. Déplacez-le dans une section pour lui donner des dates ou un seuil bas.
pantry-section-freezer = Congélateur
pantry-section-fridge = Réfrigérateur
pantry-section-pantry = Garde-manger
pantry-section-spices = Épices
pantry-section-general = Général
pantry-tab-items = Articles
pantry-tab-text = Texte
pantry-text-intro = Modifiez le fichier directement : une ligne [section], puis un article par ligne, écrit nom = "quantité" ou nom = {"{"} quantity = "…", bought = "…", expire = "…", low = "…" {"}"}. Les lignes commençant par # sont des commentaires.
pantry-text-save = Enregistrer le fichier
pantry-text-saved = Enregistré
pantry-text-conflict = Le fichier du garde-manger a été modifié ailleurs pendant votre saisie. Votre texte est conservé ; enregistrez à nouveau pour remplacer cette modification.
pantry-rename-section = Renommer la section
pantry-section-name = Nom de la section
pantry-failed-rename = Impossible de renommer la section
pantry-failed-save-file = Impossible d'enregistrer le fichier
pantry-optional = (optionnel)
pantry-failed-add = Échec de l'ajout de l'article
pantry-item-name-required = Donnez un nom à l'article
pantry-item-exists = %s est déjà dans %s
pantry-section-name-taken = Il existe déjà une section ou un article nommé %s
pantry-failed-update = Échec de la mise à jour de l'article
pantry-failed-remove = Échec de la suppression de l'article
