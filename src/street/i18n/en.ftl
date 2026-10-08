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
check-passes = { " " }: passes
check-fails = { " " }: fails
badge-fail = { " " }fail
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
