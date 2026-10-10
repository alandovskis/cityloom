# What the place search on the home page says (src/place/vm.rs, src/place/view.rs), and why a place's roads could not be had (src/place/loader.rs).

## The search box
place-search-label = Find a place
place-search-placeholder = Find a place, like Kreuzberg, Berlin
place-results = Places found
place-find = Find
place-open = Open

## How the search is going
place-searching = Searching…
place-found = { $n ->
    [one] { $n } place found
   *[other] { $n } places found
}. Use the arrow keys, then Enter.
place-nothing = No place found. Try a city, a neighbourhood or an address.
# `name` is the place's own name, as the place search gives it.
place-loading = Getting the streets of { $name }…

## Why a place's roads could not be had
# `e` is the underlying error, as the browser or a library words it (not translated). `area` is the place's name.
place-fetch-failed = the roads of { $area } could not be fetched ({ $e })
place-no-streets = OpenStreetMap has no streets around { $area }
place-roads-not-kept = the roads could not be kept: storage is blocked or full
place-roads-not-understood = the roads were not understood ({ $e })
place-search-unexpected = the place search answered something unexpected ({ $e })
place-index-not-understood = the tile index was not understood ({ $e })
place-index-no-size = the tile index has no tile size
place-area-not-kept = The place could not be kept: storage is blocked.
# What went wrong, as the browser or the OpenStreetMap reader said it, passed on as it is.
place-problem = { $e }
