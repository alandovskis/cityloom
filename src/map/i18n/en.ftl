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
    [one] The network checks found 1 thing across the streets and junctions.
   *[other] The network checks found { $n } things across the streets and junctions.
}
net-note = These look across the whole network and are counted apart from the checks on each place.
net-count-none = no findings
net-count-one = 1 finding
net-count = { $n } findings
net-rule-cycling = Cycling links
net-rule-access = Pedestrian street access
net-rule-crossings = Crossings
net-rule-lanes = Lane counts
net-rule-transit = Transit corridors
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
net-cycling-ends = The bike lane stops here: no other street that meets at this place has one.
net-access-broken = This pedestrian street has no unbroken run of sidewalk { $need } wide for emergency vehicles.
net-crossing-missing = This street takes traffic into the junction and has no pedestrian crossing.
net-crossing-wide = The pedestrian crossing here is { $width } wide, more than the { $max } it should not pass.
net-lanes-change = The traffic lanes change from { $from } to { $to } where these two streets join, more than { $step } at once.
net-transit-break = The transit lane stops here: no other street that meets at this place has one.
