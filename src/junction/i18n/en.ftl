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

## What each change was, in the history
jn-word-add = add
jn-word-remove = remove
jn-rev-remove = Remove { $arm }
jn-rev-bearing = { $street } bearing: { $degrees }°
jn-rev-offset = { $arm } offset: { $length }
jn-rev-corner = Corner after { $arm }: { $length } radius
jn-rev-crossing = { $arm } crossing: { $change }
jn-rev-setback = { $arm } crossing set back { $length }
jn-rev-crossing-width = { $arm } crossing { $length } wide
jn-rev-island = { $arm } refuge island: { $change }
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
jn-rev-bus-across = Bus lane across the middle: { $from } to { $to }
jn-rev-bus-across-remove = Bus lane across the middle: remove
jn-rev-cycle = Cycle track around the roundabout: { $length }
jn-rev-cycle-remove = Cycle track around the roundabout: remove
jn-rev-ring = Roundabout: { $length } larger

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
jn-sel-corner = Corner after { $arm }, { $radius } radius
jn-sel-crossing = Crossing on { $arm }

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
jn-measure-in-use = In use on { $arms }
jn-measure-on-street = Set on a street
jn-measure-street-editor = Street editor
jn-measure-not-modelled = Not modelled
jn-col-step = Step
jn-col-what-changed = What changed
jn-today = Junction today

## The plan (src/junction/plan.rs, plan_svg.rs)
jn-plan-editor = Junction plan editor
jn-plan-label = Plan of the junction, north up. { $arms }.
jn-plan-label-roundabout = Plan of the junction, north up. { $arms }. Roundabout.
jn-svg-scale = Scale
jn-svg-bus-only = Bus only
# Under a street's name: the way it leaves and its width, "N · 20.0 m road".
jn-svg-road = { $compass } · { $width } road
jn-svg-road-shifted = { $compass } · { $width } road · shifted { $offset }
jn-grip-turn = Turn { $arm }
jn-grip-corner = Change the corner radius
jn-grip-crossing = Move the crossing
