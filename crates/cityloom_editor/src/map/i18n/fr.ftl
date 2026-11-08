# Ce que dit la page de la carte de la ville (src/map/vm.rs, src/map/view.rs) et son balisage (web/map.html).

## Le balisage de la page
map-page-title = Carte de la ville CityLoom
map-page-description = Voyez toute la ville d’un coup d’œil et ouvrez n’importe quelle rue ou jonction pour la modifier.
map-brand = CityLoom, la carte de la ville
map-editors = Éditeurs
map-nav-map = Carte
map-basemap-label = Carte de la ville, nord en haut
map-does-it-work = Est-ce que ça fonctionne?
map-your-changes = Vos modifications
map-legend = Légende de la carte
map-fine-print = Les rues proviennent d’OpenStreetMap. Tous les seuils qu’utilisent les vérifications sont des exemples pour l’instant, pas de vraies règles.
map-about-city = À propos de cette ville
map-key-search = rechercher
map-key-open = ouvrir un lieu trouvé
map-key-zoom = zoom
map-key-move = déplacer la carte
map-key-whole = toute la ville
map-key-areas = règles de zone
map-key-panels = notes

## L’en-tête, les outils et les panneaux
map-city-map = Carte de la ville
map-view-tools = Vue de la carte
map-zoom-out = Zoom arrière
map-zoom-in = Zoom avant
map-whole-city = Toute la ville
map-places = Lieux
map-junctions = Jonctions
map-streets-heading = Rues
map-tb-city = Ville
map-tb-changed = Modifiés

## Compter
map-streets = { $n ->
    [one] { $n } rue
   *[other] { $n } rues
}
map-curb = { $n ->
    [one] { $n } usage du bord de rue
   *[other] { $n } usages du bord de rue
}
map-stops = { $n ->
    [one] { $n } arrêt d’autobus
   *[other] { $n } arrêts d’autobus
}
map-programme = { $curb }, { $stops }
map-zones = { $n ->
    [one] { $n } zone
   *[other] { $n } zones
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
map-street-sub-zones = { $ends } · { $width } · en { $zones }
map-street-sub-programme = { $ends } · { $width } · { $programme }
map-street-sub-zones-programme = { $ends } · { $width } · en { $zones } · { $programme }
map-state-works = Fonctionne
map-state-changed = Modifié
map-state-bad = À corriger

## Comment un lieu est dit à un lecteur d’écran, phrase par phrase
map-label-junction = { $name }, { $control }, { $streets }.
map-label-street = { $street }, { $ends }, { $width } de large.
map-label-street-zones = { $street }, { $ends }, { $width } de large, en { $zones }.
map-label-street-programme = { $street }, { $ends }, { $width } de large, { $programme }.
map-label-street-zones-programme = { $street }, { $ends }, { $width } de large, en { $zones }, { $programme }.
map-needs-attention = À corriger : { $failing }.
map-changed = Modifié.
map-opens-junction = Ouvre le plan de la jonction.
map-opens-street = Ouvre la coupe transversale de la rue.
map-not-editable = Ce lieu n’est pas entièrement visible. Déplacez la carte pour qu’il le soit, puis réessayez.

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
map-checks-conflicts = { $streets ->
    [one] Une rue est
   *[other] { $streets } rues sont
} en conflit avec une règle de zone. Le panneau Règles de zone { $streets ->
    [one] la
   *[other] les
} liste.
map-changes-no-places = Il n’y a aucun lieu à modifier.
map-changes-some = { $n ->
    [one] { $n } lieu modifié
   *[other] { $n } lieux modifiés
} par rapport à la ville telle que tracée au départ.
map-changes-none = Rien n’a encore été modifié. Ouvrez une rue ou une jonction pour la modifier.
map-still-works = Fonctionne toujours
map-detail-needs-attention = À corriger : { $failing }
# Une rue dans les notes : son nom et son tracé.
map-note-street = { $street }, { $ends }

## La ligne d’état
map-nothing-to-show = Aucune rue à afficher.
map-status-ok = { $n ->
    [one] { $n } lieu. Toutes les vérifications réussissent.
   *[other] { $n } lieux. Toutes les vérifications réussissent.
}
map-status-conflicts = { $n ->
    [one] { $n } lieu.
   *[other] { $n } lieux.
} { $streets ->
    [one] Une rue est
   *[other] { $streets } rues sont
} en conflit avec une règle de zone.
map-status-bad = { $n ->
    [one] { $n } lieu à corriger
   *[other] { $n } lieux à corriger
} : { $names }.
map-status-bad-more = { $n ->
    [one] { $n } lieu à corriger
   *[other] { $n } lieux à corriger
} : { $names } et d’autres.

## Recommencer

## Le balisage de la page d’accueil (web/index.html)
map-home-title = CityLoom : réaménagez les rues de votre ville
map-home-description = Choisissez n’importe quelle rue ou jonction de la ville, réorganisez-la et voyez aussitôt ce qui fonctionne toujours.
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

## Vérifications du réseau (src/map/network.rs, src/map/vm.rs)
net-heading = Vérifications du réseau
net-no-places = Il n’y a pas de réseau à vérifier.
net-lead-none = Les vérifications du réseau n’ont rien trouvé dans l’ensemble des rues et des jonctions.
net-lead = { $n ->
    [one] Les vérifications du réseau ont trouvé 1 constat dans l’ensemble des rues et des jonctions.
   *[other] Les vérifications du réseau ont trouvé { $n } constats dans l’ensemble des rues et des jonctions.
}
net-note = Elles portent sur l’ensemble du réseau et sont comptées à part des vérifications de chaque lieu. Un problème veut dire que le réseau enfreint la règle; un point à noter mérite un coup d’œil, puisque la règle peut être respectée autrement. Les règles jugent les rues telles qu’elles sont maintenant, et non par rapport à leur tracé de départ.
net-count-none = aucun constat
net-count-one = 1 constat
net-count = { $n } constats
net-rule-cycling = Liens cyclables
net-rule-access = Accès aux rues piétonnes
net-rule-crossings = Passages pour piétons
net-rule-lanes = Nombre de voies
net-rule-transit = Axes de transport en commun
net-rule-walking = Parcours piétons
net-severity-problem = Problème
net-severity-notice = À noter
net-lanes = { $n ->
    [one] { $n } voie
   *[other] { $n } voies
}
net-cycling-island = { $n ->
    [one] Cette rue a une piste cyclable coupée du reste du réseau cyclable.
   *[other] Ces { $n } rues ont des pistes cyclables coupées du reste du réseau cyclable.
}
net-cycling-ends = { $at } : la piste cyclable de { $street } s’arrête, car aucune autre rue qui s’y rencontre n’en a une.
net-access-broken = Cette rue piétonne n’a pas de trottoir continu de { $need } de large pour les véhicules d’urgence, telle qu’elle est maintenant.
net-crossing-missing = Cette rue amène la circulation à la jonction et n’a pas de passage pour piétons.
net-crossing-wide = Le passage pour piétons fait ici { $width } de large, soit plus que le maximum de { $max }.
net-lanes-change = { $a } compte { $from } là où elle rejoint { $b }, qui en compte { $to } : un changement de plus de { $step } d’un coup.
net-transit-break = { $at } : la voie réservée au transport en commun de { $street } s’arrête, car aucune autre rue qui s’y rencontre n’en a une.
net-walking-ends = { $at } : le trottoir de { $street } s’arrête, car aucune autre rue qui s’y rencontre n’en a un.
net-walking-island = { $n ->
    [one] On ne peut pas aller à pied de cette rue au reste du réseau : une autoroute, une rue sans trottoir ou une branche sans passage pour piétons la coupe du reste.
   *[other] On ne peut pas aller à pied de ces { $n } rues au reste du réseau : une autoroute, une rue sans trottoir ou une branche sans passage pour piétons les coupe du reste.
}
net-walking-block = { $street } fait { $length } de long entre ses extrémités, soit plus que le maximum de { $max } pour un îlot.
net-unreadable = { $n ->
    [one] 1 jonction n’a pas pu être lue; ses passages pour piétons ne sont donc pas vérifiés.
   *[other] { $n } jonctions n’ont pas pu être lues; leurs passages pour piétons ne sont donc pas vérifiés.
}
net-more-findings = { $n ->
    [one] 1 autre constat n’est pas listé.
   *[other] { $n } autres constats ne sont pas listés.
}
net-more-places = { $n ->
    [one] et 1 autre lieu
   *[other] et { $n } autres lieux
}

## Données sources, l’onglet Provenance (src/map/source_quality.rs, src/map/vm.rs)
prov-heading = Données sources
prov-note = Ces constats montrent où les données OpenStreetMap derrière la carte peuvent être manquantes ou erronées. Ce ne sont pas des échecs de conception : ils sont comptés à part des vérifications de chaque lieu et des vérifications du réseau, et une rue visée ici peut quand même respecter toutes les règles. L’importateur ne peut pas distinguer une largeur indiquée sur la route de celle qu’il suppose; un constat sur une largeur peut donc n’être que la valeur par défaut.
prov-no-places = Il n’y a pas de données sources à examiner.
prov-lead-none = Rien ne semble manquant ni erroné dans les données sources.
prov-lead = { $n ->
    [one] 1 lieu des données sources peut être manquant ou erroné.
   *[other] { $n } lieux des données sources peuvent être manquants ou erronés.
}
prov-count-none = rien trouvé
prov-count-one = 1 lacune possible
prov-count = { $n } lacunes possibles
prov-label-source = Source :
prov-label-impact = Effet sur la conception :
prov-kind-name = Noms de rues
prov-kind-disconnected = Rues déconnectées
prov-kind-ends = Culs-de-sac
prov-kind-width = Largeurs
prov-kind-speed = Limites de vitesse
prov-name = Le nom de { $street } ne figure pas dans OpenStreetMap; la carte lui donne donc un nom de son cru.
prov-source-name = La route n’a pas d’étiquette de nom.
prov-impact-name = Les modifications enregistrées et les notes désignent la rue par un nom inventé par la carte, et une rue du même genre pourrait être prise pour une autre.
prov-disconnected = { $n ->
    [one] Cette rue n’est reliée à aucune autre rue de la zone chargée.
   *[other] Ces { $n } rues sont reliées entre elles et à aucune autre rue de la zone chargée.
}
prov-source-disconnected = Les routes ne partagent aucun nœud avec le reste du réseau et aucune n’atteint le bord de la zone. Les routes peuvent avoir été cartographiées sans nœud commun, ou les autres routes peuvent manquer.
prov-impact-disconnected = Les vérifications du réseau sur les parcours piétons, cyclables et de transport en commun les compteront comme coupées, ce qui peut être une lacune des données et non de la conception.
prov-ends = { $street } se termine à un nœud qu’aucune autre rue n’atteint.
prov-source-ends = La route s’arrête à un nœud qui lui est propre. Ce peut être un vrai cul-de-sac, ou la route qui la prolonge peut manquer ou ne pas être reliée.
prov-impact-ends = Il peut manquer une jonction ici; les passages et les voies que les vérifications de conception attendent ne seraient alors pas dessinés.
prov-width-row-narrow = { $street } fait { $width } de large à l’importation, soit moins que les { $limit } qu’une rue est censée exiger.
prov-width-row-wide = { $street } fait { $width } de large à l’importation, soit plus que les { $limit } qu’une rue est censée exiger.
prov-width-lane-narrow = Un élément de { $street } ({ $kind }) est à la largeur minimale permise par le catalogue, soit { $limit }.
prov-width-lane-wide = Un élément de { $street } ({ $kind }) est à la largeur maximale permise par le catalogue, soit { $limit }.
prov-source-width = L’importateur ne peut pas distinguer une largeur indiquée sur la route de celle que suppose osm2streets, et il garde chaque élément dans les limites du catalogue; un élément à une limite peut donc y avoir été ramené.
prov-impact-width = Les vérifications de largeur et de capacité, et la place qui reste pour les autres éléments, reposent sur cette largeur.
prov-speed = { $street } n’a pas de limite de vitesse dans OpenStreetMap.
prov-source-speed = La route n’a pas d’étiquette maxspeed, ou en a une qui n’a pas pu être lue.
prov-impact-speed = La vitesse affichée pour la rue est « Non indiquée », et la règle de limite de vitesse d’une zone ne peut pas la juger.
prov-more-findings = { $n ->
    [one] 1 autre constat n’est pas listé.
   *[other] { $n } autres constats ne sont pas listés.
}
map-report-title = Revue de conception : { $area }
map-report-cover-title = Rapport de revue de conception
map-report-when = { $date } { $hours } h { $minutes }
map-report-exported = Date d’exportation : { $when } UTC
map-report-version = CityLoom, version { $version }
map-report-scope = Portée : Ville entière
map-report-study-notice = Ce rapport provient d’une étude de conception. Ce n’est pas un document d’ingénierie et il ne remplace pas l’examen d’un professionnel qualifié.
map-report-placeholder-notice = Les règles, les taux et les seuils derrière les vérifications sont des valeurs provisoires et n’ont pas été vérifiés.
map-report-summary = Sommaire
map-report-summary-caption = Sommaire de la ville et de ses vérifications
map-report-col-measure = Mesure
map-report-col-figure = Chiffre
map-report-col-result = Résultat
map-report-row-city = Rues et jonctions de la ville
map-report-row-edited = Lieux modifiés par rapport au tracé importé
map-report-row-failing = Lieux dont les vérifications de conception échouent
map-report-row-problems = Problèmes du réseau
map-report-row-notices = Avis du réseau
map-report-figure-city = { $streets ->
    [one] { $streets } rue
   *[other] { $streets } rues
}, { $junctions ->
    [one] { $junctions } jonction
   *[other] { $junctions } jonctions
}
map-report-pass = Réussite
map-report-fail = Échec
map-report-notice = À noter
map-report-none-found = Aucun problème ni aucun avis du réseau n’a été relevé : { $result }.
map-report-nothing = Il n’y a rien à rapporter : cette ville n’a ni rue ni jonction.
map-report-failed = Le rapport n’a pas pu être produit. Réessayez.
map-report-dismiss = Fermer

# Les pages des lieux modifiés dans le rapport
map-report-place-before = Avant
map-report-place-after = Après
map-report-place-alt = { $which } : { $what }
map-report-place-omitted = Quelques petites marques ne sont pas dessinées dans cette figure.
map-report-place-zones = Cette rue compte { $zones } zones. Les figures la montrent dessinée à sa première zone.
map-report-place-street-missing = Cette rue n’a pas pu être lue; il n’y a donc rien de plus à montrer ici.
map-report-place-width = Largeur : { $before } avant, { $after } après.
map-report-place-class = Classe : { $before } avant, { $after } après.
map-report-place-speed = Limite de vitesse : { $before } avant, { $after } après.
map-report-place-pieces = Éléments : { $added ->
    [one] { $added } ajouté
   *[other] { $added } ajoutés
}, { $removed ->
    [one] { $removed } retiré
   *[other] { $removed } retirés
}.
map-report-place-other = Le changement touche des détails que cette page ne résume pas, comme les zones, les usages en bordure de rue ou les arrêts. Les figures et les vérifications montrent le résultat.
map-report-place-width-caption = Largeur par mode, avant et après
map-report-place-col-mode = Mode
map-report-place-share = { $length } ({ $pct } %)
map-report-place-checks-intro = « Avant » est la disposition importée d’OpenStreetMap; « Après » est la disposition modifiée.
map-report-place-na = s. o.
map-report-place-checks-caption = Vérifications de conception en échec avant ou après le changement
map-report-place-col-check = Vérification
map-report-place-col-detail = Détail
map-report-place-all-pass = { $n ->
    [one] { $n } vérification réussie
   *[other] { $n } vérifications réussies
} : { $result }.
map-report-place-more = { $n ->
    [one] { $n } autre vérification en échec
   *[other] { $n } autres vérifications en échec
}
map-report-place-control = Signalisation : { $before } avant, { $after } après.
map-report-place-control-same = Signalisation : { $after } (inchangée).
map-report-place-junction-undrawn = Cette jonction n’a pas pu être dessinée, avant ou après le changement; cette page n’a donc pas de figure.
map-report-place-junction-failing = Vérifications en échec : { $failing }.
map-report-place-left-out = Le rapport montre les { $shown } premiers lieux modifiés. { $left ->
    [one] { $left } autre lieu modifié n’est pas montré.
   *[other] { $left } autres lieux modifiés ne sont pas montrés.
}

## Les pages d’examen et l’annexe du rapport (src/map/report_review.rs)
map-report-network = Effets sur le réseau
map-report-network-caption = Les vérifications du réseau et leurs résultats
map-report-col-check = Vérification
map-report-col-findings = Constats
map-report-col-severity = Gravité
map-report-col-finding = Constat
map-report-col-places = Lieux
map-report-places-and-more = { $names } { $n ->
    [one] et 1 autre lieu
   *[other] et { $n } autres lieux
}
map-report-areas = Règles de zone
map-report-areas-none = Aucune règle de zone n’a été déclarée pour cette ville.
map-report-areas-intro = Une règle de zone est une règle de circulation tracée sur une partie de la ville. Chaque rue située dans une zone est vérifiée selon sa règle.
map-report-areas-caption = Les règles de zone et les rues qui s’y trouvent
map-report-col-area = Zone
map-report-col-rule = Règle
map-report-col-inside = Rues dans la zone
map-report-col-conflicts = En conflit
map-report-areas-streets-caption = Rues qui ne respectent pas la règle de leur zone
map-report-col-street = Rue
map-report-col-why = Raison
map-report-areas-more = { $n ->
    [one] 1 autre rue n’est pas listée.
   *[other] { $n } autres rues ne sont pas listées.
}
map-report-labelled = { $label } { $text }
map-report-limits = Limites des données OpenStreetMap
map-report-col-source = Origine
map-report-col-impact = Incidence sur la conception
map-report-appendix = Annexe
map-report-appendix-thresholds = Seuils et leur statut
map-report-appendix-thresholds-status = Aucun seuil de ce rapport n’a été vérifié selon la norme d’une ville ou d’un pays. Une réussite ou un échec indique où en est la conception par rapport à un chiffre provisoire, pas si elle respecte quoi que ce soit.
map-report-appendix-glossary = Glossaire
map-report-appendix-glossary-caption = Les termes employés dans ce rapport
map-report-col-term = Terme
map-report-col-meaning = Sens
map-report-term-before-after = Avant et Après
map-report-term-network = Vérification du réseau
map-report-term-area = Règle de zone
map-report-term-placeholder = Provisoire
map-report-def-pass = La conception satisfait à la vérification.
map-report-def-fail = La conception ne satisfait pas à la vérification.
map-report-def-problem = Le réseau enfreint une règle qui porte sur plusieurs rues.
map-report-def-notice = Mérite un coup d’œil : la règle peut être respectée autrement, ou les données sont trop minces pour trancher.
map-report-def-before-after = Avant est l’aménagement tel qu’importé d’OpenStreetMap; Après est l’aménagement modifié.
map-report-def-network = Une vérification qui porte sur l’ensemble du réseau de rues et de jonctions, et non sur un seul lieu.
map-report-def-area = Une règle de circulation tracée sur une partie de la ville, comme la zone sans voitures ou une limite de vitesse.
map-report-def-placeholder = Un chiffre qui tient lieu du chiffre réel jusqu’à ce qu’une source faisant autorité le remplace.
map-report-appendix-credits = Crédits
map-report-credits-osm = Les rues, les voies et les jonctions viennent d’OpenStreetMap, © les contributeurs d’OpenStreetMap, disponible sous la licence Open Database Licence (opendatacommons.org/licenses/odbl). osm2streets (licence Apache 2.0) transforme les données cartographiques en rues et en jonctions.
map-report-credits-atlas = Les mesures de priorité au transport en commun suivent la boîte à outils du Transit Priority Atlas, publié par le Marron Institute. CityLoom n’est ni affilié ni approuvé par l’Atlas ou le Marron Institute.
map-report-credits-software = Ce rapport est produit avec krilla (licences MIT et Apache 2.0) et composé en Noto Sans, © The Noto Project Authors, sous la licence SIL Open Font License 1.1. Les crédits complets se trouvent sur la page Crédits de l’application.
