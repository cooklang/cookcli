# Pantry
pantry-title = Despensa
pantry-intro = Lo que hay en casa, según dónde se guarda. La lista de la compra deja fuera lo que ya tienes.
pantry-unreadable = No se pudo leer el archivo de la despensa
pantry-empty = Su despensa está vacía
pantry-filters = Mostrar
pantry-filter-all = Todo
pantry-filter-low = Queda poco
pantry-filter-out = Agotado
pantry-filter-expiring = Caduca pronto
pantry-filter-empty = Ningún artículo coincide con este filtro.
pantry-stock-ok = Disponible
pantry-expired = Caducado
pantry-expires-today = Caduca hoy
pantry-expires-in =
    { $count ->
        [one] En { $count } día
       *[other] En { $count } días
    }
pantry-section = { $name }
pantry-no-config = No se encontró configuración de despensa
pantry-create-config = Cree un archivo pantry.conf para rastrear su inventario
pantry-configure = Configurar despensa →
pantry-manage = Gestionar despensa

# Pantry Item Fields
pantry-item-name = Nombre del artículo
pantry-item-quantity = Cantidad
pantry-item-bought = Comprado:
pantry-item-bought-date = Fecha de compra
pantry-item-expire = Expira:
pantry-item-expire-date = Fecha de expiración
pantry-item-low = Bajo en:
pantry-item-low-threshold = Umbral de stock bajo
pantry-placeholder-quantity = p. ej. 500%g o 2%L
pantry-placeholder-low = p. ej. 100%g o 2

# Pantry Actions
pantry-add-item = Agregar artículo
pantry-add = Añadir
pantry-edit-item = Editar artículo
pantry-remove-item = Eliminar artículo
pantry-remove-confirm = ¿Eliminar { $name } de { $section }?
pantry-confirm-remove-template = ¿Eliminar %s de %s?
pantry-mark-low = Marcar como bajo
pantry-restock = Reabastecer
pantry-save = Guardar
pantry-cancel = Cancelar
pantry-section-label = Sección
pantry-section-required = Ponle un nombre a la sección
pantry-general-quantity-only = Los artículos antes de la primera sección solo pueden tener una cantidad. Muévelo a una sección para darle fechas o un mínimo.
pantry-section-freezer = Congelador
pantry-section-fridge = Refrigerador
pantry-section-pantry = Despensa
pantry-section-spices = Especias
pantry-section-general = General
pantry-tab-items = Artículos
pantry-tab-text = Texto
pantry-text-intro = Edita el archivo directamente: una línea [sección] y después un artículo por línea, escrito nombre = "cantidad" o nombre = {"{"} quantity = "…", bought = "…", expire = "…", low = "…" {"}"}. Las líneas que empiezan por # son comentarios.
pantry-text-save = Guardar archivo
pantry-text-saved = Guardado
pantry-text-conflict = El archivo de la despensa se modificó en otro lugar mientras editabas. Tu texto se conserva; guarda de nuevo para reemplazar ese cambio.
pantry-rename-section = Renombrar sección
pantry-section-name = Nombre de la sección
pantry-failed-rename = No se pudo renombrar la sección
pantry-failed-save-file = No se pudo guardar el archivo
pantry-optional = (opcional)
pantry-failed-add = Error al agregar artículo
pantry-item-name-required = Ponle un nombre al artículo
pantry-item-exists = %s ya está en %s
pantry-section-name-taken = Ya hay una sección o un artículo llamado %s
pantry-failed-update = Error al actualizar artículo
pantry-failed-remove = Error al eliminar artículo
