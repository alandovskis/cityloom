# What the city map page says (src/map/vm.rs, src/map/view.rs) and its markup (web/map.html).

## The page's markup
map-page-title = CityLoom city map
map-page-description = See the whole city at once and open any street or junction to change it.
map-brand = CityLoom, the city map
map-editors = Editors
map-nav-map = Map
map-places-toggle-title = Show or hide the places ( [ )
map-places-panel = Places in the city
map-basemap-label = Map of the city, north up
map-does-it-work = Does it work?
map-your-changes = Your changes
map-legend = Key to the map
map-fine-print = The streets are from OpenStreetMap. Every threshold the checks use is a sample for now, not a real rule.
map-about-city = About this city
map-key-search = search
map-key-next = next place
map-key-open = open
map-key-zoom = zoom
map-key-move = move the map
map-key-whole = whole city
map-key-panels = places, notes

## The heading, the tools and the panels
map-city-map = City map
map-sheet-tools = Sheet tools
map-view-tools = Map view
map-zoom-out = Zoom out
map-zoom-in = Zoom in
map-whole-city = Whole city
map-places = Places
map-places-hint = Press a junction to open its plan. Press a street to open its cross-section.
map-junctions = Junctions
map-streets-heading = Streets
map-tb-city = City
map-tb-changed = Changed

## Counting
map-streets = { $n ->
    [one] { $n } street
   *[other] { $n } streets
}
map-junction-count = { $n ->
    [one] { $n } junction
   *[other] { $n } junctions
}
# `junctions` is map-junction-count and `streets` is map-streets.
map-counts = { $junctions }, { $streets }

## A place in a list: its small print, and the tag that says how it stands
# `control` is the junction's control (in English until the junction is translated), `streets` is map-streets.
map-junction-sub = { $control }, { $streets }
map-street-sub = { $ends } · { $width }
map-state-works = Works
map-state-changed = Changed
map-state-bad = Needs attention

## How a place is told to a screen reader, sentence by sentence
map-label-junction = { $name }, { $control }, { $streets }.
map-label-street = { $kind }, { $ends }, { $width } wide.
map-needs-attention = Needs attention: { $failing }.
map-changed = Changed.
map-opens-junction = Opens the junction plan.
map-opens-street = Opens the street cross-section.

## The search
map-search-label = Search places
map-search-go = Search
map-search-results = Places found
map-search-no-places = There are no places to search.
map-search-none = No places match “{ $query }”. Clear the search to see all { $places }.
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
map-checks-pass = Every check passes in all { $n } places.
map-changes-no-places = There are no places to change.
map-changes-some = { $n ->
    [one] { $n } place
   *[other] { $n } places
} changed from the city as first laid out.
map-changes-none = Nothing changed yet. Open a street or a junction to change it.
map-still-works = Still works
map-detail-needs-attention = Needs attention: { $failing }
# A street in the notes: what it is called and where it runs.
map-note-street = { $kind }, { $ends }

## The status line
map-nothing-to-show = No roads to show.
map-status-ok = { $n } places. Every check passes.
map-status-bad = { $n ->
    [one] { $n } place needs
   *[other] { $n } places need
} attention: { $names }.
map-status-bad-more = { $n ->
    [one] { $n } place needs
   *[other] { $n } places need
} attention: { $names } and more.

## Start over
map-reset = Start over
map-reset-armed = Press again to start over
map-reset-armed-said = This puts every street and junction back as first laid out. Press again to confirm.
map-reset-done = The city is back as it was first laid out.

## Where there is no map
map-basemap-missing = The basemap could not be loaded. Build it with `just prepare`, then reload the page.
map-basemap-outside = There is no basemap for this place. The map covers the Montréal area.
map-no-roads = The roads of this place could not be loaded, so there is no map to show.
