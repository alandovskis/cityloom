# What an area restriction says (src/restriction, and the map's Restrictions tab: src/map/areas.rs, src/map/areas_view.rs).

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
restriction-check-info = { $rule }: streets carry no speed, so this is for information only.
restriction-check-informational = For information only
