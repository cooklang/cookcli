# Recipe Listing
recipes-title = Todas las Recetas
recipes-directory-title = { $name }
recipes-empty = No se encontraron recetas en este directorio.
recipes-count =
    { $count ->
        [one] { $count } receta
       *[other] { $count } recetas
    }

# Recipe Display
recipe-ingredients = Ingredientes
recipe-steps = Instrucciones
recipe-notes = Notas
recipe-cookware = Utensilios
recipe-optional = (opcional)
recipe-timers = Temporizadores
recipe-tags = Etiquetas
recipe-add-to-shopping = Agregar a la lista de compras
recipe-add-all-to-shopping = Agregar todo a la lista de compras
recipe-scale-label = Escala
recipe-servings-label = porciones
recipe-written-for =
    { $count ->
        [one] Escrita para { $count } porción
       *[other] Escrita para { $count } porciones
    }
recipe-written-for-hint = Las cantidades se han recalculado a partir de la receta original. Los tiempos de cocción, el tamaño de los moldes y el condimento pueden necesitar ajustes. Selecciona para volver a las porciones originales.
recipe-scaled-from-original = ×{ $factor } de la receta original
recipe-scaled-from-original-hint = Las cantidades se han recalculado a partir de la receta original. Los tiempos de cocción, el tamaño de los moldes y el condimento pueden necesitar ajustes. Selecciona para volver a la receta original.
recipe-print = Imprimir receta
recipe-added = ¡Agregado!
recipe-main-section = Principal
recipe-preparation = preparación

# Recipe Metadata
meta-course = Plato
meta-cuisine = Cocina
meta-diet = Dieta
meta-author = Autor
meta-source = Fuente
meta-prep-time = Tiempo de Preparación
meta-cook-time = Tiempo de Cocción
meta-total-time = Tiempo Total
meta-servings = Porciones
meta-difficulty = Dificultad
meta-description = Descripción
meta-created-by = Creado por
meta-created-at = Creado el
meta-modified-by = Modificado por
meta-modified-at = Modificado el
meta-file-title = Archivo
meta-translated-from = Traducido de

# Nutrición (metadatos personalizados `nutrition:`)
nutrition-per-serving = por porción
nutrition-proteins = proteínas
nutrition-lipids = grasas
nutrition-saturated-fat = grasas saturadas
nutrition-unsaturated-fat = grasas insaturadas
nutrition-monounsaturated-fat = grasas monoinsaturadas
nutrition-polyunsaturated-fat = grasas poliinsaturadas
nutrition-trans-fat = grasas trans
nutrition-starch = almidón
nutrition-carbohydrates = hidratos de carbono
nutrition-sugars = azúcares
nutrition-fibers = fibra
nutrition-salt = sal
nutrition-title = Nutrición
nutrition-per-100g = por 100 g
nutrition-whole-recipe = receta completa
nutrition-energy = energía
nutrition-of-which = de los cuales
nutrition-for = para
nutrition-full-table = Tabla completa
nutrition-legend = Valoración para la salud, por 100 g:
nutrition-rating-good = bueno
nutrition-rating-medium = medio
nutrition-rating-bad = malo
nutrition-legend-good = bueno para la salud
nutrition-legend-medium = medio
nutrition-legend-bad = no bueno para la salud
nutrition-legend-unrated = sin umbral

# Recipe Types
recipe-type-menu = Menú
recipe-type-plan = Plan de comidas
plan-outside = Fuera de este plan
plan-nothing-planned = Nada previsto
plan-add-to = Añadir a { $meal }, { $day }
plan-line-actions = Cambiar
plan-move-to = Mover a…
plan-copy-to = Copiar a…
plan-remove = Quitar
plan-move-title = Mover a otro día
plan-copy-title = Copiar a otro día
plan-move = Mover
plan-copy = Copiar
plan-target-day = Día
plan-target-meal = Comida
plan-changed = El plan ha cambiado desde que se cargó la página. Recárgala para verlo e inténtalo de nuevo.
plan-save-failed = No se pudo guardar el plan.
plan-saved = Plan guardado
plan-reload = Recargar
plan-pick-label = Días para la lista de la compra
plan-pick-all = Todos
plan-pick-week = Próximos 7 días
plan-pick-none = Ninguno
plan-add-days = Añadir días a la lista de la compra

# Today's Menu Banner
todays-menu-title = Menú de Hoy
todays-menu-from = De
todays-menu-view = Ver Menú

# Sort Controls
sort-by = Ordenar por:
sort-name = Nombre
sort-modified = Modificado
sort-created = Creado
sort-direction-toggle = Invertir orden
random-recipe = Receta al azar

# Standards: the layout, intakes and ratings come from the standard in use.
nutrition-more-info = Más información
nutrition-less-info = Menos información
nutrition-collapse-all = Contraer todo
nutrition-expand-all = Expandir todo
nutrition-tree-lines = Líneas de árbol
nutrition-fold = Contraer o expandir
nutrition-legend-claims = Declaraciones de la UE, por 100 g:
nutrition-legend-dv = Porcentaje del valor diario, por ración:
nutrition-well-present = Vitaminas y minerales bien presentes
nutrition-well-source = fuente de
nutrition-well-high = rico en
nutrition-well-rule-100g = al menos el { $source } % de la ingesta de referencia por 100 g («fuente de»), { $high } % o más («rico en»)
nutrition-well-rule-serving = al menos el { $source } % del valor diario por ración («fuente de»), { $high } % o más («rico en»)
nutrition-well-rule-serving-high = { $high } % o más del valor diario por ración («rico en»)
nutrition-standard-note = Norma: { $name } ({ $id }.yaml)
nutrition-carbs-without-fibre = Los hidratos de carbono se muestran sin la fibra (la receta la cuenta dentro).
nutrition-group-macronutrients = Macronutrientes
nutrition-group-other = Otros componentes
nutrition-group-minerals = Minerales
nutrition-group-macro-minerals = Macrominerales
nutrition-group-trace-elements = Oligoelementos
nutrition-group-vitamins = Vitaminas
nutrition-group-fat-soluble = Liposolubles
nutrition-group-water-soluble = Hidrosolubles
nutrition-group-other-data = Otros datos
nutrition-cholesterol = colesterol
nutrition-sodium = sodio
nutrition-added-sugars = azúcares añadidos
nutrition-polyols = polioles
nutrition-omega-3 = omega-3
nutrition-omega-6 = omega-6
nutrition-calcium = calcio
nutrition-phosphorus = fósforo
nutrition-magnesium = magnesio
nutrition-potassium = potasio
nutrition-chloride = cloruro
nutrition-iron = hierro
nutrition-zinc = zinc
nutrition-copper = cobre
nutrition-manganese = manganeso
nutrition-fluoride = fluoruro
nutrition-selenium = selenio
nutrition-chromium = cromo
nutrition-molybdenum = molibdeno
nutrition-iodine = yodo
nutrition-choline = colina
nutrition-vitamin-a = vitamina A
nutrition-vitamin-d = vitamina D
nutrition-vitamin-e = vitamina E
nutrition-vitamin-k = vitamina K
nutrition-vitamin-c = vitamina C
nutrition-vitamin-b1 = vitamina B1
nutrition-vitamin-b2 = vitamina B2
nutrition-vitamin-b3 = vitamina B3
nutrition-vitamin-b5 = vitamina B5
nutrition-vitamin-b6 = vitamina B6
nutrition-vitamin-b8 = vitamina B8
nutrition-vitamin-b9 = vitamina B9
nutrition-vitamin-b12 = vitamina B12

# Explanation bubbles: what each text says comes from the source its -source message names.
nutrition-help-what = Qué es
nutrition-help-use = Para qué sirve
nutrition-help-where = Dónde se encuentra
nutrition-help-source = Fuente: { $source }
nutrition-help-source-todo = Fuente: por documentar
nutrition-help-reference = Ingesta de referencia ({ $standard }): { $amount } al día.
nutrition-help-reference-none = Sin ingesta de referencia en la norma { $standard }.
nutrition-help-recipe = Esta receta: { $amount } por ración.
nutrition-help-recipe-pct = Esta receta: { $amount } por ración, el { $pct } % de la ingesta de referencia.
nutrition-help-rating-traffic = Semáforo británico, por 100 g: bueno hasta { $low } { $unit }, medio hasta { $high } { $unit }, por encima no es bueno.
nutrition-help-rating-dv-limit = A limitar: { $low } % del valor diario o menos es bajo, { $high } % o más es alto.
nutrition-help-rating-dv-encourage = A favorecer: { $high } % del valor diario o más es alto.
nutrition-help-rating-dv-none = Sin valor diario para este nutriente: la FDA no lo valora.
nutrition-help-rating-claim-max = Declaración de la UE «{ $name }»: hasta { $amount } { $unit } por 100 g.
nutrition-help-rating-claim-min = Declaración de la UE «{ $name }»: a partir de { $amount } { $unit } por 100 g.
nutrition-claim-low-fat = bajo en grasa
nutrition-claim-low-saturated-fat = bajo en grasas saturadas
nutrition-claim-low-sugars = bajo en azúcares
nutrition-claim-low-sodium = bajo en sal
nutrition-claim-source-of-fibre = fuente de fibra
nutrition-claim-high-fibre = alto contenido de fibra
nutrition-help-energy-source = Regulation (EU) 1169/2011, Annex XIV; NHS, Fat: the facts (2023)
nutrition-help-lipids-source = NHS, Fat: the facts (2023)
nutrition-help-saturated-fat-source = WHO, Healthy diet (2026); NHS, Fat: the facts (2023)
nutrition-help-unsaturated-fat-source = WHO, Healthy diet (2026)
nutrition-help-monounsaturated-fat-source = NHS, Fat: the facts (2023)
nutrition-help-polyunsaturated-fat-source = NHS, Fat: the facts (2023)
nutrition-help-omega-3-source = NHS, Fat: the facts (2023)
nutrition-help-omega-6-source = NHS, Fat: the facts (2023)
nutrition-help-trans-fat-source = WHO, Trans fat (2024); NHS, Fat: the facts (2023)
nutrition-help-cholesterol-source = NHS, Fat: the facts (2023)
nutrition-help-carbohydrates-source = WHO, Healthy diet (2026); Regulation (EU) 1169/2011, Annex I
nutrition-help-sugars-source = WHO, Healthy diet (2026); NHS, Sugar: the facts (2023)
nutrition-help-starch-source = NHS, Starchy foods and carbohydrates (2023)
nutrition-help-fibers-source = WHO, Healthy diet (2026); NHS, How to get more fibre into your diet (2026)
nutrition-help-proteins-source = WHO, Healthy diet (2026)
nutrition-help-salt-source = WHO, Sodium reduction (2026); NHS, Salt in your diet (2023); Regulation (EU) 1169/2011, Annex I
nutrition-help-sodium-source = WHO, Sodium reduction (2026)
nutrition-help-energy-what = Lo que el alimento aporta al cuerpo, en kilocalorías (kcal) o kilojulios (kJ).
nutrition-help-energy-use = Se calcula con factores fijos: 9 kcal por gramo de grasa, 4 de hidratos de carbono y de proteínas, 7 de alcohol, 2 de fibra.
nutrition-help-lipids-what = Las grasas y aceites de un alimento.
nutrition-help-lipids-use = La grasa aporta ácidos grasos esenciales, ayuda a absorber las vitaminas A, D y E y da 9 kcal por gramo, frente a 4 de los hidratos de carbono y las proteínas.
nutrition-help-saturated-fat-what = Grasas que proceden sobre todo de fuentes animales, y del aceite de palma y de coco.
nutrition-help-saturated-fat-use = Demasiada eleva el colesterol «malo» (LDL) y el riesgo de enfermedad cardíaca e ictus. OMS: menos del 10 % de la energía.
nutrition-help-saturated-fat-where = Carne grasa, salchichas y empanadas, mantequilla, ghee y manteca de cerdo, queso, nata, chocolate, galletas y pasteles, aceite de palma y de coco.
nutrition-help-unsaturated-fat-what = Grasas presentes en el pescado, el aguacate y los frutos secos, y en los aceites de girasol, soja, colza y oliva.
nutrition-help-unsaturated-fat-use = Preferibles a las grasas saturadas (OMS).
nutrition-help-monounsaturated-fat-what = Un tipo de grasa insaturada.
nutrition-help-monounsaturated-fat-use = Ayuda a proteger el corazón: mantiene el colesterol «bueno» (HDL) y reduce el «malo» (LDL).
nutrition-help-monounsaturated-fat-where = Aceite de oliva y de colza, aguacates, almendras, nueces de Brasil, cacahuetes.
nutrition-help-polyunsaturated-fat-what = Un tipo de grasa insaturada. Hay dos tipos principales: omega-3 y omega-6.
nutrition-help-polyunsaturated-fat-use = El cuerpo no puede fabricar algunas formas, así que una pequeña cantidad debe venir de la comida.
nutrition-help-omega-3-what = Una grasa poliinsaturada, esencial en la dieta.
nutrition-help-omega-3-use = Se aconsejan al menos 2 raciones de pescado a la semana, una de ellas de pescado azul.
nutrition-help-omega-3-where = Pescado azul (arenque, trucha, sardinas, salmón, caballa), aceites de lino y de colza, nueces.
nutrition-help-omega-6-what = Una grasa poliinsaturada, esencial en la dieta.
nutrition-help-omega-6-use = La mayoría de las personas consume suficiente.
nutrition-help-omega-6-where = Aceites de colza, de maíz y de girasol, nueces, almendras, anacardos.
nutrition-help-trans-fat-what = Ácidos grasos insaturados de origen industrial o natural.
nutrition-help-trans-fat-use = Como las grasas saturadas, elevan el colesterol. OMS: menos del 1 % de la energía.
nutrition-help-trans-fat-where = Margarina, grasas vegetales y fritos; de forma natural en la carne y los lácteos de rumiantes.
nutrition-help-cholesterol-what = Una sustancia grasa que el cuerpo fabrica sobre todo en el hígado.
nutrition-help-cholesterol-use = Las grasas saturadas y trans elevan el colesterol «malo» (LDL) en la sangre.
nutrition-help-carbohydrates-what = Azúcares, almidones y, según la norma, fibra.
nutrition-help-carbohydrates-use = La principal fuente de energía del cuerpo (OMS): 4 kcal por gramo.
nutrition-help-sugars-what = Los azúcares libres son todos los añadidos a alimentos y bebidas, más los de la miel, los jarabes y los zumos; los de la leche, la fruta y la verdura no cuentan.
nutrition-help-sugars-use = Demasiado hace ganar peso y causa caries. OMS: azúcares libres por debajo del 10 % de la energía, unos 50 g.
nutrition-help-sugars-where = Caramelos, pasteles, galletas, chocolate, bebidas gaseosas, cereales de desayuno.
nutrition-help-starch-what = El hidrato de carbono de los alimentos con almidón, nuestra principal fuente de hidratos de carbono.
nutrition-help-starch-use = Los alimentos con almidón deberían ser algo más de un tercio de lo que se come.
nutrition-help-starch-where = Patatas, pan, arroz, pasta, cereales.
nutrition-help-fibers-what = Sustancias de las paredes celulares de las verduras, las frutas, las legumbres y los cereales.
nutrition-help-fibers-use = Ayudan a la digestión y previenen el estreñimiento. OMS: al menos 25 g al día (30 g en el Reino Unido).
nutrition-help-fibers-where = Pan y pasta integrales, arroz integral, patatas con piel, legumbres, verduras, frutas, frutos secos y semillas.
nutrition-help-proteins-what = Los ladrillos del cuerpo, como los músculos.
nutrition-help-proteins-use = Entre el 10 y el 15 % de la energía suele bastar en los adultos (OMS).
nutrition-help-salt-what = La sal de mesa, cloruro de sodio. Sal = sodio × 2,5: 1 g de sal contiene 400 mg de sodio.
nutrition-help-salt-use = Demasiada sube la tensión arterial y el riesgo de infarto e ictus. OMS: menos de 5 g al día.
nutrition-help-salt-where = Pan, cereales de desayuno, productos cárnicos, platos preparados, salsas, queso.
nutrition-help-sodium-what = El mineral de la sal de mesa: 1 g de sal contiene 400 mg de sodio.
nutrition-help-sodium-use = OMS: menos de 2000 mg de sodio al día.
