# The names the app invents for the city's places and streets (src/city/model.rs).
city-name = { $name }
class-motorway = motorway
class-arterial = arterial
class-collector = collector
class-local = local
class-pedestrian = pedestrian
city-unnamed = Unnamed { $class } street
city-unnamed-motorway = Unnamed motorway
# An unnamed road named in a sentence, for a place on it that no named road reaches.
city-unnamed-in-sentence = an unnamed { $class } street
city-unnamed-motorway-in-sentence = an unnamed motorway
# The end of an unnamed road; `street` is one of the two above.
city-end-of-unnamed = End of { $street }
city-the-end-of-unnamed = the end of { $street }
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
city-a-connection = a connection
city-the-edge-of-the-map = the edge of the map
city-edge-between = { $from } to { $to }
city-edge-through = through the city
city-edge-name = { $street } · { $ends }
city-cannot-draw = Cannot be drawn with the streets as they are
city-junction-today = Junction today
city-earlier = Earlier changes
scenario-heading = Scenarios
scenario-pick = Design shown
scenario-baseline = Baseline (what the map shows)
scenario-showing-baseline = Baseline: edits change the design the map shows.
scenario-showing = Scenario “{ $name }”: edits change this scenario only.
scenario-manage = Manage scenarios
scenario-name = Scenario name
scenario-new = New from baseline
scenario-duplicate = Duplicate
scenario-rename = Rename
scenario-delete = Delete
scenario-delete-confirm = Confirm delete
scenario-created = Scenario “{ $name }” made from the baseline. Now editing it.
scenario-duplicated = Scenario “{ $from }” duplicated as “{ $name }”. Now editing the copy.
scenario-renamed = Scenario “{ $from }” renamed “{ $name }”.
scenario-deleted = Scenario “{ $name }” deleted. Now showing the baseline.
scenario-delete-ask = Delete scenario “{ $name }”? Press Confirm delete to delete it.
scenario-switched-baseline = Now showing the baseline.
scenario-switched = Now showing scenario “{ $name }”.
scenario-name-empty = Give the scenario a name.
scenario-name-too-long = A scenario name can be at most { $max } characters long.
scenario-name-taken = Another scenario is already called “{ $name }”.
scenario-full = A place keeps at most { $max } scenarios. Delete one to make another.
scenario-missing = That scenario is no longer here.
