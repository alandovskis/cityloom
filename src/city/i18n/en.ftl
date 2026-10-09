# The names the app invents for the city's places and streets (src/city/model.rs).
city-name = { $name }
# Words the junction slice still gives in English (a check it fails), until it is translated.
city-text = { $text }
class-motorway = motorway
class-arterial = arterial
class-collector = collector
class-local = local
city-unnamed = Unnamed { $class } street
city-unnamed-motorway = Unnamed motorway
city-junction-number = Junction { $n }
city-junction-of-one = { $a } junction
city-junction-of-two = { $a } and { $b }
city-junction-of-three = { $a }, { $b } and { $c }
city-junction-of-many = { $a }, { $b } and { $rest ->
    [one] 1 more
   *[other] { $rest } more
  }
city-end-of = End of { $street }
city-connection-on = Connection on { $street }
city-map-edge = Edge of the map
city-the-end-of = the end of { $street }
city-a-connection-on = a connection on { $street }
city-the-edge-of-the-map = the edge of the map
city-edge-between = { $from } to { $to }
city-edge-through = through the city
city-edge-name = { $kind } · { $ends }
city-cannot-draw = Cannot be drawn with the streets as they are
city-junction-today = Junction today
city-earlier = Earlier changes
