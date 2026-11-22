# Ce que dit la jonction : son modèle (src/junction/model/mod.rs, read_model.rs), ses annonces
# (src/junction/vm/text.rs), le tableau des virages (src/junction/view/turns.rs) et les vues de la page.

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
jn-check-stop-clear = Arrêts d’autobus dégagés de l’intersection
jn-check-stop-clear-ok = Chaque arrêt d’autobus est dégagé de l’intersection.
jn-check-stop-clear-bad = { $stops }
jn-check-stop-lane = Les autobus à l’arrêt ne sont pas pris dans une voie de virage
jn-check-stop-lane-ok = Aucun arrêt d’autobus ne retient un autobus dans une voie de virage.
jn-check-stop-lane-bad = { $stops }
jn-edge-crossing = du passage pour piétons
jn-edge-mouth = du bord de l’intersection
jn-or = { $head } ou { $tail }
jn-turn-left = à gauche
jn-turn-through = tout droit
jn-turn-right = à droite
jn-use-left = de tourner à gauche
jn-use-through = d’aller tout droit
jn-use-right = de tourner à droite
jn-stop-too-near = L’arrêt ({ $kind }) sur { $arm } se trouve à { $gap } { $edge }; il faut { $need } ou plus.
jn-stop-inside = L’arrêt ({ $kind }) sur { $arm } empiète sur l’intersection.
jn-stop-turn-only = L’arrêt ({ $kind }) sur { $arm } est dans la voie { $n }, qui ne permet que de tourner { $turn } : un autobus qui poursuit sa route doit changer de voie devant la circulation.
jn-stop-shared-turn = L’arrêt ({ $kind }) sur { $arm } est dans la voie { $n }, partagée avec les virages { $turn } : les voitures qui tournent font la file derrière l’autobus à l’arrêt.
jn-problem-stop-twice = { $arm } : la rue a déjà un arrêt ici; gardez soit la mesure ({ $measure }), soit l’arrêt de la rue, pas les deux.
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
# La zone de l’autre côté de la limite d’une zone d’extrémité, par son numéro depuis l’extrémité d’ancrage de la rue (src/street/model/beyond.rs).
zone-unnamed-mid = zone { $n }
jn-side-left = à gauche vu du centre
jn-side-right = à droite vu du centre
jn-sel-arm-stop = { $kind } { $side }, à { $at } du centre, dans la voie { $n } sur { $of }, qui permet { $uses }.
jn-sel-arm-stop-bus = { $kind } { $side }, à { $at } du centre, dans la voie réservée aux autobus.
jn-sel-arm-stop-bare = { $kind } { $side }, à { $at } du centre.
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

## La page autour du plan : en-tête, titre de la fenêtre, cartouche et historique (src/junction/view/page.rs)
jn-title = { $name } · CityLoom
jn-city-map = Carte de la ville
jn-heading = Jonction en 2,5D
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

## Les notes à côté du plan (src/junction/view/notes.rs)
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
jn-rev-step = Étape { $n }

## Le dessin (src/junction/view/drawing.rs, iso/)
jn-editor = Éditeur de la jonction
jn-iso-label = Vue en 2,5D de la jonction. { $arms }.
jn-iso-label-roundabout = Vue en 2,5D de la jonction. { $arms }. Carrefour giratoire.
jn-iso-label-raised = Vue en 2,5D de la jonction. { $arms }. Carrefour surélevé.
jn-iso-label-turbo = Vue en 2,5D de la jonction. { $arms }. { $name }.
jn-svg-bus-only = Autobus seulement
# Ce que les boutons sous le dessin parcourent : les rues, les voies, les passages et les coins, comme les touches fléchées.
jn-part-previous = Partie précédente
jn-part-next = Partie suivante

## Le balisage de la page (web/intersection.html); le titre de la fenêtre est le jn-title de l’en-tête
jn-page-description = Modifiez le fonctionnement d’une jonction de rues : coins, passages pour piétons, voies, virages et signalisation.
jn-page-details = Détails
jn-page-details-toggle-title = Afficher ou masquer les détails ( [ )
jn-page-key-select = sélectionner
jn-page-key-change = modifier
jn-page-key-turn = faire pivoter une rue
jn-page-key-remove = retirer
jn-page-key-details-notes = détails, notes
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

## La page d’une jonction qui ne peut pas être dessinée (src/junction/view/page.rs : Stuck)
jn-stuck-title = Cette jonction ne peut pas être dessinée
jn-stuck-text = Les rues qui se rencontrent ici ont été modifiées de sorte qu’elles ne forment plus une jonction. Redonnez de la place aux rues, ou recommencez la ville à partir de la carte.

## Le panneau à côté du plan : ce qui est sélectionné et de quoi le modifier (src/junction/view/inspector.rs)
jn-insp-empty = Sélectionnez une rue, un coin ou un passage pour piétons à modifier.
jn-path-label = Ce que montrent les détails
jn-path-junction = Jonction
jn-path-lane = Voie { $n }
jn-insp-junction-sub =
    { $streets ->
        [one] 1 rue
       *[other] { $streets } rues
    } · { $control }
jn-insp-streets = Rues
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
jn-fold-none = Aucune
jn-fold-bus-lane = Voie réservée
jn-fold-position = Position
jn-fold-position-set = { $degrees }°, { $shift }
jn-fold-not-shifted = sans décalage
jn-fold-shifted = décalée de { $length }
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

# Une rue piétonne à une jonction (src/junction/model/mod.rs, src/junction/view/inspector.rs).
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
jn-path-plan = Plan des feux
jn-sig-summary =
    Cycle de { $seconds } s · { $phases ->
        [one] 1 phase
       *[other] { $phases } phases
    }
jn-sig-row = { $street } : vert { $green } s, jaune { $yellow } s, rouge intégral { $all_red } s
jn-sig-row-lead = { $street } : avance { $lead } s, vert { $green } s, jaune { $yellow } s, rouge intégral { $all_red } s
jn-sig-walks = Les traverses de { $arms } sont aussi ouvertes.
jn-sig-rides = Les vélos de { $arms } roulent aussi.
jn-sig-alone = Rien d’autre ne bouge pendant cette phase.
jn-sig-no-phase = Rien n’entre par cette rue : elle n’a pas de vert à elle.
jn-sig-shorter = Plus court de { $seconds } s
jn-sig-longer = Plus long de { $seconds } s
jn-sig-range = De { $min } à { $max } s

## Giratoires turbo : les dispositions et les tailles (src/junction/model/turbo.rs)
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
jn-sig-phases-cut =
    { $n ->
        [one] { $n } phase au-delà des { $max } qu’un plan peut avoir a été laissée de côté à l’ouverture de l’intersection.
       *[other] { $n } phases au-delà des { $max } qu’un plan peut avoir ont été laissées de côté à l’ouverture de l’intersection.
    }
jn-sig-earlier-short = Avancer
jn-sig-later-short = Reculer
jn-sig-remove-short = Retirer

## Le plan des feux en dessins, un par phase
jn-ph-name = Phase { $n } : { $streets }
jn-ph-and = { $head } et { $tail }
jn-ph-cars = Voitures qui passent : { $arms }.
jn-ph-walks = Piétons qui traversent : { $arms }.
jn-ph-rides = Vélos qui passent : { $arms }.
jn-ph-join = { $head } { $tail }
jn-ph-row = Les phases des feux, dans l’ordre où elles se suivent
jn-ph-timing = Vert { $green } s, jaune { $yellow } s, rouge intégral { $all_red } s
jn-ph-timing-lead = Avance { $lead } s, vert { $green } s, jaune { $yellow } s, rouge intégral { $all_red } s
jn-rev-plan-lost = { $change }; plan de feux modifié retiré
jn-refusal-too-many-phases = Le plan a déjà le nombre maximal de phases.

## Modèles de phase prêts à l’emploi (src/junction/model/signal.rs)
jn-pattern-through = Tout droit seulement
jn-pattern-through-desc = Seule la circulation qui va tout droit passe; les traverses à côté sont franchies à pied.
jn-pattern-all = Tous les mouvements
jn-pattern-all-desc = Tous les mouvements de la rue passent; les véhicules qui tournent cèdent le passage aux piétons.
# "protected-left" : le virage qui croise la circulation en sens inverse (à droite là où l’on conduit à gauche); la clé est gardée pour sa stabilité.
jn-pattern-protected-left = Virage protégé en croisant la circulation, puis tout droit
jn-pattern-protected-left-desc = Le virage qui croise la circulation en sens inverse (à gauche en conduisant à droite, à droite en conduisant à gauche) passe d’abord, sans piéton là où il tourne, puis les autres mouvements de la rue.
jn-pattern-scramble = Phase piétonne exclusive
jn-pattern-scramble-desc = Aucun véhicule ne bouge et les piétons traversent toutes les rues.
jn-pattern-custom = Personnalisé
jn-pattern-custom-desc = Les mouvements de cette phase ne correspondent à aucun modèle prêt à l’emploi.
jn-pattern-group = Modèle de la phase { $n }, { $street }
jn-rev-phase-pattern = Phase { $n } de { $arm } : { $pattern }
jn-refusal-pattern-not-fit = Ce modèle ne convient pas aux voies de cette intersection.
jn-refusal-pattern-already = La phase suit déjà ce modèle.
jn-pattern-lost = La phase { $n } ne suit plus un modèle prêt à l’emploi.
jn-pattern-lost-many = Les phases { $list } ne suivent plus un modèle prêt à l’emploi.
jn-rev-phase-walk-on = Phase { $n } de { $arm } : les piétons traversent maintenant { $street }
jn-rev-phase-walk-off = Phase { $n } de { $arm } : les piétons ne traversent plus { $street }
jn-rev-phase-ride-on = Phase { $n } de { $arm } : les vélos circulent maintenant sur { $street }
jn-rev-phase-ride-off = Phase { $n } de { $arm } : les vélos ne circulent plus sur { $street }
jn-refusal-no-crossing = Cette rue n’a pas de passage pour piétons.
jn-refusal-no-bike-lane = Cette rue n’a pas de voie cyclable.
jn-refusal-scramble-walks = Une phase piétonne exclusive fait traverser tous les passages; choisissez un autre modèle pour les régler un à un.
jn-refusal-movement-already = La phase est déjà réglée ainsi.
jn-ph-walks-group = Piétons en marche à la phase { $n }, { $street }
jn-ph-rides-group = Vélos à la phase { $n }, { $street }
jn-ph-walks-legend = Piétons en marche
jn-ph-rides-legend = Vélos
jn-ph-walk-box = Les piétons traversent { $other }
jn-ph-ride-box = Les vélos circulent sur { $other }
jn-ph-walks-fixed = Cette phase est une phase piétonne exclusive : tout le monde traverse toutes les rues. Choisissez un autre modèle pour régler les passages un à un.

## Appels piétons (src/junction/model/calls.rs, #60) : un passage sur appel n’est ouvert que si un piéton l’appelle
jn-refusal-call-not-walked = Aucune phase du plan n’ouvre ce passage, il ne peut donc pas être sur appel : ouvrez-le d’abord dans une phase.
jn-refusal-call-already = Le passage est déjà réglé ainsi.
jn-refusal-not-on-call = Seul un passage sur appel a un nombre d’appels par heure.
jn-refusal-call-rate-out-of-range = Ce nombre d’appels par heure dépasse la borne provisoire indiquée.
jn-rev-call-on = Passage de { $arm } sur appel
jn-rev-call-off = Passage de { $arm } remis en rappel
jn-rev-calls-lost =
    { $change }; { $n ->
        [one] { $n } passage qui n’est plus sur appel
       *[other] { $n } passages qui ne sont plus sur appel
    }
jn-rev-call-rate =
    Passage de { $arm } : { $rate ->
        [one] { $rate } appel par heure
       *[other] { $rate } appels par heure
    }
jn-calls-heading = Appels piétons
jn-calls-intro = Un passage sur appel n’est ouvert que lorsqu’un piéton l’appelle, par bouton-poussoir ou détection passive. Un passage en rappel est ouvert chaque fois que ses phases passent, et les appels ne changent rien à son service.
jn-calls-bounds = Valeurs provisoires : marche de { $walk } s, puis dégagement à { $speed } m/s, arrondi à la seconde supérieure; un vert est prolongé d’au plus { $max } s; avec la détection passive, un piéton part après { $limit } s d’attente; les appels sont répartis sur une heure.
jn-calls-group = Passage de { $street }
jn-calls-on-call = Sur appel
jn-calls-rate = Appels par heure
jn-calls-rate-range = De { $min } à { $max } appels par heure (borne provisoire)
jn-calls-recall = En rappel : ouvert chaque fois que ses phases passent; les appels ne changent rien à son service.
jn-calls-need = Sur appel : marche { $walk } s + dégagement { $clearance } s = { $need } s de vert.
jn-calls-unwalked = Aucune phase n’ouvre ce passage : il ne peut pas être sur appel.
jn-calls-none = Aucun passage n’est sur appel : chaque passage est ouvert chaque fois que ses phases passent.
jn-calls-dropped =
    { $n ->
        [one] { $n } passage n’est plus sur appel depuis l’ouverture de l’intersection : elle n’est pas réglée par des feux.
       *[other] { $n } passages ne sont plus sur appel depuis l’ouverture de l’intersection : elle n’est pas réglée par des feux.
    }
jn-calls-kept =
    { $n ->
        [one] { $n } passage reste sur appel pour quand les feux fonctionneront de nouveau.
       *[other] { $n } passages restent sur appel pour quand les feux fonctionneront de nouveau.
    }
jn-calls-totals-caption = Appels en une heure, par passage
jn-calls-col-crossing = Passage
jn-calls-col-calls = Appels
jn-calls-col-served = Servis
jn-calls-col-delayed = Retardés
jn-calls-col-extended = Vert prolongé
jn-calls-col-not-served = Non servis
jn-calls-col-longest = Attente la plus longue
jn-calls-col-at = Appel à
jn-calls-col-outcome = Résultat
jn-calls-seconds = { $seconds } s
jn-calls-no-wait = –
jn-calls-no-wait-said = aucun appel servi
jn-calls-rows =
    { $n ->
        [one] L’appel, une ligne
       *[other] Chaque appel, une ligne par appel ({ $n })
    }
jn-calls-rows-caption = Chaque appel de l’heure, dans l’ordre où il a été fait
jn-calls-at = { $minutes } min { $seconds } s
jn-calls-outcome-served = Servi après { $wait } s
jn-calls-outcome-served-extended = Servi après { $wait } s, vert prolongé de { $extended } s
jn-calls-outcome-delayed = Retardé de { $wait } s
jn-calls-outcome-delayed-extended = Retardé de { $wait } s, vert prolongé de { $extended } s
jn-calls-outcome-not-served = Non servi
jn-calls-findings = Constats
jn-calls-finding-cannot = Le passage de { $arm } ne peut pas être servi en sécurité : la marche et le dégagement demandent { $need } s de vert, et aucune phase qui l’ouvre ne peut donner plus de { $most } s.
jn-calls-finding-unwalked = Le passage de { $arm } est sur appel, mais aucune phase ne l’ouvre : ses appels ne sont pas servis. Ouvrez-le dans une phase ou remettez-le en rappel.
jn-calls-said =
    Passage de { $arm } : { $calls ->
        [one] { $calls } appel
       *[other] { $calls } appels
    } dans l’heure; servis { $served }, retardés { $delayed }, vert prolongé { $extended }, non servis { $not_served }; attente la plus longue { $wait } s.
jn-calls-said-no-wait =
    Passage de { $arm } : { $calls ->
        [one] { $calls } appel
       *[other] { $calls } appels
    } dans l’heure; servis { $served }, retardés { $delayed }, vert prolongé { $extended }, non servis { $not_served }; aucun appel n’a été servi.

## Détection passive des piétons (src/junction/model/calls.rs, CLM-17) : la détection en bordure de trottoir place l’appel et l’annule quand le piéton part
jn-refusal-detection-not-walked = Aucune phase du plan n’ouvre ce passage, la détection passive ne peut donc pas le servir : ouvrez-le d’abord dans une phase.
jn-refusal-detection-needs-call = Seul un passage sur appel détecte les piétons : mettez-le d’abord sur appel.
jn-rev-call-passive = Passage de { $arm } : détection passive
jn-rev-call-push-button = Passage de { $arm } : bouton-poussoir
jn-calls-detection = Détection
jn-calls-detection-push-button = Bouton-poussoir
jn-calls-detection-passive = Détection passive
jn-calls-detection-push-button-help = Le piéton appuie sur le bouton pour appeler la marche; l’appel reste placé après son départ, et la marche est donc donnée même à un passage vide.
jn-calls-detection-passive-help = Un radar, un capteur infrarouge ou une caméra en bordure de trottoir place l’appel pendant qu’un piéton attend; celui qui part après { $limit } s d’attente l’annule, et une marche que plus personne n’attend est omise. La marche et le dégagement ne sont jamais raccourcis.
jn-calls-col-cancelled = Annulés
jn-calls-col-skipped = Marches omises
jn-calls-outcome-cancelled = Annulé : parti après { $wait } s d’attente
jn-calls-said-passive =
    Détection passive : { $cancelled ->
        [one] { $cancelled } appel annulé
       *[other] { $cancelled } appels annulés
    }, { $skipped ->
        [one] { $skipped } marche omise
       *[other] { $skipped } marches omises
    }.

## Avertissements : mouvements d’une phase qui se rencontrent (src/junction/model/signal.rs `conflicts`)
jn-conflict-move = les voitures du côté { $from } qui vont { $turn }
jn-conflict-vehicles = Phase { $n } : { $a } croisent { $b }.
jn-conflict-walk = Phase { $n } : { $mv } rencontrent les piétons qui traversent du côté { $side }.
jn-conflict-ride = Phase { $n } : { $mv } croisent les vélos du côté { $side }.
jn-conflict-list = Avertissements de la phase { $n }
jn-conflict-heard = Attention : { $conflict }
jn-conflict-heard-many = Attention : { $count } mouvements se rencontrent maintenant, { $phases ->
    [one] à la phase
   *[other] aux phases
} { $list }. { $first }
jn-conflict-more = { $n ->
    [one] Et { $n } autre rencontre.
   *[other] Et { $n } autres rencontres.
}
jn-check-signal-conflicts = Mouvements qui se rencontrent
jn-check-signal-conflicts-ok = Aucune phase ne réunit des mouvements qui se rencontrent
jn-check-signal-conflicts-bad = { $list }
jn-check-curb-near = Bordure dégagée des traverses
jn-check-curb-near-ok = Aucun stationnement ni aucune zone de chargement n’est à moins de { $need } d’une traverse.
jn-check-curb-near-bad = { $uses }
jn-curb-near = Bordure : { $use } sur { $arm } à { $gap } de la traverse; il en faut { $need } ou plus.
jn-curb-near-inside = Bordure : { $use } sur { $arm } atteint la traverse.
jn-sel-arm-curb = { $use } { $side }, à { $at } du centre.

# Ce qui distingue deux conceptions d’une jonction (src/junction/model/compare.rs).
cmp-jn-control = Signalisation : { $a } → { $b }
cmp-jn-turbo = Giratoire turbo : { $a } → { $b }
cmp-jn-turbo-none = aucun
cmp-jn-entering = { $street } : voies entrantes, { $a } → { $b }
cmp-jn-leaving = { $street } : voies sortantes, { $a } → { $b }
cmp-jn-crossing-added = { $street } : un passage pour piétons est ajouté
cmp-jn-crossing-removed = { $street } : le passage pour piétons est retiré
cmp-jn-bike-box-added = { $street } : un sas cyclable est ajouté
cmp-jn-bike-box-removed = { $street } : le sas cyclable est retiré
cmp-jn-phases = Phases des feux : { $a } → { $b }
cmp-jn-cycle = Cycle des feux : { $a } s → { $b } s
cmp-other-arm-details = les détails des rues à la jonction

## Actionnement par les véhicules (src/junction/model/actuation.rs, CLM-12) : la phase d’une rue à la demande est appelée et prolongée par les véhicules détectés à son approche
jn-refusal-actuation-no-phase = Aucune phase du plan n’est propre à cette rue, elle ne peut donc pas être à la demande : donnez-lui d’abord une phase.
jn-refusal-actuation-already = La rue est déjà réglée ainsi.
jn-refusal-not-actuated = Seule une rue à la demande a un vert minimal, un intervalle de passage, un vert maximal et un nombre de véhicules par heure : mettez-la d’abord à la demande.
jn-refusal-actuation-out-of-range = Cette valeur dépasse la borne provisoire indiquée.
jn-refusal-min-above-max = Le vert minimal ne peut pas être plus long que le vert maximal.
jn-rev-actuated-on = { $arm } : à la demande
jn-rev-actuated-off = { $arm } remise en rappel
jn-rev-actuation = { $arm } : { $field } { $value } s
jn-rev-actuation-vehicles =
    { $arm } : { $value ->
        [one] { $value } véhicule par heure
       *[other] { $value } véhicules par heure
    }
jn-act-min-green = Vert minimal
jn-act-gap = Intervalle de passage
jn-act-max-green = Vert maximal
jn-act-vehicles = Véhicules par heure
jn-act-heading = Actionnement par les véhicules
jn-act-intro = La phase d’une rue à la demande est appelée par les véhicules détectés à son approche : omise quand aucun n’attend, prolongée tant qu’ils continuent d’arriver et terminée tôt au premier intervalle libre. Une rue en rappel a sa phase à chaque cycle, à son vert.
jn-act-bounds = Valeurs provisoires : une rue mise à la demande a un vert minimal de { $min } s, un intervalle de passage de { $gap } s, un vert maximal de { $max } s et { $vehicles } véhicules par heure, répartis sur une heure. Une marche servie retient le vert aussi longtemps qu’il le faut; le jaune et le rouge intégral ne sont jamais raccourcis.
jn-act-group = Phase de { $street }
jn-act-mode = Phase
jn-act-mode-recall = En rappel
jn-act-mode-actuated = À la demande
jn-act-recall = Phase en rappel : elle passe à chaque cycle, à son vert, quelle que soit la circulation.
jn-act-trigger = À la demande : les véhicules détectés à son approche appellent sa phase, qui est omise quand aucun n’attend. Son vert dure au moins { $min } s, chaque véhicule le retient { $gap } s de plus, jusqu’à { $max } s.
jn-act-floor = Une marche en rappel dans sa phase demande { $floor } s de vert : son vert dure donc au moins autant.
jn-act-no-phase = Aucune phase du plan n’est propre à cette rue : elle ne peut pas être à la demande.
jn-act-effect =
    Dans l’heure, son vert a été donné { $greens } fois, omis { $skipped } fois, prolongé { $extended } fois de { $extended_s } s en tout et terminé tôt { $early } fois.
jn-act-range-seconds = De { $min } à { $max } s (borne provisoire)
jn-act-range-vehicles = De { $min } à { $max } véhicules par heure (borne provisoire)
jn-act-unit-seconds = s
jn-act-unit-vehicles = véh./h
jn-act-less = { $field }, { $step } de moins
jn-act-more = { $field }, { $step } de plus
jn-act-none = Aucune rue n’est à la demande : chaque phase passe à chaque cycle, à son vert.
jn-act-phases-caption = Phases à la demande en une heure
jn-act-approaches-caption = Véhicules en une heure, par approche
jn-act-col-phase = Phase
jn-act-col-skipped = Omises
jn-act-col-extended = Prolongées
jn-act-col-extended-by = Prolongées de
jn-act-col-ended-early = Terminées tôt
jn-act-col-approach = Approche
jn-act-col-vehicles = Véhicules
jn-act-col-average = Attente moyenne
jn-act-no-wait-said = aucun véhicule servi
jn-act-finding-no-phase = { $arm } est à la demande, mais aucune phase du plan ne lui est propre : ses véhicules n’appellent rien. Donnez-lui une phase ou remettez-la en rappel.
jn-act-finding-walk-past-max = La marche sur { $crossing } demande { $need } s de vert, plus que le vert maximal de { $max } s de { $arm } : la marche retient le vert au-delà du maximum chaque fois qu’elle est servie.
jn-act-said =
    { $arm } : { $vehicles ->
        [one] { $vehicles } véhicule
       *[other] { $vehicles } véhicules
    } dans l’heure; servis { $served }, non servis { $not_served }; attente moyenne { $average } s, la plus longue { $longest } s. Son vert : donné { $greens }, omis { $skipped }, prolongé { $extended } de { $extended_s } s en tout, terminé tôt { $early }.
jn-act-said-no-wait =
    { $arm } : { $vehicles ->
        [one] { $vehicles } véhicule
       *[other] { $vehicles } véhicules
    } dans l’heure; servis { $served }, non servis { $not_served }; aucun véhicule n’a été servi. Son vert : donné { $greens }, omis { $skipped }, prolongé { $extended } de { $extended_s } s en tout, terminé tôt { $early }.

## Détection des vélos (src/junction/model/bicycles.rs, CLM-13) : un mouvement vélo sur détection est servi quand un cycliste est détecté, avec un vert minimal et un dégagement pour les vélos
jn-refusal-bike-not-ridden = Aucune phase du plan ne fait passer les vélos de cette rue, ils ne peuvent donc pas être sur détection : faites-les d’abord passer dans une phase.
jn-refusal-bike-detection-already = Le mouvement vélo est déjà réglé ainsi.
jn-refusal-not-on-bike-detection = Seul un mouvement vélo sur détection a un nombre de cyclistes par heure : mettez-le d’abord sur détection.
jn-refusal-cyclists-out-of-range = Ce nombre de cyclistes par heure dépasse la borne provisoire indiquée.
jn-rev-bike-detection-on = Vélos de { $arm } : sur détection
jn-rev-bike-detection-off = Vélos de { $arm } remis en rappel
jn-rev-cyclists =
    Vélos de { $arm } : { $rate ->
        [one] { $rate } cycliste par heure
       *[other] { $rate } cyclistes par heure
    }
jn-bikes-heading = Détection des vélos
jn-bikes-intro = Un mouvement vélo sur détection ne passe que lorsqu’un cycliste est détecté à l’approche de sa rue, et la phase qui sert l’appel donne au cycliste un vert minimal et un dégagement pour traverser. Un mouvement vélo en rappel passe avec chaque phase qui le laisse passer, et les cyclistes ne changent rien à son service.
jn-bikes-bounds = Valeurs provisoires : un mouvement vélo mis sur détection a { $rate } cyclistes par heure, répartis sur une heure. Chaque appel demande un vert minimal de { $min } s, puis un dégagement à { $speed } m/s sur le trajet à travers l’intersection, arrondi à la seconde supérieure; un vert est prolongé d’au plus { $max } s. Quand un appel piéton demande plus dans la même phase, le plus long est retenu : aucun des deux dégagements n’est raccourci, ni le jaune et le rouge intégral. Un cycliste appelle une phase à la demande, et un appel servi retient son vert au-delà de son maximum.
jn-bikes-group = Vélos de { $street }
jn-bikes-group-to = Vélos de { $street } vers { $to }
jn-bikes-mode = Vélos
jn-bikes-mode-recall = En rappel
jn-bikes-mode-detection = Sur détection
jn-bikes-recall = En rappel : passe avec chaque phase qui le laisse passer; les cyclistes ne changent rien à son service.
jn-bikes-trigger = Sur détection : un cycliste détecté à l’approche appelle le mouvement, et la prochaine phase qui le fait passer sert l’appel. Chaque appel demande un vert minimal de { $min } s + un dégagement de { $clearance } s sur { $length } = { $need } s de vert; le vert d’une phase est prolongé d’au plus { $max } s pour lui.
jn-bikes-not-ridden = Aucune phase ne fait passer les vélos de cette rue : ils ne peuvent pas être sur détection.
jn-bikes-rate = Cyclistes par heure
jn-bikes-rate-range = De { $min } à { $max } cyclistes par heure (borne provisoire)
jn-bikes-effect =
    Dans l’heure, { $calls ->
        [one] { $calls } appel
       *[other] { $calls } appels
    } : servis { $served }, retardés { $delayed }, vert prolongé { $extended } fois de { $extended_s } s en tout, non servis { $not_served }.
jn-bikes-none = Aucun mouvement vélo n’est sur détection : chacun passe avec chaque phase qui le laisse passer.
jn-bikes-totals-caption = Appels des cyclistes en une heure, par mouvement vélo
jn-bikes-col-movement = Mouvement vélo
jn-bikes-col-extended = Prolongés
jn-bikes-extended-cell = { $count } ({ $seconds } s)
jn-bikes-no-wait-said = aucun cycliste servi
jn-bikes-rows =
    { $n ->
        [one] L’appel du cycliste, une ligne
       *[other] Chaque appel de cycliste, une ligne par appel ({ $n })
    }
jn-bikes-rows-caption = Chaque appel de cycliste de l’heure, dans l’ordre où il a été fait
jn-bikes-finding-cannot = Les vélos de { $arm } ne peuvent pas être servis en sécurité : leur vert minimal et leur dégagement demandent { $need } s de vert, et aucune phase qui les fait passer ne peut donner plus de { $most } s.
jn-bikes-finding-not-ridden = Les vélos de { $arm } sont sur détection, mais aucune phase ne les fait passer : leurs appels ne sont pas servis. Faites-les passer dans une phase ou remettez-les en rappel.
jn-bikes-finding-past-max = Les vélos de { $arm } demandent { $need } s de vert, plus que le vert maximal de { $max } s de { $phase } : un appel servi là retient le vert au-delà du maximum.
jn-bikes-said =
    Vélos de { $arm } : { $calls ->
        [one] { $calls } appel
       *[other] { $calls } appels
    } dans l’heure; servis { $served }, retardés { $delayed }, vert prolongé { $extended } de { $extended_s } s en tout, non servis { $not_served }; attente la plus longue { $wait } s.
jn-bikes-said-no-wait =
    Vélos de { $arm } : { $calls ->
        [one] { $calls } appel
       *[other] { $calls } appels
    } dans l’heure; servis { $served }, retardés { $delayed }, vert prolongé { $extended } de { $extended_s } s en tout, non servis { $not_served }; aucun appel n’a été servi.

## Transit signal priority (src/junction/model/transit.rs, CLM-14): a street's buses are given an early green, a green extension or both, within bounds
jn-refusal-priority-no-transit = Cette rue n’a pas de transport en commun, ses autobus ne peuvent donc pas recevoir la priorité : donnez-lui d’abord une voie réservée, un dépassement de file, une porte d’autobus, un arrêt ou un filtre pour le transport en commun.
jn-refusal-priority-no-phase = Aucune phase du plan n’est propre à cette rue, ses autobus ne peuvent donc pas recevoir la priorité : donnez-lui d’abord une phase.
jn-refusal-not-on-priority = Seule une rue dont les autobus ont la priorité a des autobus par heure : donnez-leur d’abord la priorité.
jn-refusal-buses-out-of-range = Ce nombre d’autobus par heure dépasse la borne provisoire indiquée.
jn-rev-priority-early-green = Autobus de { $arm } : vert hâtif
jn-rev-priority-extension = Autobus de { $arm } : prolongation du vert
jn-rev-priority-both = Autobus de { $arm } : vert hâtif et prolongation du vert
jn-rev-priority-off = Autobus de { $arm } : sans priorité
jn-rev-buses =
    Autobus de { $arm } : { $rate ->
        [one] { $rate } autobus par heure
       *[other] { $rate } autobus par heure
    }
jn-transit-heading = Priorité aux feux pour le transport en commun
jn-transit-intro = Une rue qui a du transport en commun et une phase propre peut donner la priorité à ses autobus. Le vert hâtif écourte les phases qui précèdent la sienne quand un autobus arrive au rouge; la prolongation du vert retient son vert pour un autobus qui arrive juste après sa fin. Sans priorité, un autobus attend le vert de sa rue comme le reste de la circulation.
jn-transit-bounds = Valeurs provisoires : une rue qui reçoit la priorité a { $rate } autobus par heure, répartis sur une heure. Le vert hâtif avance le vert de la rue d’au plus { $early } s, et la prolongation le retient d’au plus { $extension } s; une rue qui reçoit la priorité pour un vert n’en reçoit pas pour le suivant. Le jaune et le rouge intégral ne sont jamais raccourcis, ni le vert minimal d’une phase, une marche et son dégagement ou un dégagement vélo : un vert hâtif qu’ils écourtent est un constat. L’avance pour le transport en commun de la rue (sa minuterie) reste fixe à chaque cycle, quoi qu’il arrive.
jn-transit-group = Autobus de { $street }
jn-transit-mode = Priorité
jn-transit-mode-off = Aucune
jn-transit-mode-early-green = Vert hâtif
jn-transit-mode-extension = Prolongation du vert
jn-transit-mode-both = Les deux
jn-transit-off = Sans priorité : un autobus attend le vert de la rue comme le reste de la circulation.
jn-transit-trigger-early-green = Vert hâtif : un autobus qui arrive au rouge écourte le vert des phases qui précèdent la sienne, d’au plus { $early } s en tout, jamais sous leur vert minimal, une marche et son dégagement ou un dégagement vélo. Pas pour deux verts de suite.
jn-transit-trigger-extension = Prolongation du vert : un autobus qui arrive dans les { $extension } s après la fin prévue du vert de la rue retient le vert jusqu’à son passage. Pas pour deux verts de suite.
jn-transit-trigger-both = Vert hâtif et prolongation du vert : un autobus qui arrive au rouge avance le vert de la rue d’au plus { $early } s, sans jamais écourter une phase sous son vert minimal, une marche et son dégagement ou un dégagement vélo; un autobus qui arrive dans les { $extension } s après la fin prévue du vert le retient jusqu’à son passage. Pas pour deux verts de suite.
jn-transit-no-transit = Cette rue n’a pas de transport en commun : ses autobus ne peuvent pas recevoir la priorité.
jn-transit-no-phase = Aucune phase du plan n’est propre à cette rue : ses autobus ne peuvent pas recevoir la priorité.
jn-transit-rate = Autobus par heure
jn-transit-rate-range = De { $min } à { $max } autobus par heure (borne provisoire)
jn-transit-effect =
    Dans l’heure, { $buses ->
        [one] { $buses } autobus
       *[other] { $buses } autobus
    } : à l’arrivée { $on_arrival }, vert hâtif { $early }, prolongés { $extended }, retardés { $delayed }, non servis { $not_served }; retard reporté sur les autres approches { $moved } s.
jn-transit-moved = Retard reporté sur les autres approches : { $list }.
jn-transit-moved-item = { $arm } { $seconds } s
jn-transit-totals-caption = Autobus en une heure, par rue
jn-transit-col-street = Rue
jn-transit-col-buses = Autobus
jn-transit-col-on-arrival = À l’arrivée
jn-transit-col-early-green = Vert hâtif
jn-transit-col-extended = Prolongés
jn-transit-col-delayed = Retardés
jn-transit-col-moved = Retard reporté sur les autres
jn-transit-rows =
    { $n ->
        [one] L’autobus, une ligne
       *[other] Chaque autobus, une ligne par autobus ({ $n })
    }
jn-transit-rows-caption = Chaque autobus de l’heure, dans l’ordre d’arrivée
jn-transit-col-bus = Autobus
jn-transit-col-arrived = Arrivée
jn-transit-col-wait = Attente
jn-transit-outcome-on-arrival = À l’arrivée
jn-transit-outcome-early-green = Vert hâtif ({ $early } s plus tôt)
jn-transit-outcome-extended = Prolongé (retenu { $extended } s)
jn-transit-outcome-delayed = Retardé
jn-transit-no-wait-said = aucun autobus servi
jn-transit-finding-no-transit = Les autobus de { $arm } ont la priorité, mais la rue n’a plus de transport en commun : ils n’en reçoivent aucune. Redonnez-lui du transport en commun ou retirez la priorité.
jn-transit-finding-no-phase = Les autobus de { $arm } ont la priorité, mais aucune phase du plan n’est propre à la rue : ils n’en reçoivent aucune. Donnez-lui une phase ou retirez la priorité.
jn-transit-finding-clearance =
    Un vert hâtif pour les autobus de { $arm } a été écourté moins que demandé { $times ->
        [one] une fois
       *[other] { $times } fois
    }, de { $held } s en tout, dans la phase de { $phase } : elle servait une marche et son dégagement ou un dégagement vélo, qui ne sont jamais raccourcis.
jn-transit-said =
    Autobus de { $arm } : { $buses ->
        [one] { $buses } autobus
       *[other] { $buses } autobus
    } dans l’heure; à l’arrivée { $on_arrival }, vert hâtif { $early }, prolongés { $extended }, retardés { $delayed }, non servis { $not_served }; attente la plus longue { $wait } s; retard reporté sur les autres approches { $moved } s.
jn-transit-said-no-wait =
    Autobus de { $arm } : { $buses ->
        [one] { $buses } autobus
       *[other] { $buses } autobus
    } dans l’heure; à l’arrivée { $on_arrival }, vert hâtif { $early }, prolongés { $extended }, retardés { $delayed }, non servis { $not_served }; aucun autobus n’a été servi.
jn-transit-none = Aucune rue ne donne la priorité à ses autobus : chaque autobus attend le vert de sa rue comme le reste de la circulation.

## Emergency vehicle pre-emption (src/junction/model/preemption.rs, CLM-15): an emergency call takes the signal to its street's phase after a safe clearance
jn-refusal-preemption-no-phase = Aucune phase du plan n’est propre à cette rue, ses appels d’urgence ne peuvent donc pas préempter les feux : donnez-lui d’abord une phase.
jn-refusal-not-on-preemption = Seule une rue dont la préemption est activée a des appels d’urgence par heure : activez d’abord la préemption.
jn-refusal-emergency-calls-out-of-range = Ce nombre d’appels d’urgence par heure dépasse la borne provisoire indiquée.
jn-rev-preemption-on = Véhicules d’urgence de { $arm } : préemption activée
jn-rev-preemption-off = Véhicules d’urgence de { $arm } : préemption désactivée
jn-rev-emergency-calls =
    Véhicules d’urgence de { $arm } : { $rate ->
        [one] { $rate } appel d’urgence par heure
       *[other] { $rate } appels d’urgence par heure
    }
jn-preempt-heading = Préemption pour les véhicules d’urgence
jn-preempt-intro = Une rue qui a une phase propre peut laisser ses véhicules d’urgence préempter les feux. La préemption a la plus haute priorité de toutes, au-dessus des appels piétons et vélos, de l’actionnement par les véhicules et de la priorité au transport en commun : un appel d’urgence met fin au vert en cours après un dégagement sécuritaire et donne aussitôt le vert à la rue. Sans préemption, un véhicule d’urgence attend le vert de sa rue comme le reste de la circulation.
jn-preempt-bounds = Valeurs provisoires : une rue qui reçoit la préemption a { $rate } appels d’urgence par heure, répartis sur une heure, et son vert dure alors au moins { $dwell } s pour le véhicule, qu’on suppose passé à ce moment. Une marche encore affichée est écourtée, mais un dégagement piéton ou vélo en cours, le jaune et le rouge intégral durent toujours en entier. Après le vert d’urgence, le plan reprend avec la phase qui suit celle interrompue. Les appels sont servis dans l’ordre d’arrivée : un appel qui arrive pendant la préemption d’une autre rue attend qu’elle se termine.
jn-preempt-group = Véhicules d’urgence de { $street }
jn-preempt-mode = Préemption
jn-preempt-mode-off = Désactivée
jn-preempt-mode-on = Activée
jn-preempt-off = Désactivée : un véhicule d’urgence attend le vert de la rue comme le reste de la circulation.
jn-preempt-trigger = Activée, la plus haute priorité : un appel d’urgence sur cette rue met fin au vert en cours et donne le vert à la rue, au-dessus de toute autre politique. Une marche encore affichée est écourtée, mais un dégagement piéton ou vélo en cours, le jaune et le rouge intégral durent d’abord en entier, si bien que le vert arrive au plus { $entry } s après l’appel (l’entrée sécuritaire). Il dure ensuite au moins { $dwell } s, et le plan reprend avec la phase qui suit celle interrompue.
jn-preempt-no-phase = Aucune phase du plan n’est propre à cette rue : ses appels d’urgence ne peuvent pas préempter les feux.
jn-preempt-rate = Appels d’urgence par heure
jn-preempt-rate-range = De { $min } à { $max } appels d’urgence par heure (borne provisoire)
jn-preempt-effect =
    Dans l’heure, { $calls ->
        [one] { $calls } appel d’urgence
       *[other] { $calls } appels d’urgence
    } : délai avant le vert le plus long { $longest } s; préemptés { $preempted }; retardés { $delayed }.
jn-preempt-effect-no-call = Dans l’heure, aucun appel d’urgence n’a été servi.
jn-preempt-totals-caption = Appels d’urgence en une heure, par rue
jn-preempt-col-street = Rue
jn-preempt-col-calls = Appels
jn-preempt-col-longest = Délai avant le vert le plus long
jn-preempt-col-preempted = Préemptés
jn-preempt-col-delayed = Retardés
jn-preempt-rows =
    { $n ->
        [one] L’appel d’urgence, une ligne
       *[other] Chaque appel d’urgence, une ligne par appel ({ $n })
    }
jn-preempt-rows-caption = Chaque appel d’urgence de l’heure, dans l’ordre d’arrivée
jn-preempt-col-call = Appel
jn-preempt-col-arrived = Arrivée
jn-preempt-col-to-green = Délai avant le vert
jn-preempt-cut = Vert de { $street } écourté : { $list }
jn-preempt-cut-none = Aucun vert écourté
jn-preempt-not-served = Non servi
jn-preempt-pedestrians =
    { $n ->
        [one] { $n } piéton
       *[other] { $n } piétons
    }
jn-preempt-cyclists =
    { $n ->
        [one] { $n } cycliste
       *[other] { $n } cyclistes
    }
jn-preempt-buses =
    { $n ->
        [one] { $n } autobus
       *[other] { $n } autobus
    }
jn-preempt-vehicles =
    { $n ->
        [one] { $n } véhicule
       *[other] { $n } véhicules
    }
jn-preempt-no-call-said = aucun appel d’urgence servi
jn-preempt-nobody = personne
jn-preempt-finding-no-phase = { $arm } a la préemption activée, mais aucune phase du plan ne lui est propre : ses appels d’urgence ne mènent les feux nulle part. Donnez-lui une phase ou désactivez la préemption.
jn-preempt-said =
    Véhicules d’urgence de { $arm } : { $calls ->
        [one] { $calls } appel d’urgence
       *[other] { $calls } appels d’urgence
    } dans l’heure; délai avant le vert le plus long { $longest } s, entrée sécuritaire { $entry } s; préemptés { $preempted }; retardés { $delayed }.
jn-preempt-said-no-call =
    Véhicules d’urgence de { $arm } : { $calls ->
        [one] { $calls } appel d’urgence
       *[other] { $calls } appels d’urgence
    } dans l’heure; aucun appel n’a été servi.
jn-preempt-none = Aucune rue n’a la préemption activée : un véhicule d’urgence attend le vert de sa rue comme le reste de la circulation.
## Plans selon l’heure (CLM-16)
jn-plan-default = Plan par défaut
jn-day-weekday = En semaine (du lundi au vendredi)
jn-day-saturday = Samedi
jn-day-sunday = Dimanche
jn-day-on-weekday = en semaine
jn-day-on-saturday = le samedi
jn-day-on-sunday = le dimanche
jn-rev-in-plan = { $change }, dans le plan « { $plan } »
jn-rev-plan-add = Ajout du plan « { $name } »
jn-rev-plan-rename = Plan « { $from } » renommé « { $to } »
jn-rev-plan-remove =
    { $n ->
        [0] Retrait du plan « { $name } »
        [one] Retrait du plan « { $name } » : sa plage horaire suit le plan par défaut
       *[other] Retrait du plan « { $name } » : ses { $n } plages horaires suivent le plan par défaut
    }
jn-rev-period-add = « { $plan } » à partir de { $time } { $day }
jn-rev-period-remove = Retrait de la plage horaire de { $time } { $day }
jn-rev-period-plan = La plage horaire de { $time } { $day } suit le plan « { $plan } »
jn-rev-period-start = La plage horaire de { $time } { $day } commence maintenant à { $to }
jn-refusal-plan-name-empty = Un plan doit avoir un nom.
jn-refusal-plan-name-too-long = Le nom d’un plan compte au plus 40 caractères.
jn-refusal-plan-name-taken = Un autre plan porte déjà ce nom.
jn-refusal-too-many-plans = Une jonction compte au plus 8 plans, le plan par défaut compris.
jn-refusal-default-plan-kept = Le plan par défaut ne peut pas être retiré : il s’applique chaque fois qu’aucun autre plan n’est prévu.
jn-refusal-default-plan-named = Le plan par défaut garde son nom.
jn-refusal-period-out-of-day = Une plage horaire commence après 00:00 et avant minuit. Pour changer ce qui s’applique dès 00:00, choisissez le plan de la première plage horaire de la journée.
jn-refusal-period-taken = Une autre plage horaire commence déjà à cette heure ce jour-là.
jn-refusal-too-many-periods = Une journée compte au plus 12 plages horaires, celle de 00:00 comprise.
jn-refusal-first-period-kept = La première plage horaire de la journée commence à 00:00 et reste : choisissez plutôt son plan.
jn-plans-heading = Plans selon l’heure
jn-plans-intro = Des feux peuvent appliquer d’autres plans à d’autres moments de la semaine. Chaque plan a son propre réglage et ses propres réglages de réponse (appels piétons, actionnement par les véhicules, détection des vélos, priorité au transport en commun, préemption); les phases, leur ordre et leurs modèles sont les mêmes dans tous les plans. Ce que l’horaire ne couvre pas suit le plan par défaut.
jn-plans-shown = Plan affiché et modifié
jn-plans-shown-help = Le plan des feux ci-dessous, son bilan de l’heure et ses vérifications, ainsi que le réglage de chaque rue, sont ceux de ce plan : un réglage modifié l’est dans ce plan seulement.
jn-plans-shown-said = Plan « { $plan } » affiché : son réglage et ses réglages de réponse sont affichés et modifiés.
jn-plans-viewing = Réglage et réglages de réponse du plan « { $plan } ».
jn-plans-bounds = Jusqu’à { $plans } plans avec le plan par défaut, des noms d’au plus { $chars } caractères, jusqu’à { $periods } plages horaires par jour (bornes provisoires).
jn-plans-list = Plans
jn-plans-default-mark = par défaut, ne peut pas être retiré
jn-plans-cycle = Cycle de { $seconds } s
jn-plans-sound = Aucun constat
jn-plans-findings =
    { $n ->
        [one] 1 constat
       *[other] { $n } constats
    }
jn-plans-findings-of = Constats du plan « { $plan } »
jn-plans-name-of = Nom du plan « { $plan } »
jn-plans-remove = Retirer
jn-plans-remove-of = Retirer le plan « { $plan } »
jn-plans-new-name = Nom d’un nouveau plan
jn-plans-add = Ajouter un plan
jn-plans-schedule = Horaire de la semaine
jn-plans-col-start = Début
jn-plans-col-end = Fin
jn-plans-col-plan = Plan
jn-plans-col-change = Modifier
jn-plans-midnight = 24:00
jn-period-start-of = Début de la plage horaire de { $time } { $day }
jn-period-plan-of = Plan de la plage horaire de { $time } { $day }
jn-period-remove-of = Retirer la plage horaire de { $time } { $day }
jn-period-first = Début de la journée
jn-period-new-start = Début d’une nouvelle plage horaire { $day }
jn-period-new-plan = Plan d’une nouvelle plage horaire { $day }
jn-period-new = Nouvelle plage horaire { $day }
jn-period-add = Ajouter la plage horaire
jn-period-add-of = Ajouter la plage horaire { $day }
jn-plans-runs-heading = Quel plan s’applique
jn-plans-runs-day = Jour
jn-plans-runs-time = Heure
jn-plans-runs-at = À { $time } { $day }, le plan « { $plan } » s’applique.
jn-plans-transition = Un changement de plan prend effet à la fin du cycle en cours au début de sa plage horaire : il n’y a pas de période de transition.
