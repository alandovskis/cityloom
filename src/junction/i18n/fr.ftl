# Ce que dit la jonction : son modèle (src/junction/model.rs, read_model.rs), ses annonces
# (src/junction/text.rs) et le tableau des virages (src/junction/turns.rs).

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
jn-measure-g1 = Voie de saut de file décalée
jn-measure-g2 = Voie de saut de file en bordure
jn-measure-g3 = Voie de saut de file virtuelle
jn-measure-h1 = Barrière pour autobus commandée par feux
jn-measure-h2 = Barrière pour autobus avec cédez-le-passage
jn-measure-m1 = Saillie d’arrêt d’autobus
jn-measure-m2 = Quai sur rue protégé par feux
jn-measure-l1 = Virage à gauche indirect par un autre itinéraire
jn-measure-l2 = Virage à gauche indirect dans l’intersection
jn-measure-l3 = Entrée et sortie à droite seulement
jn-measure-l4 = Mise en cul-de-sac d’une rue latérale

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

## Ce qu’a été chaque modification, dans l’historique
jn-word-add = ajout
jn-word-remove = retrait
jn-rev-remove = Retrait : { $arm }
jn-rev-bearing = { $street }, orientation : { $degrees }°
jn-rev-offset = { $arm }, décalage : { $length }
jn-rev-corner = Coin après { $arm } : rayon de { $length }
jn-rev-crossing = { $arm }, passage pour piétons : { $change }
jn-rev-setback = { $arm }, passage pour piétons en retrait de { $length }
jn-rev-crossing-width = { $arm }, passage pour piétons de { $length } de large
jn-rev-island = { $arm }, îlot refuge : { $change }
jn-rev-bulb-left = { $arm }, saillie de trottoir à gauche : { $change }
jn-rev-bulb-right = { $arm }, saillie de trottoir à droite : { $change }
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
jn-rev-bus-across = Voie d’autobus par le centre : { $from } vers { $to }
jn-rev-bus-across-remove = Voie d’autobus par le centre : retrait
jn-rev-cycle = Piste cyclable autour du carrefour giratoire : { $length }
jn-rev-cycle-remove = Piste cyclable autour du carrefour giratoire : retrait
jn-rev-ring = Carrefour giratoire : { $length } plus grand

## Les vérifications
jn-check-lanes-cover = Les voies couvrent chaque virage
jn-check-lanes-cover-ok = Chaque virage permis a sa voie
jn-check-lanes-cover-bad = Aucune voie pour { $turns }
jn-no-lane-left = { $from } vers { $to } (virage à gauche)
jn-no-lane-through = { $from } vers { $to } (tout droit)
jn-no-lane-right = { $from } vers { $to } (virage à droite)
jn-check-lanes-follow = Les voies respectent les virages interdits
jn-check-lanes-follow-ok = Aucune voie ne mène à un virage interdit
jn-check-lanes-follow-bad = Mènent à des virages interdits : { $lanes }
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
jn-check-transit = Les mesures pour le transport collectif fonctionnent
jn-check-transit-ok = Chaque mesure a ce qu’il lui faut
jn-problem-offset-parking = { $arm } : une voie de saut de file décalée a besoin d’un stationnement le long de la bordure
jn-problem-virtual-signal = { $arm } : une voie de saut de file virtuelle a besoin de feux de circulation
jn-problem-gate-bus-lane = { $arm } : une barrière pour autobus laisse passer une voie réservée, il lui faut donc une voie réservée aux autobus
jn-problem-gate-lane = { $arm } : une barrière pour autobus a besoin d’une voie de circulation à retenir
jn-problem-bulb-parking = { $arm } : une saillie d’arrêt d’autobus avance la bordure sur le stationnement, et il n’y en a pas
jn-problem-platform-signal = { $arm } : un quai protégé par feux a besoin de feux de circulation
jn-problem-platform-crossing = { $arm } : un quai dans la rue a besoin d’un passage pour piétons qui y mène
jn-problem-filter-bus-lane = { $arm } : un filtre modal a besoin d’une voie réservée pour les autobus qui passent
jn-check-signal = Les feux ont de la place
jn-check-signal-ok = Assez peu de rues pour des feux
jn-check-signal-bad = { $n ->
    [one] { $n } rue, c’est trop pour un seul jeu de feux
   *[other] { $n } rues, c’est trop pour un seul jeu de feux
}

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
jn-sel-corner = Coin après { $arm }, rayon de { $radius }
jn-sel-crossing = Passage pour piétons sur { $arm }

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
