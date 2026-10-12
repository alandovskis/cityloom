# Ce que dit une restriction de zone (src/restriction, et l’onglet Règles de zone de la carte : src/map/areas.rs, src/map/areas_view.rs).

## Refus d’un contour
restriction-refused-too-few = Une zone a besoin d’au moins trois coins, pas tous alignés.
restriction-refused-crossing = Le contour se croise. Déplacez ou retirez un coin.

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
