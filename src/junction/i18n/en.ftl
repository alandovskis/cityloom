# What the junction says: its model (src/junction/model.rs, read_model.rs), its announcements
# (src/junction/text.rs), the turns table (src/junction/turns.rs) and the page's views.

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

## The page around the plan: heading, window title, title block and history (src/junction/page.rs)
jn-title = { $name } · CityLoom
jn-city-map = City map
jn-plan = Junction plan
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

## The notes beside the plan (src/junction/notes.rs)
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
jn-col-step = Step
jn-col-what-changed = What changed

## The plan (src/junction/plan.rs, plan_svg.rs)
jn-plan-editor = Junction plan editor
# The choice over the drawing between the flat plan, which is edited, and the 2.5D picture, which is looked at.
jn-view-group = View
jn-view-plan = Plan
jn-view-iso = 2.5D
jn-view-shown-plan = Showing the plan.
jn-view-shown-iso = Showing the junction in 2.5D.
jn-iso-label = 2.5D view of the junction. { $arms }.
jn-iso-label-turbo = 2.5D view of the junction. { $arms }. { $name }.
jn-plan-label = Plan of the junction, north up. { $arms }.
jn-plan-label-roundabout = Plan of the junction, north up. { $arms }. Roundabout.
jn-plan-label-raised = Plan of the junction, north up. { $arms }. Raised table.
jn-plan-label-turbo = Plan of the junction, north up. { $arms }. { $name }.
jn-svg-scale = Scale
jn-svg-bus-only = Bus only
# Under a street's name: the way it leaves and its width, "N · 20.0 m road".
jn-svg-road = { $compass } · { $width } road
jn-svg-road-shifted = { $compass } · { $width } road · shifted { $offset }
jn-grip-turn = Turn { $arm }
jn-grip-corner = Change the corner radius
jn-grip-crossing = Move the crossing

## The page's markup (web/intersection.html); the window title is the header's jn-title
jn-page-description = Change how a junction of streets works: corners, crossings, lanes, turns and control.
jn-page-editors = Editors
jn-page-map = Map
jn-page-details = Details
jn-page-details-toggle-title = Show or hide the details ( [ )
jn-page-key-select = select
jn-page-key-change = change
jn-page-key-turn = turn a street
jn-page-key-remove = remove
jn-page-key-details-notes = details, notes
jn-page-key-view = plan, 2.5D
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
jn-provenance-heading = Where this junction came from
jn-no-source = This junction was not made from OpenStreetMap.

## The page of a junction that cannot be drawn (src/junction/page.rs: Stuck)
jn-stuck-title = This junction cannot be drawn
jn-stuck-text = The streets that meet here have been changed so that they no longer make a junction. Give the streets their room back, or start the city over from the map.

## The panel beside the plan: what is selected and the controls to change it (src/junction/inspector.rs)
jn-insp-empty = Select a street, corner or crossing to change it.
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

# A pedestrian street at a junction (src/junction/model.rs, inspector.rs).
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
jn-sig-row = { $street }: green { $green } s, yellow { $yellow } s, all-red { $all_red } s
jn-sig-row-lead = { $street }: early green { $lead } s, green { $green } s, yellow { $yellow } s, all-red { $all_red } s
jn-sig-walks = Also walks: crossings on { $arms }.
jn-sig-rides = Also rides: bikes from { $arms }.
jn-sig-alone = Nothing else runs during this phase.
jn-sig-no-phase = Nothing enters from this street, so it has no green of its own.
jn-sig-shorter = Shorter by { $seconds } s
jn-sig-longer = Longer by { $seconds } s
jn-sig-range = { $min } to { $max } s

## Turbo roundabouts: the layouts and the sizes (src/junction/turbo.rs)
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

## Ready-made patterns for a phase (src/junction/signal.rs)
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
jn-ph-walk-box = Phase { $n }, { $street }: people walk across { $other }
jn-ph-ride-box = Phase { $n }, { $street }: bikes go on { $other }
