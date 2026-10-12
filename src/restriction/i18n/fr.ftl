# Ce que dit une restriction de zone (src/restriction, et l’onglet Règles de zone de la carte : src/map/areas.rs, src/map/areas_view.rs).

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
restriction-check-informational = À titre indicatif

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
restriction-saved = Zone « { $area } » enregistrée.
restriction-not-kept = Les règles de zone n’ont pas pu être enregistrées dans ce navigateur.
restriction-selected = Zone « { $area } » sélectionnée : { $rule }.
restriction-renamed = Zone renommée « { $area } ».
restriction-rule-changed = La règle de « { $area } » est maintenant : { $rule }.
restriction-redrawn = Zone « { $area } » retracée.
restriction-deleted = Zone « { $area } » supprimée.
restriction-restored = Zone « { $area } » rétablie.

## Le chapeau de l’onglet
restriction-future = Ces règles de zone ont été enregistrées par une version plus récente de CityLoom; elles ne peuvent pas être affichées ni modifiées ici.
restriction-lead-none = Aucune règle de zone pour l’instant. Déclarez-en une pour vérifier les rues qu’elle contient.
restriction-lead-some = { $n ->
    [one] { $n } règle de zone.
   *[other] { $n } règles de zone.
} Sélectionnez-en une pour voir les rues qu’elle contient.
