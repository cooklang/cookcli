# Navigation
nav-recipes = Recetas
nav-shopping-list = Lista de Compras
nav-pantry = Despensa
nav-preferences = Preferencias

# Search
search-placeholder = Buscar recetas...
search-no-results = No se encontraron recetas

# Common Actions
action-add = Agregar
action-remove = Eliminar
action-edit = Editar
action-save = Guardar
action-cancel = Cancelar
action-back = Volver
action-delete = Borrar
action-clear = Limpiar
action-print = Imprimir
action-preview = Vista previa
action-done = Listo

# Common Labels
label-scale = Escala
label-servings = Porciones
label-time = Tiempo
label-difficulty = Dificultad
label-name = Nombre
label-description = Descripción

# Editor
editor-unsaved-changes = Cambios sin guardar
editor-saved = Guardado
editor-saving = Guardando...
editor-save-failed = Error al guardar
editor-placeholder = Escribe tu receta aquí...

# Editor toolbar
editor-toolbar-label = Formato Cooklang
editor-toolbar-inline = Elementos en línea
editor-toolbar-block = Elementos de línea
editor-toolbar-ingredient = Ingrediente
editor-toolbar-ingredient-title = Insertar un ingrediente (@) o convertir la selección en ingrediente
editor-toolbar-cookware = Utensilio
editor-toolbar-cookware-title = Insertar un utensilio (#) o convertir la selección en utensilio
editor-toolbar-timer = Temporizador
editor-toolbar-timer-title = Insertar un temporizador (~) en minutos o nombrarlo según la selección
editor-toolbar-section = Sección
editor-toolbar-section-title = Empezar una nueva sección (== Sección ==) con el nombre de la selección
editor-toolbar-section-default = Sección
editor-toolbar-note = Nota
editor-toolbar-note-title = Convertir las líneas actuales en una nota (>) o volver a pasos
editor-toolbar-comment = Comentario
editor-toolbar-comment-title = Comentar las líneas actuales (--) o la selección dentro de una línea
editor-toolbar-metadata = Metadatos
editor-toolbar-metadata-title = Añadir una línea de metadatos al encabezado (---)
editor-toolbar-menu = Elementos del menú
editor-toolbar-other = Otros elementos
editor-toolbar-day = Día
editor-toolbar-day-title = Empezar un nuevo día (== Día ==), con fecha si se elige una
editor-toolbar-day-default = Día
editor-toolbar-day-date = Fecha del próximo día
editor-toolbar-day-date-title = Fecha opcional del próximo día, como en == Sábado (2026-03-07) ==
editor-toolbar-meal = Comida
editor-toolbar-meal-title = Empezar una comida (Desayuno: \) con una primera viñeta
editor-toolbar-meal-breakfast = Desayuno
editor-toolbar-meal-lunch = Almuerzo
editor-toolbar-meal-dinner = Cena
editor-toolbar-meal-snacks = Tentempiés
editor-toolbar-add-recipe = Añadir receta
editor-toolbar-add-recipe-title = Añadir una receta a la comida actual (- @./Receta{"{}"})
editor-toolbar-recipe-reference = Referencia a receta
editor-toolbar-recipe-reference-title = Hacer referencia a otra receta (@./Receta{"{}"})

# Recipe picker
recipe-picker-title = Elegir una receta
recipe-picker-search = Buscar recetas
recipe-picker-results = Recetas
recipe-picker-servings = Porciones
recipe-picker-servings-hint = Déjalo vacío para usar las porciones de la receta.
recipe-picker-insert = Insertar
recipe-picker-no-results = No se encontraron recetas
recipe-picker-load-failed = No se pudieron cargar las recetas

# LSP Status
lsp-connected = LSP conectado
lsp-disconnected = Desconectado
lsp-error = Error de LSP

# New Recipe
new-recipe = Nueva Receta
new-recipe-path = Ruta de la receta
new-recipe-filename = Nombre de la receta
new-recipe-placeholder = Cena/Italiano/Pasta Carbonara
new-recipe-hint = Formato: carpeta/nombre-receta
new-recipe-create = Crear Receta

# New Menu
new-menu = Nuevo Menú
new-menu-path = Ruta del menú
new-menu-placeholder = Planes/Semana 12
new-menu-hint = Formato: carpeta/nombre-menú
new-menu-create = Crear Menú
new-plan = Nuevo Plan de Comidas
new-plan-path = Ruta del plan
new-plan-placeholder = Planes/Octubre
new-plan-hint = Usa el formato carpeta/nombre-del-plan
new-plan-create = Crear Plan
new-plan-start = Primer día
new-plan-today = Hoy
new-plan-this-week = Esta semana
new-plan-next-week = La próxima semana
new-plan-days = Número de días
new-plan-one-week = 1 semana
new-plan-two-weeks = 2 semanas
new-plan-meals = Comidas

# Delete Recipe
delete-recipe = Eliminar Receta
delete-recipe-confirm = ¿Estás seguro de que quieres eliminar esta receta?
delete-recipe-warning = Esta acción no se puede deshacer.

# Rename Recipe
action-rename = Renombrar
rename-title = Renombrar archivo
rename-label = Nuevo nombre
rename-hint = El archivo se queda en su carpeta. Sus fotos se renombran con él, y las recetas y menús que lo usan se actualizan con el nuevo nombre.
rename-failed = No se pudo renombrar: %s
rename-skipped = Renombrado, pero algunas referencias no se cambiaron: %s
rename-write-failed = Renombrado, pero estos archivos no se pudieron actualizar: %s
rename-shopping-list = La lista de la compra aún usa el nombre antiguo; vuelve a añadirlo desde su nueva página.

# Title Picture
picture-button = Foto
picture-title = Foto de la receta
picture-none = Todavía no hay foto. Elige una o suéltala aquí.
picture-choose = Elegir foto
picture-replace = Reemplazar foto
picture-remove = Quitar
picture-remove-confirm = ¿Quitar esta foto? El archivo se eliminará y no se podrá recuperar.
picture-hint = JPEG, PNG o WebP. Se guarda como JPEG; las fotos grandes se reducen a 2048 px.
picture-from-metadata = La foto de esta receta la define el campo image de sus metadatos. Elimina esa línea para usar una foto subida.
picture-uploading = Subiendo...
picture-removing = Quitando...
picture-load-failed = No se pudo cargar la foto
picture-upload-failed = Error al subir
picture-remove-failed = No se pudo quitar la foto
picture-too-large = Esta foto es demasiado grande. El límite es de 10 MB.
picture-heif = Las fotos HEIC y AVIF no se pueden leer. Súbela desde el navegador del propio teléfono, que la convierte a JPEG, o configura la cámara del iPhone en «Más compatible» (Ajustes > Cámara > Formatos).
picture-for = Foto para
picture-step-title = Foto del paso
picture-target-step = Paso { $step }: { $text }
picture-target-section = Sección { $section }
picture-step-note = La foto de un paso va con su posición, no con su texto: si añades o quitas un paso anterior en esta sección, pasará a otro paso.
step-picture-add = Añadir una foto a este paso
step-picture-change = Cambiar la foto de este paso
# Sign-in (users are managed on the server with `cook server user`)
sign-in = Iniciar sesión
sign-out = Cerrar sesión
sign-in-intro = Inicia sesión para agregar, editar o eliminar recetas y para cambiar la despensa o la lista de compras.
sign-in-username = Nombre de usuario
sign-in-password = Contraseña
sign-in-failed = Nombre de usuario o contraseña incorrectos.
signed-in-as = Sesión iniciada como
role-forbidden = Tu cuenta no puede hacer esto. Pide otro rol a quien administra este servidor.

# Errors
error-title = Algo salió mal
error-back-home = Volver a las recetas

# Icon button labels (aria-label / title)
aria-toggle-theme = Cambiar tema
aria-keyboard-shortcuts = Atajos de teclado
aria-more-options = Más opciones
aria-preferences = Preferencias
aria-dismiss = Cerrar
aria-decrease-scale = Reducir escala
aria-increase-scale = Aumentar escala
aria-decrease-servings = Reducir porciones
aria-increase-servings = Aumentar porciones
aria-close = Cerrar
