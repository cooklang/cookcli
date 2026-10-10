# Pantry
pantry-title = Jaki-tokia
pantry-intro = Etxean dagoena, non gordetzen den arabera. Erosketa-zerrendak lehendik daukazuna kanpoan uzten du.
pantry-unreadable = Ezin izan da jakitegiko fitxategia irakurri
pantry-empty = Zure jaki-tokia hutsik dago
pantry-filters = Erakutsi
pantry-filter-all = Guztiak
pantry-filter-low = Gutxi geratzen da
pantry-filter-out = Bukatuta
pantry-filter-expiring = Laster iraungitzen da
pantry-filter-empty = Ez dago iragazki honekin bat datorren elementurik.
pantry-stock-ok = Badago
pantry-expired = Iraungita
pantry-expires-today = Gaur iraungitzen da
pantry-expires-in =
    { $count ->
        [one] { $count } egun barru
       *[other] { $count } egun barru
    }
pantry-section = { $name }
pantry-no-config = Ez da jaki-tokiaren konfiguraziorik aurkitu
pantry-create-config = Sortu pantry.conf artxibo bat inbentarioaren jarraipena egiteko
pantry-configure = Konfiguratu jaki-tokia →
pantry-manage = Kudeatu jaki-tokia

# Pantry Item Fields
pantry-item-name = Elementuaren izena
pantry-item-quantity = Kopurua
pantry-item-bought = Erosita:
pantry-item-bought-date = Erosketa data
pantry-item-expire = Iraungi:
pantry-item-expire-date = Iraungitze data
pantry-item-low = Gutxienez:
pantry-item-low-threshold = Stock baxuaren atalasea
pantry-placeholder-quantity = adib. 500%g edo 2%L
pantry-placeholder-low = adib. 100%g edo 2

# Pantry Actions
pantry-add-item = Gehitu elementua
pantry-add = Gehitu
pantry-edit-item = Editatu elementua
pantry-remove-item = Ezabatu elementua
pantry-remove-confirm = Ezabatu { $name } { $section }-tik?
pantry-confirm-remove-template = Ezabatu %s %s-tik?
pantry-mark-low = Gutxi bezala markatu.
pantry-restock = Berhornitu
pantry-save = Gorde
pantry-cancel = Ezeztatu
pantry-section-label = Saila
pantry-section-required = Eman izen bat sailari
pantry-general-quantity-only = Lehen sailaren aurreko elementuek kopurua bakarrik izan dezakete. Eraman sail batera datak edo gutxieneko bat emateko.
pantry-section-freezer = Izozkailua
pantry-section-fridge = Hozkailua
pantry-section-pantry = Jaki-tokia
pantry-section-spices = Espeziak
pantry-section-general = Orokorra
pantry-tab-items = Elementuak
pantry-tab-text = Testua
pantry-text-intro = Editatu fitxategia zuzenean: [atala] lerro bat, eta gero elementu bat lerro bakoitzeko, izena = "kantitatea" edo izena = {"{"} quantity = "…", bought = "…", expire = "…", low = "…" {"}"} idatzita. # ikurrarekin hasten diren lerroak iruzkinak dira.
pantry-text-save = Gorde fitxategia
pantry-text-saved = Gordeta
pantry-text-conflict = Jakitegiko fitxategia beste nonbait aldatu da editatzen ari zinen bitartean. Zure testua gorde da; gorde berriro aldaketa hori ordezkatzeko.
pantry-rename-section = Aldatu atalaren izena
pantry-section-name = Atalaren izena
pantry-failed-rename = Ezin izan da atalaren izena aldatu
pantry-failed-save-file = Ezin izan da fitxategia gorde
pantry-optional = (aukerakoa)
pantry-failed-add = Errorea elementua gehitzerakoan
pantry-item-name-required = Eman izen bat elementuari
pantry-item-exists = %s %s sailean dago jada
pantry-section-name-taken = Badago jada %s izeneko sail edo elementu bat
pantry-failed-update = Errorea elementua eguneratzerakoan
pantry-failed-remove = Errorea elementua ezabatzerakoan
