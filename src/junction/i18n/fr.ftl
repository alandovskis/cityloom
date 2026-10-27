# Ce que dit la jonction : son modèle (src/junction/model.rs, read_model.rs), ses annonces
# (src/junction/text.rs), le tableau des virages (src/junction/turns.rs) et les vues de la page.

## Comment la jonction est réglée
jn-control-uncontrolled = Aucune signalisation
jn-control-priority = Arrêt sur les rues secondaires
jn-control-stop = Arrêt toutes directions
jn-control-signal = Feux de circulation
jn-control-roundabout = Carrefour giratoire

## La direction d’où part une branche, dans une phrase et sur une étiquette
jn-compass-n = nord
jn-compass-ne = nord-est
jn-compass-e = est
jn-compass-se = sud-est
jn-compass-s = sud
jn-compass-sw = sud-ouest
jn-compass-w = ouest
jn-compass-nw = nord-ouest
jn-compass-short-n = N
jn-compass-short-ne = NE
jn-compass-short-e = E
jn-compass-short-se = SE
jn-compass-short-s = S
jn-compass-short-sw = SO
jn-compass-short-w = O
jn-compass-short-nw = NO
# Une branche : sa rue et sa direction, « Rue Principale (nord) ».
jn-arm-name = { $street } ({ $compass })
jn-arm-to-arm = { $from } vers { $to }
# Des éléments dits l’un après l’autre.
jn-list = { $head }; { $tail }
jn-list-comma = { $head }, { $tail }

## Les mesures qu’une branche peut avoir, par code de l’Atlas
jn-approach-none = Aucune
jn-stop-none = Aucun arrêt d’autobus
jn-rule-none = Aucune gestion des virages
jn-measure-g1 = Voie décalée de dépassement de file
jn-measure-g2 = Voie de dépassement de file en bordure de trottoir
jn-measure-g3 = Voie virtuelle de dépassement de file
jn-measure-h1 = Porte d’autobus à feux de signalisation
jn-measure-h2 = Porte d’autobus à cédez le passage
jn-measure-m1 = Avancée de trottoir pour autobus
jn-measure-m2 = Quai sur rue protégé par des feux
jn-measure-l1 = Virage à gauche indirect par un autre itinéraire
jn-measure-l2 = Virage à gauche indirect dans l’intersection
jn-measure-l3 = Entrée et sortie à droite seulement
jn-measure-l4 = Mise en cul-de-sac d’une rue transversale

## Pourquoi un virage est impossible
jn-blocked-dead-end = Un cul-de-sac
jn-blocked-filter = Un filtre modal ne laisse passer que les autobus
jn-blocked-riro-leave = Entrée et sortie à droite : seuls les virages à droite sortent
jn-blocked-riro-enter = Entrée et sortie à droite : seuls les virages à droite entrent
jn-blocked-around = Les virages à gauche passent par une autre rue

## Pourquoi une modification est refusée
jn-refusal-needs-three-streets = Une jonction compte au moins trois rues.
jn-refusal-linked-no-remove = Les rues de cette jonction appartiennent à la ville; on ne peut pas les retirer.
jn-refusal-roundabout-too-big = Les rues sont trop larges pour un carrefour giratoire.
jn-refusal-bearing-blocked = C’est trop près d’une rue voisine, ou cela laisse un écart plus grand qu’une rue droite.
jn-refusal-last-way-out = Une rue doit garder au moins une sortie.
jn-refusal-lane-needs-street = Une voie doit mener à au moins une rue.
jn-refusal-island-road-too-narrow = Cette chaussée est trop étroite pour un îlot.
jn-refusal-bulb-no-parking = Il n’y a pas de stationnement à céder de ce côté.
jn-refusal-does-not-fit = Cette modification ne convient pas.
jn-refusal-raised-roundabout = Un carrefour giratoire ne peut pas être surélevé.
jn-refusal-bike-box-not-here = Un sas cyclable exige une piste cyclable sur une rue où les voitures entrent et s’arrêtent, à un carrefour qui n’est pas giratoire.
jn-refusal-protected-not-here = Un coin protégé exige une piste cyclable le long de la bordure de chacune des deux rues, un vrai coin, assez de place pour l’îlot et pas de carrefour giratoire.
jn-refusal-turbo-skewed = Une rue s’écarterait de plus de 20° de l’équerre qu’exige un giratoire turbo.
jn-refusal-turbo-entry-too-narrow = Cette rue est trop étroite pour l’élargir aux voies d’entrée que demande cette disposition.
jn-refusal-turbo-skewed-arm = La rue en cause : { $arm }.
jn-refusal-turbo-entry-too-narrow-arm = La rue trop étroite : { $arm }.
jn-refusal-turbo-lanes-fixed = Sur un giratoire turbo, la disposition fixe la destination de chaque voie.
jn-refusal-bus-on-turbo = Un giratoire turbo n’a pas de voie d’autobus à travers son îlot.
jn-refusal-queue-jump-on-turbo = Un giratoire turbo n’a pas de voie de dépassement de file à ses approches.

## Ce qu’a été chaque modification, dans l’historique
jn-word-add = ajout
jn-word-remove = retrait
jn-rev-remove = Retrait : { $arm }
jn-rev-bearing = { $street }, orientation : { $degrees }°
jn-rev-offset = { $arm }, décalage : { $length }
jn-rev-corner = Coin après { $arm } : rayon de { $length }
jn-rev-protected = Coin après { $arm }, protégé : { $change }
jn-rev-crossing = { $arm }, passage pour piétons : { $change }
jn-rev-setback = { $arm }, passage pour piétons en retrait de { $length }
jn-rev-crossing-width = { $arm }, passage pour piétons de { $length } de large
jn-rev-island = { $arm }, îlot refuge : { $change }
jn-rev-bike-box = { $arm }, sas cyclable : { $change }
jn-rev-continuous = { $arm }, trottoir continu : { $change }
jn-rev-continuous-island = { $arm }, trottoir continu : { $change }; îlot refuge retiré
jn-rev-bulb-left = { $arm }, avancée de trottoir à gauche : { $change }
jn-rev-bulb-right = { $arm }, avancée de trottoir à droite : { $change }
jn-rev-lane-to = { $arm }, voie { $lane } : vers { $dest }
jn-rev-lane-not-to = { $arm }, voie { $lane } : pas vers { $dest }
jn-rev-bus-lane = { $arm }, voie réservée aux autobus : { $change }
jn-rev-measure = { $arm } : { $code } { $measure }
jn-rev-no-approach = { $arm } : aucune mesure d’approche
jn-rev-no-stop = { $arm } : aucun arrêt d’autobus
jn-rev-no-rule = { $arm } : aucune gestion des virages
jn-rev-approach-len = { $arm }, mesure d’approche : { $length }
jn-rev-filter = { $arm }, filtre modal (N1) : { $change }
jn-rev-turn-allow = { $from } vers { $to } : permis
jn-rev-turn-ban = { $from } vers { $to } : virage interdit
jn-rev-control = Signalisation : { $control }
jn-rev-control-boxes = Signalisation : { $control }; sas cyclables retirés
jn-rev-control-protected = Signalisation : { $control }; coins protégés retirés
jn-rev-control-both = Signalisation : { $control }; sas cyclables et coins protégés retirés
jn-rev-bus-across = Voie d’autobus par le centre : { $from } vers { $to }
jn-rev-bus-across-remove = Voie d’autobus par le centre : retrait
jn-rev-cycle = Piste cyclable autour du carrefour giratoire : { $length }
jn-rev-cycle-remove = Piste cyclable autour du carrefour giratoire : retrait
jn-rev-raised = Carrefour surélevé : { $change }
jn-rev-ring = Carrefour giratoire : { $length } plus grand
jn-rev-turbo-layout = Disposition du giratoire : turbo { $layout }
jn-rev-turbo-off = Giratoire : une seule voie
jn-rev-turbo-size = Taille du giratoire turbo : { $size }
jn-rev-turbo-main = Axe principal du giratoire turbo : { $from } et { $to }

## Les vérifications
jn-check-lanes-cover = Les voies couvrent chaque virage
jn-check-lanes-cover-ok = Chaque virage permis a sa voie
jn-check-lanes-cover-bad = Aucune voie pour { $turns }
jn-no-lane-left = { $from } vers { $to } (virage à gauche)
jn-no-lane-through = { $from } vers { $to } (tout droit)
jn-no-lane-right = { $from } vers { $to } (virage à droite)
jn-check-lanes-follow = Les voies respectent les virages interdits
jn-check-lanes-follow-ok = Aucune voie ne mène à un virage interdit
jn-check-lanes-follow-bad = Voies menant à des virages interdits : { $lanes }
jn-lane-of = { $arm }, voie { $n }
jn-check-crossing = Distance de traversée
jn-check-crossing-none = Aucun passage pour piétons marqué
jn-check-crossing-ok = Plus longue traversée d’un seul coup
jn-check-crossing-island = { $arm } : trop long à traverser d’un seul coup. Un îlot refuge au centre couperait la traversée en deux
jn-check-crossing-far = { $arm } : trop long à traverser d’un seul coup
jn-check-turning-speed = Virages lents aux passages pour piétons
jn-check-turning-speed-ok = Les virages sont assez lents près de chaque passage pour piétons
jn-check-turning-speed-bad = Coin rapide après { $arms }
jn-check-corner-room = Le trottoir survit au coin
jn-check-corner-room-ok = Chaque coin laisse de la place pour attendre
jn-check-corner-room-bad = Le coin après { $arms } gruge le trottoir
jn-check-transit = Les mesures pour le transport en commun fonctionnent
jn-check-transit-ok = Chaque mesure a ce qu’il lui faut
jn-check-approach = Approches
jn-check-approach-ok = Chaque zone d’extrémité dépasse sa ligne d’arrêt et sa transition tient
jn-check-approach-bad = { $problems }
jn-approach-short = La zone d’extrémité de { $arm } finit à { $reach } du centre, avant les { $need } que demandent sa ligne d’arrêt et ses flèches.
jn-approach-room = La transition sur { $arm } demande { $need }, mais n’en a que { $room }.
jn-approach-drop = Une voie sur { $arm } se termine sans voie voisine où s’insérer.
jn-problem-offset-parking = { $arm } : une voie décalée de dépassement de file a besoin d’un stationnement le long de la bordure
jn-problem-virtual-signal = { $arm } : une voie virtuelle de dépassement de file a besoin de feux de circulation
jn-problem-gate-bus-lane = { $arm } : une porte d’autobus laisse passer une voie réservée, il lui faut donc une voie réservée aux autobus
jn-problem-gate-lane = { $arm } : une porte d’autobus a besoin d’une voie de circulation à retenir
jn-problem-bulb-parking = { $arm } : une avancée de trottoir pour autobus avance la bordure sur le stationnement, et il n’y en a pas
jn-problem-platform-signal = { $arm } : un quai protégé par des feux a besoin de feux de circulation
jn-problem-platform-crossing = { $arm } : un quai dans la rue a besoin d’un passage pour piétons qui y mène
jn-problem-filter-bus-lane = { $arm } : un filtre modal a besoin d’une voie réservée pour les autobus qui passent
jn-check-signal = Les feux ont de la place
jn-check-signal-ok = Assez peu de rues pour des feux
jn-check-signal-bad = { $n ->
    [one] { $n } rue, c’est trop pour un seul jeu de feux
   *[other] { $n } rues, c’est trop pour un seul jeu de feux
}

jn-check-turbo-square = Les rues abordent l’anneau d’équerre
jn-check-turbo-square-ok = Chaque rue aborde l’anneau à moins de 10° de l’équerre
jn-check-turbo-square-bad = { $arm } s’écarte de { $degrees }° de l’équerre.

## Ce qui est dit après une modification
jn-undone = Annulé.
jn-started-over = Retour à la jonction telle qu’elle est aujourd’hui.
jn-arms = { $n ->
    [one] { $n } rue
   *[other] { $n } rues
}
jn-fit-ok = { $arms }, { $control }. Toutes les vérifications réussissent.
jn-fit-bad = { $n ->
    [one] Une vérification est à corriger : { $checks }.
   *[other] { $n } vérifications sont à corriger : { $checks }.
}
jn-edit = { $edit }. { $fit }
jn-sel-bus = Voie d’autobus par le centre
jn-sel-cycle = Piste cyclable, { $width } de large
jn-sel-lane = Voie { $n } sur { $count }, { $arm }
jn-sel-arm = { $arm }, { $degrees ->
    [one] { $degrees } degré
   *[other] { $degrees } degrés
}
jn-sel-arm-approach = { $arm }, { $degrees ->
    [one] { $degrees } degré
   *[other] { $degrees } degrés
}. Sa zone d’extrémité va jusqu’à { $reach } du centre, puis une transition de { $length }. Zone suivante : { $zone }.
jn-sel-arm-jog = { $arm }, { $degrees ->
    [one] { $degrees } degré
   *[other] { $degrees } degrés
}. Sa zone d’extrémité va jusqu’à { $reach } du centre. Zone suivante : { $zone }.
jn-sel-corner = Coin après { $arm }, rayon de { $radius }
jn-sel-corner-protected = Coin protégé après { $arm }, rayon de { $radius }
jn-sel-crossing = Passage pour piétons sur { $arm }
jn-sel-crossing-continuous = Trottoir continu à travers { $arm }

## Le tableau des virages
jn-turns-caption = Virages permis. Les lignes sont la rue d’où vient la circulation, les colonnes la rue où elle va.
jn-turns-from = De
jn-turn-cell-left = { $from } vers { $to } : virage à gauche
jn-turn-cell-through = { $from } vers { $to } : tout droit
jn-turn-cell-right = { $from } vers { $to } : virage à droite
jn-turn-blocked = { $turn } : impossible. { $why }
jn-turn-banned = { $turn }, interdit
jn-turn-unserved = { $turn }, permis, aucune voie ne le dessert
jn-turn-allowed = { $turn }, permis

## La page autour du plan : en-tête, titre de la fenêtre, cartouche et historique (src/junction/page.rs)
jn-title = { $name } · CityLoom
jn-city-map = Carte de la ville
jn-plan = Plan de la jonction
jn-block-junction = Jonction
jn-block-streets = Rues
jn-block-changes = Modifications apportées
jn-block-osm = OpenStreetMap
jn-block-data = Données
jn-block-edited = Modifiée
jn-block-imported = Telle qu’importée
jn-undo = Annuler
jn-redo = Rétablir
jn-reset = Recommencer

## Les notes à côté du plan (src/junction/notes.rs)
jn-across-caption = La distance à traverser sur chaque rue, et le nombre de voies entrantes
jn-across-street = Rue
jn-across-to-cross = À traverser
jn-across-lanes-in = Voies entrantes
jn-across-none = aucun
jn-conflicts-caption = Points où se rencontrent les trajectoires des virages permis
jn-conflicts-kind = Type
jn-conflicts-points = Points
jn-conflicts-crossing = Croisement
jn-conflicts-merging = Convergence
jn-conflicts-splitting = Divergence
jn-conflicts-all = Total
jn-conflicts-signal = Les feux alternent les mouvements : des trajectoires qui se croisent ne se rencontrent pas en même temps.
jn-conflicts-roundabout = Dans un carrefour giratoire, la circulation ne fait que converger et diverger; elle ne se croise jamais.
jn-check-passes = { " " }: réussite
jn-check-fails = { " " }: échec
# Le détail d’une vérification avec la longueur dont il parle : « Plus longue traversée d’un seul coup : 11,4 m ».
jn-check-length = { $detail } : { $length }
jn-col-step = Étape
jn-col-what-changed = Ce qui a changé

## Le plan (src/junction/plan.rs, plan_svg.rs)
jn-plan-editor = Éditeur du plan de la jonction
# Le choix, sur le dessin, entre le plan à plat, qu’on modifie, et l’image en 2,5D, qu’on regarde.
jn-view-group = Affichage
jn-view-plan = Plan
jn-view-iso = 2,5D
jn-view-shown-plan = Plan affiché.
jn-view-shown-iso = Jonction affichée en 2,5D.
jn-iso-label = Vue en 2,5D de la jonction. { $arms }.
jn-iso-label-turbo = Vue en 2,5D de la jonction. { $arms }. { $name }.
jn-plan-label = Plan de la jonction, nord en haut. { $arms }.
jn-plan-label-roundabout = Plan de la jonction, nord en haut. { $arms }. Carrefour giratoire.
jn-plan-label-raised = Plan de la jonction, nord en haut. { $arms }. Carrefour surélevé.
jn-plan-label-turbo = Plan de la jonction, nord en haut. { $arms }. { $name }.
jn-svg-scale = Échelle
jn-svg-bus-only = Autobus seulement
# Sous le nom d’une rue : sa direction et sa largeur, « N · chaussée de 20,0 m ».
jn-svg-road = { $compass } · chaussée de { $width }
jn-svg-road-shifted = { $compass } · chaussée de { $width } · décalée de { $offset }
jn-grip-turn = Faire pivoter { $arm }
jn-grip-corner = Modifier le rayon du coin
jn-grip-crossing = Déplacer le passage pour piétons

## Le balisage de la page (web/intersection.html); le titre de la fenêtre est le jn-title de l’en-tête
jn-page-description = Modifiez le fonctionnement d’une jonction de rues : coins, passages pour piétons, voies, virages et signalisation.
jn-page-editors = Éditeurs
jn-page-map = Carte
jn-page-details = Détails
jn-page-details-toggle-title = Afficher ou masquer les détails ( [ )
jn-page-key-select = sélectionner
jn-page-key-change = modifier
jn-page-key-turn = faire pivoter une rue
jn-page-key-remove = retirer
jn-page-key-details-notes = détails, notes
jn-page-key-view = plan, 2,5D
jn-page-key-undo = annuler
jn-page-details-panel = Rue, coin ou passage pour piétons sélectionné
jn-page-plan-key = Légende des bandes
jn-page-tab-streets = Rues
jn-page-across = Traverser
jn-page-turns = Virages permis
jn-page-turns-hint = Chaque ligne est une rue d’où vient la circulation. Appuyez sur une case pour permettre ou interdire ce virage.
jn-page-conflicts = Où les trajectoires se rencontrent
jn-page-does-it-work = Est-ce que ça fonctionne?
jn-page-your-changes = Vos modifications
jn-page-fine-print = Chiffres et limites d’exemple pour l’instant, pas de vraies mesures.
jn-page-about = À propos de cette jonction
jn-provenance-heading = D’où vient cette jonction
jn-no-source = Cette jonction ne vient pas d’OpenStreetMap.

## La page d’une jonction qui ne peut pas être dessinée (src/junction/page.rs : Stuck)
jn-stuck-title = Cette jonction ne peut pas être dessinée
jn-stuck-text = Les rues qui se rencontrent ici ont été modifiées de sorte qu’elles ne forment plus une jonction. Redonnez de la place aux rues, ou recommencez la ville à partir de la carte.

## Le panneau à côté du plan : ce qui est sélectionné et de quoi le modifier (src/junction/inspector.rs)
jn-insp-empty = Sélectionnez une rue, un coin ou un passage pour piétons à modifier.
jn-insp-control = Signalisation de la jonction
jn-insp-ring-size = Taille du carrefour giratoire
# L’étiquette après la taille du carrefour giratoire : « m de diamètre ».
jn-insp-unit-across = { $unit } de diamètre
jn-insp-ring-hint = Diamètre extérieur. Au moins { $length } avec ces rues.
# Les boutons de part et d’autre d’un champ numérique, pour un lecteur d’écran : « Réduire de 0,5 m ».
jn-insp-smaller = Réduire de { $step }
jn-insp-larger = Agrandir de { $step }
jn-insp-narrower = Rétrécir de { $step }
jn-insp-wider = Élargir de { $step }
jn-insp-tighter = Resserrer de { $step }
jn-insp-closer = Rapprocher de { $step }
jn-insp-farther = Éloigner de { $step }
jn-insp-shorter = Raccourcir de { $step }
jn-insp-longer = Allonger de { $step }
jn-insp-shift-left = Décaler de { $step } vers la gauche
jn-insp-shift-right = Décaler de { $step } vers la droite
jn-insp-anticlockwise = Faire pivoter de { $degrees }° dans le sens antihoraire
jn-insp-clockwise = Faire pivoter de { $degrees }° dans le sens horaire
# Les longueurs qu’un champ accepte : « De 1,0 m à 15,0 m ».
jn-insp-range = De { $min } à { $max }
jn-insp-track-width = Largeur de la piste
jn-insp-track-hint-inside = De { $min } à { $max }. Elle prend de la place à la chaussée, dans le même cercle.
jn-insp-track-hint-crossing = De { $min } à { $max }. Les cyclistes traversent chaque rue là où elle rejoint l’anneau.
jn-insp-bus-across = Voie d’autobus par le centre
jn-insp-bus-none = Aucune voie d’autobus
jn-insp-bus-note = Une voie de { $width } réservée aux autobus, tout droit à travers l’îlot. Elle traverse l’anneau là où elle entre et là où elle sort.
jn-insp-turbo = Disposition du giratoire
jn-insp-turbo-single = Une seule voie
jn-insp-turbo-size = Taille du giratoire turbo
jn-insp-turbo-smaller = Taille inférieure
jn-insp-turbo-larger = Taille supérieure
jn-insp-turbo-main = Axe principal
jn-insp-turbo-main-is = { $from } et { $to } forment l’axe principal.
jn-insp-turbo-main-switch = Prendre les autres rues comme axe principal
jn-insp-turbo-main-swap = Mettre les deux voies d’entrée sur l’autre rue
jn-insp-lane-fixed = La disposition turbo fixe la destination de cette voie.
jn-insp-bus-offer = Permet aux autobus de couper à travers l’îlot entre deux rues.
jn-insp-cycle = Piste cyclable
jn-insp-cycle-option = Piste tout autour, à l’extérieur
jn-insp-bus = Voie d’autobus
jn-insp-bus-remove = Retirer la voie d’autobus
jn-insp-cycle-sub = Tout autour du carrefour giratoire, à l’extérieur
jn-insp-cycle-remove = Retirer la piste cyclable
jn-insp-raised = Carrefour surélevé
jn-insp-raised-option = Surélever tout le carrefour au niveau du trottoir
jn-insp-continuous = Trottoir continu
jn-insp-continuous-option = Le trottoir se poursuit à travers la rue
jn-insp-corner = Coin
jn-insp-curb-radius = Rayon de la bordure
jn-insp-corner-hint = De { $min } à { $max }. Les voitures tournent ici à environ { $speed }{ " " }km/h.
jn-insp-corner-note = Un coin serré ralentit les voitures qui tournent et raccourcit la traversée à pied. Un coin large les laisse tourner plus vite.
# Le bouton d’un virage d’une voie, pour un lecteur d’écran : « Voie 1 vers Rue Principale (sud), tout droit ».
jn-insp-dest-left = Voie { $n } vers { $street }, à gauche
jn-insp-dest-through = Voie { $n } vers { $street }, tout droit
jn-insp-dest-right = Voie { $n } vers { $street }, à droite
jn-insp-lane = Voie { $n }
# La place d’une voie parmi celles de sa rue.
jn-insp-lane-only = la seule voie
jn-insp-lane-middle = la plus près du centre
jn-insp-lane-curb = la plus près de la bordure
jn-insp-lane-title = Voie { $n } sur { $count }
# Sous le titre d’une voie : sa rue et sa place, « Rue Principale (nord) · la plus près de la bordure ».
jn-insp-lane-sub = { $arm } · { $place }
jn-insp-lane-banned = Chaque rue où elle mène est interdite. Ajoutez une rue, ou permettez un virage.
jn-insp-lane-width = { $width } de large. Une voie doit mener à au moins une rue.
jn-insp-goes-left = À gauche vers { $street }
jn-insp-goes-through = Tout droit vers { $street }
jn-insp-goes-right = À droite vers { $street }
# Une rue où une voie ne peut pas mener, parce que sa circulation ne fait qu’entrer.
jn-insp-one-way-in = { $goes } (sens unique vers la jonction)
jn-insp-lane-goes = Où mène cette voie
jn-insp-whole-street = Sélectionner toute la rue
jn-insp-setback = Retrait par rapport à la jonction
jn-insp-crossing-width = Largeur du passage pour piétons
jn-insp-bulb-left = Avancée de trottoir à gauche
jn-insp-bulb-right = Avancée de trottoir à droite
jn-insp-no-parking = { $bulb } (pas de stationnement de ce côté)
jn-insp-crossing = Passage pour piétons
jn-insp-crossing-stages = { $n ->
    [one] { $n } traversée de { $length }
   *[other] { $n } traversées de { $length }
}
jn-insp-crossing-one-go = { $length } à traverser
jn-insp-too-far = { $crossing }. Trop long à traverser d’un seul coup.
jn-insp-crossing-remove = Retirer le passage pour piétons
jn-insp-crossing-mark = Marquer un passage pour piétons
jn-insp-halfway = Îlot à mi-traversée
jn-insp-island = Îlot refuge au centre
jn-insp-island-narrow = Îlot refuge (chaussée trop étroite)
jn-insp-island-continuous = Îlot refuge (pas sur un trottoir continu)
jn-insp-bike-box = Sas cyclable
jn-insp-bike-box-option = Sas d’attente pour les cyclistes devant les voitures
jn-insp-bike-box-roundabout = Sas cyclable (pas à un carrefour giratoire)
jn-insp-bike-box-no-lane = Sas cyclable (exige une piste cyclable sur cette rue)
jn-insp-bike-box-no-entry = Sas cyclable (exige une voie d’entrée pour les voitures)
jn-insp-bike-box-free = Sas cyclable (les voitures ne s’arrêtent pas sur cette rue)
jn-insp-protected = Coin protégé
jn-insp-protected-option = Piste cyclable autour d’un îlot de coin
jn-insp-protected-roundabout = Coin protégé (pas à un carrefour giratoire)
jn-insp-protected-straight = Coin protégé (ce coin est une ligne droite)
jn-insp-protected-no-lane = Coin protégé (exige une piste cyclable le long de la bordure des deux rues)
jn-insp-protected-no-room = Coin protégé (la rue est trop étroite pour l’îlot)
jn-insp-shorten = Raccourcir la traversée
jn-insp-transit = Priorité au transport en commun
jn-insp-bus-lane-in = Voie réservée aux autobus en entrée
jn-insp-approach = À l’approche
jn-insp-stop = Arrêt d’autobus
jn-insp-rule = Virages
# Une mesure dans une liste, avec son code de l’Atlas : « G2 Voie de dépassement de file en bordure de trottoir ».
jn-insp-coded = { $code } { $name }
jn-insp-filter = Filtre modal pour le transport en commun
jn-insp-queue-length = Longueur de la voie de dépassement de file
jn-insp-gate-distance = Distance de la porte d’autobus en amont
# Sous le nom d’une rue : sa direction, son orientation et sa largeur, « E, 90° · chaussée de 11,4 m ».
jn-insp-arm-sub = { $compass }, { $degrees }° · chaussée de { $width }
jn-insp-direction = Orientation
jn-insp-direction-hint = Dans le sens horaire à partir du nord. Au moins { $degrees }° de ses voisines.
jn-insp-shift = Décalage latéral
jn-insp-shift-hint = Jusqu’à { $length } d’un côté ou de l’autre. Vu depuis la jonction.
jn-insp-street = Rue
jn-insp-cross-section = Ouvrir la coupe transversale
jn-insp-street-note = Cette rue appartient à la ville. Son aménagement se modifie dans l’éditeur de rue, et les modifications faites là-bas s’affichent ici.
jn-insp-remove-street = Retirer cette rue
jn-insp-min-arms = { $n ->
    [one] Une jonction compte au moins { $n } rue.
   *[other] Une jonction compte au moins { $n } rues.
}
jn-insp-lanes-in = Voies entrantes
jn-insp-one-way-out = Sens unique sortant. Aucune voie n’entre.

# Une rue piétonne à une jonction (src/junction/model.rs, inspector.rs).
jn-refusal-crossing-pedestrian = Une rue piétonne n’a pas de chaussée à traverser; elle n’a donc pas de passage pour piétons.
jn-refusal-measure-pedestrian = Une rue piétonne n’accueille pas de circulation; elle ne prend donc aucune mesure de circulation.
jn-insp-crossing-pedestrian = Une rue piétonne n’a pas de chaussée à traverser.
jn-insp-lanes-pedestrian = Une rue piétonne n’a pas de voies de circulation.
jn-insp-control-pedestrian = { $n ->
    [0] Aucune rue ici n’accueille la circulation.
   *[other] Une seule rue ici accueille la circulation.
}

jn-refusal-not-signal = Le réglage des feux n’existe qu’à un carrefour à feux de circulation.
jn-refusal-timing-out-of-range = Cette durée est hors de ce qu’un feu peut utiliser.
jn-refusal-no-transit-lane = Une avance pour le transport en commun exige une voie réservée sur cette rue.
jn-timing-green = Vert
jn-timing-yellow = Jaune
jn-timing-all-red = Rouge intégral
jn-timing-transit-lead = Avance pour le transport en commun
jn-rev-timing = { $arm } : { $field } { $seconds } s
jn-rev-control-timing = Signalisation : { $control }; réglage des feux retiré
jn-rev-control-boxes-timing = Signalisation : { $control }; sas cyclables et réglage des feux retirés
jn-rev-control-protected-timing = Signalisation : { $control }; coins protégés et réglage des feux retirés
jn-rev-control-all = Signalisation : { $control }; sas cyclables, coins protégés et réglage des feux retirés

jn-check-signal-walk = Temps de traversée
jn-check-signal-walk-ok = Chaque traverse est ouverte assez longtemps
jn-check-signal-walk-bad = Pas assez de temps pour traverser à { $arms }
jn-check-signal-cycle = Cycle des feux
jn-check-signal-cycle-ok = Le cycle dure { $seconds } s
jn-check-signal-cycle-bad = Le cycle dure { $seconds } s, au-delà des { $max } s qu’un feu devrait garder

jn-insp-signal = Réglage des feux
jn-sig-cycle = Cycle : { $seconds } s
jn-sig-row = { $street } : vert { $green } s, jaune { $yellow } s, rouge intégral { $all_red } s
jn-sig-row-lead = { $street } : avance { $lead } s, vert { $green } s, jaune { $yellow } s, rouge intégral { $all_red } s
jn-sig-walks = Les traverses de { $arms } sont aussi ouvertes.
jn-sig-rides = Les vélos de { $arms } roulent aussi.
jn-sig-alone = Rien d’autre ne bouge pendant cette phase.
jn-sig-no-phase = Rien n’entre par cette rue : elle n’a pas de vert à elle.
jn-sig-shorter = Plus court de { $seconds } s
jn-sig-longer = Plus long de { $seconds } s
jn-sig-range = De { $min } à { $max } s

## Giratoires turbo : les dispositions et les tailles (src/junction/turbo.rs)
jn-turbo-basic = De base
jn-turbo-egg = En œuf
jn-turbo-knee = En genou
jn-turbo-spiral = En spirale
jn-turbo-rotor = Rotor
jn-turbo-size-mini = Mini
jn-turbo-size-regular = Normale
jn-turbo-size-medium = Moyenne
jn-turbo-size-large = Grande
# Ce qu’est un dessin de la jonction, pour un lecteur d’écran : « Giratoire turbo de base, taille normale ».
jn-turbo-label-basic = Giratoire turbo de base, taille { $size }
jn-turbo-label-egg = Giratoire turbo en œuf, taille { $size }
jn-turbo-label-knee = Giratoire turbo en genou, taille { $size }
jn-turbo-label-spiral = Giratoire turbo en spirale, taille { $size }
jn-turbo-label-rotor = Giratoire turbo rotor, taille { $size }

jn-refusal-phase-serves-nothing = Rien n’entre par cette rue : il n’y a pas de phase à lui ajouter.
jn-refusal-last-phase = Un feu garde au moins une phase.
jn-refusal-phase-at-end = Cette phase ne peut pas aller plus loin dans ce sens.
jn-refusal-plan-suggested = Le plan des feux est déjà celui qui est suggéré.
jn-rev-phase-add = Phase { $n } ajoutée pour { $arm }
jn-rev-phase-remove = Phase { $n } de { $arm } retirée
jn-rev-phase-earlier = Phase de { $arm } avancée, maintenant { $n } sur { $total }
jn-rev-phase-later = Phase de { $arm } reculée, maintenant { $n } sur { $total }
jn-rev-phase-restore = Plan des feux rétabli à la suggestion
jn-check-signal-phases = Chaque rue a une phase
jn-check-signal-phases-ok = Chaque rue d’où part la circulation a une phase
jn-check-signal-phases-bad = Aucune phase ne dessert la circulation qui entre par { $arms }
jn-sig-suggested = Plan suggéré
jn-sig-edited = Plan modifié
jn-sig-add = Ajouter une phase pour { $street }
jn-sig-earlier = Avancer la phase de { $street }
jn-sig-later = Reculer la phase de { $street }
jn-sig-remove = Retirer la phase de { $street }
jn-sig-restore = Rétablir le plan suggéré
jn-sig-earlier-short = Avancer
jn-sig-later-short = Reculer
jn-sig-remove-short = Retirer
jn-rev-plan-lost = { $change }; plan de feux modifié retiré
