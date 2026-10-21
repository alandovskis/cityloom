# Ce que dit la page de la rue : le balisage de street.html.
st-page-description = Réorganisez la largeur fixe d’une rue : ajoutez, déplacez et redimensionnez ses éléments et voyez aussitôt si tout y entre.
st-editors = Éditeurs
st-map = Carte
st-piece-details = Détails de l’élément
st-details-toggle-title = Afficher ou masquer les détails de l’élément ( [ )
st-key-select = sélectionner
st-key-reorder = réordonner
st-key-resize = redimensionner
st-key-bigger = plus grand
st-key-edit-width = modifier la largeur
st-key-remove = supprimer
st-key-details-notes = détails, notes
st-key-undo = annuler
st-key-view = coupe, 2,5D
st-cross-section = Coupe transversale
st-street-2-5d = Rue en 2,5D
st-focus-hint = Les flèches sélectionnent. Maj + flèches réordonnent. + et − redimensionnent, avec Maj pour de plus grands pas. Entrée modifie la largeur.
st-touch-cue = Sur un écran tactile, faites glisser la rue de côté pour la voir en entier.
st-space-heading = Où va la largeur
st-people-moved = Personnes déplacées
st-sample-numbers = chiffres d’exemple
st-does-it-work = Est-ce que ça fonctionne?
st-sample-limits = limites d’exemple
st-your-changes = Vos modifications
st-transit-heading = Priorité au transport en commun
st-about-street = À propos de cette rue
st-provenance-heading = D’où vient cette rue
st-no-source = Cette rue ne vient pas d’OpenStreetMap.

# Le catalogue : les genres d’éléments, les modes et les groupes du menu d’ajout (src/shared/catalogue.rs).
kind-sidewalk = Trottoir
kind-planting = Bande plantée
kind-bike = Piste cyclable
kind-travel = Voie de circulation
kind-bus = Voie réservée au transport en commun
kind-parking = Stationnement
kind-median = Terre-plein central planté
kind-loading = Zone de chargement
kind-shoulder = Accotement
kind-bikerack = Support à vélos
kind-bikeshare = Station de vélos en libre-service
kind-pole = Poteau de services publics
kind-busshelter = Abribus
kind-busstation = Station d’autobus
kind-bench = Banc
kind-terrace = Terrasse de café
kind-streetlamp = Lampadaire
kind-buffer = Zone tampon
mode-foot = Marche
mode-bike = Vélo
mode-transit = Transport en commun
mode-vehicle = Autos et camions
mode-green = Verdure
mode-neutral = Zone tampon
group-walking = Marche
group-greenery = Verdure
group-cycling = Vélo
group-transit = Transport en commun
group-roadway = Chaussée
group-furniture = Mobilier urbain
group-utilities = Services publics

# Où en sont les éléments par rapport à la largeur de la rue, et ce qui est dit après une modification.
fit-used = Chaque mètre de la rue est utilisé
fit-left = Il reste { $amount } à utiliser
fit-over = { $amount } de trop. Rétrécissez un élément ou retirez-en un
status-used = { fit-used }.
status-unused = Il reste { $amount } de la rue à utiliser.
status-over = { fit-over }.
edit-done = { $label }. { $fit }.
edit-undone = Annulé. { $fit }.
edit-redone = Rétabli. { $fit }.
selection-position = { $kind }, { $width }, { $index } sur { $total }
started-over = Retour à la rue telle qu’elle est aujourd’hui. Annulez pour retrouver vos modifications.
measure-refused = Cette mesure ne convient pas à cette rue.

# Les notes à côté de la coupe.
units-metres = mètres
units-feet = pieds
space-caption = Largeur par usage, en { $units }
col-use = Usage
col-measure = Mesure
col-today = Aujourd’hui
col-design = Votre aménagement
col-change = Variation
col-step = Étape
col-what-changed = Ce qui a changé
street-today = La rue aujourd’hui
capacity-caption = Personnes par heure, taux d’exemple
capacity-row = Personnes par heure
check-fits-full = Chaque mètre est utilisé
check-fits-over = { $amount } de trop. Rétrécissez ou retirez un élément.
check-access-ok = Une voie de { $amount } ou plus
check-access-bad = Aucune voie de { $amount } ou plus
check-passes = { " " }: réussite
check-fails = { " " }: échec
badge-fail = { " " }en échec
failing =
    { $n ->
        [one] { $n } vérification échoue
       *[other] { $n } vérifications échouent
    }
measure-arrange-label = Réorganiser la rue selon { $code } { $name }

# Le tour de la coupe : titre, titre de la fenêtre, cartouche, historique, menu d’ajout, horloge et accueil (src/street/page.rs).
header-city-map = Carte de la ville
header-section = Coupe transversale de la rue
header-wide = de large
header-between = entre
header-and = et
ends-and = { $a } et { $b }
title-between = { $street } entre { $ends } · CityLoom
title-street-editor = Éditeur de rues CityLoom
block-street = Rue
block-width = Largeur
block-changes = Modifications apportées
block-osm = OpenStreetMap
block-data = Données
block-edited = Modifié
block-imported = Tel qu’importé
history-tools = Outils de la feuille
history-undo = Annuler
history-redo = Rétablir
history-reset-title = Retour à la rue telle qu’elle est aujourd’hui. Annuler rétablit vos modifications.
history-reset = Recommencer
add-button = Ajouter un élément
add-hint = Se place après l’élément sélectionné.
clock-time-of-day = Heure de la journée
clock-except = { $kind } sauf { $others }
clock-window = { $kind } { $from }–{ $to }
clock-window-days = { $kind } { $days } { $from }–{ $to }
clock-numbers-for = Les chiffres valent pour { $time }.
day-mo = lun.
day-tu = mar.
day-we = mer.
day-th = jeu.
day-fr = ven.
day-sa = sam.
day-su = dim.
welcome-text = Ajoutez un élément, puis faites-le glisser pour le réorganiser. La largeur est fixe; un élément qui ne rentre pas devient orange.
welcome-dismiss = Compris

# Le dessin de la coupe (src/street/view.rs).
view-section-label = Éditeur de coupe transversale de la rue
view-iso-label = Éditeur de la rue, vue 2,5D
# La vue du dessin, la coupe ou la rue en 2,5D (src/street/vm.rs, src/street/page.rs).
view-shown-section = Coupe affichée
view-shown-iso = Rue affichée en 2,5D
view-mode-label = Afficher la rue en
view-mode-section = Coupe
view-mode-iso = 2,5D

# The Transit Priority Atlas measures, by lowercase code (src/shared/atlas.rs).
atlas-a1-name = Rues de transport en commun
atlas-a2-name = Voies de transport en commun
atlas-a3-name = Rues de transport en commun et d’accès direct
atlas-b1-name = Voies centrales réservées au transport en commun
atlas-b2-name = Voies centrales réservées au transport en commun sur terre-plein d’autoroute
atlas-b3-name = Voies centrales réservées au transport en commun à sens alterné fixe
atlas-b4-name = Voies centrales réservées au transport en commun à sens alterné dynamique
atlas-c1-name = Voies latérales bidirectionnelles réservées au transport en commun
atlas-d1-name = Voies décalées réservées au transport en commun
atlas-e1-name = Voies réservées au transport en commun en bordure de trottoir
atlas-e2-name = Voies réversibles de stationnement et de transport en commun en bordure de trottoir
atlas-e3-name = Voies réservées au transport en commun sur l’accotement d’autoroute
atlas-f1-name = Voies réservées au transport en commun à contresens
atlas-f2-name = Voies décalées réservées au transport en commun à contresens
atlas-g1-name = Voies décalées de dépassement de file
atlas-g2-name = Voies de dépassement de file en bordure de trottoir
atlas-g3-name = Voie virtuelle de dépassement de file
atlas-h1-name = Portes d’autobus à feux de signalisation
atlas-h2-name = Portes d’autobus à cédez le passage
atlas-l1-name = Virage à gauche indirect par un autre itinéraire
atlas-l2-name = Virage à gauche indirect dans l’intersection
atlas-l3-name = Entrée et sortie à droite seulement
atlas-l4-name = Mise en cul-de-sac des rues transversales
atlas-m1-name = Avancées de trottoir pour autobus
atlas-m2-name = Quais sur rue protégés par des feux
atlas-n1-name = Filtre modal pour le transport en commun
atlas-tsp-name = Priorité aux feux pour le transport en commun
atlas-tsp-note = L’Atlas la présente comme étant en développement, et CityLoom n’a pas de phasage de feux à prioriser.
atlas-z1-name = Zones à circulation restreinte
atlas-z1-note = Un secteur compte de nombreuses rues, et CityLoom modifie une seule rue ou une seule intersection.
atlas-w-d-name = Tarification dynamique de la congestion
atlas-w-d-note = L’Atlas la présente comme étant en développement, et un prix a besoin d’une ville et d’une demande pour agir.
atlas-w-f-name = Tarification routière à tarif fixe
atlas-w-f-note = L’Atlas la présente comme étant en développement, et un prix a besoin d’une ville et d’une demande pour agir.

# Ce qui manque à une mesure reconnue (src/street/measures.rs).
problem-narrow-lane = Une voie d’autobus mesure { $width }{ "\u00A0" }mm de large, alors qu’une voie de transport en commun en exige { $min }{ "\u00A0" }mm
problem-no-platform = Les voies centrales ont besoin d’un terre-plein ou d’un quai à côté pour les arrêts

# Les matériaux, les bordures et les sens du catalogue (src/shared/catalogue.rs).
material-asphalt = Asphalte
material-concrete = Béton
material-permeable = Revêtement perméable
material-brick = Pavés de brique
material-grass = Gazon
material-planted = Plate-bande
material-gravel = Gravier
material-trees = Arbres de rue
curb-granite = Granit
curb-concrete = Béton
curb-asphalt = Asphalte
curb-planted = Planté
curb-kassel = Bordure adaptée aux autobus
curb-bikefriendly = Bordure adaptée aux vélos
curb-island = Îlot d’embarquement d’autobus
direction-away = S’éloigne de vous
direction-toward = Vient vers vous

# Le panneau de l’élément sélectionné (src/street/inspector.rs).
inspector-empty = Sélectionnez un élément pour modifier sa largeur et son revêtement.
inspector-sub = { $width } de large · { $at } sur { $total }
inspector-sub-timed = { $width } de large · { $at } sur { $total } · { $time }
inspector-width = Largeur
inspector-step-wider = Élargir de { $step }{ "\u00A0" }{ $unit }
inspector-step-narrower = Rétrécir de { $step }{ "\u00A0" }{ $unit }
inspector-allowed = Valeurs permises : de { $min } à { $max }{ "\u00A0" }{ $unit }
inspector-surface = Revêtement
inspector-surface-note = Le matériau qui le recouvre.
inspector-planting = Plantation
inspector-planting-note = Ce qui y est planté.
inspector-vehicle = Véhicule
inspector-bus = Autobus
inspector-tram = Tramway
inspector-times = Autres périodes
inspector-times-base = { $kind } le reste de la journée.
inspector-times-add = Ajouter d’autres périodes
inspector-variant-type = Type { $n }
inspector-variant-direction = Direction { $n }
inspector-variant-remove = Retirer l’élément { $kind } de { $from } à { $to }
inspector-from = De
inspector-to = À
inspector-until = à
inspector-direction = Direction
inspector-direction-note = Le sens de la circulation.
inspector-two-way = Double sens
inspector-curb = Bordure
inspector-curb-note = Le rebord surélevé, s’il y en a un.
inspector-curb-none = Aucune (au ras du sol)

# Les sigles des éléments, là où le nom ne tient pas (src/shared/catalogue.rs).
kind-mark-sidewalk = TO
kind-mark-planting = PL
kind-mark-bike = PC
kind-mark-travel = VC
kind-mark-bus = VR
kind-mark-parking = ST
kind-mark-median = TP
kind-mark-loading = ZC
kind-mark-shoulder = AC
kind-mark-bikerack = SV
kind-mark-bikeshare = VP
kind-mark-pole = PO
kind-mark-busshelter = AB
kind-mark-busstation = SA
kind-mark-bench = BC
kind-mark-terrace = TE
kind-mark-streetlamp = LA
kind-mark-buffer = ZT

# Le dessin de la coupe (src/street/svg.rs).
svg-today = Aujourd’hui
svg-your-design = Votre aménagement
svg-change = Variation { $n }
svg-street-edge = Limite de la rue
svg-unused = Inutilisé
svg-unused-length = Inutilisé : { $length }
svg-too-wide = { $length } de trop
svg-too-wide-clipped = { $length } de trop (suite hors écran)
svg-street-width = Largeur de la rue : { $length }
svg-design-width = Votre aménagement : { $length }
svg-scale = Échelle
svg-label = Coupe transversale de { $name }. { $count ->
        [one] { $count } élément
       *[other] { $count } éléments
    }, { $total } sur { $row }. { $fit }.
svg-iso-label = { $name } en 2,5D, sur un tronçon de { $length }. { $count ->
        [one] { $count } élément
       *[other] { $count } éléments
    }.

# Ce que le modèle dit avoir fait, mis en mots par `text::say` (src/street/model.rs) : la liste des modifications et ce qui est annoncé après une modification.
rev-earlier = Modifications antérieures
rev-reset = Retour à l’état existant
rev-add = Ajout : { $kind }
rev-remove = Retrait : { $kind }
rev-move = Déplacement : { $kind }
rev-resize = Redimensionnement : { $kind }
rev-resize-pair = Redimensionnement : { $a } et { $b }
rev-surface = { $kind }, surface : { $material }
rev-curb = { $kind }, bordure : { $curb }
rev-vehicle = { $kind }, véhicule : { $vehicle }
rev-direction = { $kind }, sens : { $direction }
rev-window = { $base } devient { $kind } de { $from } à { $to }
rev-variant-direction = { $base } en tant que { $kind }, sens : { $direction }
rev-variant-remove = Retrait des autres périodes : { $kind }
rev-measure = { $code } { $name }
rev-word-none = aucune
rev-word-two-way = double sens
rev-word-tram = tramway
rev-word-bus = autobus
side-keeps-right = La circulation se fait à droite
side-keeps-left = La circulation se fait à gauche
check-label-fits = Respecte la largeur de la rue
check-label-edges = Trottoir des deux côtés
check-label-access = Place pour les véhicules d’urgence
check-detail-no-sidewalks = Une autoroute n’a pas de trottoir
check-detail-both-sides = Des deux côtés
check-detail-one-side = Un côté n’a pas de trottoir
check-detail-side-ok = Les voies vont dans le sens de la circulation de cette région
check-detail-side-bad = Une voie circule à contre-sens de la circulation, qui se fait { $side }
side-word-right = à droite
side-word-left = à gauche

# Une rue piétonne (src/street/model.rs, page.rs, vm.rs).
make-pedestrian-button = Rendre piétonne
make-pedestrian-hint = Chaque voie de circulation, espace de stationnement, zone de chargement, voie réservée ou accotement devient le genre d’élément choisi.
transform-button = Transformer
transform-transit-hint = Réorganisez la rue selon l’une de ces mesures pour le transport en commun. Chacune reconstruit la chaussée en une seule modification annulable.
rev-made-pedestrian = Rendue piétonne
street-now-pedestrian = { $street } est maintenant une rue piétonne.
street-traffic-again = { $street } accueille de nouveau la circulation.
check-access-ped-ok = Les trottoirs offrent un passage continu de { $amount } ou plus
check-access-ped-bad = Aucun trottoir continu de { $amount } ou plus
check-detail-no-edges-pedestrian = Pas de chaussée, donc pas de trottoir exigé

# Zones (src/street/model/zones.rs) : ce que le modèle dit avoir fait aux zones d’une rue, et pourquoi il refuse.
zone-unnamed = Zone { $n }
rev-zone-split = Division de la rue à { $at }
rev-zone-merge = Fusion des zones, en gardant { $kept }
rev-zone-move = Déplacement du début de { $zone } à { $to }
rev-zone-name = Nom de la zone : { $zone }
rev-zone-unname = Effacement du nom de { $zone }
rev-zone-reset = Retour de { $zone } à la coupe de la rue
rev-zone-use = { $zone } devient la coupe de toute la rue
zone-refused-no-anchor = Les deux extrémités de cette rue sont coupées par le bord de la carte : on ne peut pas y placer de zones.
zone-refused-elsewhere = Les zones de cette rue ont été tracées sur une rue d’une autre longueur ou depuis son autre extrémité : on ne peut pas les réorganiser ici.
zone-refused-too-short = Une zone doit mesurer au moins { $min }.
zone-refused-outside = Cette position est hors de la rue.
zone-refused-no-neighbour = Il n’y a pas de zone de ce côté avec laquelle fusionner.
zone-refused-no-boundary = La première zone commence à l’extrémité de la rue : elle n’a pas de limite à déplacer.
zone-refused-cut-off = Une partie de cette rue est coupée par le bord de la carte.
zone-refused-dragging = Terminez le glissement avant de modifier les zones.
