# Pantry
pantry-title = Αποθέματα κουζίνας
pantry-intro = Τι υπάρχει στο σπίτι, ανάλογα με το πού φυλάσσεται. Η λίστα αγορών παραλείπει ό,τι έχετε ήδη.
pantry-unreadable = Το αρχείο αποθεμάτων δεν ήταν δυνατό να διαβαστεί
pantry-empty = Τα αποθέματά σας είναι άδεια
pantry-filters = Εμφάνιση
pantry-filter-all = Όλα
pantry-filter-low = Τελειώνουν
pantry-filter-out = ÎÎ¾Î±Î½ÏÎ»Î·Î¼Î­Î½Î±
pantry-filter-expiring = Λήγουν σύντομα
pantry-filter-empty = Κανένα είδος δεν ταιριάζει με αυτό το φίλτρο.
pantry-stock-ok = Διαθέσιμο
pantry-expired = Έληξε
pantry-expires-today = Λήγει σήμερα
pantry-expires-in =
    { $count ->
        [one] Σε { $count } μέρα
       *[other] Σε { $count } μέρες
    }
pantry-section = { $name }
pantry-no-config = Δεν βρέθηκε ρύθμιση αποθεμάτων
pantry-create-config = Δημιουργήστε ένα αρχείο pantry.conf για να παρακολουθείτε τα αποθέματά σας
pantry-configure = Ρύθμιση αποθεμάτων →
pantry-manage = Διαχείριση αποθεμάτων

# Pantry Item Fields
pantry-item-name = Όνομα είδους
pantry-item-quantity = Ποσότητα
pantry-item-bought = Αγοράστηκε:
pantry-item-bought-date = Ημερομηνία αγοράς
pantry-item-expire = Λήγει:
pantry-item-expire-date = Ημερομηνία λήξης
pantry-item-low = Î§Î±Î¼Î·Î»Ï Î±ÏÏÎ¸ÎµÎ¼Î±:
pantry-item-low-threshold = Όριο χαμηλού αποθέματος
pantry-placeholder-quantity = π.χ. 500%g ή 2%L
pantry-placeholder-low = π.χ. 100%g ή 2

# Pantry Actions
pantry-add-item = Προσθήκη είδους
pantry-add = Προσθήκη
pantry-edit-item = Επεξεργασία είδους
pantry-remove-item = Αφαίρεση είδους
pantry-remove-confirm = ÎÎ± Î±ÏÎ±Î¹ÏÎµÎ¸ÎµÎ¯ ÏÎ¿ { $name } Î±ÏÏ ÏÎ·Î½ ÎµÎ½ÏÏÎ·ÏÎ± { $section };
pantry-confirm-remove-template = ÎÎ± Î±ÏÎ±Î¹ÏÎµÎ¸ÎµÎ¯ ÏÎ¿ %s Î±ÏÏ ÏÎ·Î½ ÎµÎ½ÏÏÎ·ÏÎ± %s;
pantry-mark-low = Î£Î®Î¼Î±Î½ÏÎ· ÏÏ ÏÎ±Î¼Î·Î»Ï
pantry-restock = Αναπλήρωση
pantry-save = Αποθήκευση
pantry-cancel = Ακύρωση
pantry-section-label = Ενότητα
pantry-section-required = Δώστε όνομα στην ενότητα
pantry-general-quantity-only = Τα είδη πάνω από την πρώτη ενότητα μπορούν να έχουν μόνο ποσότητα. Μετακινήστε ένα σε ενότητα για να του δώσετε ημερομηνίες ή όριο χαμηλού αποθέματος.
pantry-section-freezer = Καταψύκτης
pantry-section-fridge = Ψυγείο
pantry-section-pantry = Ντουλάπι
pantry-section-spices = Μπαχαρικά
pantry-section-general = Γενικά
pantry-tab-items = Είδη
pantry-tab-text = Κείμενο
pantry-text-intro = Επεξεργαστείτε απευθείας το αρχείο: μια γραμμή [ενότητα], και μετά ένα είδος ανά γραμμή, γραμμένο όνομα = "ποσότητα" ή όνομα = {"{"} quantity = "…", bought = "…", expire = "…", low = "…" {"}"}. Οι γραμμές που ξεκινούν με # είναι σχόλια.
pantry-text-save = Αποθήκευση αρχείου
pantry-text-saved = Αποθηκεύτηκε
pantry-text-conflict = Το αρχείο αποθεμάτων άλλαξε από αλλού όσο το επεξεργαζόσασταν. Το κείμενό σας διατηρείται· αποθηκεύστε ξανά για να αντικαταστήσετε εκείνη την αλλαγή.
pantry-rename-section = Μετονομασία ενότητας
pantry-section-name = Όνομα ενότητας
pantry-failed-rename = Αποτυχία μετονομασίας ενότητας
pantry-failed-save-file = Δεν ήταν δυνατή η αποθήκευση του αρχείου
pantry-optional = (προαιρετικό)
pantry-failed-add = Αποτυχία προσθήκης είδους
pantry-item-name-required = Δώστε όνομα στο είδος
pantry-item-exists = Î¤Î¿ %s ÏÏÎ¬ÏÏÎµÎ¹ Î®Î´Î· ÏÏÎ·Î½ ÎµÎ½ÏÏÎ·ÏÎ± %s
pantry-section-name-taken = Υπάρχει ήδη ενότητα ή είδος με το όνομα %s
pantry-failed-update = Αποτυχία ενημέρωσης είδους
pantry-failed-remove = Αποτυχία αφαίρεσης είδους
