# What the street page says: the markup of street.html.
st-page-description = Rearrange a street's fixed width: add, move and resize its pieces and see at once whether it fits.
st-editors = Editors
st-map = Map
st-piece-details = Piece details
st-details-toggle-title = Show or hide the piece details ( [ )
st-key-select = select
st-key-reorder = reorder
st-key-resize = resize
st-key-bigger = bigger
st-key-edit-width = edit width
st-key-remove = remove
st-key-details-notes = details, notes
st-key-undo = undo
st-key-view = section, 2.5D
st-cross-section = Cross-section
st-street-2-5d = Street in 2.5D
st-focus-hint = Arrows select. Shift+arrows reorder. + and − resize, with Shift for bigger steps. Enter edits the width.
st-touch-cue = On a touch screen, swipe the street sideways to see all of it.
st-space-heading = Where the width goes
st-people-moved = People moved
st-sample-numbers = sample numbers
st-does-it-work = Does it work?
st-sample-limits = sample limits
st-your-changes = Your changes
st-transit-heading = Transit priority
st-about-street = About this street
st-provenance-heading = Where this street came from
st-no-source = This street was not made from OpenStreetMap.

# The catalogue: kinds of piece, modes and the groups of the add menu (src/shared/catalogue.rs).
kind-sidewalk = Sidewalk
kind-planting = Planting strip
kind-bike = Bike lane
kind-travel = Driving lane
kind-bus = Transit lane
kind-parking = Parking
kind-median = Planted median
kind-loading = Loading zone
kind-shoulder = Shoulder
kind-bikerack = Bike rack
kind-bikeshare = Bikeshare station
kind-pole = Utility pole
kind-busshelter = Bus shelter
kind-busstation = Bus station
kind-bench = Bench
kind-terrace = Café terrace
kind-streetlamp = Street lamp
kind-buffer = Buffer
mode-foot = Walking
mode-bike = Biking
mode-transit = Transit
mode-vehicle = Cars and trucks
mode-green = Greenery
mode-neutral = Buffer
group-walking = Walking
group-greenery = Greenery
group-cycling = Cycling
group-transit = Transit
group-roadway = Roadway
group-furniture = Furniture
group-utilities = Utilities

# How the pieces stand against the street's width, and what is said after an edit.
fit-used = Every metre of the street is used
fit-left = { $amount } left to use
fit-over = { $amount } too wide. Make a piece narrower or remove one
status-used = { fit-used }.
status-unused = { $amount } of the street is still unused.
status-over = { fit-over }.
edit-done = { $label }. { $fit }.
edit-undone = Undone. { $fit }.
edit-redone = Redone. { $fit }.
selection-position = { $kind }, { $width }, { $index } of { $total }
started-over = Started over from the street as it is today. Undo brings your changes back.
measure-refused = That measure does not suit this street.

# The notes beside the section.
units-metres = metres
units-feet = feet
space-caption = Width by use, in { $units }
col-use = Use
col-measure = Measure
col-today = Today
col-design = Your design
col-change = Change
col-step = Step
col-what-changed = What changed
street-today = Street today
capacity-caption = People per hour, placeholder rates
capacity-row = People per hour
check-fits-full = Every metre is used
check-fits-over = { $amount } too wide. Narrow or remove a piece.
check-access-ok = A lane of { $amount } or more
check-access-bad = No lane of { $amount } or more
check-passes = : passes
check-fails = : fails
badge-fail = { " " }fail
failing =
    { $n ->
        [one] 1 check fails
       *[other] { $n } checks fail
    }
measure-arrange-label = Arrange the street as { $code } { $name }

# The page around the section: heading, window title, title block, history, add menu, clock and welcome (src/street/page.rs).
header-city-map = City map
header-section = Street cross-section
header-wide = wide
header-between = between
header-and = and
ends-and = { $a } and { $b }
title-between = { $street } between { $ends } · CityLoom
title-street-editor = CityLoom street editor
block-street = Street
block-width = Width
block-changes = Changes made
block-osm = OpenStreetMap
block-data = Data
block-edited = Edited
block-imported = As imported
history-tools = Sheet tools
history-undo = Undo
history-redo = Redo
history-reset-title = Back to the street as it is today. Undo brings your changes back.
history-reset = Start over
add-button = Add a piece
add-hint = Goes after the selected piece.
clock-time-of-day = Time of day
clock-except = { $kind } except { $others }
clock-window = { $kind } { $from }–{ $to }
clock-window-days = { $kind } { $days } { $from }–{ $to }
clock-numbers-for = Numbers are for { $time }.
day-mo = Mo
day-tu = Tu
day-we = We
day-th = Th
day-fr = Fr
day-sa = Sa
day-su = Su
welcome-text = Add a piece, then drag to arrange. The width is fixed; a piece that does not fit turns orange.
welcome-dismiss = Got it

# The drawing of the section (src/street/view.rs).
view-section-label = Street cross-section editor
view-iso-label = Street editor, 2.5D view
# The view of the drawing, the section or the street in 2.5D (src/street/vm.rs, src/street/page.rs).
view-shown-section = Showing the section
view-shown-iso = Showing the street in 2.5D
view-mode-label = Show the street as
view-mode-section = Section
view-mode-iso = 2.5D

# The Transit Priority Atlas measures, by lowercase code (src/shared/atlas.rs).
atlas-a1-name = Transit Streets
atlas-a2-name = Transit Ways
atlas-a3-name = Transit and Direct Access Streets
atlas-b1-name = Center-Running Transit Lanes
atlas-b2-name = Center-Running Transit Lanes on Freeway Medians
atlas-b3-name = Static Alternate-Direction Center-Running Transit Lanes
atlas-b4-name = Dynamic Alternate-Direction Center-Running Transit Lanes
atlas-c1-name = Edge-Running Bidirectional Transit Lanes
atlas-d1-name = Offset Transit Lanes
atlas-e1-name = Curb-Adjacent Transit Lanes
atlas-e2-name = Curb-Adjacent Reversible Parking and Transit Lanes
atlas-e3-name = Transit Lanes on Freeway Shoulders
atlas-f1-name = Contraflow Transit Lanes
atlas-f2-name = Offset Contraflow Transit Lanes
atlas-g1-name = Offset Queue-Jump Lanes
atlas-g2-name = Curbside Queue-Jump Lanes
atlas-g3-name = Virtual Queue-Jump Lane
atlas-h1-name = Signal-Controlled Bus Gates
atlas-h2-name = Yield-Controlled Bus Gates
atlas-l1-name = Indirect Left Turn via Alternative Itinerary
atlas-l2-name = Indirect Left Turn within the Intersection
atlas-l3-name = Right-In/Right-Out
atlas-l4-name = Dead-Ending of Lateral Streets
atlas-m1-name = Bus Bulbs
atlas-m2-name = Signal-Protected On-Street Platforms
atlas-n1-name = Transit Modal Filter
atlas-tsp-name = Transit Signal Priority
atlas-tsp-note = The Atlas lists it as under development, and CityLoom has no signal timing to prioritise.
atlas-z1-name = Limited Traffic Areas
atlas-z1-note = An area of many streets, and CityLoom edits one street or one junction.
atlas-w-d-name = Dynamic Congestion Pricing
atlas-w-d-note = The Atlas lists it as in development, and a price needs a city and demand to act on.
atlas-w-f-name = Fixed-Fee Road Pricing
atlas-w-f-note = The Atlas lists it as in development, and a price needs a city and demand to act on.

# What a recognised measure is missing (src/street/measures.rs).
problem-narrow-lane = A bus lane is { $width } mm wide, and a transit lane wants { $min } mm
problem-no-platform = Center-running lanes need a median or platform beside them for stops

# The materials, curbs and directions of the catalogue (src/shared/catalogue.rs).
material-asphalt = Asphalt
material-concrete = Concrete
material-permeable = Permeable paving
material-brick = Brick pavers
material-grass = Grass
material-planted = Planted bed
material-gravel = Gravel
material-trees = Street trees
curb-granite = Granite
curb-concrete = Concrete
curb-asphalt = Asphalt
curb-planted = Planted
curb-kassel = Bus-friendly curb
curb-bikefriendly = Bike-friendly curb
curb-island = Bus boarding island
direction-away = Away from you
direction-toward = Toward you

# The panel of the selected piece (src/street/inspector.rs).
inspector-empty = Select a piece to change its width and surface.
inspector-sub = { $width } wide · { $at } of { $total }
inspector-sub-timed = { $width } wide · { $at } of { $total } · { $time }
inspector-width = Width
inspector-step-wider = Wider by { $step } { $unit }
inspector-step-narrower = Narrower by { $step } { $unit }
inspector-allowed = Allowed { $min } to { $max } { $unit }
inspector-surface = Surface
inspector-surface-note = What it is paved with.
inspector-planting = Planting
inspector-planting-note = What is planted in it.
inspector-vehicle = Vehicle
inspector-bus = Bus
inspector-tram = Tram
inspector-times = Other times
inspector-times-base = { $kind } the rest of the day.
inspector-times-add = Add other times
inspector-variant-type = Type { $n }
inspector-variant-direction = Direction { $n }
inspector-variant-remove = Remove { $kind } at { $from } to { $to }
inspector-from = From
inspector-to = To
inspector-until = to
inspector-direction = Direction
inspector-direction-note = Which way traffic goes.
inspector-two-way = Two-way
inspector-curb = Curb
inspector-curb-note = The raised edge, if it has one.
inspector-curb-none = None (flush)

# The marks of the kinds, shown where a name does not fit (src/shared/catalogue.rs).
kind-mark-sidewalk = SW
kind-mark-planting = PL
kind-mark-bike = BL
kind-mark-travel = TL
kind-mark-bus = TR
kind-mark-parking = PK
kind-mark-median = MD
kind-mark-loading = LZ
kind-mark-shoulder = SD
kind-mark-bikerack = BR
kind-mark-bikeshare = BS
kind-mark-pole = UP
kind-mark-busshelter = SH
kind-mark-busstation = ST
kind-mark-bench = BN
kind-mark-terrace = CT
kind-mark-streetlamp = SL
kind-mark-buffer = BF

# The drawing of the section (src/street/svg.rs).
svg-today = Today
svg-your-design = Your design
svg-change = Change { $n }
svg-street-edge = Street edge
svg-unused = Unused
svg-unused-length = Unused { $length }
svg-too-wide = { $length } too wide
svg-too-wide-clipped = { $length } too wide (more off-screen)
svg-street-width = Street width { $length }
svg-design-width = Your design { $length }
svg-scale = Scale
svg-label = Cross-section of { $name }. { $count ->
        [one] { $count } segment
       *[other] { $count } segments
    }, { $total } of { $row }. { $fit }.
svg-iso-label = { $name } in 2.5D, along a { $length } stretch. { $count ->
        [one] { $count } segment
       *[other] { $count } segments
    }.
svg-iso-label-zone = { $name } in 2.5D, { $zone }, { $from } to { $to }, along a { $length } stretch. { $count ->
        [one] { $count } segment
       *[other] { $count } segments
    }.
svg-iso-label-joint = { $name } in 2.5D, along a { $length } stretch centred on the boundary between { $near } in front and { $far } beyond. The section changes abruptly there. { $zone } has { $count ->
        [one] { $count } segment
       *[other] { $count } segments
    }.

# What the model says it did, put into words by `text::say` (src/street/model.rs): the list of changes and what is announced after an edit.
rev-earlier = Earlier changes
rev-reset = Reset to existing
rev-add = Add { $kind }
rev-remove = Remove { $kind }
rev-move = Move { $kind }
rev-resize = Resize { $kind }
rev-resize-pair = Resize { $a } and { $b }
rev-surface = { $kind } surface: { $material }
rev-curb = { $kind } curb: { $curb }
rev-vehicle = { $kind } vehicle: { $vehicle }
rev-direction = { $kind } direction: { $direction }
rev-window = { $base } is { $kind } { $from }-{ $to }
rev-variant-direction = { $base } { $kind } direction: { $direction }
rev-variant-remove = Remove other times from { $kind }
rev-measure = { $code } { $name }
rev-word-none = none
rev-word-two-way = two-way
rev-word-tram = tram
rev-word-bus = bus
side-keeps-right = Traffic keeps right
side-keeps-left = Traffic keeps left
check-label-fits = Fits the street width
check-label-edges = Sidewalk on both sides
check-label-access = Room for emergency vehicles
check-detail-no-sidewalks = A freeway has no sidewalks
check-detail-both-sides = Both sides
check-detail-one-side = One side has no sidewalk
check-detail-side-ok = Lanes run the way this region drives
check-detail-side-bad = A lane runs against traffic that keeps { $side }
side-word-right = right
side-word-left = left

# A pedestrian street (src/street/model.rs, page.rs, vm.rs).
make-pedestrian-button = Make pedestrian
make-pedestrian-hint = Every driving, parking, loading, bus and shoulder piece becomes the kind you pick.
transform-button = Transform
transform-transit-hint = Arrange the street as one of these transit measures. Each one rebuilds the roadway as one change you can undo.
rev-made-pedestrian = Made pedestrian
rev-speed = Speed limit { $kmh } km/h
rev-speed-not-set = Speed limit not set
street-now-pedestrian = { $street } is now a pedestrian street.
street-traffic-again = { $street } carries traffic again.
check-access-ped-ok = Sidewalks give an unbroken { $amount } or more
check-access-ped-bad = No unbroken sidewalk of { $amount } or more
check-detail-no-edges-pedestrian = No roadway, so no sidewalk is asked for

# Zones (src/street/model/zones.rs): what the model says it did to a street's zones, and why it refused.
zone-unnamed = Zone { $n }
zone-unnamed-mid = Zone { $n }
rev-zone-split = Split the street at { $at }
rev-zone-merge = Merge zones, keeping { $kept }
rev-zone-move = Move the start of { $zone } to { $to }
rev-zone-name = Zone name: { $zone }
rev-zone-unname = Clear the name of { $zone }
rev-zone-reset = Reset { $zone } to the street's section
rev-zone-use = Use { $zone } for the whole street
zone-refused-no-anchor = Both ends of this street are cut by the edge of the map, so zones cannot be placed along it.
zone-refused-elsewhere = The zones of this street were drawn on a road of another length or from its other end, so they cannot be rearranged here.
zone-refused-too-short = A zone must be at least { $min } long.
zone-refused-outside = That position is outside the street.
zone-refused-no-neighbour = There is no zone on that side to merge with.
zone-refused-no-boundary = The first zone starts at the end of the street, so it has no boundary to move.
zone-refused-dragging = Finish the drag before changing the zones.
zones-elsewhere = The zones were drawn on this street when it was { $drawn } long; here it is { $loaded }.
check-curb-in = { $check }: { $instance }
check-stop-in = { $check }: { $instance }
check-zone-in = { $check } in { $zone }
zone-selected = { $zone }, { $from } to { $to }, { $length } long, { $section ->
        [own] its own section
       *[base] the base section
    }, { $n } of { $total }
zone-cut-off = Cut off by the edge of the map

# The zone strip (src/street/strip.rs).
st-zones-heading = Zones along the street
st-zones-strip-label = Zones of the street, from start to end
st-zones-from = Measured from { $end }
st-zones-end-first = the first end
st-zones-end-second = the second end

# The commands beside the zone strip (src/street/zone_tools.rs).
zone-bad-number = That is not a number.
st-zone-tools = Zone commands
st-zone-split = Split
st-zone-split-at = Split at ({ $unit })
st-zone-start = Start of this zone ({ $unit })
st-zone-merge-prev = Merge with the previous zone
st-zone-merge-next = Merge with the next zone
st-zone-merge-choice = Which section to keep
st-zone-keep = Keep the section of { $zone }
st-zone-keep-cancel = Cancel the merge
st-zone-name = Zone name
st-zone-reset = Reset to the street's section
st-zone-use = Use this section for the whole street
st-speed-label = Speed limit
st-speed-not-set = Not set
st-speed-kmh = { $kmh } km/h
# The speed limit sign in the 2.5D: its heading, the same word on a Canadian sign in both languages.
iso-sign-maximum = MAXIMUM
svg-iso-transition = A transition of { $length } leads into { $zone }.
svg-iso-speed = Speed limit { $kmh } km/h.
svg-curb-uses = Curbside, { $side }: { $count ->
        [one] one use
       *[other] { $count } uses
    } along the street.
svg-iso-curb = { $count ->
        [one] One curbside use is shown.
       *[other] { $count } curbside uses are shown.
    }
svg-iso-stops = { $count ->
        [one] One transit stop is shown.
       *[other] { $count } transit stops are shown.
    }
check-boundary = between { $a } and { $b }
check-label-transition-room = Room for the transition
check-label-lane-drop = Lane drops
check-detail-transition-room = The transition needs { $need } but { $zone } is { $room } long.
check-detail-transition-room-ok = Every transition fits in its zone.
check-detail-lane-drop = A lane ends with no lane beside it to merge into.
check-detail-lane-drop-ok = Every lane that ends can merge.

# Curbside programmes (src/street/model/curb.rs): the uses along a street's curbside lane, what the model says it did to them, and why it refused.
curb-use-parking = Parking
curb-use-loading = Loading
curb-use-tree = Street tree
curb-use-planting = Planting
curb-use-rain-garden = Rain garden
curb-use-patio = Patio
curb-use-bike-parking = Bike parking
curb-use-platform = Transit platform
rev-curb-add = Add { $use } at { $at }
rev-curb-use = Change the use at { $at } to { $use }
rev-curb-resize = Resize the { $use } at { $at }
rev-curb-move = Move the { $use } to { $at }
rev-curb-remove = Remove the { $use } at { $at }
curb-refused-elsewhere = The uses of this curbside were drawn on a road of another length or from its other end, so they cannot be rearranged here.
curb-refused-overlap = That would overlap another use on the same side.
curb-refused-too-long = That would run past the end of the street.
curb-refused-time-windows = This lane changes with the time of day, so it cannot be divided into uses.
curb-refused-dragging = Finish the drag before changing the curbside.

# The checks of the curbside uses (src/street/model/curb.rs).
curb-instance = { $use } at { $at }, { $side }
check-label-curb-length = Curbside uses are long enough
check-label-curb-lane = Curbside uses have a lane
check-label-curb-crossing = Parking clear of crossings
check-label-curb-bike = Curbside uses clear of bike lanes
check-label-curb-sidewalk = Patios leave room on the sidewalk
check-detail-curb-length = { $use } is { $length } long, under the { $min } it needs.
check-detail-curb-length-ok = Every use is as long as it needs.
check-detail-curb-lane = There is no parking or loading lane here.
check-detail-curb-lane-ok = Every use lies on a parking or loading lane.
check-detail-curb-crossing = Parking and loading must stay { $amount } from a pedestrian crossing.
check-detail-curb-crossing-ok = Parking and loading keep clear of pedestrian crossings.
check-detail-curb-bike-door = Doors would open into the bike lane beside the parking. A buffer of { $amount } or more is needed.
check-detail-curb-bike-platform = Passengers cross the bike lane to board. The crossing is not drawn, so check it.
check-detail-curb-bike-ok = No use opens onto a bike lane.
check-detail-curb-sidewalk = The patio leaves { $have } of sidewalk; { $need } or more is needed.
check-detail-curb-sidewalk-ok = Every patio leaves room on the sidewalk.

# The curbside strips and their commands (src/street/curb_strip.rs, src/street/curb_tools.rs).
curb-gap = Free
curb-cell = { $name }, { $from } to { $to }, { $length } long, { $n } of { $total }
curb-cell-failing = Fails a check. See the checks.
curb-choose-gap = Choose a free stretch of the curbside first.
curb-choose-use = Choose a use on the curbside first.
st-curb-heading = Curbside uses along the street
st-curb-caption = Curbside, { $side }
st-curb-strip-label = Curbside uses, { $side }
st-curb-tools = Curbside commands
st-curb-use = Use
st-curb-length = Length ({ $unit })
st-curb-add = Add use
st-curb-remove = Remove use

# Transit stops (src/street/model/stop.rs).
stop-kind-curb = Curb stop
stop-kind-bulb = Curb extension
stop-kind-island = Boarding island
stop-word-sheltered = with a shelter
stop-word-unsheltered = without a shelter
rev-stop-add = Add { $kind } at { $at }
rev-stop-kind = Change the stop at { $at } to { $kind }
rev-stop-shelter = Stop at { $at }: { $state }
rev-stop-name = Name the { $kind } at { $at }
rev-stop-resize = Resize the { $kind } at { $at }
rev-stop-move = Move the { $kind } to { $at }
rev-stop-remove = Remove the { $kind } at { $at }

# The checks of the transit stops (src/street/model/stop.rs).
stop-instance = { $kind } at { $at }, { $side }
stop-instance-named = { $name } ({ $kind }, { $at })
check-label-stop-lane = Stops have a lane for the bus
check-label-stop-kind = Stops are built as their kind needs
check-label-stop-length = Stops are long enough for a bus
check-label-stop-width = Stops have room to board
check-label-stop-bike = Stops clear of bike lanes
check-label-stop-shelter = Shelters have room
check-detail-stop-lane = There is no bus or driving lane here for buses going the way traffic goes on this side.
check-detail-stop-lane-ok = Every stop has a lane for the bus.
check-detail-stop-kind-curb = A curb stop needs the bus's lane to run along the curb, with no parking or loading lane beside it.
check-detail-stop-kind-bulb = A curb extension needs a parking or loading lane beside the bus's lane.
check-detail-stop-kind-island = A boarding island needs a parking, loading or buffer piece beside the bus's lane, with a bike lane behind it.
check-detail-stop-kind-ok = Every stop is built as its kind needs.
check-detail-stop-length = The stop is { $length } long, under the { $min } a bus needs.
check-detail-stop-length-ok = Every stop is long enough for a bus.
check-detail-stop-width-curb = The sidewalk beside the stop is { $have } wide; { $need } or more is needed.
check-detail-stop-width-bulb = The curb extension and the sidewalk give { $have }; { $need } or more is needed.
check-detail-stop-width-island = The island is { $have } wide; { $need } or more is needed.
check-detail-stop-width-ok = Every stop has room to board.
check-detail-stop-bike-cross = Riders would cross the boarding area. The crossing is not drawn, so check it.
check-detail-stop-bike-narrow = The bike lane behind the island is { $have } wide; { $need } or more is needed.
check-detail-stop-bike-ok = No stop sends riders across the boarding area.
check-detail-stop-shelter-curb = The sidewalk beside the stop is { $have } wide; a shelter needs { $need } or more.
check-detail-stop-shelter-bulb = The curb extension and the sidewalk give { $have }; a shelter needs { $need } or more.
check-detail-stop-shelter-island = The island is { $have } wide; a shelter needs { $need } or more.
check-detail-stop-shelter-ok = Every shelter has room.

# The stops rows and their commands (src/street/row.rs, src/street/stop_tools.rs, src/street/model/stop.rs).
stop-label = { $kind }
stop-label-named = { $name } ({ $kind })
stop-label-sheltered = { $label }, with a shelter
stop-cell = { $name }, { $from } to { $to }, { $length } long, { $n } of { $total }
stop-choose-gap = Choose a free stretch of the street's edge first.
stop-choose-stop = Choose a stop first.
st-stop-heading = Stops along the street
st-stop-caption = Stops, { $side }
st-stop-strip-label = Stops, { $side }
st-stop-tools = Stop commands
st-stop-kind = Kind
st-stop-name = Name
st-stop-length = Length ({ $unit })
st-stop-shelter = Shelter
st-stop-add = Add stop
st-stop-remove = Remove stop
stop-refused-no-anchor = Both ends of this street are cut by the edge of the map, so stops cannot be placed along it.
stop-refused-elsewhere = The stops of this street were drawn on a road of another length or from its other end, so they cannot be rearranged here.
stop-refused-overlap = That would overlap another stop on the same side.
stop-refused-too-long = That would run past the end of the street.
stop-refused-time-windows = A stop cannot be placed on a lane that changes with the time of day.
stop-refused-dragging = Finish the drag before changing the stops.
