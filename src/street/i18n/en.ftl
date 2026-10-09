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
st-cross-section = Cross-section
st-focus-hint = Arrows select. Shift+arrows reorder. + and − resize, with Shift for bigger steps. Enter edits the width.
st-touch-cue = On a touch screen, swipe the street sideways to see all of it.
st-space-heading = Where the width goes
st-people-moved = People moved
st-sample-numbers = sample numbers
st-does-it-work = Does it work?
st-sample-limits = sample limits
st-your-changes = Your changes
st-measures-heading = Transit priority measures
st-measures-hint-a = Ways to give buses priority, from the
st-measures-hint-b = Arrange keeps the sidewalks and lays the roadway out again as that measure. You can undo it.
st-about-street = About this street

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
mode-foot = Walking
mode-bike = Biking
mode-transit = Transit
mode-vehicle = Cars and trucks
mode-green = Greenery
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
group-name-linear = Linear continuous measures
group-name-local = Localized measures
group-name-area = Area-wide measures
group-note-linear = Lane arrangements along the street. Arrange lays the roadway out as that measure.
group-note-local = Features at one junction. Set them on the intersection page.
group-note-area = They cover many streets, so they are not modelled here.
measure-arrange = Arrange
measure-arrange-label = Arrange the street as { $code } { $name }
measure-this-street = This street
measure-will-not-fit = Will not fit
measure-set-at-junction = Set at a junction
measure-not-modelled = Not modelled

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
unavailable-freeways-only = Freeways only
unavailable-not-freeway = Not for a freeway
