# Ce que dit une restriction de zone (src/restriction, et le panneau Règles de zone de la carte : src/map/areas.rs, src/map/areas_view.rs).

## Refus d’un contour
restriction-refused-too-few = Une zone a besoin d’au moins trois coins, pas tous alignés.
restriction-refused-crossing = Le contour se croise. Déplacez ou retirez un coin.

restriction-refused-duplicate = Il y a déjà un coin ici. Déplacez la carte, puis ajoutez le suivant.

## Les règles
restriction-rule-car-free = Zone sans voitures
restriction-rule-bus-and-bike = Autobus et vélos seulement
restriction-rule-speed-limit = Limite de vitesse de { $kmh }{" "}km/h
restriction-rule-speed-limit-choice = Limite de vitesse
restriction-rule-no-parking = Stationnement et chargement interdits
restriction-named = { $name }

## Ce qu’une rue a et qu’une règle n’admet pas
restriction-conflict-travel = { $n ->
    [one] Elle a une voie de circulation.
   *[other] Elle a { $n } voies de circulation.
}
restriction-conflict-parking = { $n ->
    [one] Elle a une voie de stationnement.
   *[other] Elle a { $n } voies de stationnement.
}
restriction-conflict-loading = { $n ->
    [one] Elle a une zone de chargement.
   *[other] Elle a { $n } zones de chargement.
}

## La vérification d’une rue dans une zone
restriction-check = Règle de zone : { $area }
restriction-check-keeps = { $rule } : la rue la respecte.
restriction-check-breaks = { $rule } : la rue ne la respecte pas.
restriction-check-info = { $rule } : les rues n’ont pas de vitesse, c’est donc à titre indicatif.
restriction-check-informational = { " " }: à titre indicatif

## Le tracé d’un contour, dit à mesure
restriction-started = Tracé d’une zone. Cliquez sur la carte, ou appuyez sur Entrée sur celle-ci, pour placer le premier coin.
restriction-redraw-started = Nouveau tracé de « { $area } ». Placez ses coins de nouveau.
restriction-corner-added = Coin { $n } placé.
restriction-corner-removed = { $n ->
    [one] Coin retiré. Il reste { $n } coin.
   *[other] Coin retiré. Il reste { $n } coins.
}
restriction-corner-moved = Coin déplacé.
restriction-corner-stale = Le contour a été modifié dans un autre onglet; le coin n’a donc pas été déplacé.
restriction-closed = Le contour est fermé. Choisissez sa règle et son nom, puis enregistrez-le.
restriction-cancelled = Tracé annulé.
restriction-corners = { $n ->
    [one] { $n } coin
   *[other] { $n } coins
}
restriction-kmh = { $kmh }{" "}km/h

## Garder et modifier une zone
restriction-saved = Zone enregistrée : « { $area } ».
restriction-not-kept = Les règles de zone n’ont pas pu être enregistrées dans ce navigateur.
restriction-selected = Zone sélectionnée : « { $area } » ({ $rule }).
restriction-selected-unnamed = Zone sélectionnée : « { $area } ».
restriction-renamed = Zone renommée « { $area } ».
restriction-rule-changed = La règle de « { $area } » a changé ({ $rule }).
restriction-rule-changed-unnamed = La règle de la zone a changé ({ $rule }).
restriction-redrawn = Zone retracée : « { $area } ».
restriction-deleted = Zone supprimée : « { $area } ».
restriction-restored = Zone rétablie : « { $area } ».

## Le chapeau de l’onglet
restriction-future = Ces règles de zone ont été enregistrées par une version plus récente de CityLoom; elles ne peuvent pas être affichées ni modifiées ici.
restriction-lead-none = Aucune règle de zone pour l’instant. Déclarez-en une pour vérifier les rues qu’elle contient.
restriction-lead-some = { $n ->
    [one] { $n } règle de zone.
   *[other] { $n } règles de zone.
} Sélectionnez-en une pour voir les rues qu’elle contient.
restriction-lead-no-map = Une zone se trace sur la carte, et il n’y a pas de carte ici.

## Une zone dans l’onglet, et les rues qu’elle contient
restriction-summary-none = Aucune rue chargée à l’intérieur.
restriction-summary = { $inside ->
    [one] { $inside } rue à l’intérieur
   *[other] { $inside } rues à l’intérieur
}; { $conflicts ->
    [0] aucune n’est en conflit avec la règle.
    [one] une est en conflit avec la règle.
   *[other] { $conflicts } sont en conflit avec la règle.
}
restriction-summary-info = { $inside ->
    [one] { $inside } rue à l’intérieur.
   *[other] { $inside } rues à l’intérieur.
}
restriction-row-sub = { $rule } · { $summary }
restriction-street-keeps = Respecte la règle.
restriction-street-inside = À l’intérieur de la zone.
restriction-legend-conflict = En conflit avec une règle de zone

## Le panneau
restriction-panel = Règles de zone
restriction-toggle-title = Afficher ou masquer les règles de zone ( [ )
restriction-heading = Règles sur une zone
restriction-declare = Déclarer une zone
restriction-drawing-help = Cliquez sur la carte pour placer chaque coin, puis cliquez sur le premier coin ou appuyez sur Terminer. Au clavier, déplacez la carte avec les flèches, appuyez sur Entrée sur celle-ci pour placer un coin en son centre, et sur Maj+Entrée pour terminer. Retour arrière retire le dernier coin; Échap annule lorsque la carte a le focus.
restriction-naming-help = Choisissez la règle qui s’applique dans la zone, et nommez-la si vous le souhaitez.
restriction-finish = Terminer
restriction-cancel = Annuler
restriction-save = Enregistrer
restriction-name-label = Nom
restriction-rule-label = Règle
restriction-speed-label = Limite de vitesse
restriction-redraw = Retracer
restriction-delete = Supprimer
restriction-undo = Annuler la suppression
restriction-streets-heading = Rues à l’intérieur
