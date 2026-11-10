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
st-street-2-5d = Rue en 2,5D
st-focus-hint = Les flèches sélectionnent. Maj + flèches réordonnent. + et − redimensionnent, avec Maj pour de plus grands pas. Entrée modifie la largeur.
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
group-stops = Arrêts de transport en commun
group-green = Infrastructure verte
group-calming = Apaisement de la circulation

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
header-steps = Le long de la rue
header-prev = Rue précédente
header-next = Rue suivante
header-toward = vers { $place }
header-step-none = Aucune rue ne se poursuit à partir de cette extrémité.
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
view-iso-label = Éditeur de la rue, vue 2,5D
# La vue du dessin, la coupe ou la rue en 2,5D (src/street/vm.rs, src/street/page.rs).

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
material-rain_garden = Jardin de pluie
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

# Le dessin de la coupe (src/street/svg.rs).
svg-too-wide = { $length } de trop
svg-iso-label = { $name } en 2,5D, sur un tronçon de { $length }. { $count ->
        [one] { $count } élément
       *[other] { $count } éléments
    }.
svg-iso-label-zone = { $name } en 2,5D, { $zone }, de { $from } à { $to }, sur un tronçon de { $length }. { $count ->
        [one] { $count } élément
       *[other] { $count } éléments
    }.
svg-iso-label-joint = { $name } en 2,5D, sur un tronçon de { $length } centré sur la limite entre { $near } au premier plan et { $far } au-delà. La coupe change brusquement à cet endroit. { $zone } compte { $count ->
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
rev-speed = Limite de vitesse : { $kmh }{" "}km/h
rev-speed-not-set = Limite de vitesse non indiquée
rev-zone-speed = Limite de vitesse de { $kmh }{" "}km/h : { $zone }
rev-zone-speed-not-set = Limite de vitesse non indiquée : { $zone }
rev-zone-speed-street = Retour à la limite de vitesse de la rue : { $zone }
street-now-pedestrian = { $street } est maintenant une rue piétonne.
street-traffic-again = { $street } accueille de nouveau la circulation.
check-access-ped-ok = Les trottoirs offrent un passage continu de { $amount } ou plus
check-access-ped-bad = Aucun trottoir continu de { $amount } ou plus
check-detail-no-edges-pedestrian = Pas de chaussée, donc pas de trottoir exigé

# Zones (src/street/model/zones.rs) : ce que le modèle dit avoir fait aux zones d’une rue, et pourquoi il refuse.
zone-unnamed = Zone { $n }
zone-unnamed-mid = zone { $n }
rev-zone-split = Division de la rue à { $at }
rev-zone-merge = Fusion des zones; coupe gardée : { $kept }
rev-zone-move = Début de zone déplacé à { $to } : { $zone }
rev-zone-name = Nom de la zone : { $zone }
rev-zone-unname = Nom effacé : { $zone }
rev-zone-reset = Retour à la coupe de la rue : { $zone }
rev-zone-use = Coupe appliquée à toute la rue : { $zone }
zone-refused-no-anchor = Les deux extrémités de cette rue sont coupées par le bord de la carte : on ne peut pas y placer de zones.
zone-refused-elsewhere = Les zones de cette rue ont été tracées sur une rue d’une autre longueur ou depuis son autre extrémité : on ne peut pas les réorganiser ici.
zone-refused-too-short = Une zone doit mesurer au moins { $min }.
zone-refused-outside = Cette position est hors de la rue.
zone-refused-no-neighbour = Il n’y a pas de zone de ce côté avec laquelle fusionner.
zone-refused-no-boundary = La première zone commence à l’extrémité de la rue : elle n’a pas de limite à déplacer.
zone-refused-dragging = Terminez le glissement avant de modifier les zones.
zones-elsewhere = Les zones ont été tracées sur cette rue lorsqu’elle mesurait { $drawn }; ici, elle mesure { $loaded }.
check-curb-in = { $check } : { $instance }
check-stop-in = { $check } : { $instance }
check-zone-in = { $check } : { $zone }
zone-selected = { $zone }, de { $from } à { $to }, { $length } de long, { $section ->
        [own] sa propre coupe
       *[base] la coupe de base
    }, { $n } sur { $total }
zone-cut-off = Coupée par le bord de la carte

# La liste des éléments (src/street/piece_list.rs).
st-pieces-heading = Éléments de la rue
st-pieces-label = Éléments, de gauche à droite

# La bande des zones (src/street/strip.rs).
st-zones-heading = Zones le long de la rue
st-zones-strip-label = Zones de la rue, du début à la fin
st-zones-from = Origine des mesures : { $end }
st-zones-end-first = la première extrémité
st-zones-end-second = la seconde extrémité

# The commands beside the zone strip (src/street/zone_tools.rs).
zone-bad-number = Ce n’est pas un nombre.
st-zone-tools = Commandes des zones
st-zone-split = Diviser
st-zone-split-at = Diviser à ({ $unit })
st-zone-start = Début de cette zone ({ $unit })
st-zone-merge-prev = Fusionner avec la zone précédente
st-zone-merge-next = Fusionner avec la zone suivante
st-zone-merge-choice = Quelle coupe garder
st-zone-keep = Garder la coupe : { $zone }
st-zone-keep-cancel = Annuler la fusion
st-zone-name = Nom de la zone
st-zone-reset = Revenir à la coupe de la rue
st-zone-use = Utiliser cette coupe pour toute la rue
st-panel-tabs-label = Sections des détails
st-panel-tab-piece = Élément
st-panel-tab-street = Rue
st-speed-label = Limite de vitesse
st-speed-not-set = Non indiquée
st-speed-kmh = { $kmh }{" "}km/h
st-speed-street = Comme la rue : { $kmh }{" "}km/h
st-speed-street-not-set = Comme la rue : non indiquée
svg-iso-transition = Une transition de { $length } mène à { $zone }.
svg-iso-curb = { $count ->
        [one] Un usage du bord de rue est montré.
       *[other] { $count } usages du bord de rue sont montrés.
    }
svg-iso-calming = { $count ->
        [one] Un plateau surélevé est montré.
       *[other] { $count } plateaux surélevés sont montrés.
    }
svg-iso-chicane = { $count ->
        [one] Une chicane est montrée.
       *[other] { $count } chicanes sont montrées.
    }
svg-iso-filter = { $count ->
        [one] Un filtre modal est montré.
       *[other] { $count } filtres modaux sont montrés.
    }
svg-iso-green = Arbres et plantations montrés : { $count }.
svg-iso-stops = { $count ->
        [one] Un arrêt d’autobus est montré ({ $kinds }).
       *[other] { $count } arrêts d’autobus sont montrés ({ $kinds }).
    }
check-boundary = entre { $a } et { $b }
check-label-transition-room = Place pour la transition
check-label-lane-drop = Fins de voie
check-detail-transition-room = La transition demande { $need }, mais { $zone } mesure { $room }.
check-detail-transition-room-ok = Chaque transition tient dans sa zone.
check-detail-lane-drop = Une voie se termine sans voie voisine où s’insérer.
check-detail-lane-drop-ok = Chaque voie qui se termine peut s’insérer.

# Usages du bord de rue (src/street/model/curb.rs) : les usages le long de la voie de bordure d’une rue, ce que le modèle dit avoir fait, et pourquoi il refuse.
curb-use-parking = Stationnement
curb-use-loading = Zone de chargement
curb-use-tree = Fosse d’arbre
curb-use-planting = Plantation
curb-use-rain-garden = Jardin de pluie
curb-use-patio = Terrasse
curb-use-bike-parking = Supports à vélos
curb-use-platform = Quai de transport en commun
rev-curb-add = Usage ajouté à { $at } : { $use }
rev-curb-use = Usage changé à { $at } : { $use }
rev-curb-resize = Longueur modifiée à { $at } : { $use }
rev-curb-move = Usage déplacé à { $at } : { $use }
rev-curb-remove = Usage retiré à { $at } : { $use }
curb-refused-elsewhere = Les usages de ce bord de rue ont été tracés sur une rue d’une autre longueur ou depuis son autre extrémité : on ne peut pas les réorganiser ici.
curb-refused-overlap = Cet usage chevaucherait un autre usage du même côté.
curb-refused-too-long = Cet usage dépasserait l’extrémité de la rue.
curb-refused-time-windows = Cette voie change selon l’heure de la journée : on ne peut pas la diviser en usages.
curb-refused-dragging = Terminez le glissement avant de modifier le bord de rue.

# Les vérifications des usages du bord de rue (src/street/model/curb.rs).
curb-instance = { $use } à { $at }, { $side }
check-label-curb-length = Usages du bord de rue assez longs
check-label-curb-lane = Usages du bord de rue sur une voie
check-label-curb-crossing = Stationnement loin des passages pour piétons
check-label-curb-bike = Usages du bord de rue loin des pistes cyclables
check-label-curb-sidewalk = Terrasses : place sur le trottoir
check-detail-curb-length = { $use } : { $length } de long, moins que les { $min } demandés.
check-detail-curb-length-ok = Chaque usage a la longueur demandée.
check-detail-curb-lane = Il n’y a pas de voie de stationnement ou de chargement ici.
check-detail-curb-lane-ok = Chaque usage est sur une voie de stationnement ou de chargement.
check-detail-curb-crossing = Le stationnement et le chargement doivent rester à { $amount } d’un passage pour piétons.
check-detail-curb-crossing-ok = Le stationnement et le chargement restent loin des passages pour piétons.
check-detail-curb-bike-door = Les portières s’ouvriraient dans la piste cyclable voisine du stationnement. Une zone tampon de { $amount } ou plus est nécessaire.
check-detail-curb-bike-platform = Les passagers traversent la piste cyclable pour monter à bord. Le passage n’est pas dessiné : à vérifier.
check-detail-curb-bike-ok = Aucun usage ne donne sur une piste cyclable.
check-detail-curb-sidewalk = La terrasse laisse { $have } de trottoir; il en faut { $need } ou plus.
check-detail-curb-sidewalk-ok = Chaque terrasse laisse de la place sur le trottoir.

# Les bandes du bord de rue et leurs commandes (src/street/curb_strip.rs, src/street/curb_tools.rs).
curb-gap = Libre
curb-cell = { $name }, de { $from } à { $to }, { $length } de long, { $n } sur { $total }
curb-cell-failing = Ne passe pas une vérification. Voir les vérifications.
curb-choose-gap = Choisissez d’abord un tronçon libre du bord de rue.
curb-choose-use = Choisissez d’abord un usage du bord de rue.
st-curb-heading = Usages du bord de rue
st-curb-caption = Bord de rue, { $side }
st-curb-strip-label = Usages du bord de rue, { $side }
st-curb-tools = Commandes du bord de rue
st-curb-use = Usage
st-curb-length = Longueur ({ $unit })
st-curb-add = Ajouter un usage
st-curb-remove = Retirer un usage

# Arrêts de transport en commun (src/street/model/stop.rs).
stop-kind-curb = Arrêt au trottoir
stop-kind-bulb = Avancée de trottoir
stop-kind-island = Îlot d’embarquement
stop-word-sheltered = avec abribus
stop-word-unsheltered = sans abribus
rev-stop-add = Arrêt ajouté à { $at } : { $kind }
rev-stop-kind = Type d’arrêt changé à { $at } : { $kind }
rev-stop-shelter = Arrêt à { $at } : { $state }
rev-stop-name = Arrêt nommé à { $at } : { $kind }
rev-stop-resize = Longueur modifiée à { $at } : { $kind }
rev-stop-move = Arrêt déplacé à { $at } : { $kind }
rev-stop-remove = Arrêt retiré à { $at } : { $kind }

# Les vérifications des arrêts de transport en commun (src/street/model/stop.rs).
stop-instance = { $kind } à { $at }, { $side }
stop-instance-named = { $name } ({ $kind }, { $at })
check-label-stop-lane = Voie pour l’autobus aux arrêts
check-label-stop-kind = Arrêts aménagés comme leur type l’exige
check-label-stop-length = Arrêts assez longs pour un autobus
check-label-stop-width = Arrêts avec place pour embarquer
check-label-stop-bike = Arrêts loin des pistes cyclables
check-label-stop-shelter = Abribus avec place suffisante
check-detail-stop-lane = Il n’y a pas ici de voie d’autobus ou de circulation dans le sens de la circulation de ce côté.
check-detail-stop-lane-ok = Chaque arrêt a une voie pour l’autobus.
check-detail-stop-kind-curb = Un arrêt au trottoir demande que la voie de l’autobus longe le trottoir, sans voie de stationnement ni de chargement entre les deux.
check-detail-stop-kind-bulb = Une avancée de trottoir demande une voie de stationnement ou de chargement à côté de la voie de l’autobus.
check-detail-stop-kind-island = Un îlot d’embarquement demande, à côté de la voie de l’autobus, une voie de stationnement, de chargement ou une zone tampon, avec une piste cyclable derrière.
check-detail-stop-kind-ok = Chaque arrêt est aménagé comme son type l’exige.
check-detail-stop-length = L’arrêt fait { $length } de long, moins que les { $min } nécessaires à un autobus.
check-detail-stop-length-ok = Chaque arrêt est assez long pour un autobus.
check-detail-stop-width-curb = Le trottoir à côté de l’arrêt fait { $have } de large; il en faut { $need } ou plus.
check-detail-stop-width-bulb = L’avancée et le trottoir donnent { $have }; il en faut { $need } ou plus.
check-detail-stop-width-island = L’îlot fait { $have } de large; il en faut { $need } ou plus.
check-detail-stop-width-ok = Chaque arrêt laisse la place d’embarquer.
check-detail-stop-bike-cross = Les cyclistes traverseraient le quai. Le passage n’est pas dessiné : à vérifier.
check-detail-stop-bike-narrow = La piste cyclable derrière l’îlot fait { $have } de large; il en faut { $need } ou plus.
check-detail-stop-bike-ok = Aucun arrêt n’envoie les cyclistes à travers le quai.
check-detail-stop-shelter-curb = Le trottoir à côté de l’arrêt fait { $have } de large; un abribus demande { $need } ou plus.
check-detail-stop-shelter-bulb = L’avancée et le trottoir donnent { $have }; un abribus demande { $need } ou plus.
check-detail-stop-shelter-island = L’îlot fait { $have } de large; un abribus demande { $need } ou plus.
check-detail-stop-shelter-ok = Chaque abribus a la place voulue.
check-label-stop-junction = Arrêts dégagés de l’intersection
check-detail-stop-junction-crossing = Un autobus à l’arrêt se trouverait à { $gap } du passage pour piétons; il faut { $need } ou plus.
check-detail-stop-junction-mouth = Un autobus à l’arrêt se trouverait à { $gap } du début de l’intersection; il faut { $need } ou plus.
check-detail-stop-junction-inside = L’arrêt empiète sur l’intersection.
check-detail-stop-junction-ok = Chaque arrêt est dégagé de l’intersection.

# Les rangées d’arrêts et leurs commandes (src/street/row.rs, src/street/stop_tools.rs, src/street/model/stop.rs).
stop-label = { $kind }
stop-label-named = { $name } ({ $kind })
stop-label-sheltered = { $label }, avec abribus
stop-cell = { $name }, de { $from } à { $to }, { $length } de long, { $n } sur { $total }
stop-choose-gap = Choisissez d’abord un tronçon libre du bord de la rue.
stop-choose-stop = Choisissez d’abord un arrêt.
st-stop-heading = Arrêts le long de la rue
st-curb-stop-heading = Usages du bord de rue et arrêts le long de la rue
st-stop-caption = Arrêts, { $side }
st-stop-strip-label = Arrêts, { $side }
st-stop-tools = Commandes des arrêts
st-stop-name = Nom
st-stop-length = Longueur ({ $unit })
st-stop-remove = Retirer un arrêt
stop-refused-no-anchor = Les deux extrémités de cette rue sont coupées par le bord de la carte : on ne peut pas y placer d’arrêts.
stop-refused-elsewhere = Les arrêts de cette rue ont été tracés sur une rue d’une autre longueur ou depuis son autre extrémité : on ne peut pas les réorganiser ici.
stop-refused-overlap = Cet arrêt chevaucherait un autre arrêt du même côté.
stop-refused-too-long = Cet arrêt dépasserait l’extrémité de la rue.
stop-refused-time-windows = On ne peut pas placer un arrêt sur une voie qui change selon l’heure de la journée.
stop-refused-dragging = Terminez le glissement avant de modifier les arrêts.

green-kind-tree = Arbre
green-kind-bed = Plate-bande
green-kind-rain-garden = Jardin de pluie
green-kind-bioswale = Noue végétalisée
green-refused-elsewhere = Les arbres et plantations de cette rue ont été tracés sur une rue d’une autre longueur ou depuis son autre extrémité : on ne peut pas les réorganiser ici.
green-refused-dragging = Terminez le glissement avant de modifier les arbres et plantations.
green-refused-off-street = Cela dépasserait l’extrémité de la rue ou le bord de sa coupe.
green-refused-on-roadway = Cela ne peut pas se trouver sur la chaussée.
green-refused-overlap = Cela chevaucherait un autre arbre ou une autre plantation.
rev-green-add = Ajout à { $at } : { $kind }
rev-green-size = Taille modifiée à { $at } : { $kind }
rev-green-resize = Dimensions modifiées à { $at } : { $kind }
rev-green-move = Déplacement à { $at } : { $kind }
rev-green-remove = Retrait à { $at } : { $kind }
green-instance = { $kind } à { $at }, sur l’élément { $across }
green-instance-off = { $kind } à { $at }
check-green-in = { $check } : { $instance }
check-label-green-host = Plantations sur un élément qui peut les accueillir
check-label-green-clear-path = Trottoirs avec un dégagement pour circuler
check-label-green-spacing = Arbres espacés selon leur couronne
check-label-green-soil = Fosses d’arbre avec assez de terre
check-label-green-sightline = Plantations à l’écart des coins aux intersections
check-label-green-continuity = Noues végétalisées continues
check-label-green-stop = Arbres et plantations à l’écart des quais d’arrêt
check-detail-green-host-roadway = On ne peut pas planter sur la chaussée ({ $host }).
check-detail-green-host-wide = Sa largeur, { $width }, dépasse celle de l’élément en dessous ({ $host }, { $have }).
check-detail-green-host-overhang = Son bord dépasse de { $over } l’élément en dessous ({ $host }).
check-detail-green-host-off = Son centre se trouve au-delà du bord de la coupe à cet endroit.
check-detail-green-host-ok = Chaque arbre et chaque plantation reposent sur un élément qui peut les accueillir.
check-detail-green-clear-path = Il ne reste que { $have } de trottoir à côté; il en faut { $need } ou plus.
check-detail-green-clear-path-ok = Chaque trottoir garde un dégagement à côté de ses arbres et de ses plantations.
check-detail-green-spacing = { $tree } est à { $gap } de { $other }; la plus grande des deux couronnes demande { $need } entre leurs centres.
check-detail-green-spacing-ok = Chaque arbre a la place que sa couronne demande.
check-detail-green-soil = La fosse mesure { $length } sur { $width }, trop petite pour { $tree }.
green-tree-s = un petit arbre
green-tree-m = un arbre moyen
green-tree-l = un grand arbre
check-detail-green-soil-ok = Chaque fosse d’arbre a assez de terre.
check-detail-green-sightline = Il se trouve à { $gap } d’une intersection au bout de la rue; il en faut { $need } ou plus pour garder la vue dégagée.
check-detail-green-sightline-ok = Aucun arbre ni aucune plate-bande ne bloque la vue aux intersections.
check-detail-green-continuity-short = La noue végétalisée fait { $length } de long; il en faut { $min } ou plus pour qu’elle achemine l’eau.
check-detail-green-continuity-host = La noue végétalisée passe au-dessus d’un changement d’élément, ce qui interrompt le canal.
check-detail-green-continuity-ok = Chaque noue végétalisée est continue.
check-detail-green-stop = Il se trouve sur le quai de cet arrêt : { $stop }. Les arbres et les plantations restent à l’écart de l’endroit où l’on embarque.
check-detail-green-stop-ok = Aucun arbre ni aucune plate-bande ne se trouve sur le quai d’un arrêt.

# La rangée des arbres et plantations, ses commandes et ce que dit la coupe (src/street/green_strip.rs, src/street/green_tools.rs).
green-cell = { $name }, de { $from } à { $to }, { $length } de long, { $n } sur { $total }
green-choose-element = Choisissez d’abord un arbre ou une plantation.
green-host-choice = { $kind }, { $width }
green-size-s = Petit
green-size-m = Moyen
green-size-l = Grand
rev-green-change = Modification à { $at } : { $kind }
st-green-heading = Arbres et plantations le long de la rue
st-green-caption = Arbres et plantations
st-green-strip-label = Arbres et plantations le long de la rue. Les flèches en choisissent un; Alt avec une flèche le déplace de 1 m le long de la rue, avec Maj de 10 m.
st-green-tools = Commandes des arbres et plantations
st-green-kind = Type
st-green-size = Taille
st-green-at = Le long de la rue, depuis le début ({ $unit })
st-green-host = Dans la coupe, sur
st-green-fine = Dans la coupe, depuis le milieu de cet élément (cm)
st-green-length = Longueur ({ $unit })
st-green-width = Largeur ({ $unit })
st-green-change = Modifier
st-green-remove = Retirer

# Noms et refus d’un plateau surélevé ou d’un passage surélevé (src/street/model/calming.rs).
calming-kind-table = Plateau surélevé
calming-kind-crossing = Passage piéton surélevé
calming-kind-chicane = Chicane
calming-kind-modal-filter = Filtre modal
calming-refused-no-anchor = Les deux extrémités de cette rue sont coupées par le bord de la carte : on ne peut pas y placer de plateaux surélevés.
calming-refused-elsewhere = Les plateaux surélevés de cette rue ont été tracés sur une rue d’une autre longueur ou depuis son autre extrémité : on ne peut pas les réorganiser ici.
calming-refused-dragging = Terminez le glissement avant de modifier les plateaux surélevés.
calming-refused-outside = Cela dépasserait une extrémité de la rue.
calming-refused-overlap = Cela chevaucherait un autre plateau surélevé.
calming-refused-too-short = Cet élément doit mesurer au moins { $min }.
calming-refused-freeway = On ne peut pas placer de plateau surélevé sur une autoroute.
calming-refused-no-roadway = Cette rue n’a pas de chaussée à surélever.
calming-refused-time-windows = On ne peut pas placer de plateau surélevé là où la chaussée change selon l’heure de la journée.
rev-calming-add = Ajout : { $kind } à { $at }
rev-calming-kind = Élément surélevé à { $at } changé en { $kind }
rev-calming-resize = Longueur modifiée à { $at } : { $kind }
rev-calming-move = Déplacement : { $kind } à { $at }
rev-calming-remove = Retrait : { $kind } à { $at }
rev-chicane-first = Côté de départ modifié : { $kind } à { $at }
# Ce que la dernière modification a changé à la rue (src/street/impact.rs et la fiche de l’onglet Incidence).
st-impact-heading = Dernière modification
st-impact-empty = Aucune modification pour l’instant. Modifiez la rue pour voir son effet sur l’espace et sur les vérifications.
impact-no-effect = Aucun effet sur l’espace ni sur les vérifications
impact-space-heading = Espace de la rue
impact-passing-heading = Nouvelles réussites
impact-failing-heading = Nouveaux échecs
impact-changed = La rue a changé
impact-started-over = Retour à la rue d’aujourd’hui
impact-undone = Annulé : { $what }
impact-redone = Rétabli : { $what }
impact-use-more = { $use } : { $before } → { $after } ({ $amount } de plus)
impact-use-less = { $use } : { $before } → { $after } ({ $amount } de moins)
impact-verdict-improved =
    { $passing ->
        [one] Amélioration : 1 vérification réussit maintenant.
       *[other] Amélioration : { $passing } vérifications réussissent maintenant.
    }
impact-verdict-regressed =
    { $failing ->
        [one] Dégradation : 1 vérification échoue maintenant.
       *[other] Dégradation : { $failing } vérifications échouent maintenant.
    }
impact-verdict-mixed =
    Résultat mitigé : { $passing ->
        [one] 1 vérification réussit maintenant
       *[other] { $passing } vérifications réussissent maintenant
    }, { $failing ->
        [one] 1 échoue maintenant
       *[other] { $failing } échouent maintenant
    }.
impact-verdict-unchanged = Aucun changement dans les vérifications.

# Modèles de rue (src/street/model/templates.rs, page.rs, vm.rs).
rev-template = Modèle : { $name }
template-commercial = Rue commerciale
template-school = Rue scolaire
template-greenway = Voie verte de quartier
template-bus-corridor = Corridor d’autobus
template-industrial = Rue d’accès industriel
transform-templates-heading = Modèles de rue
transform-templates-hint = Aménagez la rue selon l’un de ces modèles. Les éléments de la zone choisie sont remplacés par des éléments ordinaires, modifiables et annulables en une seule modification.
template-school-applied-zone = La zone scolaire va de { $from } à { $to }.
template-school-skipped-zone = Aucune zone scolaire distincte n’a été créée : la zone sélectionnée tout entière est la zone scolaire. { $reason }
template-school-room = La zone est plus courte qu’une zone scolaire de { $length }, avec { $min } de marge de chaque côté.
template-school-applied-speed = La limite de vitesse y est de { $kmh }{" "}km/h.
template-school-skipped-speed = La limite de vitesse reste à { $kmh }{" "}km/h : elle ne dépasse pas { $limit }{" "}km/h.
template-school-applied-table = Un plateau surélevé est placé à { $at }.
template-school-skipped-table = Aucun plateau surélevé n’a été ajouté. { $reason }
template-school-table-room = La zone scolaire est plus courte qu’un plateau surélevé de { $length }.
template-refused-freeway = Le modèle « { $name } » ne peut pas s’appliquer à une autoroute.
template-refused-narrow = Le modèle « { $name } » demande une rue d’au moins { $min } de largeur. Cette rue mesure { $row } de largeur.
template-refused-wide = Le modèle « { $name } » ne peut pas servir une rue de plus de { $max } de largeur. Cette rue mesure { $row } de largeur.

# Les vérifications des éléments surélevés (src/street/model/calming.rs).
calming-instance = { $kind } à { $at }
calming-instance-zone = { $kind } à { $at }, dans { $zone }
check-calming-in = { $check } : { $instance }
check-label-calming-length = Éléments surélevés assez longs, sur une chaussée
check-detail-calming-length = Un élément surélevé de { $length } est trop court : avec des rampes de { $ramp }, il en faut { $min } ou plus.
check-detail-calming-roadway = Il n’y a pas de chaussée à surélever ici.
check-detail-calming-length-ok = Chaque élément surélevé est assez long et repose sur une chaussée.
check-label-calming-speed = Limite de vitesse adaptée aux éléments surélevés
check-detail-calming-speed = La limite de { $kmh }{" "}km/h dépasse les { $max }{" "}km/h prévus pour un élément surélevé.
check-detail-calming-speed-ok = La limite de { $kmh }{" "}km/h convient aux éléments surélevés.
check-detail-calming-speed-unknown = Aucune limite de vitesse n’est indiquée : on ne peut pas la juger par rapport aux { $max }{" "}km/h prévus pour un élément surélevé.
check-label-calming-stop = Éléments surélevés dégagés des arrêts de transport en commun
check-detail-calming-stop-overlap = L’élément surélevé chevauche l’aire d’embarquement d’un arrêt de transport en commun. Arrêt en cause : { $stop }.
check-detail-calming-stop-near = L’élément surélevé est à { $gap } de l’arrêt de transport en commun le plus proche; il en faut { $need } ou plus. Arrêt en cause : { $stop }.
check-detail-calming-stop-ok = Chaque élément surélevé est dégagé des arrêts de transport en commun.
check-label-calming-junction = Éléments surélevés dégagés de l’intersection
check-detail-calming-junction-crossing = L’élément surélevé se trouverait à { $gap } du passage pour piétons; il en faut { $need } ou plus.
check-detail-calming-junction-mouth = L’élément surélevé se trouverait à { $gap } du début de l’intersection; il en faut { $need } ou plus.
check-detail-calming-junction-inside = L’élément surélevé empiète sur l’intersection.
check-detail-calming-junction-ok = Chaque élément surélevé est dégagé de l’intersection.
check-detail-chicane-length = Une chicane de { $length } est trop courte : il lui faut deux avancées et l’espace entre elles, soit { $min } ou plus.
check-label-calming-lane = Chicanes sans voie d’autobus ni piste cyclable
check-detail-calming-lane = Une chicane rétrécit la chaussée à un seul passage : une voie d’autobus ou une piste cyclable ne peut pas la traverser.
check-detail-calming-lane-ok = Aucune chicane ne traverse une voie d’autobus ou une piste cyclable.
# La bande, les commandes et les mots dits en choisissant un élément surélevé (src/street/calming_strip.rs, calming_tools.rs).
check-detail-filter-length = Un filtre modal de { $length } est trop court : il lui faut { $min } ou plus.
check-label-calming-filter-bus = Filtres modaux sans voie d’autobus
check-detail-filter-bus = Un filtre modal bloque la circulation motorisée : une voie d’autobus ne peut pas le traverser.
check-detail-filter-bus-ok = Aucun filtre modal ne traverse une voie d’autobus.
check-label-calming-passage = Filtres modaux franchissables à pied et à vélo
check-detail-filter-passage = Un filtre modal doit avoir un trottoir ou une piste cyclable à côté pour que les piétons et les cyclistes puissent le contourner.
check-detail-filter-passage-ok = Chaque filtre modal a un trottoir ou une piste cyclable pour le contourner.
calming-gap = Libre
calming-cell = { $name }, de { $from } à { $to }, { $length } de long, { $n } sur { $total }
calming-cell-failing = Ne passe pas une vérification. Voir les vérifications.
calming-choose-gap = Choisissez d’abord un tronçon libre de la rue.
calming-choose-element = Choisissez d’abord un plateau, un passage surélevé, une chicane ou un filtre modal.
st-calming-heading = Plateaux, passages surélevés, chicanes et filtres modaux le long de la rue
st-calming-caption = Plateaux, passages surélevés, chicanes et filtres modaux
st-calming-strip-label = Plateaux, passages surélevés, chicanes et filtres modaux le long de la rue
st-calming-tools = Commandes des éléments surélevés
st-calming-kind = Type
st-calming-length = Longueur ({ $unit })
st-calming-first = Commence à
st-calming-first-left = Gauche
st-calming-first-right = Droite
st-calming-remove = Retirer un élément surélevé

# Les sources d’une règle (src/shared/sources.rs, #78), sous une vérification des pages de la rue et de l’intersection et dans le rapport.
src-summary = Source : { $status }
src-confidence-placeholder = provisoire, sans source
src-confidence-unchecked = citée, chiffres non vérifiés
src-applies-placeholder = Une valeur provisoire de CityLoom. Elle en tient lieu jusqu’à ce qu’une source faisant autorité la remplace.
src-applies-transition = S’applique là où deux zones d’une rue ont des sections différentes : il dimensionne l’empattement entre elles. Les chiffres sont provisoires et n’ont pas encore été vérifiés auprès de ces sources.
src-jurisdiction-ca = Canada
src-jurisdiction-qc = Québec
src-source = { $publisher }, { $title } ({ $jurisdiction })
src-open = Ouvrir la référence
src-open-label = Ouvrir la référence : { $title } (s’ouvre dans un nouvel onglet)
src-applies-door-buffer = Stationnement à côté d’une bande cyclable : le MTQ garde 0,5 m entre la bande et les marques de stationnement, ou élargit la bande à 2 m, pour réduire le danger des portières qui s’ouvrent (§ 15.4.2.2). L’outil sur la largeur de voies de la Ville de Montréal demande les mêmes 0,5 m là où les portières posent problème, et la zone tampon de CityLoom est ce 0,5 m. Le 1 m idéal de la fiche Vélorue à côté d’une voie à contresens n’est pas modélisé. Les autres usages à côté d’une bande cyclable n’ont pas de source.
src-applies-parking-setback = Stationnement et chargement près d’une intersection : la dernière case de stationnement se termine à au moins 5 m du début du rayon de bordure du coin (MTQ § 14.3.2.1, dessin normalisé 002). L’outil de la Ville de Montréal concorde : 5 m à partir de la fin du rayon du coin, ou de la traversée lorsqu’elle se trouve au-delà. Le dégagement de 5 m de CityLoom correspond à ce chiffre, mais il est mesuré à partir de la traversée et non du rayon de bordure.
src-applies-refuge = Un îlot séparateur médian qui sert de refuge aux piétons a au moins 2 m de largeur (MTQ § 8.10.2, tableau 8.10–1). La fiche de la Ville de Montréal demande au moins 1,8 m de largeur (2,4 m visés, soit la largeur des bandes du passage piéton) et 2,2 m de profondeur, avec une bordure de 150 mm, et favorise un refuge là où la vitesse permise dépasse 30 km/h ou là où il y a deux voies ou plus par sens. CityLoom propose un îlot là où une traversée est trop longue; ses propres chiffres sont provisoires.
src-applies-sight-triangle = Les obstacles sont tenus hors du triangle de visibilité d’une intersection (MTQ § 7.11.1). La Ville de Montréal demande 5 m dégagés à partir de la fin du rayon du coin, avec la vue ouverte entre 0,6 et 2,3 m de hauteur, et limite la plantation d’une avancée de trottoir à 0,6 m (un conducteur à 30 m doit voir un piéton à 1,5 m de la bordure). CityLoom garde les arbres et les plates-bandes à 5 m d’une intersection, comme Montréal; la bande de hauteur de la vue et la limite de plantation ne sont pas modélisées.
src-applies-stall-length = Longueur d’une case de stationnement : le MTQ dessine de 6,7 à 7,5 m pour une case et de 6 à 6,5 m pour la première et la dernière, pour un véhicule de 5 m de longueur (dessin normalisé 001). Le minimum de CityLoom pour un stationnement est plus bas et est un chiffre provisoire. Pour tout autre usage, le chiffre est provisoire et sans source.
src-summary-for = { $summary } ({ $check })
src-jurisdiction-mtl = Montréal
src-jurisdiction-none = Aucune administration en particulier
src-applies-diverter = Un déviateur sur une vélorue laisse un passage de 1,8 m aux cyclistes dans les deux sens (fiche Vélorue de la Ville de Montréal). La vérification de CityLoom demande seulement qu’un trottoir ou une bande cyclable permette de passer à pied ou à vélo; ses propres chiffres sont provisoires.
src-applies-patio-sidewalk = Café-terrasse sur le trottoir : la fiche Café-terrasse et placottoir de la Ville de Montréal garde un corridor de marche d’au moins 1,8 m sur une rue locale et 2,1 m sur une collectrice ou une artère (2,5 m sur un trottoir de plus de 5 m de largeur à forte circulation piétonne). CityLoom garde 1,8 m, le chiffre d’une rue locale; les deux autres ne sont pas modélisés.
src-applies-stop-length = Longueur d’un arrêt d’autobus : la fiche de la Ville de Montréal demande un quai d’embarquement d’au moins 8 m de longueur pour un autobus simple et 16 m pour un autobus articulé, à valider avec la STM. Le minimum de CityLoom est le 8 m d’un autobus simple; le 16 m n’est pas modélisé.
src-applies-stop-width = Aire d’embarquement : la fiche sur l’arrêt d’autobus de la Ville de Montréal retient une aire d’embarquement libre d’au moins 2,4 m de profondeur (à partir de la bordure) et 1,5 m de largeur (CSA B651), et un quai recommandé d’au moins 2,2 m de largeur, 1,8 m au minimum. Le 2,4 m de CityLoom correspond à la profondeur; ses autres largeurs sont provisoires.
src-applies-stop-bike = Bande cyclable derrière un îlot d’embarquement : la fiche sur l’arrêt d’autobus de la Ville de Montréal donne à une piste unidirectionnelle surélevée, à l’arrêt, un minimum absolu de 1,5 m (1,8 m au plus pour la partie surélevée), et 2,3 m là où la piste reste au niveau de la chaussée. CityLoom demande 1,5 m derrière un îlot; son chiffre est provisoire et correspond au minimum surélevé.
src-applies-clear-path = Passage libre à côté de la plantation : l’outil sur la largeur des corridors piétons de la Ville de Montréal donne 1,8 m comme largeur minimale recommandée (1,5 m est le minimum absolu du ministère), et plus là où la circulation piétonne est forte. Le 1,8 m de CityLoom correspond à ce minimum.
src-applies-pedestrian-access = Accès d’une rue piétonne : l’outil sur la largeur des corridors piétons de la Ville de Montréal vise de 2,0 à 2,3 m là où la circulation piétonne est faible et de 2,5 à 3,3 m là où elle est moyenne, jamais moins de 1,8 m. CityLoom demande une section continue de 3,0 m; son chiffre est provisoire et se situe dans la plage de circulation moyenne.
src-applies-raised-crossing = Traversée surélevée à une petite intersection : la fiche de la Ville de Montréal rend le plateau plus large que 2,8 m le long de la rue, typiquement de 150 mm de hauteur, avec des rampes pour véhicules de 12,5 % (8 % au minimum). Les éléments surélevés de CityLoom la suivent : un plateau d’au moins 3 m, de 150 mm de hauteur, avec des rampes de 1,2 m (12,5 %).
src-applies-atlas = Mesures de priorité au transport en commun : le menu Transformer reprend les codes et les noms de ses mesures de la boîte à outils de l’Atlas. Ce que chacune fait ici (les voies qu’elle exige, les largeurs qu’elle vérifie) est un paramètre provisoire de CityLoom, et non les indications de l’Atlas.
src-atlas-junction-label = Priorité au transport en commun aux intersections
src-applies-atlas-junction = Mesures de priorité au transport en commun à une intersection : les codes et les noms des mesures que peut avoir une approche (voies de dépassement, portes pour autobus et autres) viennent de la boîte à outils de l’Atlas. Ce que chacune fait ici est un paramètre provisoire de CityLoom, et non les indications de l’Atlas.
