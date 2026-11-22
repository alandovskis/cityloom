# What the junction says: its model (src/junction/model/mod.rs, read_model.rs), its announcements
# (src/junction/vm/text.rs), the turns table (src/junction/view/turns.rs) and the page's views.

## How a junction is run
jn-control-uncontrolled = No control
jn-control-priority = Side streets stop
jn-control-stop = All-way stop
jn-control-signal = Traffic signal
jn-control-roundabout = Roundabout

## The way an arm leaves the junction, in a sentence and on a tag
jn-compass-n = north
jn-compass-ne = north-east
jn-compass-e = east
jn-compass-se = south-east
jn-compass-s = south
jn-compass-sw = south-west
jn-compass-w = west
jn-compass-nw = north-west
jn-compass-short-n = N
jn-compass-short-ne = NE
jn-compass-short-e = E
jn-compass-short-se = SE
jn-compass-short-s = S
jn-compass-short-sw = SW
jn-compass-short-w = W
jn-compass-short-nw = NW
# An arm: its street and the way it leaves, "Main Street (north)".
jn-arm-name = { $street } ({ $compass })
jn-arm-to-arm = { $from } to { $to }
# Things said one after another.
jn-list = { $head }; { $tail }
jn-list-comma = { $head }, { $tail }

## The measures an arm can have, by Atlas code
jn-approach-none = None
jn-stop-none = No bus stop
jn-rule-none = No turn management
jn-measure-g1 = Offset queue-jump lane
jn-measure-g2 = Curbside queue-jump lane
jn-measure-g3 = Virtual queue-jump lane
jn-measure-h1 = Signal-controlled bus gate
jn-measure-h2 = Yield-controlled bus gate
jn-measure-m1 = Bus bulb
jn-measure-m2 = Signal-protected on-street platform
jn-measure-l1 = Indirect left turn via alternative itinerary
jn-measure-l2 = Indirect left turn within the intersection
jn-measure-l3 = Right-in/right-out
jn-measure-l4 = Dead-ending of a lateral street

## Why a turn cannot be made
jn-blocked-dead-end = A dead end
jn-blocked-filter = A transit modal filter lets only buses through
jn-blocked-riro-leave = Right-in/right-out: only right turns leave
jn-blocked-riro-enter = Right-in/right-out: only right turns enter
jn-blocked-around = Left turns go round by another street

## Why an edit was refused
jn-refusal-needs-three-streets = A junction needs at least three streets.
jn-refusal-linked-no-remove = The streets here belong to the city, so they cannot be removed.
jn-refusal-roundabout-too-big = The streets are too wide to fit a roundabout.
jn-refusal-bearing-blocked = That is too close to a neighbouring street, or leaves a gap wider than a straight road.
jn-refusal-last-way-out = A street has to keep at least one way out.
jn-refusal-lane-needs-street = A lane has to go to at least one street.
jn-refusal-island-road-too-narrow = This road is too narrow for an island.
jn-refusal-bulb-no-parking = There is no parking on that side to give up.
jn-refusal-does-not-fit = That change does not fit.
jn-refusal-raised-roundabout = A roundabout cannot be a raised table.
jn-refusal-bike-box-not-here = A bike box needs a bike lane on a street where cars enter and stop, at a junction that is not a roundabout.
jn-refusal-protected-not-here = A protected corner needs a bike lane at the curb of each of the two streets, a real corner, room for the island and no roundabout.
jn-refusal-turbo-skewed = A street would be more than 20° off the square a turbo roundabout needs.
jn-refusal-turbo-entry-too-narrow = This street is too narrow to widen for the entry lanes this layout needs.
jn-refusal-turbo-skewed-arm = { $arm } is the one that would be off.
jn-refusal-turbo-entry-too-narrow-arm = { $arm } is the one that is too narrow.
jn-refusal-turbo-lanes-fixed = On a turbo roundabout the layout sets where each lane goes.
jn-refusal-bus-on-turbo = A turbo roundabout has no bus lane across its island.
jn-refusal-queue-jump-on-turbo = A turbo roundabout has no queue-jump lane on its approaches.

## What each change was, in the history
jn-word-add = add
jn-word-remove = remove
jn-rev-remove = Remove { $arm }
jn-rev-bearing = { $street } bearing: { $degrees }°
jn-rev-offset = { $arm } offset: { $length }
jn-rev-corner = Corner after { $arm }: { $length } radius
jn-rev-protected = Corner after { $arm }, protected: { $change }
jn-rev-crossing = { $arm } crossing: { $change }
jn-rev-setback = { $arm } crossing set back { $length }
jn-rev-crossing-width = { $arm } crossing { $length } wide
jn-rev-island = { $arm } refuge island: { $change }
jn-rev-bike-box = { $arm } bike box: { $change }
jn-rev-continuous = { $arm } continuous sidewalk: { $change }
jn-rev-continuous-island = { $arm } continuous sidewalk: { $change }; refuge island removed
jn-rev-bulb-left = { $arm } left bulb-out: { $change }
jn-rev-bulb-right = { $arm } right bulb-out: { $change }
jn-rev-lane-to = { $arm } lane { $lane }: to { $dest }
jn-rev-lane-not-to = { $arm } lane { $lane }: not to { $dest }
jn-rev-bus-lane = { $arm } bus lane: { $change }
jn-rev-measure = { $arm }: { $code } { $measure }
jn-rev-no-approach = { $arm }: no approach measure
jn-rev-no-stop = { $arm }: no bus stop
jn-rev-no-rule = { $arm }: no turn management
jn-rev-approach-len = { $arm } approach measure: { $length }
jn-rev-filter = { $arm } transit modal filter (N1): { $change }
jn-rev-turn-allow = { $from } to { $to }: allow
jn-rev-turn-ban = { $from } to { $to }: no turn
jn-rev-control = Control: { $control }
jn-rev-control-boxes = Control: { $control }; bike boxes removed
jn-rev-control-protected = Control: { $control }; protected corners removed
jn-rev-control-both = Control: { $control }; bike boxes and protected corners removed
jn-rev-bus-across = Bus lane across the middle: { $from } to { $to }
jn-rev-bus-across-remove = Bus lane across the middle: remove
jn-rev-cycle = Cycle track around the roundabout: { $length }
jn-rev-cycle-remove = Cycle track around the roundabout: remove
jn-rev-raised = Raised table: { $change }
jn-rev-ring = Roundabout: { $length } larger
jn-rev-turbo-layout = Roundabout layout: { $layout } turbo
jn-rev-turbo-off = Roundabout: single lane
jn-rev-turbo-size = Turbo roundabout size: { $size }
jn-rev-turbo-main = Main road of the turbo roundabout: { $from } and { $to }

## The checks
jn-check-lanes-cover = Lanes cover every turn
jn-check-lanes-cover-ok = Each allowed turn has a lane
jn-check-lanes-cover-bad = No lane for { $turns }
jn-no-lane-left = { $from } to { $to } (left turn)
jn-no-lane-through = { $from } to { $to } (through turn)
jn-no-lane-right = { $from } to { $to } (right turn)
jn-check-lanes-follow = Lanes follow the turn bans
jn-check-lanes-follow-ok = No lane points at a banned turn
jn-check-lanes-follow-bad = Points at banned turns: { $lanes }
jn-lane-of = { $arm } lane { $n }
jn-check-crossing = Crossing distance
jn-check-crossing-none = No crossings marked
jn-check-crossing-ok = Longest crossing in one go
jn-check-crossing-island = { $arm } is too far to cross in one go. A refuge island in the middle would split it in two
jn-check-crossing-far = { $arm } is too far to cross in one go
jn-check-turning-speed = Slow turns at crossings
jn-check-turning-speed-ok = Turns are slow enough beside every crossing
jn-check-turning-speed-bad = Fast corner after { $arms }
jn-check-corner-room = Sidewalk survives the corner
jn-check-corner-room-ok = Every corner leaves room to stand
jn-check-corner-room-bad = Corner after { $arms } eats the sidewalk
jn-check-transit = Transit measures work
jn-check-transit-ok = Each measure has what it needs
jn-check-approach = Approaches
jn-check-approach-ok = Every end zone reaches past its stop line and its transition fits
jn-check-approach-bad = { $problems }
jn-check-stop-clear = Bus stops clear of the junction
jn-check-stop-clear-ok = Every bus stop stands clear of the junction.
jn-check-stop-clear-bad = { $stops }
jn-check-stop-lane = Buses at a stop are not trapped in a turning lane
jn-check-stop-lane-ok = No bus stop traps a bus in a turning lane.
jn-check-stop-lane-bad = { $stops }
jn-edge-crossing = the crossing
jn-edge-mouth = the edge of the junction
jn-or = { $head } or { $tail }
jn-turn-left = left
jn-turn-through = straight on
jn-turn-right = right
jn-use-left = left
jn-use-through = straight on
jn-use-right = right
jn-stop-too-near = The { $kind } on { $arm } stands { $gap } from { $edge }; { $need } or more is needed.
jn-stop-inside = The { $kind } on { $arm } reaches into the junction.
jn-stop-turn-only = The { $kind } on { $arm } is in lane { $n }, which only turns { $turn }: a bus going on has to change lanes in front of traffic.
jn-stop-shared-turn = The { $kind } on { $arm } is in lane { $n }, shared with { $turn } turns: turning cars queue behind the stopped bus.
jn-problem-stop-twice = { $arm }: the street already has a stop here; keep the { $measure } or the street’s stop, not both.
jn-approach-short = The end zone of { $arm } ends { $reach } from the centre, short of the { $need } its stop line and arrows need.
jn-approach-room = The transition on { $arm } needs { $need } but has { $room }.
jn-approach-drop = A lane on { $arm } ends with no lane beside it to merge into.
jn-problem-offset-parking = { $arm }: an offset queue jump needs parking beside the curb to sit beside
jn-problem-virtual-signal = { $arm }: a virtual queue jump needs a traffic signal
jn-problem-gate-bus-lane = { $arm }: a bus gate lets a bus lane through, so it needs a bus lane
jn-problem-gate-lane = { $arm }: a bus gate needs a traffic lane to hold back
jn-problem-bulb-parking = { $arm }: a bus bulb extends the curb into parking, and there is none
jn-problem-platform-signal = { $arm }: a signal-protected platform needs a traffic signal
jn-problem-platform-crossing = { $arm }: a platform in the street needs a crossing to reach it
jn-problem-filter-bus-lane = { $arm }: a modal filter needs a bus lane for the buses that pass
jn-check-signal = Signal has room
jn-check-signal-ok = Few enough streets for a signal
jn-check-signal-bad = { $n ->
    [one] { $n } street is too many for one signal
   *[other] { $n } streets is too many for one signal
}

jn-check-turbo-square = Arms meet the ring square
jn-check-turbo-square-ok = Every street meets the ring within 10° of square
jn-check-turbo-square-bad = { $arm } is { $degrees }° off square.

## What is said after a change
jn-undone = Undone.
jn-started-over = Started over from the junction as it is today.
jn-arms = { $n ->
    [one] { $n } street
   *[other] { $n } streets
}
jn-fit-ok = { $arms }, { $control }. Every check passes.
jn-fit-bad = { $n ->
    [one] One check needs attention: { $checks }.
   *[other] { $n } checks need attention: { $checks }.
}
jn-edit = { $edit }. { $fit }
jn-sel-bus = Bus lane across the middle
jn-sel-cycle = Cycle track, { $width } wide
jn-sel-lane = Lane { $n } of { $count }, { $arm }
jn-sel-arm = { $arm }, { $degrees ->
    [one] { $degrees } degree
   *[other] { $degrees } degrees
}
jn-sel-arm-approach = { $arm }, { $degrees ->
    [one] { $degrees } degree
   *[other] { $degrees } degrees
}. Its end zone reaches { $reach } from the centre, then a transition of { $length } into { $zone }.
jn-sel-arm-jog = { $arm }, { $degrees ->
    [one] { $degrees } degree
   *[other] { $degrees } degrees
}. Its end zone reaches { $reach } from the centre, then { $zone }.
# The zone across an end zone's boundary, by its number from the street's anchored end (src/street/model/beyond.rs).
zone-unnamed-mid = Zone { $n }
jn-side-left = on the left as seen from the centre
jn-side-right = on the right as seen from the centre
jn-sel-arm-stop = { $kind } { $side }, { $at } from the centre, in lane { $n } of { $of }, which goes { $uses }.
jn-sel-arm-stop-bus = { $kind } { $side }, { $at } from the centre, in the bus lane.
jn-sel-arm-stop-bare = { $kind } { $side }, { $at } from the centre.
jn-sel-corner = Corner after { $arm }, { $radius } radius
jn-sel-corner-protected = Protected corner after { $arm }, { $radius } radius
jn-sel-crossing = Crossing on { $arm }
jn-sel-crossing-continuous = Continuous sidewalk across { $arm }

## The turns table
jn-turns-caption = Turns allowed. Rows are the street traffic comes from, columns the street it goes to.
jn-turns-from = From
jn-turn-cell-left = { $from } to { $to }: left turn
jn-turn-cell-through = { $from } to { $to }: straight on turn
jn-turn-cell-right = { $from } to { $to }: right turn
jn-turn-blocked = { $turn }: not possible. { $why }
jn-turn-banned = { $turn }, not allowed
jn-turn-unserved = { $turn }, allowed, no lane serves it
jn-turn-allowed = { $turn }, allowed

## The page around the plan: heading, window title, title block and history (src/junction/view/page.rs)
jn-title = { $name } · CityLoom
jn-city-map = City map
jn-heading = Junction in 2.5D
jn-block-junction = Junction
jn-block-streets = Streets
jn-block-changes = Changes made
jn-block-osm = OpenStreetMap
jn-block-data = Data
jn-block-edited = Edited
jn-block-imported = As imported
jn-undo = Undo
jn-redo = Redo
jn-reset = Start over

## The notes beside the plan (src/junction/view/notes.rs)
jn-across-caption = How far it is to cross each street, and how many lanes come in
jn-across-street = Street
jn-across-to-cross = To cross
jn-across-lanes-in = Lanes in
jn-across-none = none
jn-conflicts-caption = Points where the paths of allowed turns meet
jn-conflicts-kind = Kind
jn-conflicts-points = Points
jn-conflicts-crossing = Crossing
jn-conflicts-merging = Merging
jn-conflicts-splitting = Splitting
jn-conflicts-all = All
jn-conflicts-signal = A signal takes turns, so paths that cross do not meet at the same time.
jn-conflicts-roundabout = Traffic in a roundabout only merges and splits; it never crosses.
jn-check-passes = : passes
jn-check-fails = : fails
# A check's detail with the length it speaks of: "Longest crossing in one go: 11.4 m".
jn-check-length = { $detail }: { $length }
jn-rev-step = Step { $n }

## The drawing (src/junction/view/drawing.rs, iso/)
jn-editor = Junction editor
jn-iso-label = 2.5D view of the junction. { $arms }.
jn-iso-label-roundabout = 2.5D view of the junction. { $arms }. Roundabout.
jn-iso-label-raised = 2.5D view of the junction. { $arms }. Raised table.
jn-iso-label-turbo = 2.5D view of the junction. { $arms }. { $name }.
jn-svg-bus-only = Bus only
# What the buttons under the drawing step through: the streets, lanes, crossings and corners, as the arrow keys do.
jn-part-previous = Previous part
jn-part-next = Next part

## The page's markup (web/intersection.html); the window title is the header's jn-title
jn-page-description = Change how a junction of streets works: corners, crossings, lanes, turns and control.
jn-page-details = Details
jn-page-details-toggle-title = Show or hide the details ( [ )
jn-page-key-select = select
jn-page-key-change = change
jn-page-key-turn = turn a street
jn-page-key-remove = remove
jn-page-key-details-notes = details, notes
jn-page-key-undo = undo
jn-page-details-panel = Selected street, corner or crossing
jn-page-plan-key = Key to the strips
jn-page-tab-streets = Streets
jn-page-across = Getting across
jn-page-turns = Turns allowed
jn-page-turns-hint = Each row is a street traffic comes from. Press a cell to allow or ban that turn.
jn-page-conflicts = Where paths meet
jn-page-does-it-work = Does it work?
jn-page-your-changes = Your changes
jn-page-fine-print = Sample numbers and limits for now, not real measurements.
jn-page-about = About this junction

## The page of a junction that cannot be drawn (src/junction/view/page.rs: Stuck)
jn-stuck-title = This junction cannot be drawn
jn-stuck-text = The streets that meet here have been changed so that they no longer make a junction. Give the streets their room back, or start the city over from the map.

## The panel beside the plan: what is selected and the controls to change it (src/junction/view/inspector.rs)
jn-insp-empty = Select a street, corner or crossing to change it.
jn-path-label = What the details are of
jn-path-junction = Junction
jn-path-lane = Lane { $n }
jn-insp-junction-sub =
    { $streets ->
        [one] 1 street
       *[other] { $streets } streets
    } · { $control }
jn-insp-streets = Streets
jn-insp-control = Junction control
jn-insp-ring-size = Size of the roundabout
# The tag after the roundabout's size: "m across".
jn-insp-unit-across = { $unit } across
jn-insp-ring-hint = Across the outside. No smaller than { $length } with these streets.
# The buttons either side of a number field, for a screen reader: "Smaller by 0.5 m".
jn-insp-smaller = Smaller by { $step }
jn-insp-larger = Larger by { $step }
jn-insp-narrower = Narrower by { $step }
jn-insp-wider = Wider by { $step }
jn-insp-tighter = Tighter by { $step }
jn-insp-closer = Closer by { $step }
jn-insp-farther = Farther by { $step }
jn-insp-shorter = Shorter by { $step }
jn-insp-longer = Longer by { $step }
jn-insp-shift-left = Shift left by { $step }
jn-insp-shift-right = Shift right by { $step }
jn-insp-anticlockwise = Turn anticlockwise by { $degrees }°
jn-insp-clockwise = Turn clockwise by { $degrees }°
# The lengths a field takes: "1.0 m to 15.0 m".
jn-insp-range = { $min } to { $max }
jn-insp-track-width = Track width
jn-insp-track-hint-inside = { $min } to { $max }. It takes space from the carriageway inside the same circle.
jn-insp-track-hint-crossing = { $min } to { $max }. Cyclists cross every street where it meets the ring.
jn-insp-bus-across = Bus lane through the middle
jn-insp-bus-none = No bus lane
jn-insp-bus-note = A { $width } bus-only lane straight across the island. It crosses the ring where it enters and leaves.
jn-insp-turbo = Roundabout layout
jn-insp-turbo-single = Single lane
jn-insp-turbo-size = Turbo size
jn-insp-turbo-smaller = Next smaller size
jn-insp-turbo-larger = Next larger size
jn-insp-turbo-main = Main road
jn-insp-turbo-main-is = { $from } and { $to } are the main road.
jn-insp-turbo-main-switch = Use the other streets as the main road
jn-insp-turbo-main-swap = Move the two entry lanes to the other street
jn-insp-lane-fixed = The turbo layout sets where this lane goes.
jn-insp-bus-offer = Lets buses cut across the island between two streets.
jn-insp-cycle = Cycle track
jn-insp-cycle-option = Track around the outside
jn-insp-bus = Bus lane
jn-insp-bus-remove = Remove the bus lane
jn-insp-cycle-sub = Round the outside of the roundabout
jn-insp-cycle-remove = Remove the cycle track
jn-insp-raised = Raised table
jn-insp-raised-option = Raise the whole junction to sidewalk height
jn-insp-continuous = Continuous sidewalk
jn-insp-continuous-option = Sidewalk runs on across the street
jn-insp-corner = Corner
jn-insp-curb-radius = Curb radius
jn-insp-corner-hint = { $min } to { $max }. Cars turn here at about { $speed } km/h.
jn-insp-corner-note = A tight corner slows turning cars and shortens the walk across. A wide one lets them swing through faster.
# A lane's turn button, for a screen reader: "Lane 1 to Main Street (south), straight on".
jn-insp-dest-left = Lane { $n } to { $street }, left
jn-insp-dest-through = Lane { $n } to { $street }, straight on
jn-insp-dest-right = Lane { $n } to { $street }, right
jn-insp-lane = Lane { $n }
# Where a lane sits among its street's lanes.
jn-insp-lane-only = the only lane
jn-insp-lane-middle = nearest the middle
jn-insp-lane-curb = nearest the curb
jn-insp-lane-title = Lane { $n } of { $count }
# Under a lane's title: its street and where it sits, "Main Street (north) · nearest the curb".
jn-insp-lane-sub = { $arm } · { $place }
jn-insp-lane-banned = Every street it goes to is banned. Add a street, or allow a turn.
jn-insp-lane-width = { $width } wide. A lane has to go to at least one street.
jn-insp-goes-left = Left to { $street }
jn-insp-goes-through = Straight on to { $street }
jn-insp-goes-right = Right to { $street }
# A street a lane cannot go to, because its traffic only comes in.
jn-insp-one-way-in = { $goes } (one way in)
jn-insp-lane-goes = Where this lane goes
jn-insp-whole-street = Select the whole street
jn-insp-setback = Set back from the junction
jn-insp-crossing-width = Crossing width
jn-insp-bulb-left = Left curb bulge
jn-insp-bulb-right = Right curb bulge
jn-insp-no-parking = { $bulb } (no parking there)
jn-insp-crossing = Crossing
jn-insp-crossing-stages = { $n ->
    [one] { $n } stage of { $length }
   *[other] { $n } stages of { $length }
}
jn-insp-crossing-one-go = { $length } to cross
jn-insp-too-far = { $crossing }. Too far in one go.
jn-insp-crossing-remove = Remove the crossing
jn-insp-crossing-mark = Mark a crossing
jn-insp-halfway = Halfway island
jn-insp-island = Refuge island in the middle
jn-insp-island-narrow = Refuge island (road too narrow)
jn-insp-island-continuous = Refuge island (not on a continuous sidewalk)
jn-insp-bike-box = Bike box
jn-insp-bike-box-option = Waiting box for cyclists ahead of the cars
jn-insp-bike-box-roundabout = Bike box (not at a roundabout)
jn-insp-bike-box-no-lane = Bike box (needs a bike lane on this street)
jn-insp-bike-box-no-entry = Bike box (needs a lane that cars enter by)
jn-insp-bike-box-free = Bike box (cars do not stop on this street)
jn-insp-protected = Protected corner
jn-insp-protected-option = Bike track round a corner island
jn-insp-protected-roundabout = Protected corner (not at a roundabout)
jn-insp-protected-straight = Protected corner (this corner is a straight line)
jn-insp-protected-no-lane = Protected corner (needs a bike lane at the curb of both streets)
jn-insp-protected-no-room = Protected corner (the street is too narrow for the island)
jn-insp-shorten = Shorten the crossing
jn-insp-transit = Transit priority
jn-insp-bus-lane-in = Bus lane along the way in
jn-insp-approach = At the approach
jn-insp-stop = Bus stop
jn-insp-rule = Turns
# A measure in a list, with its Atlas code: "G2 Curbside queue-jump lane".
jn-insp-coded = { $code } { $name }
jn-insp-filter = Transit modal filter
jn-insp-queue-length = Length of the queue jump
jn-insp-gate-distance = Gate distance upstream
# Under a street's name: the way it leaves, its bearing and its width, "E, 90° · 11.4 m road".
jn-insp-arm-sub = { $compass }, { $degrees }° · { $width } road
jn-insp-direction = Direction
jn-insp-direction-hint = Clockwise from north. At least { $degrees }° from its neighbours.
jn-insp-shift = Shift sideways
jn-fold-none = None set
jn-fold-bus-lane = Bus lane
jn-fold-position = Position
jn-fold-position-set = { $degrees }°, { $shift }
jn-fold-not-shifted = not shifted
jn-fold-shifted = shifted { $length }
jn-insp-shift-hint = Up to { $length } either way. Looking out from the junction.
jn-insp-street = Street
jn-insp-cross-section = Open the cross-section
jn-insp-street-note = This street belongs to the city. Its layout is edited in the street editor, and changes there show here.
jn-insp-remove-street = Remove this street
jn-insp-min-arms = { $n ->
    [one] A junction needs at least { $n } street.
   *[other] A junction needs at least { $n } streets.
}
jn-insp-lanes-in = Lanes coming in
jn-insp-one-way-out = One way out. No lanes come in.

# A pedestrian street at a junction (src/junction/model/mod.rs, src/junction/view/inspector.rs).
jn-refusal-crossing-pedestrian = A pedestrian street has no roadway to cross, so it takes no crossing.
jn-refusal-measure-pedestrian = A pedestrian street carries no traffic, so it takes no traffic measures.
jn-insp-crossing-pedestrian = A pedestrian street has no roadway to cross.
jn-insp-lanes-pedestrian = A pedestrian street has no lanes.
jn-insp-control-pedestrian = { $n ->
    [0] No street here carries traffic.
   *[other] Only one street here carries traffic.
}

jn-refusal-not-signal = Signal timing is only set at a junction run by a traffic signal.
jn-refusal-timing-out-of-range = That time is outside what a signal can use.
jn-refusal-no-transit-lane = An early green for transit needs a transit lane on this street.
jn-timing-green = Green
jn-timing-yellow = Yellow
jn-timing-all-red = All-red
jn-timing-transit-lead = Transit early green
jn-rev-timing = { $arm }: { $field } { $seconds } s
jn-rev-control-timing = Control: { $control }; signal timing removed
jn-rev-control-boxes-timing = Control: { $control }; bike boxes and signal timing removed
jn-rev-control-protected-timing = Control: { $control }; protected corners and signal timing removed
jn-rev-control-all = Control: { $control }; bike boxes, protected corners and signal timing removed

jn-check-signal-walk = Time to cross
jn-check-signal-walk-ok = Every crossing is walked long enough
jn-check-signal-walk-bad = Not enough time to cross at { $arms }
jn-check-signal-cycle = Signal cycle
jn-check-signal-cycle-ok = The cycle is { $seconds } s
jn-check-signal-cycle-bad = The cycle is { $seconds } s, over the { $max } s a signal should keep

jn-insp-signal = Signal timing
jn-sig-cycle = Cycle: { $seconds } s
jn-path-plan = Signal plan
jn-sig-summary =
    Cycle of { $seconds } s · { $phases ->
        [one] 1 phase
       *[other] { $phases } phases
    }
jn-sig-row = { $street }: green { $green } s, yellow { $yellow } s, all-red { $all_red } s
jn-sig-row-lead = { $street }: early green { $lead } s, green { $green } s, yellow { $yellow } s, all-red { $all_red } s
jn-sig-walks = Also walks: crossings on { $arms }.
jn-sig-rides = Also rides: bikes from { $arms }.
jn-sig-alone = Nothing else runs during this phase.
jn-sig-no-phase = Nothing enters from this street, so it has no green of its own.
jn-sig-shorter = Shorter by { $seconds } s
jn-sig-longer = Longer by { $seconds } s
jn-sig-range = { $min } to { $max } s

## Turbo roundabouts: the layouts and the sizes (src/junction/model/turbo.rs)
jn-turbo-basic = Basic
jn-turbo-egg = Egg
jn-turbo-knee = Knee
jn-turbo-spiral = Spiral
jn-turbo-rotor = Rotor
jn-turbo-size-mini = Mini
jn-turbo-size-regular = Regular
jn-turbo-size-medium = Medium
jn-turbo-size-large = Large
# What a drawing of the junction is, for a screen reader: "Basic turbo roundabout, regular size".
jn-turbo-label-basic = Basic turbo roundabout, { $size } size
jn-turbo-label-egg = Egg turbo roundabout, { $size } size
jn-turbo-label-knee = Knee turbo roundabout, { $size } size
jn-turbo-label-spiral = Spiral turbo roundabout, { $size } size
jn-turbo-label-rotor = Rotor turbo roundabout, { $size } size

jn-refusal-phase-serves-nothing = Nothing enters from that street, so there is no phase to add for it.
jn-refusal-last-phase = A signal keeps at least one phase.
jn-refusal-phase-at-end = That phase cannot move any further that way.
jn-refusal-plan-suggested = The signal plan is already the suggested one.
jn-rev-phase-add = Phase { $n } added for { $arm }
jn-rev-phase-remove = Phase { $n } for { $arm } removed
jn-rev-phase-earlier = Phase for { $arm } moved earlier, now { $n } of { $total }
jn-rev-phase-later = Phase for { $arm } moved later, now { $n } of { $total }
jn-rev-phase-restore = Signal plan restored to the suggestion
jn-check-signal-phases = Every street has a phase
jn-check-signal-phases-ok = Every street that traffic enters from has a phase
jn-check-signal-phases-bad = No phase serves the traffic entering from { $arms }
jn-sig-suggested = Suggested plan
jn-sig-edited = Edited plan
jn-sig-add = Add a phase for { $street }
jn-sig-earlier = Move the phase for { $street } earlier
jn-sig-later = Move the phase for { $street } later
jn-sig-remove = Remove the phase for { $street }
jn-sig-restore = Restore the suggested plan
jn-sig-phases-cut =
    { $n ->
        [one] { $n } phase beyond the { $max } a plan can hold was left out as the junction opened.
       *[other] { $n } phases beyond the { $max } a plan can hold were left out as the junction opened.
    }
jn-sig-earlier-short = Earlier
jn-sig-later-short = Later
jn-sig-remove-short = Remove

## The signal plan as drawings, one per phase
jn-ph-name = Phase { $n }: { $streets }
jn-ph-and = { $head } and { $tail }
jn-ph-cars = Cars go: { $arms }.
jn-ph-walks = People cross: { $arms }.
jn-ph-rides = Bikes go: { $arms }.
jn-ph-join = { $head } { $tail }
jn-ph-row = The phases of the signal, in the order they run
jn-ph-timing = Green { $green } s, yellow { $yellow } s, all-red { $all_red } s
jn-ph-timing-lead = Early green { $lead } s, green { $green } s, yellow { $yellow } s, all-red { $all_red } s
jn-rev-plan-lost = { $change }; edited signal plan removed
jn-refusal-too-many-phases = The plan has as many phases as it can hold.

## Ready-made patterns for a phase (src/junction/model/signal.rs)
jn-pattern-through = Through only
jn-pattern-through-desc = Only the traffic going straight on moves; the crossings beside it are walked.
jn-pattern-all = All movements
jn-pattern-all-desc = Every movement of the street goes; turning drivers give way to the people crossing.
# "Protected-left" is the turn across oncoming traffic (a right turn where traffic keeps left); the key is kept for stability.
jn-pattern-protected-left = Protected turn across traffic, then through
jn-pattern-protected-left-desc = The turn across oncoming traffic (left when driving on the right, right when driving on the left) goes first with no one crossing where it turns, then the other movements of the street.
jn-pattern-scramble = Pedestrian scramble
jn-pattern-scramble-desc = No vehicle moves and people cross every street.
jn-pattern-custom = Custom
jn-pattern-custom-desc = The movements of this phase match none of the ready-made patterns.
jn-pattern-group = Pattern for phase { $n }, { $street }
jn-rev-phase-pattern = Phase { $n } for { $arm } set to { $pattern }
jn-refusal-pattern-not-fit = That pattern does not fit this junction's lanes.
jn-refusal-pattern-already = The phase is that pattern already.
jn-pattern-lost = Phase { $n } no longer follows a ready-made pattern.
jn-pattern-lost-many = Phases { $list } no longer follow a ready-made pattern.
jn-rev-phase-walk-on = Phase { $n } for { $arm }: people now walk across { $street }
jn-rev-phase-walk-off = Phase { $n } for { $arm }: people no longer walk across { $street }
jn-rev-phase-ride-on = Phase { $n } for { $arm }: bikes now go on { $street }
jn-rev-phase-ride-off = Phase { $n } for { $arm }: bikes no longer go on { $street }
jn-refusal-no-crossing = That street has no pedestrian crossing.
jn-refusal-no-bike-lane = That street has no bike lane.
jn-refusal-scramble-walks = A scramble walks every crossing there is; choose another pattern to set them one by one.
jn-refusal-movement-already = The phase is set that way already.
jn-ph-walks-group = People walking in phase { $n }, { $street }
jn-ph-rides-group = Bikes in phase { $n }, { $street }
jn-ph-walks-legend = People walking
jn-ph-rides-legend = Bikes
jn-ph-walk-box = People walk across { $other }
jn-ph-ride-box = Bikes go on { $other }
jn-ph-walks-fixed = This phase is a pedestrian scramble: everyone crosses every street. Choose another pattern to set crossings one by one.

## Pedestrian calls (src/junction/model/calls.rs, #60): a crossing on call is walked only when a pedestrian calls it
jn-refusal-call-not-walked = No phase of the plan walks this crossing, so it cannot be on call: walk it in a phase first.
jn-refusal-call-already = The crossing is set that way already.
jn-refusal-not-on-call = Only a crossing on call has calls an hour.
jn-refusal-call-rate-out-of-range = That many calls an hour is outside the placeholder bound shown.
jn-rev-call-on = Crossing of { $arm } on call
jn-rev-call-off = Crossing of { $arm } back on recall
jn-rev-calls-lost =
    { $change }; { $n ->
        [one] 1 crossing no longer on call
       *[other] { $n } crossings no longer on call
    }
jn-rev-call-rate =
    Crossing of { $arm }: { $rate ->
        [one] 1 call an hour
       *[other] { $rate } calls an hour
    }
jn-calls-heading = Pedestrian calls
jn-calls-intro = A crossing on call is walked only when a pedestrian calls it, by a push button or passive detection. One on recall is walked every time its phases run, and calls cannot change its service.
jn-calls-bounds = Placeholder figures: walk { $walk } s, then clearance at { $speed } m/s, rounded up to the second; a green is extended by at most { $max } s; with passive detection a pedestrian leaves after waiting { $limit } s; the calls are spread over one hour.
jn-calls-group = Crossing of { $street }
jn-calls-on-call = On call
jn-calls-rate = Calls an hour
jn-calls-rate-range = { $min } to { $max } calls an hour (placeholder bound)
jn-calls-recall = On recall: walked every time its phases run; calls cannot change its service.
jn-calls-need = On call: walk { $walk } s + clearance { $clearance } s = { $need } s of green.
jn-calls-unwalked = No phase walks this crossing, so it cannot be on call.
jn-calls-none = No crossing is on call: every crossing is walked every time its phases run.
jn-calls-dropped =
    { $n ->
        [one] { $n } crossing was taken off call as the junction opened: it is not run by a signal.
       *[other] { $n } crossings were taken off call as the junction opened: it is not run by a signal.
    }
jn-calls-kept =
    { $n ->
        [one] 1 crossing stays on call for when the signal runs again.
       *[other] { $n } crossings stay on call for when the signal runs again.
    }
jn-calls-totals-caption = Calls in one hour, by crossing
jn-calls-col-crossing = Crossing
jn-calls-col-calls = Calls
jn-calls-col-served = Served
jn-calls-col-delayed = Delayed
jn-calls-col-extended = Green extended
jn-calls-col-not-served = Not served
jn-calls-col-longest = Longest wait
jn-calls-col-at = Called at
jn-calls-col-outcome = Outcome
jn-calls-seconds = { $seconds } s
jn-calls-no-wait = –
jn-calls-no-wait-said = none served
jn-calls-rows =
    { $n ->
        [one] The call, one row
       *[other] Every call, one row each ({ $n })
    }
jn-calls-rows-caption = Each call of the hour, in the order made
jn-calls-at = { $minutes } min { $seconds } s
jn-calls-outcome-served = Served after { $wait } s
jn-calls-outcome-served-extended = Served after { $wait } s, green extended by { $extended } s
jn-calls-outcome-delayed = Delayed { $wait } s
jn-calls-outcome-delayed-extended = Delayed { $wait } s, green extended by { $extended } s
jn-calls-outcome-not-served = Not served
jn-calls-findings = Findings
jn-calls-finding-cannot = The crossing of { $arm } cannot be served safely: its walk and clearance need { $need } s of green, and no phase that walks it can give more than { $most } s.
jn-calls-finding-unwalked = The crossing of { $arm } is on call, but no phase walks it, so its calls are not served. Walk it in a phase, or put it back on recall.
jn-calls-said =
    Crossing of { $arm }: { $calls ->
        [one] 1 call
       *[other] { $calls } calls
    } in the hour; served { $served }, delayed { $delayed }, green extended { $extended }, not served { $not_served }; longest wait { $wait } s.
jn-calls-said-no-wait =
    Crossing of { $arm }: { $calls ->
        [one] 1 call
       *[other] { $calls } calls
    } in the hour; served { $served }, delayed { $delayed }, green extended { $extended }, not served { $not_served }; no call was served.

## Passive pedestrian detection (src/junction/model/calls.rs, CLM-17): kerbside detection places a call and cancels it when the pedestrian leaves
jn-refusal-detection-not-walked = No phase of the plan walks this crossing, so passive detection cannot serve it: walk it in a phase first.
jn-refusal-detection-needs-call = Only a crossing on call detects pedestrians: put it on call first.
jn-rev-call-passive = Crossing of { $arm }: passive detection
jn-rev-call-push-button = Crossing of { $arm }: push button
jn-calls-detection = Detection
jn-calls-detection-push-button = Push button
jn-calls-detection-passive = Passive detection
jn-calls-detection-push-button-help = A pedestrian pushes the button to call the walk; the call stays placed after they leave, so the walk is given even to an empty crossing.
jn-calls-detection-passive-help = Radar, infrared or video at the curb places the call while a pedestrian waits; one who leaves after waiting { $limit } s cancels it, and a walk with nobody left waiting is skipped. Walk and clearance are never shortened.
jn-calls-col-cancelled = Cancelled
jn-calls-col-skipped = Walks skipped
jn-calls-outcome-cancelled = Cancelled: left after waiting { $wait } s
jn-calls-said-passive =
    Passive detection: { $cancelled ->
        [one] 1 call cancelled
       *[other] { $cancelled } calls cancelled
    }, { $skipped ->
        [one] 1 walk skipped
       *[other] { $skipped } walks skipped
    }.

## Warnings: movements of a phase that meet (src/junction/model/signal.rs `conflicts`)
jn-conflict-move = cars from the { $from } side going { $turn }
jn-conflict-vehicles = Phase { $n }: { $a } cross { $b }.
jn-conflict-walk = Phase { $n }: { $mv } meet people walking across on the { $side } side.
jn-conflict-ride = Phase { $n }: { $mv } cross bikes riding on the { $side } side.
jn-conflict-list = Warnings for phase { $n }
jn-conflict-heard = Warning: { $conflict }
jn-conflict-heard-many = Warning: { $count } movements now meet, in { $phases ->
    [one] phase
   *[other] phases
} { $list }. { $first }
jn-conflict-more = { $n ->
    [one] And { $n } more meeting.
   *[other] And { $n } more meetings.
}
jn-check-signal-conflicts = Movements that meet
jn-check-signal-conflicts-ok = No phase puts movements together that meet
jn-check-signal-conflicts-bad = { $list }
jn-check-curb-near = Curbside clear of crossings
jn-check-curb-near-ok = No parking or loading stands within { $need } of a crossing.
jn-check-curb-near-bad = { $uses }
jn-curb-near = The { $use } on { $arm } stands { $gap } from the crossing; { $need } or more is needed.
jn-curb-near-inside = The { $use } on { $arm } reaches the crossing.
jn-sel-arm-curb = { $use } { $side }, { $at } from the centre.

# What differs between two designs of a junction (src/junction/model/compare.rs).
cmp-jn-control = Control: { $a } → { $b }
cmp-jn-turbo = Turbo layout: { $a } → { $b }
cmp-jn-turbo-none = none
cmp-jn-entering = { $street }: lanes coming in, { $a } → { $b }
cmp-jn-leaving = { $street }: lanes going out, { $a } → { $b }
cmp-jn-crossing-added = { $street }: a crossing is added
cmp-jn-crossing-removed = { $street }: the crossing is removed
cmp-jn-bike-box-added = { $street }: a bike box is added
cmp-jn-bike-box-removed = { $street }: the bike box is removed
cmp-jn-phases = Signal phases: { $a } → { $b }
cmp-jn-cycle = Signal cycle: { $a } s → { $b } s
cmp-other-arm-details = details of the streets at the junction

## Vehicle actuation (src/junction/model/actuation.rs, CLM-12): an actuated street's phase is called and extended by the vehicles detected on its approach
jn-refusal-actuation-no-phase = No phase of the plan is this street's own, so it cannot be actuated: give it a phase first.
jn-refusal-actuation-already = The street is set that way already.
jn-refusal-not-actuated = Only an actuated street has a minimum green, a gap, a maximum green and vehicles an hour: make it actuated first.
jn-refusal-actuation-out-of-range = That value is outside the placeholder bound shown.
jn-refusal-min-above-max = The minimum green cannot be longer than the maximum green.
jn-rev-actuated-on = { $arm }: actuated
jn-rev-actuated-off = { $arm } back on recall
jn-rev-actuation = { $arm }: { $field } { $value } s
jn-rev-actuation-vehicles =
    { $arm }: { $value ->
        [one] 1 vehicle an hour
       *[other] { $value } vehicles an hour
    }
jn-act-min-green = Minimum green
jn-act-gap = Gap
jn-act-max-green = Maximum green
jn-act-vehicles = Vehicles an hour
jn-act-heading = Vehicle actuation
jn-act-intro = An actuated street's phase is called by the vehicles detected on its approach: skipped when none is waiting, extended while they keep coming, and ended early at the first gap. A street on recall has its phase every cycle at its green.
jn-act-bounds = Placeholder figures: a street made actuated has a minimum green of { $min } s, a gap of { $gap } s, a maximum green of { $max } s and { $vehicles } vehicles an hour, spread over one hour. A walk being served holds the green as long as it needs; the yellow and all-red are never shortened.
jn-act-group = Phase of { $street }
jn-act-mode = Phase
jn-act-mode-recall = On recall
jn-act-mode-actuated = Actuated
jn-act-recall = Phase on recall: runs every cycle at its green, whatever the traffic.
jn-act-trigger = Actuated: vehicles detected on its approach call its phase, which is skipped when none is waiting. Its green runs at least { $min } s, each vehicle holds it { $gap } s more, up to { $max } s.
jn-act-floor = A walk on recall in its phase needs { $floor } s of green, so its green runs at least that.
jn-act-no-phase = No phase of the plan is this street's own, so it cannot be actuated.
jn-act-effect =
    In the hour its green was given { $greens } times, skipped { $skipped }, extended { $extended } times by { $extended_s } s in all, and ended early { $early } times.
jn-act-range-seconds = { $min } to { $max } s (placeholder bound)
jn-act-range-vehicles = { $min } to { $max } vehicles an hour (placeholder bound)
jn-act-unit-seconds = s
jn-act-unit-vehicles = veh./h
jn-act-less = { $field }, { $step } less
jn-act-more = { $field }, { $step } more
jn-act-none = No street is actuated: every phase runs every cycle at its green.
jn-act-phases-caption = Actuated phases in one hour
jn-act-approaches-caption = Vehicles in one hour, by approach
jn-act-col-phase = Phase
jn-act-col-skipped = Skipped
jn-act-col-extended = Extended
jn-act-col-extended-by = Extended by
jn-act-col-ended-early = Ended early
jn-act-col-approach = Approach
jn-act-col-vehicles = Vehicles
jn-act-col-average = Average wait
jn-act-no-wait-said = no vehicle served
jn-act-finding-no-phase = { $arm } is actuated, but no phase of the plan is its own, so its vehicles call nothing. Give it a phase, or put it back on recall.
jn-act-finding-walk-past-max = The walk across { $crossing } needs { $need } s of green, longer than the { $max } s maximum green of { $arm }: the walk holds the green past the maximum whenever it is served.
jn-act-said =
    { $arm }: { $vehicles ->
        [one] 1 vehicle
       *[other] { $vehicles } vehicles
    } in the hour; served { $served }, not served { $not_served }; average wait { $average } s, longest { $longest } s. Its green: given { $greens }, skipped { $skipped }, extended { $extended } by { $extended_s } s in all, ended early { $early }.
jn-act-said-no-wait =
    { $arm }: { $vehicles ->
        [one] 1 vehicle
       *[other] { $vehicles } vehicles
    } in the hour; served { $served }, not served { $not_served }; no vehicle was served. Its green: given { $greens }, skipped { $skipped }, extended { $extended } by { $extended_s } s in all, ended early { $early }.

## Bicycle detection (src/junction/model/bicycles.rs, CLM-13): a bike movement on detection is served when a cyclist is detected, with a bicycle minimum green and clearance
jn-refusal-bike-not-ridden = No phase of the plan runs this street's bike movement, so it cannot be on detection: let its bikes go in a phase first.
jn-refusal-bike-detection-already = The bike movement is set that way already.
jn-refusal-not-on-bike-detection = Only a bike movement on detection has cyclists an hour: put it on detection first.
jn-refusal-cyclists-out-of-range = That many cyclists an hour is outside the placeholder bound shown.
jn-rev-bike-detection-on = Bikes from { $arm }: on detection
jn-rev-bike-detection-off = Bikes from { $arm } back on recall
jn-rev-cyclists =
    Bikes from { $arm }: { $rate ->
        [one] 1 cyclist an hour
       *[other] { $rate } cyclists an hour
    }
jn-bikes-heading = Bicycle detection
jn-bikes-intro = A bike movement on detection runs only when a cyclist is detected on its street's approach, and the phase that serves the call gives the cyclist a minimum green and a clearance to cross. One on recall runs with every phase that lets it go, and cyclists cannot change its service.
jn-bikes-bounds = Placeholder figures: a bike movement put on detection has { $rate } cyclists an hour, spread over one hour. Each call needs a minimum green of { $min } s, then a clearance at { $speed } m/s over the ride across the junction, rounded up to the second; a green is extended by at most { $max } s. When a pedestrian call needs more in the same phase, the longer is kept: neither clearance is shortened, nor the yellow and all-red. A cyclist calls an actuated phase, and a call being served holds its green past its maximum.
jn-bikes-group = Bikes from { $street }
jn-bikes-group-to = Bikes from { $street } to { $to }
jn-bikes-mode = Bikes
jn-bikes-mode-recall = On recall
jn-bikes-mode-detection = On detection
jn-bikes-recall = On recall: runs with every phase that lets it go; cyclists cannot change its service.
jn-bikes-trigger = On detection: a cyclist detected on the approach calls the movement, and the next phase that runs it serves the call. Each call needs a minimum green of { $min } s + a clearance of { $clearance } s over { $length } = { $need } s of green; a phase's green is extended by at most { $max } s for it.
jn-bikes-not-ridden = No phase runs this street's bike movement, so it cannot be on detection.
jn-bikes-rate = Cyclists an hour
jn-bikes-rate-range = { $min } to { $max } cyclists an hour (placeholder bound)
jn-bikes-effect =
    In the hour { $calls ->
        [one] 1 call
       *[other] { $calls } calls
    }: served { $served }, delayed { $delayed }, green extended { $extended } times by { $extended_s } s in all, not served { $not_served }.
jn-bikes-none = No bike movement is on detection: every one runs with every phase that lets it go.
jn-bikes-totals-caption = Cyclists' calls in one hour, by bike movement
jn-bikes-col-movement = Bike movement
jn-bikes-col-extended = Extended
jn-bikes-extended-cell = { $count } ({ $seconds } s)
jn-bikes-no-wait-said = no cyclist served
jn-bikes-rows =
    { $n ->
        [one] The cyclist's call, one row
       *[other] Every cyclist's call, one row each ({ $n })
    }
jn-bikes-rows-caption = Each cyclist's call of the hour, in the order made
jn-bikes-finding-cannot = The bikes from { $arm } cannot be served safely: their minimum green and clearance need { $need } s of green, and no phase that runs them can give more than { $most } s.
jn-bikes-finding-not-ridden = The bikes from { $arm } are on detection, but no phase runs them, so their calls are not served. Let them go in a phase, or put them back on recall.
jn-bikes-finding-past-max = The bikes from { $arm } need { $need } s of green, longer than the { $max } s maximum green of { $phase }: a call served there holds the green past the maximum.
jn-bikes-said =
    Bikes from { $arm }: { $calls ->
        [one] 1 call
       *[other] { $calls } calls
    } in the hour; served { $served }, delayed { $delayed }, green extended { $extended } by { $extended_s } s in all, not served { $not_served }; longest wait { $wait } s.
jn-bikes-said-no-wait =
    Bikes from { $arm }: { $calls ->
        [one] 1 call
       *[other] { $calls } calls
    } in the hour; served { $served }, delayed { $delayed }, green extended { $extended } by { $extended_s } s in all, not served { $not_served }; no call was served.

## Transit signal priority (src/junction/model/transit.rs, CLM-14): a street's buses are given an early green, a green extension or both, within bounds
jn-refusal-priority-no-transit = This street has no transit, so its buses cannot be given priority: give it a bus lane, a queue jump, a bus gate, a bus stop or a transit filter first.
jn-refusal-priority-no-phase = No phase of the plan is this street's own, so its buses cannot be given priority: give it a phase first.
jn-refusal-not-on-priority = Only a street whose buses have priority has buses an hour: give them priority first.
jn-refusal-buses-out-of-range = That many buses an hour is outside the placeholder bound shown.
jn-rev-priority-early-green = Buses on { $arm }: early green
jn-rev-priority-extension = Buses on { $arm }: green extension
jn-rev-priority-both = Buses on { $arm }: early green and green extension
jn-rev-priority-off = Buses on { $arm }: no priority
jn-rev-buses =
    Buses on { $arm }: { $rate ->
        [one] 1 bus an hour
       *[other] { $rate } buses an hour
    }
jn-transit-heading = Transit signal priority
jn-transit-intro = A street with transit and a phase of its own can give its buses priority. An early green cuts short the phases before its own when a bus comes during red; a green extension holds its green for a bus coming just after it would end. Without priority a bus waits for its street's green like other traffic.
jn-transit-bounds = Placeholder figures: a street given priority has { $rate } buses an hour, spread over one hour. An early green brings the street's green forward by at most { $early } s, and an extension holds it by at most { $extension } s; a street given priority for one green gets none for the next. Yellow and all-red are never shortened, nor a phase's minimum green, a walk and its clearance or a bicycle clearance: an early green they keep shorter is a finding. The street's transit early green (its timing) stays a fixed lead every cycle, whatever comes.
jn-transit-group = Buses on { $street }
jn-transit-mode = Priority
jn-transit-mode-off = None
jn-transit-mode-early-green = Early green
jn-transit-mode-extension = Green extension
jn-transit-mode-both = Both
jn-transit-off = No priority: a bus waits for the street's green like other traffic.
jn-transit-trigger-early-green = Early green: a bus coming while the street is red cuts short the greens of the phases before its own, by at most { $early } s in all, never below their minimum green, a walk and its clearance or a bicycle clearance. Not for two greens running.
jn-transit-trigger-extension = Green extension: a bus coming within { $extension } s after the street's green would end holds the green until it has passed. Not for two greens running.
jn-transit-trigger-both = Early green and green extension: a bus coming during red brings the street's green forward by at most { $early } s, never cutting a phase below its minimum green, a walk and its clearance or a bicycle clearance; a bus coming within { $extension } s after the green would end holds it until it has passed. Not for two greens running.
jn-transit-no-transit = This street has no transit, so its buses cannot be given priority.
jn-transit-no-phase = No phase of the plan is this street's own, so its buses cannot be given priority.
jn-transit-rate = Buses an hour
jn-transit-rate-range = { $min } to { $max } buses an hour (placeholder bound)
jn-transit-effect =
    In the hour { $buses ->
        [one] 1 bus
       *[other] { $buses } buses
    }: on arrival { $on_arrival }, early green { $early }, extended { $extended }, delayed { $delayed }, not served { $not_served }; delay moved to other approaches { $moved } s.
jn-transit-moved = Delay moved to other approaches: { $list }.
jn-transit-moved-item = { $arm } { $seconds } s
jn-transit-totals-caption = Buses in one hour, by street
jn-transit-col-street = Street
jn-transit-col-buses = Buses
jn-transit-col-on-arrival = On arrival
jn-transit-col-early-green = Early green
jn-transit-col-extended = Extended
jn-transit-col-delayed = Delayed
jn-transit-col-moved = Delay moved to others
jn-transit-rows =
    { $n ->
        [one] The bus, one row
       *[other] Every bus, one row each ({ $n })
    }
jn-transit-rows-caption = Each bus of the hour, in the order they came
jn-transit-col-bus = Bus
jn-transit-col-arrived = Arrived
jn-transit-col-wait = Wait
jn-transit-outcome-on-arrival = On arrival
jn-transit-outcome-early-green = Early green ({ $early } s sooner)
jn-transit-outcome-extended = Extended (held { $extended } s)
jn-transit-outcome-delayed = Delayed
jn-transit-no-wait-said = no bus served
jn-transit-finding-no-transit = The buses on { $arm } have priority, but the street has no transit any more, so they are given none. Give it transit again, or take the priority off.
jn-transit-finding-no-phase = The buses on { $arm } have priority, but no phase of the plan is the street's own, so they are given none. Give it a phase, or take the priority off.
jn-transit-finding-clearance =
    An early green for the buses on { $arm } was kept shorter { $times ->
        [one] once
       *[other] { $times } times
    }, by { $held } s in all, in the phase of { $phase }: it was serving a walk and its clearance or a bicycle clearance, which are never shortened.
jn-transit-said =
    Buses on { $arm }: { $buses ->
        [one] 1 bus
       *[other] { $buses } buses
    } in the hour; on arrival { $on_arrival }, early green { $early }, extended { $extended }, delayed { $delayed }, not served { $not_served }; longest wait { $wait } s; delay moved to other approaches { $moved } s.
jn-transit-said-no-wait =
    Buses on { $arm }: { $buses ->
        [one] 1 bus
       *[other] { $buses } buses
    } in the hour; on arrival { $on_arrival }, early green { $early }, extended { $extended }, delayed { $delayed }, not served { $not_served }; no bus was served.
jn-transit-none = No street gives its buses priority: each bus waits for its street's green like other traffic.

## Emergency vehicle pre-emption (src/junction/model/preemption.rs, CLM-15): an emergency call takes the signal to its street's phase after a safe clearance
jn-refusal-preemption-no-phase = No phase of the plan is this street's own, so its emergency calls cannot pre-empt the signal: give it a phase first.
jn-refusal-not-on-preemption = Only a street with pre-emption on has emergency calls an hour: turn pre-emption on first.
jn-refusal-emergency-calls-out-of-range = That many emergency calls an hour is outside the placeholder bound shown.
jn-rev-preemption-on = Emergency vehicles on { $arm }: pre-emption on
jn-rev-preemption-off = Emergency vehicles on { $arm }: pre-emption off
jn-rev-emergency-calls =
    Emergency vehicles on { $arm }: { $rate ->
        [one] 1 emergency call an hour
       *[other] { $rate } emergency calls an hour
    }
jn-preempt-heading = Emergency vehicle pre-emption
jn-preempt-intro = A street with a phase of its own can let its emergency vehicles pre-empt the signal. Pre-emption has the highest priority of all, above pedestrian and bicycle calls, vehicle actuation and transit priority: an emergency call ends the green running after a safe clearance and gives the street its green at once. With pre-emption off an emergency vehicle waits for its street's green like other traffic.
jn-preempt-bounds = Placeholder figures: a street given pre-emption has { $rate } emergency calls an hour, spread over one hour, and its green then runs at least { $dwell } s for the vehicle, which is taken to have passed by then. A walk still showing is cut short, but a pedestrian or bicycle clearance under way, the yellow and the all-red always run in full. After the emergency green the plan goes on with the phase after the one interrupted. Calls are served in the order they come: one coming during another street's pre-emption waits for it to end.
jn-preempt-group = Emergency vehicles on { $street }
jn-preempt-mode = Pre-emption
jn-preempt-mode-off = Off
jn-preempt-mode-on = On
jn-preempt-off = Off: an emergency vehicle waits for the street's green like other traffic.
jn-preempt-trigger = On, the highest priority: an emergency call on this street ends the green running and gives the street its green, above every other policy. A walk still showing is cut short, but a pedestrian or bicycle clearance under way, the yellow and the all-red run in full first, so the green comes at most { $entry } s after the call (the safe entry). It then runs at least { $dwell } s, and the plan goes on with the phase after the one interrupted.
jn-preempt-no-phase = No phase of the plan is this street's own, so its emergency calls cannot pre-empt the signal.
jn-preempt-rate = Emergency calls an hour
jn-preempt-rate-range = { $min } to { $max } emergency calls an hour (placeholder bound)
jn-preempt-effect =
    In the hour { $calls ->
        [one] 1 emergency call
       *[other] { $calls } emergency calls
    }: longest time to green { $longest } s; pre-empted { $preempted }; delayed { $delayed }.
jn-preempt-effect-no-call = In the hour no emergency call was served.
jn-preempt-totals-caption = Emergency calls in one hour, by street
jn-preempt-col-street = Street
jn-preempt-col-calls = Calls
jn-preempt-col-longest = Longest time to green
jn-preempt-col-preempted = Pre-empted
jn-preempt-col-delayed = Delayed
jn-preempt-rows =
    { $n ->
        [one] The emergency call, one row
       *[other] Every emergency call, one row each ({ $n })
    }
jn-preempt-rows-caption = Each emergency call of the hour, in the order they came
jn-preempt-col-call = Call
jn-preempt-col-arrived = Arrived
jn-preempt-col-to-green = Time to green
jn-preempt-cut = Green of { $street } cut short: { $list }
jn-preempt-cut-none = No green cut short
jn-preempt-not-served = Not served
jn-preempt-pedestrians =
    { $n ->
        [one] 1 pedestrian
       *[other] { $n } pedestrians
    }
jn-preempt-cyclists =
    { $n ->
        [one] 1 cyclist
       *[other] { $n } cyclists
    }
jn-preempt-buses =
    { $n ->
        [one] 1 bus
       *[other] { $n } buses
    }
jn-preempt-vehicles =
    { $n ->
        [one] 1 vehicle
       *[other] { $n } vehicles
    }
jn-preempt-no-call-said = no emergency call served
jn-preempt-nobody = nobody
jn-preempt-finding-no-phase = { $arm } has pre-emption on, but no phase of the plan is its own, so its emergency calls take the signal nowhere. Give it a phase, or turn pre-emption off.
jn-preempt-said =
    Emergency vehicles on { $arm }: { $calls ->
        [one] 1 emergency call
       *[other] { $calls } emergency calls
    } in the hour; longest time to green { $longest } s, safe entry { $entry } s; pre-empted { $preempted }; delayed { $delayed }.
jn-preempt-said-no-call =
    Emergency vehicles on { $arm }: { $calls ->
        [one] 1 emergency call
       *[other] { $calls } emergency calls
    } in the hour; no call was served.
jn-preempt-none = No street has pre-emption on: an emergency vehicle waits for its street's green like other traffic.
## Time-of-day plans (CLM-16)
jn-plan-default = Default plan
jn-day-weekday = Weekdays (Monday to Friday)
jn-day-saturday = Saturday
jn-day-sunday = Sunday
jn-day-on-weekday = on weekdays
jn-day-on-saturday = on Saturdays
jn-day-on-sunday = on Sundays
jn-rev-in-plan = { $change }, in plan “{ $plan }”
jn-rev-plan-add = Add plan “{ $name }”
jn-rev-plan-rename = Rename plan “{ $from }” to “{ $to }”
jn-rev-plan-remove =
    { $n ->
        [0] Remove plan “{ $name }”
        [one] Remove plan “{ $name }”: its period runs the default plan
       *[other] Remove plan “{ $name }”: its { $n } periods run the default plan
    }
jn-rev-period-add = “{ $plan }” from { $time } { $day }
jn-rev-period-remove = Remove the period from { $time } { $day }
jn-rev-period-plan = Period from { $time } { $day } runs “{ $plan }”
jn-rev-period-start = Period from { $time } { $day } now starts at { $to }
jn-refusal-plan-name-empty = A plan needs a name.
jn-refusal-plan-name-too-long = A plan's name can have at most 40 characters.
jn-refusal-plan-name-taken = Another plan already has that name.
jn-refusal-too-many-plans = A junction holds at most 8 plans, the default included.
jn-refusal-default-plan-kept = The default plan cannot be removed: it runs whenever no other plan is scheduled.
jn-refusal-default-plan-named = The default plan keeps its name.
jn-refusal-period-out-of-day = A period starts after 00:00 and before midnight. To change what runs from 00:00, choose the plan of the day's first period.
jn-refusal-period-taken = Another period already starts at that time on that day.
jn-refusal-too-many-periods = A day has at most 12 periods, the one from 00:00 included.
jn-refusal-first-period-kept = The day's first period starts at 00:00 and stays: choose its plan instead.
jn-plans-heading = Time-of-day plans
jn-plans-intro = A signal can run other plans at other times of the week. Each plan has its own timing and its own response settings (pedestrian calls, vehicle actuation, bicycle detection, transit priority, pre-emption); the phases, their order and their patterns are the same in every plan. Whatever the schedule leaves runs the default plan.
jn-plans-shown = Plan shown and edited
jn-plans-shown-help = The signal plan below, its summary of the hour and its checks, and each street's timing are those of this plan: a timing or response setting changed is changed in it alone.
jn-plans-shown-said = Showing plan “{ $plan }”: its timing and response settings are shown and edited.
jn-plans-viewing = Timing and response settings of plan “{ $plan }”.
jn-plans-bounds = Up to { $plans } plans with the default, names of up to { $chars } characters, up to { $periods } periods a day (placeholder bounds).
jn-plans-list = Plans
jn-plans-default-mark = default, cannot be removed
jn-plans-cycle = Cycle of { $seconds } s
jn-plans-sound = Nothing found
jn-plans-findings =
    { $n ->
        [one] 1 finding
       *[other] { $n } findings
    }
jn-plans-findings-of = Findings of plan “{ $plan }”
jn-plans-name-of = Name of plan “{ $plan }”
jn-plans-remove = Remove
jn-plans-remove-of = Remove plan “{ $plan }”
jn-plans-new-name = Name of a new plan
jn-plans-add = Add plan
jn-plans-schedule = Weekly schedule
jn-plans-col-start = Starts
jn-plans-col-end = Ends
jn-plans-col-plan = Plan
jn-plans-col-change = Change
jn-plans-midnight = 24:00
jn-period-start-of = Start of the period from { $time } { $day }
jn-period-plan-of = Plan of the period from { $time } { $day }
jn-period-remove-of = Remove the period from { $time } { $day }
jn-period-first = Starts the day
jn-period-new-start = Start of a new period { $day }
jn-period-new-plan = Plan of a new period { $day }
jn-period-new = New period { $day }
jn-period-add = Add period
jn-period-add-of = Add the new period { $day }
jn-plans-runs-heading = Which plan runs
jn-plans-runs-day = Day
jn-plans-runs-time = Time
jn-plans-runs-at = At { $time } { $day }, plan “{ $plan }” runs.
jn-plans-transition = A change of plan takes effect at the end of the cycle running when its period starts: there is no transition period.
