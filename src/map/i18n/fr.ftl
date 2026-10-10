# Ce que dit la page de la carte de la ville (src/map/vm.rs, src/map/view.rs) et son balisage (web/map.html).

## Le balisage de la page
map-page-title = Carte de la ville CityLoom
map-page-description = Voyez toute la ville d’un coup d’œil et ouvrez n’importe quelle rue ou jonction pour la modifier.
map-brand = CityLoom, la carte de la ville
map-editors = Éditeurs
map-nav-map = Carte
map-places-toggle-title = Afficher ou masquer les lieux ( [ )
map-places-panel = Lieux de la ville
map-basemap-label = Carte de la ville, nord en haut
map-does-it-work = Est-ce que ça fonctionne?
map-your-changes = Vos modifications
map-legend = Légende de la carte
map-fine-print = Les rues proviennent d’OpenStreetMap. Tous les seuils qu’utilisent les vérifications sont des exemples pour l’instant, pas de vraies règles.
map-about-city = À propos de cette ville
map-key-search = rechercher
map-key-next = lieu suivant
map-key-open = ouvrir
map-key-zoom = zoom
map-key-move = déplacer la carte
map-key-whole = toute la ville
map-key-panels = lieux, notes

## L’en-tête, les outils et les panneaux
map-city-map = Carte de la ville
map-sheet-tools = Outils de la feuille
map-view-tools = Vue de la carte
map-zoom-out = Zoom arrière
map-zoom-in = Zoom avant
map-whole-city = Toute la ville
map-places = Lieux
map-places-hint = Appuyez sur une jonction pour ouvrir son plan. Appuyez sur une rue pour ouvrir sa coupe transversale.
map-junctions = Jonctions
map-streets-heading = Rues
map-tb-city = Ville
map-tb-changed = Modifiés

## Compter
map-streets = { $n ->
    [one] { $n } rue
   *[other] { $n } rues
}
map-junction-count = { $n ->
    [one] { $n } jonction
   *[other] { $n } jonctions
}
# `junctions` est map-junction-count et `streets` est map-streets.
map-counts = { $junctions }, { $streets }

## Un lieu dans une liste : ses précisions, et l’étiquette qui dit où il en est
# `control` est la signalisation de la jonction (un message jn-control-*), `streets` est map-streets.
map-junction-sub = { $control }, { $streets }
map-street-sub = { $ends } · { $width }
map-state-works = Fonctionne
map-state-changed = Modifié
map-state-bad = À corriger

## Comment un lieu est dit à un lecteur d’écran, phrase par phrase
map-label-junction = { $name }, { $control }, { $streets }.
map-label-street = { $kind }, { $ends }, { $width } de large.
map-needs-attention = À corriger : { $failing }.
map-changed = Modifié.
map-opens-junction = Ouvre le plan de la jonction.
map-opens-street = Ouvre la coupe transversale de la rue.

## La recherche
map-search-label = Rechercher des lieux
map-search-go = Rechercher
map-search-results = Lieux trouvés
map-search-no-places = Il n’y a aucun lieu à rechercher.
map-search-none = Aucun lieu ne correspond à « { $query } ». Effacez la recherche pour revoir { $places ->
    [one] l’unique lieu
   *[other] les { $places } lieux
}.
map-search-found = { $n ->
    [one] { $n } lieu correspond
   *[other] { $n } lieux correspondent
}
map-search-found-more = { $n ->
    [one] { $n } lieu correspond
   *[other] { $n } lieux correspondent
} · { $shown } premiers affichés

## Les notes
map-checks-no-places = Il n’y a aucun lieu à vérifier.
map-checks-failing = { $n ->
    [one] { $n } lieu est à corriger. Ouvrez-le pour voir ce qui ne va pas et le corriger.
   *[other] { $n } lieux sont à corriger. Ouvrez-en un pour voir ce qui ne va pas et le corriger.
}
map-checks-pass = { $n ->
    [one] Toutes les vérifications réussissent dans l’unique lieu.
   *[other] Toutes les vérifications réussissent dans les { $n } lieux.
}
map-changes-no-places = Il n’y a aucun lieu à modifier.
map-changes-some = { $n ->
    [one] { $n } lieu modifié
   *[other] { $n } lieux modifiés
} par rapport à la ville telle que tracée au départ.
map-changes-none = Rien n’a encore été modifié. Ouvrez une rue ou une jonction pour la modifier.
map-still-works = Fonctionne toujours
map-detail-needs-attention = À corriger : { $failing }
# Une rue dans les notes : son nom et son tracé.
map-note-street = { $kind }, { $ends }

## La ligne d’état
map-nothing-to-show = Aucune rue à afficher.
map-status-ok = { $n ->
    [one] { $n } lieu. Toutes les vérifications réussissent.
   *[other] { $n } lieux. Toutes les vérifications réussissent.
}
map-status-bad = { $n ->
    [one] { $n } lieu à corriger
   *[other] { $n } lieux à corriger
} : { $names }.
map-status-bad-more = { $n ->
    [one] { $n } lieu à corriger
   *[other] { $n } lieux à corriger
} : { $names } et d’autres.

## Recommencer
map-reset = Recommencer
map-reset-armed = Appuyez de nouveau pour recommencer
map-reset-armed-said = Toutes les rues et jonctions reviendront telles que tracées au départ. Appuyez de nouveau pour confirmer.
map-reset-done = La ville est revenue telle que tracée au départ.

## Le balisage de la page d’accueil (web/index.html)
map-home-title = CityLoom : réaménagez les rues de votre ville
map-home-description = Choisissez n’importe quelle rue ou jonction de la ville, réorganisez-la et voyez aussitôt ce qui fonctionne toujours.
map-home-brand = CityLoom, accueil
map-home-heading = Réaménagez les rues de votre ville.
map-home-lead = Trouvez un lieu, choisissez une rue ou une jonction sur sa carte et réorganisez-la. Vous voyez tout de suite ce qui fonctionne toujours.
map-home-open-map = Ouvrir la carte de la ville
# La mention est en trois morceaux autour d’un lien : map-home-streets-by, puis le lien map-home-osm-contributors, puis un point dans le balisage.
map-home-streets-by = Rues ©
map-home-osm-contributors = les contributeurs d’OpenStreetMap
map-home-placeholders = Les largeurs et les règles qu’utilisent les vérifications sont provisoires.

## Le bandeau de la page d’accueil (src/map/home.rs, src/map/svg.rs)
# `city` est le nom de la ville (une donnée), `counts` est map-counts.
map-hero-facts = { $city } : { $counts }.
# Le nom d’une rue modifiée sur le dessin ; `name` est celui de la rue. « Modifié » s’accorde avec « lieu ».
map-svg-name-changed = { $name } · modifié
# L’étiquette sous une jonction modifiée sur le dessin.
map-svg-changed = modifié

## Là où il n’y a pas de carte
map-basemap-missing = Le fond de carte n’a pas pu être chargé. Construisez-le avec `just prepare`, puis rechargez la page.
map-basemap-outside = Il n’y a pas de fond de carte pour ce lieu. La carte couvre la région de Montréal.
map-no-roads = Les rues de ce lieu n’ont pas pu être chargées, il n’y a donc pas de carte à afficher.
