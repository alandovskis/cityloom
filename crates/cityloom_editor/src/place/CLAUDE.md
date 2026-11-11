# Place slice (`src/place/`)

Loaded when working in this folder; the root `CLAUDE.md` has the project-wide rules.

- `area.rs` (`Area`, kept under its position), `nominatim.rs` (requests and readers, pure), `tiles.rs` (the metro tile index and the tiles that hold a place), `loader.rs` (`Loader`: search, fetch, merge, cut, keep roads, via ports), `vm.rs` / `view.rs` (`AreaVm`, home place search), `prepare_city` export.
- `Loader::load` reads roads from the metro network tiles only: it fetches `data/metro/index.json`, then the tile whose core holds the place's centre and any other listed tile the place's box reaches, merges them by id (`Network::absorb`), cuts the place out (`Network::clip_to` into `area.bounds().frame()`) and keeps it with the tile's checksum plus `cityloom_model::IMPORTER_VERSION`. There is no Overpass and no importer in the browser: a place no tile holds is not covered (`place-not-covered`, nothing kept); roads already kept stand when the index or a tile cannot be had.
