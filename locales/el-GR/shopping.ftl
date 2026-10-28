# Shopping List
shopping-title = Λίστα αγορών
shopping-empty = Η λίστα αγορών σας είναι άδεια
shopping-no-items = Δεν υπάρχουν είδη στη λίστα αγορών. Προσθέστε συνταγές για να ξεκινήσετε.
shopping-no-recipes = Δεν έχουν επιλεγεί συνταγές. Προσθέστε συνταγές από τη σελίδα της συνταγής.
shopping-item-count =
    { $count ->
        [one] { $count } είδος
       *[other] { $count } είδη
    }
shopping-from-recipe = Από { $recipeName }
shopping-clear-confirm = Είστε σίγουροι ότι θέλετε να καθαρίσετε όλα τα είδη;
shopping-added = { $itemName }: ÏÏÎ¿ÏÏÎ­Î¸Î·ÎºÎµ ÏÏÎ· Î»Î¯ÏÏÎ± Î±Î³Î¿ÏÏÎ½
shopping-removed = { $itemName }: Î±ÏÎ±Î¹ÏÎ­Î¸Î·ÎºÎµ Î±ÏÏ ÏÎ· Î»Î¯ÏÏÎ± Î±Î³Î¿ÏÏÎ½
shopping-cleared = Η λίστα αγορών καθαρίστηκε
shopping-export = Εξαγωγή
shopping-import = Εισαγωγή
shopping-mark-done = Σήμανση ως αγορασμένο
shopping-mark-undone = Αναίρεση σήμανσης
shopping-selected-recipes = Επιλεγμένες συνταγές
shopping-clear-all = Καθαρισμός όλων
shopping-in-pantry = Στα αποθέματά σας
shopping-pantry-unused = Το αρχείο αποθεμάτων δεν ήταν δυνατό να διαβαστεί, οπότε τίποτα από τα αποθέματά σας δεν αφαιρέθηκε από αυτή τη λίστα.
shopping-pantry-open = Ανοίξτε τα αποθέματα για να το διορθώσετε
shopping-remove = Αφαίρεση
shopping-add-recipes-first = Προσθέστε πρώτα μερικές συνταγές
shopping-failed-to-generate = Αποτυχία δημιουργίας λίστας αγορών
shopping-warnings = Ορισμένες συνταγές χρειάζονται έλεγχο, οπότε στη λίστα μπορεί να λείπει κάτι:
shopping-warning-not-a-number = { $recipe }: η συνταγή { $reference } δεν ήταν δυνατό να προσαρμοστεί σε { $amount }, γιατί δεν είναι αριθμός. Μπαίνει στη λίστα όπως είναι γραμμένη, όχι ως τα υλικά της.
shopping-warning-no-servings = { $recipe }: η συνταγή { $reference } δεν ήταν δυνατό να προσαρμοστεί σε { $amount }: δεν έχει ακέραιο αριθμό μερίδων, όπως `servings: 4`. Μπαίνει στη λίστα όπως είναι γραμμένη, όχι ως τα υλικά της.
shopping-warning-no-yield = { $recipe }: η συνταγή { $reference } δεν ήταν δυνατό να προσαρμοστεί σε { $amount }: δεν αναφέρει πόση ποσότητα βγάζει. Προσθέστε απόδοση, γραμμένη αριθμός%μονάδα, όπως `yield: 500%{ $unit }`. Μπαίνει στη λίστα όπως είναι γραμμένη, όχι ως τα υλικά της.
shopping-warning-yield-unit = { $recipe }: η συνταγή { $reference } δεν ήταν δυνατό να προσαρμοστεί σε { $amount }: η απόδοσή της είναι σε { $unit }, οπότε ζητήστε την σε { $unit }. Μπαίνει στη λίστα όπως είναι γραμμένη, όχι ως τα υλικά της.
shopping-warning-non-positive-base = { $recipe }: η συνταγή { $reference } δεν ήταν δυνατό να προσαρμοστεί σε { $amount }: δηλώνει `{ $declared }`, οπότε δεν υπάρχει βάση για προσαρμογή. Μπαίνει στη λίστα όπως είναι γραμμένη, όχι ως τα υλικά της.
shopping-failed-to-add = Αποτυχία προσθήκης στη λίστα αγορών
shopping-error = Σφάλμα
shopping-print = Εκτύπωση
shopping-include-in-list = Συμπερίληψη στη λίστα αγορών
shopping-optional = (προαιρετικό)
shopping-copy = Αντιγραφή
shopping-copied = Αντιγράφηκε!
shopping-copy-failed = Η αντιγραφή απέτυχε
shopping-copy-options = Επιλογές αντιγραφής
shopping-copy-include-aisles = Συμπερίληψη ονομάτων διαδρόμων
shopping-copy-include-amounts = Συμπερίληψη ποσοτήτων
