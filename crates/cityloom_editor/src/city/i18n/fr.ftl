# Les noms que l’application invente pour les lieux et les rues de la ville (src/city/model.rs).
city-name = { $name }
class-motorway = autoroutière
class-arterial = artérielle
class-collector = collectrice
class-local = locale
class-pedestrian = piétonne
city-unnamed = Rue { $class } sans nom
city-unnamed-motorway = Autoroute sans nom
# Une route sans nom, nommée dans une phrase, pour un lieu qu’aucune route nommée n’atteint.
city-unnamed-in-sentence = une rue { $class } sans nom
city-unnamed-motorway-in-sentence = une autoroute sans nom
# Le bout d’une route sans nom ; `street` est l’un des deux messages ci-dessus.
city-end-of-unnamed = Bout d’{ $street }
city-the-end-of-unnamed = le bout d’{ $street }
city-junction-number = Jonction { $n }
city-junction-of-one = Jonction { $a }
city-junction-of-two = { $a } et { $b }
city-junction-of-three = { $a }, { $b } et { $c }
city-junction-of-many = { $a }, { $b } et { $rest ->
    [one] 1 autre
   *[other] { $rest } autres
  }
city-end-of = Bout de { $street }
city-connection-on = Raccordement sur { $street }
city-map-edge = Limite de la carte
city-the-end-of = le bout de { $street }
city-a-connection-on = un raccordement sur { $street }
city-a-connection = un raccordement
city-the-edge-of-the-map = la limite de la carte
city-edge-between = entre { $from } et { $to }
city-edge-through = à travers la ville
city-edge-name = { $street } · { $ends }
city-cannot-draw = Ne peut pas être dessinée avec les rues telles qu’elles sont
city-junction-today = Jonction aujourd’hui
city-earlier = Modifications antérieures
scenario-heading = Scénarios
scenario-pick = Conception affichée
scenario-baseline = Référence (ce que montre la carte)
scenario-showing-baseline = Référence : les modifications changent la conception que montre la carte.
scenario-showing = Scénario « { $name } » : les modifications ne changent que ce scénario.
scenario-manage = Gérer les scénarios
scenario-name = Nom du scénario
scenario-new = Nouveau à partir de la référence
scenario-duplicate = Dupliquer
scenario-rename = Renommer
scenario-delete = Supprimer
scenario-delete-confirm = Confirmer la suppression
scenario-created = Scénario « { $name } » créé à partir de la référence. Vous le modifiez maintenant.
scenario-duplicated = Scénario « { $from } » dupliqué sous le nom « { $name } ». Vous modifiez maintenant la copie.
scenario-renamed = Scénario « { $from } » renommé « { $name } ».
scenario-deleted = Scénario « { $name } » supprimé. La référence est maintenant affichée.
scenario-delete-ask = Supprimer le scénario « { $name } »? Appuyez sur Confirmer la suppression pour le supprimer.
scenario-switched-baseline = La référence est maintenant affichée.
scenario-switched = Le scénario « { $name } » est maintenant affiché.
scenario-name-empty = Donnez un nom au scénario.
scenario-name-too-long = Un nom de scénario compte au plus { $max } caractères.
scenario-name-taken = Un autre scénario s’appelle déjà « { $name } ».
scenario-full = Un lieu garde au plus { $max } scénarios. Supprimez-en un pour en créer un autre.
scenario-missing = Ce scénario n’existe plus.
scenario-gone = Ce scénario n’existe plus : il a été supprimé ailleurs. La référence est maintenant affichée.
scenario-gone-edit = Ce scénario n’existe plus : il a été supprimé ailleurs, et votre dernière modification n’a donc pas été conservée. La référence est maintenant affichée.
cmp-title = Comparer les conceptions · { $place }
cmp-pick-a = Conception A
cmp-pick-b = Conception B
cmp-baseline = Référence
cmp-col-a = A : { $name }
cmp-col-b = B : { $name }
cmp-summary-h = Ce qui diffère dans B
cmp-same = { $b } et { $a } sont identiques.
cmp-differs =
    { $n ->
        [one] { $b } diffère de { $a } sur { $n } point.
       *[other] { $b } diffère de { $a } sur { $n } points.
    }
cmp-also = Diffère aussi par : { $list }
cmp-back-street = Retour à la rue
cmp-back-junction = Retour à la jonction
cmp-h-space = Espace
cmp-h-lanes = Voies
cmp-h-pieces = Éléments
cmp-h-checks = Vérifications
cmp-h-control = Signalisation
cmp-h-arms = Branches
cmp-h-signal = Feux de circulation
cmp-only-passes = Réussi seulement dans B : { $check }
cmp-only-fails = Échoue seulement dans B : { $check }
cmp-other-speed = les limites de vitesse
cmp-other-stretches = les tronçons
cmp-other-curbside = les usages du bord de rue
cmp-other-stops = les arrêts
cmp-other-calming = l’apaisement de la circulation
cmp-other-green = la verdure
cmp-other-ring = l’anneau du carrefour giratoire
cmp-other-raised = les éléments surélevés
cmp-other-bus = la priorité aux autobus
cmp-other-corners = les coins
