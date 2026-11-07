# What the city map page says (src/map/vm.rs, src/map/view.rs) and its markup (web/map.html).

## The page's markup
map-page-title = CityLoom city map
map-page-description = See the whole city at once and open any street or junction to change it.
map-brand = CityLoom, the city map
map-editors = Editors
map-nav-map = Map
map-basemap-label = Map of the city, north up
map-does-it-work = Does it work?
map-your-changes = Your changes
map-legend = Key to the map
map-fine-print = The streets are from OpenStreetMap. Every threshold the checks use is a sample for now, not a real rule.
map-about-city = About this city
map-key-search = search
map-key-open = open a place found
map-key-zoom = zoom
map-key-move = move the map
map-key-whole = whole city
map-key-areas = area rules
map-key-panels = notes

## The heading, the tools and the panels
map-city-map = City map
map-view-tools = Map view
map-zoom-out = Zoom out
map-zoom-in = Zoom in
map-whole-city = Whole city
map-places = Places
map-junctions = Junctions
map-streets-heading = Streets
map-tb-city = City
map-tb-changed = Changed

## Counting
map-streets = { $n ->
    [one] { $n } street
   *[other] { $n } streets
}
map-curb = { $n ->
    [one] { $n } curbside use
   *[other] { $n } curbside uses
}
map-stops = { $n ->
    [one] { $n } stop
   *[other] { $n } stops
}
map-programme = { $curb }, { $stops }
map-zones = { $n ->
    [one] { $n } zone
   *[other] { $n } zones
}
map-junction-count = { $n ->
    [one] { $n } junction
   *[other] { $n } junctions
}
# `junctions` is map-junction-count and `streets` is map-streets.
map-counts = { $junctions }, { $streets }

## A place in a list: its small print, and the tag that says how it stands
# `control` is the junction's control (a jn-control-* message), `streets` is map-streets.
map-junction-sub = { $control }, { $streets }
map-street-sub = { $ends } · { $width }
map-street-sub-zones = { $ends } · { $width } · { $zones }
map-street-sub-programme = { $ends } · { $width } · { $programme }
map-street-sub-zones-programme = { $ends } · { $width } · { $zones } · { $programme }
map-state-works = Works
map-state-changed = Changed
map-state-bad = Needs attention

## How a place is told to a screen reader, sentence by sentence
map-label-junction = { $name }, { $control }, { $streets }.
map-label-street = { $street }, { $ends }, { $width } wide.
map-label-street-zones = { $street }, { $ends }, { $width } wide, { $zones }.
map-label-street-programme = { $street }, { $ends }, { $width } wide, { $programme }.
map-label-street-zones-programme = { $street }, { $ends }, { $width } wide, { $zones }, { $programme }.
map-needs-attention = Needs attention: { $failing }.
map-changed = Changed.
map-opens-junction = Opens the junction plan.
map-opens-street = Opens the street cross-section.
map-not-editable = Not all of this place is in view. Move the map so all of it shows, then try again.

## The search
map-search-label = Search places
map-search-go = Search
map-search-results = Places found
map-search-no-places = There are no places to search.
map-search-none = No places match “{ $query }”. Clear the search to see { $places ->
    [one] the one place
   *[other] all { $places }
}.
map-search-found = { $n ->
    [one] { $n } place matches
   *[other] { $n } places match
}
map-search-found-more = { $n ->
    [one] { $n } place matches
   *[other] { $n } places match
} · showing the first { $shown }

## The notes
map-checks-no-places = There are no places to check.
map-checks-failing = { $n ->
    [one] { $n } place needs
   *[other] { $n } places need
} attention. Open one to see what is wrong and fix it.
map-checks-pass = { $n ->
    [one] Every check passes in the one place.
   *[other] Every check passes in all { $n } places.
}
map-checks-conflicts = { $streets ->
    [one] One street conflicts
   *[other] { $streets } streets conflict
} with an area rule. The Area rules panel lists { $streets ->
    [one] it
   *[other] them
}.
map-changes-no-places = There are no places to change.
map-changes-some = { $n ->
    [one] { $n } place
   *[other] { $n } places
} changed from the city as first laid out.
map-changes-none = Nothing changed yet. Open a street or a junction to change it.
map-still-works = Still works
map-detail-needs-attention = Needs attention: { $failing }
# A street in the notes: what it is called and where it runs.
map-note-street = { $street }, { $ends }

## The status line
map-nothing-to-show = No roads to show.
map-status-ok = { $n ->
    [one] { $n } place. Every check passes.
   *[other] { $n } places. Every check passes.
}
map-status-conflicts = { $n ->
    [one] { $n } place.
   *[other] { $n } places.
} { $streets ->
    [one] One street conflicts
   *[other] { $streets } streets conflict
} with an area rule.
map-status-bad = { $n ->
    [one] { $n } place needs
   *[other] { $n } places need
} attention: { $names }.
map-status-bad-more = { $n ->
    [one] { $n } place needs
   *[other] { $n } places need
} attention: { $names } and more.

## Start over

## The home page's markup (web/index.html)
map-home-title = CityLoom: redesign the streets of your city
map-home-description = Pick any street or junction of the city, rearrange it, and see at once what still works.
map-home-heading = Redesign the streets of your city.
map-home-lead = Find a place, pick a street or junction on its map, and rearrange it. You see straight away what still works.
map-home-open-map = Open the city map
# The credit is three pieces around a link: map-home-streets-by, then the link map-home-osm-contributors, then a full stop in the markup.
map-home-streets-by = Streets ©
map-home-osm-contributors = OpenStreetMap contributors
map-home-placeholders = The widths and rules the checks use are placeholders.

## The home page's hero (src/map/home.rs, src/map/svg.rs)
# `city` is the city's name (data), `counts` is map-counts.
map-hero-facts = { $city }: { $counts }.
# The name of a changed street on the drawing; `name` is the street's.
map-svg-name-changed = { $name } · changed
# The tag under a changed junction on the drawing.
map-svg-changed = changed

## Where there is no map
map-basemap-missing = The basemap could not be loaded. Build it with `just prepare`, then reload the page.
map-basemap-outside = There is no basemap for this place. The map covers the Montréal area.
map-no-roads = The roads of this place could not be loaded, so there is no map to show.

## Network checks (src/map/network.rs, src/map/vm.rs)
net-heading = Network checks
net-no-places = There is no network to check.
net-lead-none = The network checks found nothing across the streets and junctions.
net-lead = { $n ->
    [one] The network checks found 1 finding across the streets and junctions.
   *[other] The network checks found { $n } findings across the streets and junctions.
}
net-note = These look across the whole network and are counted apart from the checks on each place. A problem means the network breaks the rule; a notice is worth a look, since the rule may be met another way. The rules judge the streets as they stand now, not against how they were first laid out.
net-count-none = no findings
net-count-one = 1 finding
net-count = { $n } findings
net-rule-cycling = Cycling links
net-rule-access = Pedestrian street access
net-rule-crossings = Crossings
net-rule-lanes = Lane counts
net-rule-transit = Transit corridors
net-rule-walking = Walking routes
net-severity-problem = Problem
net-severity-notice = Notice
net-lanes = { $n ->
    [one] { $n } lane
   *[other] { $n } lanes
}
net-cycling-island = { $n ->
    [one] This street has a bike lane that is cut off from the rest of the cycling network.
   *[other] These { $n } streets have bike lanes that are cut off from the rest of the cycling network.
}
net-cycling-ends = { $at }: the bike lane of { $street } stops, since no other street that meets there has one.
net-access-broken = This pedestrian street has no unbroken run of sidewalk { $need } wide for emergency vehicles, as it stands now.
net-crossing-missing = This street takes traffic into the junction and has no pedestrian crossing.
net-crossing-wide = The pedestrian crossing here is { $width } wide, wider than the { $max } maximum.
net-lanes-change = { $a } has { $from } where it joins { $b }, which has { $to }: a change of more than { $step } at once.
net-transit-break = { $at }: the transit lane of { $street } stops, since no other street that meets there has one.
net-walking-ends = { $at }: the sidewalk of { $street } stops, since no other street that meets there has one.
net-walking-island = { $n ->
    [one] People cannot walk from this street to the rest of the network: a freeway, a street with no sidewalk or an arm with no pedestrian crossing cuts it off.
   *[other] People cannot walk from these { $n } streets to the rest of the network: a freeway, a street with no sidewalk or an arm with no pedestrian crossing cuts them off.
}
net-walking-block = { $street } is { $length } long between its ends, longer than the { $max } maximum for a block.
net-unreadable = { $n ->
    [one] 1 junction could not be read, so its crossings are not checked.
   *[other] { $n } junctions could not be read, so their crossings are not checked.
}
net-more-findings = { $n ->
    [one] 1 more finding is not listed.
   *[other] { $n } more findings are not listed.
}
net-more-places = { $n ->
    [one] and 1 more place
   *[other] and { $n } more places
}

## Source data, the Provenance tab (src/map/source_quality.rs, src/map/vm.rs)
prov-heading = Source data
prov-note = These show where the OpenStreetMap data behind the map may be missing or wrong. They are not design failures: they are counted apart from the checks on each place and from the network checks, and a street with a finding here can still pass every rule. The importer cannot tell a width the road is tagged with from the one it assumes, so a width finding may be only the default.
prov-no-places = There is no source data to inspect.
prov-lead-none = Nothing looks missing or wrong in the source data.
prov-lead = { $n ->
    [one] 1 place in the source data may be missing or wrong.
   *[other] { $n } places in the source data may be missing or wrong.
}
prov-count-none = nothing found
prov-count-one = 1 possible gap
prov-count = { $n } possible gaps
prov-label-source = Source:
prov-label-impact = Design impact:
prov-kind-name = Street names
prov-kind-disconnected = Disconnected streets
prov-kind-ends = Dead ends
prov-kind-width = Widths
prov-kind-speed = Speed limits
prov-name = The name of { $street } is not in OpenStreetMap, so the map gives it a name of its own.
prov-source-name = The road has no name tag.
prov-impact-name = Saved edits and the notes name the street by what the map made up, and a street of the same sort could be taken for another.
prov-disconnected = { $n ->
    [one] This street is joined to no other street in the loaded area.
   *[other] These { $n } streets are joined to each other and to no other street in the loaded area.
}
prov-source-disconnected = The roads share no node with the rest of the network, and none of them reaches the edge of the area. The roads may be mapped without a shared node, or the other roads may be missing.
prov-impact-disconnected = Network checks on walking, cycling and transit routes will count them as cut off, which may be a gap in the data and not in the design.
prov-ends = { $street } ends at a node that no other street reaches.
prov-source-ends = The road stops at its own node. It may be a real dead end, or the road that continues it may be missing or not joined.
prov-impact-ends = A junction may be missing here, so the crossings and lanes the design checks expect would not be drawn.
prov-width-row-narrow = { $street } is { $width } wide as imported, narrower than the { $limit } a street is taken to need.
prov-width-row-wide = { $street } is { $width } wide as imported, wider than the { $limit } a street is taken to need.
prov-width-lane-narrow = A piece of { $street } ({ $kind }) is at the least width the catalogue allows, { $limit }.
prov-width-lane-wide = A piece of { $street } ({ $kind }) is at the most width the catalogue allows, { $limit }.
prov-source-width = The importer cannot tell a width the road is tagged with from the one osm2streets assumes, and it keeps each piece within the catalogue's limits, so a piece at a limit may have been cut to it.
prov-impact-width = Width and capacity checks, and the room left for other pieces, rest on this width.
prov-speed = { $street } has no speed limit in OpenStreetMap.
prov-source-speed = The road has no maxspeed tag, or one that could not be read.
prov-impact-speed = The speed shown for the street is Not set, and the Speed limit rule of an area cannot judge it.
prov-more-findings = { $n ->
    [one] 1 more finding is not listed.
   *[other] { $n } more findings are not listed.
}
