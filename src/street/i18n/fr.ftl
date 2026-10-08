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
st-cross-section = Coupe transversale
st-focus-hint = Les flèches sélectionnent. Maj + flèches réordonnent. + et − redimensionnent, avec Maj pour de plus grands pas. Entrée modifie la largeur.
st-touch-cue = Sur un écran tactile, faites glisser la rue de côté pour la voir en entier.
st-space-heading = Où va la largeur
st-people-moved = Personnes déplacées
st-sample-numbers = chiffres d’exemple
st-does-it-work = Est-ce que ça fonctionne?
st-sample-limits = limites d’exemple
st-your-changes = Vos modifications
st-measures-heading = Mesures de priorité pour le transport en commun
st-measures-hint-a = Des façons de donner la priorité aux autobus, d’après le
st-measures-hint-b = Réorganiser conserve les trottoirs et redessine la chaussée selon cette mesure. Vous pouvez l’annuler.
st-about-street = À propos de cette rue

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
mode-foot = Marche
mode-bike = Vélo
mode-transit = Transport en commun
mode-vehicle = Autos et camions
mode-green = Verdure
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
group-name-linear = Mesures linéaires continues
group-name-local = Mesures ponctuelles
group-name-area = Mesures à l’échelle d’un secteur
group-note-linear = Aménagements de voies le long de la rue. Réorganiser dispose la chaussée selon cette mesure.
group-note-local = Éléments propres à une intersection. Réglez-les sur la page de l’intersection.
group-note-area = Elles couvrent de nombreuses rues; elles ne sont donc pas modélisées ici.
measure-arrange = Réorganiser
measure-arrange-label = Réorganiser la rue selon { $code } { $name }
measure-this-street = Cette rue
measure-will-not-fit = Ne rentre pas
measure-set-at-junction = Se règle à une intersection
measure-not-modelled = Non modélisé
