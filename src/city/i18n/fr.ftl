# Les noms que l’application invente pour les lieux et les rues de la ville (src/city/model.rs).
city-name = { $name }
class-motorway = autoroutière
class-arterial = artérielle
class-collector = collectrice
class-local = locale
city-unnamed = Rue { $class } sans nom
city-unnamed-motorway = Autoroute sans nom
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
city-the-edge-of-the-map = la limite de la carte
city-edge-between = { $kind } · entre { $from } et { $to }
city-edge-through = { $kind } · à travers la ville
city-cannot-draw = Ne peut pas être dessinée avec les rues telles qu’elles sont
city-junction-today = Jonction aujourd’hui
city-earlier = Modifications antérieures
