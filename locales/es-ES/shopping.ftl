# Shopping List
shopping-title = Lista de Compras
shopping-empty = Su lista de compras está vacía
shopping-no-items = No hay artículos en la lista de compras. Agregue recetas para comenzar.
shopping-no-recipes = No hay recetas seleccionadas. Agregue recetas desde la página de recetas.
shopping-item-count =
    { $count ->
        [one] { $count } artículo
       *[other] { $count } artículos
    }
shopping-from-recipe = De { $recipeName }
shopping-clear-confirm = ¿Está seguro de que desea borrar todos los artículos?
shopping-added = { $itemName } agregado a la lista de compras
shopping-removed = { $itemName } eliminado de la lista de compras
shopping-cleared = Lista de compras limpiada
shopping-export = Exportar
shopping-import = Importar
shopping-mark-done = Marcar como hecho
shopping-mark-undone = Marcar como no hecho
shopping-selected-recipes = Recetas seleccionadas
shopping-clear-all = Borrar todo
shopping-in-pantry = En su despensa
shopping-pantry-unused = No se pudo leer el archivo de la despensa, así que no se ha descontado nada de su despensa en esta lista.
shopping-pantry-open = Abrir la despensa para corregirlo
shopping-remove = Eliminar
shopping-add-recipes-first = Por favor, agregue primero algunas recetas
shopping-failed-to-generate = Error al generar la lista de compras
shopping-warnings = Conviene revisar algunas recetas; puede que falte algo en la lista:
shopping-warning-not-a-number = { $recipe }: no se pudo ajustar la receta { $reference } a { $amount }, que no es un número. Aparece en la lista tal como está escrita, no como sus ingredientes.
shopping-warning-no-servings = { $recipe }: no se pudo ajustar la receta { $reference } a { $amount }: no indica un número entero de raciones, como `servings: 4`. Aparece en la lista tal como está escrita, no como sus ingredientes.
shopping-warning-no-yield = { $recipe }: no se pudo ajustar la receta { $reference } a { $amount }: no indica cuánto rinde. Añada un rendimiento con el formato número%unidad, como `yield: 500%{ $unit }`. Aparece en la lista tal como está escrita, no como sus ingredientes.
shopping-warning-yield-unit = { $recipe }: no se pudo ajustar la receta { $reference } a { $amount }: su rendimiento está en { $unit }, así que pídala en { $unit }. Aparece en la lista tal como está escrita, no como sus ingredientes.
shopping-failed-to-add = Error al agregar a la lista de compras
shopping-error = Error
shopping-print = Imprimir
shopping-include-in-list = Incluir en la lista de compras
shopping-optional = (opcional)
shopping-copy = Copiar
shopping-copied = ¡Copiado!
shopping-copy-failed = Error al copiar
shopping-copy-options = Opciones de copia
shopping-copy-include-aisles = Incluir nombres de secciones
shopping-copy-include-amounts = Incluir cantidades
