# What an area restriction says (src/restriction, and the map's Area rules panel: src/map/areas.rs, src/map/areas_view.rs).

## Refusals of an outline
restriction-refused-too-few = An area needs at least three corners, not all in a line.
restriction-refused-crossing = The outline crosses itself. Move or remove a corner.

# Said when a corner is added where one already is (the last, or the first).
restriction-refused-duplicate = A corner is already there. Move the map, then add the next one.

## The rules
restriction-rule-car-free = Car-free
restriction-rule-bus-and-bike = Bus and bike only
restriction-rule-speed-limit = Speed limit of { $kmh } km/h
# The speed limit among the choices of a rule, before its speed is chosen.
restriction-rule-speed-limit-choice = Speed limit
restriction-rule-no-parking = No parking or loading
# An area's name: the resident's own words, said as they are.
restriction-named = { $name }

## What a street has that a rule does not allow
restriction-conflict-travel = { $n ->
    [one] It has a driving lane.
   *[other] It has { $n } driving lanes.
}
restriction-conflict-parking = { $n ->
    [one] It has a parking lane.
   *[other] It has { $n } parking lanes.
}
restriction-conflict-loading = { $n ->
    [one] It has a loading zone.
   *[other] It has { $n } loading zones.
}

## A street's check against an area it is inside
# `area` is the area's name (restriction-named, or its rule's name).
restriction-check = Area rule: { $area }
# `rule` is a restriction-rule-* message.
restriction-check-keeps = { $rule }: the street keeps to it.
restriction-check-breaks = { $rule }: the street does not keep to it.
restriction-check-speed-over = Speed limit { $street } km/h, over the area’s { $limit } km/h.
restriction-check-speed-unknown = Speed not known: OpenStreetMap gives none and none is set.

## Drawing an outline, said as it happens (`area` is the area's name)
restriction-started = Drawing an area. Click the map, or press Enter on it, to place the first corner.
restriction-redraw-started = Redrawing “{ $area }”. Place its corners again.
restriction-corner-added = Corner { $n } placed.
restriction-corner-removed = { $n ->
    [one] Corner removed. One corner left.
   *[other] Corner removed. { $n } corners left.
}
restriction-corner-moved = Corner moved.
# A corner dropped on an outline whose corners another tab changed meanwhile.
restriction-corner-stale = The outline was changed in another tab, so the corner was not moved.
restriction-closed = The outline is closed. Choose its rule and name, then save it.
restriction-cancelled = Drawing cancelled.
restriction-corners = { $n ->
    [one] { $n } corner
   *[other] { $n } corners
}
# A speed among the choices of a speed limit.
restriction-kmh = { $kmh } km/h

## Keeping and changing an area (`rule` is a restriction-rule-* message)
restriction-saved = “{ $area }” saved.
restriction-not-kept = The area rules could not be saved in this browser.
restriction-selected = “{ $area }” selected: { $rule }.
# An area with no name of its own: `area` is its rule's name, said once.
restriction-selected-unnamed = “{ $area }” selected.
restriction-renamed = Renamed “{ $area }”.
restriction-rule-changed = The rule of “{ $area }” is now: { $rule }.
# The same for an area with no name of its own, which is called by its rule.
restriction-rule-changed-unnamed = The area’s rule is now: { $rule }.
restriction-redrawn = “{ $area }” redrawn.
restriction-deleted = “{ $area }” deleted.
restriction-restored = “{ $area }” restored.

## The tab's lead
restriction-future = These area rules were saved by a newer version of CityLoom, so they cannot be shown or changed here.
restriction-lead-none = No area rules yet. Declare one to check the streets inside it.
restriction-lead-some = { $n ->
    [one] One area rule.
   *[other] { $n } area rules.
} Select one to see the streets inside it.
# Said in the tab where the page has no map to draw an area on.
restriction-lead-no-map = An area is drawn on the map, and there is no map here.

## An area in the tab, and the streets inside it
restriction-summary-none = No loaded street is inside.
restriction-summary = { $inside ->
    [one] One street inside
   *[other] { $inside } streets inside
}; { $conflicts ->
    [0] none conflicts with the rule.
    [one] one conflicts with the rule.
   *[other] { $conflicts } conflict with the rule.
}
# `rule` is a restriction-rule-* message, `summary` one of the three above.
restriction-row-sub = { $rule } · { $summary }
restriction-street-keeps = Keeps to the rule.
restriction-legend-conflict = Conflicts with an area rule

## The panel (web/map.html and src/map/areas_view.rs)
restriction-panel = Area rules
restriction-toggle-title = Show or hide the area rules ( [ )
restriction-heading = Rules over an area
restriction-declare = Declare an area
restriction-drawing-help = Click the map to place each corner, then click the first corner or press Finish. From the keyboard, move the map with the arrow keys, press Enter on it to place a corner at its centre, and Shift+Enter to finish. Backspace removes the last corner; Escape cancels while the map has focus.
restriction-naming-help = Choose the rule that applies inside the area, and name it if you like.
restriction-finish = Finish
restriction-cancel = Cancel
restriction-save = Save
restriction-name-label = Name
restriction-rule-label = Rule
restriction-speed-label = Speed limit
restriction-redraw = Redraw
restriction-delete = Delete
restriction-undo = Undo
restriction-streets-heading = Streets inside
