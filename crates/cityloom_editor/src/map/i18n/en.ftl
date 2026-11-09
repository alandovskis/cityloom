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
map-key-select = click to select a street
map-key-select-found = select a place found
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
map-report-title = Design review: { $area }
map-report-cover-title = Design review report
map-report-when = { $date } { $hours }:{ $minutes }
map-report-exported = Exported: { $when } UTC
map-report-version = CityLoom version { $version }
map-report-scope = Scope: Whole city
map-report-study-notice = This report comes from a design study. It is not an engineering document and does not replace the review of a qualified professional.
map-report-placeholder-notice = The rules, rates and thresholds behind the checks are placeholders and have not been verified.
map-report-summary = Summary
map-report-summary-caption = Summary of the city and its checks
map-report-col-measure = Measure
map-report-col-figure = Figure
map-report-col-result = Result
map-report-row-city = Streets and junctions in the city
map-report-row-edited = Places changed from the imported layout
map-report-row-failing = Places whose design checks fail
map-report-row-problems = Network problems
map-report-row-notices = Network notices
map-report-figure-city = { $streets ->
    [one] 1 street
   *[other] { $streets } streets
}, { $junctions ->
    [one] 1 junction
   *[other] { $junctions } junctions
}
map-report-pass = Pass
map-report-fail = Fail
map-report-notice = Notice
map-report-none-found = No network problems and no notices were found: { $result }.
map-report-nothing = There is nothing to report: this city has no streets or junctions.
map-report-failed = The report could not be made. Try again.
map-report-dismiss = Dismiss

# The pages of the changed places in the report
map-report-place-before = Before
map-report-place-after = After
map-report-place-alt = { $which }: { $what }
map-report-place-omitted = Some small marks are not drawn in this figure.
map-report-place-zones = This street has { $zones } zones. The figures show it drawn at its first zone.
map-report-place-street-missing = This street could not be read, so there is nothing more to show here.
map-report-place-width = Width: { $before } before, { $after } after.
map-report-place-class = Class: { $before } before, { $after } after.
map-report-place-speed = Speed limit: { $before } before, { $after } after.
map-report-place-pieces = Pieces: { $added ->
    [one] 1 added
   *[other] { $added } added
}, { $removed ->
    [one] 1 removed
   *[other] { $removed } removed
}.
map-report-place-other = The change is to details this page does not summarise, such as zones, curbside uses or stops. The figures and the checks show the result.
map-report-place-width-caption = Width by mode, before and after
map-report-place-col-mode = Mode
map-report-place-share = { $length } ({ $pct } %)
map-report-place-checks-intro = Before is the layout as imported from OpenStreetMap; After is the layout as changed.
map-report-place-na = n/a
map-report-place-checks-caption = Design checks that fail before or after the change
map-report-place-col-check = Check
map-report-place-col-detail = Detail
map-report-place-all-pass = { $n ->
    [one] The only check passes
   *[other] All { $n } checks pass
}: { $result }.
map-report-place-more = { $n ->
    [one] 1 more failing check
   *[other] { $n } more failing checks
}
map-report-place-control = Control: { $before } before, { $after } after.
map-report-place-control-same = Control: { $after } (unchanged).
map-report-place-junction-undrawn = This junction could not be drawn, before or after the change, so this page has no figures.
map-report-place-junction-failing = Checks that fail: { $failing }.
map-report-place-left-out = The report shows the first { $shown } changed places. { $left ->
    [one] 1 more changed place is not shown.
   *[other] { $left } more changed places are not shown.
}
map-report-sources = Sources
map-report-sources-intro = These are the rules that name a source. Naming a source records where a formula was taken from; the figures have not been checked against it.
map-report-sources-placeholders = Every other rule and recommendation in CityLoom is a placeholder of its own, with no source, until an authoritative one replaces it.
map-report-sources-continued = Sources (continued)
map-report-source-line = { $status }. { $applies }

## The report's review pages and appendix (src/map/report_review.rs)
map-report-network = Network effects
map-report-network-caption = The network checks and their results
map-report-col-check = Check
map-report-col-findings = Findings
map-report-col-severity = Severity
map-report-col-finding = Finding
map-report-col-places = Places
map-report-places-and-more = { $names } { $n ->
    [one] and 1 more place
   *[other] and { $n } more places
}
map-report-areas = Area rules
map-report-areas-none = No area rules have been declared for this city.
map-report-areas-intro = An area rule is a traffic rule drawn over part of the city. Each street inside an area is checked against its rule.
map-report-areas-caption = Area rules and the streets inside them
map-report-col-area = Area
map-report-col-rule = Rule
map-report-col-inside = Streets inside
map-report-col-conflicts = In conflict
map-report-areas-streets-caption = Streets that do not keep their area's rule
map-report-col-street = Street
map-report-col-why = Why
map-report-areas-more = { $n ->
    [one] 1 more street is not listed.
   *[other] { $n } more streets are not listed.
}
map-report-labelled = { $label } { $text }
map-report-limits = Limits of the OpenStreetMap data
map-report-col-source = Source
map-report-col-impact = Design impact
map-report-appendix = Appendix
map-report-appendix-thresholds = Thresholds and their status
map-report-appendix-thresholds-status = No threshold in this report has been checked against the standard of any city or country. A Pass or a Fail says how the design stands against a stand-in figure, not whether it complies with anything.
map-report-appendix-glossary = Glossary
map-report-appendix-glossary-caption = The words this report uses
map-report-col-term = Term
map-report-col-meaning = Meaning
map-report-term-before-after = Before and After
map-report-term-network = Network check
map-report-term-area = Area rule
map-report-term-placeholder = Placeholder
map-report-def-pass = The design meets the check.
map-report-def-fail = The design does not meet the check.
map-report-def-problem = The network breaks a rule that looks across many streets.
map-report-def-notice = Worth a look: the rule may be met some other way, or there is too little data to say.
map-report-def-before-after = Before is the layout as imported from OpenStreetMap; After is the layout as changed.
map-report-def-network = A check that looks across the whole network of streets and junctions, not at one place.
map-report-def-area = A traffic rule drawn over part of the city, such as car-free or a speed limit.
map-report-def-placeholder = A figure that stands in for a real one until an authoritative source replaces it.
map-report-appendix-credits = Credits
map-report-credits-osm = The streets, lanes and junctions come from OpenStreetMap, © OpenStreetMap contributors, available under the Open Database Licence (opendatacommons.org/licenses/odbl). osm2streets (Apache License 2.0) turns the map data into streets and junctions.
map-report-credits-atlas = The transit priority measures follow the toolbox of the Transit Priority Atlas, published by the Marron Institute. CityLoom is not affiliated with or endorsed by the Atlas or the Marron Institute.
map-report-credits-software = This report is written with krilla (MIT and Apache 2.0 licences) and set in Noto Sans, © The Noto Project Authors, under the SIL Open Font License 1.1. The full credits are on the application's Credits page.

## Selected streets
sel-heading = Selected streets
sel-empty = No streets selected. Shift-click a street on the map, or press Shift+Enter on a search result.
sel-lead = { $count ->
    [one] 1 street selected.
   *[other] { $count } streets selected.
}
sel-added = { $name } added to the selection. { $count ->
    [one] One street selected.
   *[other] { $count } streets selected.
}
sel-removed = { $name } removed from the selection. { $count ->
    [0] No streets selected.
    [one] One street selected.
   *[other] { $count } streets selected.
}
sel-cleared = Selection cleared.
sel-not-a-street = Only streets can be selected.
sel-remove = Remove { $name }
sel-remove-short = Remove
sel-clear = Clear the selection
sel-list-label = Streets in the selection

## Changing the selected streets
corridor-refused-no-motor = The change does not fit: no piece of this street takes motor traffic.
corridor-refused-gone = This street is no longer in the city.
corridor-done = { $name }: { $changed ->
    [one] 1 street changed.
   *[other] { $changed } streets changed.
} { $left ->
    [0] Every street was changed.
    [one] 1 was left as it was.
   *[other] { $left } were left as they were.
}
corridor-heading = Change the selected streets
corridor-op-label = Change to
corridor-review-button = Review the change
corridor-review = { $name }: { $ready ->
    [one] 1 street can be changed.
   *[other] { $ready } streets can be changed.
} { $refused ->
    [0] Every selected street can take it.
    [one] 1 street cannot, and will be left as it is.
   *[other] { $refused } streets cannot, and will be left as they are.
}
corridor-apply = { $count ->
    [one] Change 1 street
   *[other] Change { $count } streets
}
corridor-cancel = Cancel
corridor-close = Close
corridor-row-ready = Can be changed.
corridor-row-changed = Changed.
corridor-row-not-kept = Could not be kept, so it is as it was.
corridor-review-list = Review of the change, street by street
corridor-report-list = What was changed, street by street
corridor-done-failed = { $name }: { $changed ->
    [one] 1 street changed.
   *[other] { $changed } streets changed.
} { $failed ->
    [one] 1 street could not be kept and is as it was.
   *[other] { $failed } streets could not be kept and are as they were.
} Try again.
