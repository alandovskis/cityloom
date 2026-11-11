# The `.citymodel` holds tiled street networks, not OSM

Date: 2026-11-10. One pull request.

## Intent

Today the `.citymodel` archive carries raw OpenStreetMap PBF tiles. The browser fetches one tile
for a place, loads a second wasm module (`osm_import`, which runs osm2streets) and makes the
street network there, clipped to the place's box. If no tile holds the place, it asks Overpass.

The archive should instead carry the **model**: the serialized `cityloom_model::Network`. The
import runs once, when the archive is packed, on a machine that has the whole extract. The
browser only reads finished networks. That removes the importer, osm2streets and Overpass from
the browser, makes every place come from one curated source, and removes the tile-edge clipping
artefacts the per-place import has today.

## Decisions (the user's)

1. **Whole-area import.** `citymodel_builder` imports the whole area once, not tile by tile. The
   result is one `Network` for the area.
2. **Tiled for the map.** That network is then cut into the existing tile grid, and each tile is
   one member of the archive. The editor fetches the tile a place needs, as it does now.
3. **JSON.** A tile is the JSON of `cityloom_model::Network`, as `osm_import` and the editor
   already write and read it. No second wire format.
4. **No Overpass.** A place that no tile holds does not load. There is no network fallback.
5. **`osm_import` merges into `citymodel_builder`.** The crate stops existing; the importer is a
   module of the builder, behind a Cargo feature so the server does not build it.
6. **One pull request** does all of this, with a version bump (minor: the archive format
   changes) and `cargo update -p cityloom_editor`.

## Goals

- `just prepare` and `just pack` produce a `.citymodel` whose members are tiled Networks.
- The browser never runs osm2streets. `web/pkg/osm_import*`, `web/city.js` and
  `window.cityloomImportOsm` are gone, and so is the importer port in the editor.
- A place inside the archive's coverage opens as it does today: same streets, lanes, names,
  junctions, corridors, same edits kept and following the place.
- A place outside the coverage says so, in English and fr-CA, instead of trying the network.
- The server stays free of osm2streets: its release build and `just check-no-fake` are unchanged
  in what they link.

## Non-goals

- Changing what the importer makes of OSM (lanes, merging dual carriageways, corridors). The
  output for the same data stays the same.
- A new Network schema. The types are kept as written.
- Serving tiles by byte range out of the archive, or any change to how the server unpacks and
  serves `metro/`.
- Replacing Nominatim place search.

## Anti-goals

- A tile that cannot be read as the Network of its place. A place near a tile edge must open
  whole, as it does today, with no road missing and no road cut at the tile's edge.
- Kept areas silently showing different streets after the change. Areas kept from the old tiles
  read again, once (see Importer version).

## Constraints from the codebase

- `citymodel_builder` is not built to wasm; the wasm build excludes it. After the merge nothing
  of the importer is built to wasm.
- The server depends on the builder's `archive` library with default features off (only `tar`,
  `sha2`, `thiserror`). The importer's git dependencies (`osm2streets`, `streets_reader`,
  `abstutil`, `geom`) sit behind an optional feature that the `cli` feature turns on.
- The archive index (`cityloom.idx`, binary, magic `CLOOMIDX`, version 1) lists members by kind,
  x, y, size, SHA-256 and path; today only `Kind::Road` and `roads/<x>_<y>.osm.pbf`. The server's
  unpack verifies every member and writes `metro/` with an `index.json` that the app reads
  (`place/tiles.rs`: grid, tile names, a checksum of each).
- `IMPORTER_VERSION` (`place/tiles.rs`) is part of what a kept area is made from. The e2e test
  `world.spec.ts` names it.
- Tests: the failing test first; Rust logic only; `just test` must pass with `RUSTFLAGS=-D warnings`;
  both languages for any new message, with `docs/glossary-fr-ca.md`.

## The design

### Packing

`citymodel_builder pack` (and `prepare`, which ends in it) now does, after the extract is cut into
roads:

1. Read the road data of the whole area and import it once with the importer (the code of
   `osm_import::import`, without a clip box), producing one `Network` in one coordinate frame.
2. Cut that network along the existing grid. A tile holds every road that reaches into the tile's
   core widened by the margin the grid already uses, whole, with its junctions, so a place is read
   from a single file. A road that reaches into several tiles is in each of them, with the same
   ids. This is today's rule for the PBF tiles, applied to roads instead of OSM ways.
3. Serialize each tile as JSON and write it as a member `networks/<x>_<y>.json`. Members are in a
   fixed order and the tar stays deterministic, so the same input gives the same bytes.

The archive index gets a new member kind for a network tile. Its version is raised from 1 to 2;
a version-1 archive is refused with a plain message. The `.citymodel` stays one file in one
folder.

### Serving

Unchanged in shape. Unpacking writes the network tiles to `DATA_ROOT/<area>/metro/` and the
`index.json` the app reads, with a checksum per tile. The tile file names change from
`<x>_<y>.osm.pbf` to `<x>_<y>.json`, and `place/tiles.rs` follows.

### Reading in the editor

`place::Loader::load` fetches the index, finds the tile whose core holds the place, fetches it and
parses the `Network`. If the place's box reaches into other tiles that the index lists (decision
7), it fetches those too and merges them by id; roads and junctions share ids across tiles, so
a merge is a union. It then keeps from the network what lies in the place's box, in the form
the store keeps now, with the checksum of the tile that holds the place plus `IMPORTER_VERSION`. The importer port, the
Overpass request, `place/overpass.rs` and the "listed but cannot be had, ask Overpass" branch are
removed. Cutting a place out of a tile is Rust in the editor and has native tests.

A place with no tile, a tile that cannot be had, or a tile with no roads in the place's box is an
error with its own message. A place that is already kept and whose tile cannot be had stays as it
is, as today.

### Merging the crate

`crates/osm_import` moves into `crates/citymodel_builder` as an `importer` module, with its tests
and test data (`kreuzberg.osm`, `plateau.osm`, `real_city.rs`). `osm2network` becomes a subcommand
of the builder (`import`, reading OSM and writing Network JSON), and `just default-area` calls it.
The `cdylib` crate type and the `wasm_bindgen` export go. `cityloom_model` stays its own crate:
the editor depends on it, and the builder is its producer.

### Everything that names what is removed

`scripts/build.sh` (no `osm_import.wasm`), `scripts/ci/spec-map.json` (the `osm_import` path;
the builder's rule already runs the E2E tier), `justfile`, `.github`, `Cargo.lock`,
`e2e/tests/delivery.spec.ts` (the importer-retry test), `e2e/tests/fixtures.ts`, `home.spec.ts`
and `map.spec.ts` (the Overpass stubs, and the OSM extract the home spec feeds in), the root
`CLAUDE.md`, `place/CLAUDE.md`, `city/CLAUDE.md`, `e2e/CLAUDE.md`, `citymodel_builder/CLAUDE.md`,
the README, `DESIGN.md`, and the getting-started pages in `website/` (English and fr-CA).
`src/i18n_guard.rs` entries that name Overpass.

### Importer version

Raised once, because kept areas must be read again from the new tiles. The constant lives in
`cityloom_model` (see "Calls I made"); nothing is stamped into the archive. The tile checksum in
`index.json` changes with the tile's bytes, and the editor appends `+i<version>` as today.

## More decisions (the user's, after the builder check)

7. **A place that straddles a tile edge is read from every tile its box touches**, merged by id.
   The grid's margin is kept, so a place is normally one file, and merging is for the places the
   margin does not cover. It replaces "a single file" as a guarantee. A place no tile holds still
   says "not covered".
8. **The one-pass import is measured before anything is built.** The existing `osm2network` runs
   on the merged Montreal roads, and time and peak memory are reported. If one pass does not fit,
   the design is revisited (chunked import with overlap and de-duplication) before the build.

9. **The packing runs on GitHub Actions**, so it is sized for that runner, not for a developer's
   machine. The repository is public: a standard runner has 4 vCPUs, 16 GB of RAM and about 14 GB
   of disk. The deploy job (90 minutes) already runs `just prepare --batch 2` when the tile cache
   misses; osmium needs about a gigabyte per tile it cuts at once. The whole-area import runs
   after the cutting, never beside it. Budget for the import: peak memory well under 10 GB (the
   runner's 16 GB less the OS and the cache restore) and a time that leaves the rest of the job
   its minutes. The measurement in decision 8 is judged against this, and a developer's 36 GB
   machine says nothing by itself.

### Result of decision 8 (measured, then profiled)

The existing `osm2network` on the whole Montreal roads (332,000 ways, 1.78 million nodes) did not
finish in 30 minutes, twice. Peak memory was 6.9 GB and flat. It is single-threaded. Three stack
samples (2, 8 and 15 minutes) put 99.98% of the time inside `osm_import::import_in` itself, in one
tight loop, with `memcmp` and `hypot` in it; osm2streets had finished before the first sample. The
code that fits is `merge::merge_dual_carriageways`: for every road it scans every road (a
`name` string comparison per pair, so R² of them), and inside it `at()` scans all junctions
linearly four times per candidate pair; its tail loop scans all junctions and all roads once per
merged pair. A per-place import has hundreds of roads and never shows it. This is inferred from
the code and the samples, not from line-level profile: the loop is inlined into `import_in`.

So the one-pass design stands only if that merge is made to scale, which is part of this change:
index roads by (name, road type) and junctions by id, so the pair search and the node lookups do
not scan everything. Its output must stay the same (its tests, and the Plateau and Kreuzberg
extracts, pin that). The whole-area import is then to be timed again, and judged against decision
9; if another phase proves slow, the chunked design (import each tile's roads at pack time,
without merging neighbours, so decision 7 falls) is the fallback.

**Spike (thrown away, not kept).** A patch to `merge_dual_carriageways` that looked roads up by
name in a hash map and junction positions by id in another (the pair search only; the tail loop
was left as it was) made the same whole-area import **finish in 92.9 s** (91.8 s of CPU, one
thread), peak memory **7.2 GB**, output **280 MB** of JSON. On the Plateau and Kreuzberg
extracts its output is byte-for-byte the output of the original code, and the ten merge tests
passed. So the cause is confirmed and the one-pass design (decisions 1 to 3) stands, with the
merge made to scale as part of this change. On a 4-vCPU runner with slower cores the import may
take two or three times as long (my estimate, not measured), which fits decision 9's budget; its
memory fits the 16 GB. Still to do in the real change: write the fix test-first, index the
tail loop too, and check the time and memory again on a runner-like machine. The 280 MB is
before tiling, which stores a road in every tile it touches; the builder checks the tiled size.

## Settled by the project (from the code)

- **The tile cache key must follow the importer.** The deploy job caches `web/data/metro` under
  `tiles-<hash of citymodel_builder's tiles.rs and metro_index.rs>`. The tiles are now made by the
  importer too, so the key hashes the importer module and `cityloom_model` (where
  `IMPORTER_VERSION` now lives) as well; a change to either makes the tiles again, and a change to
  neither does not. Without that, a deploy would keep serving tiles made by an older importer.

- **Edits survive a new import.** A kept edit is keyed by OSM ids, not network ids: `n<id>` for a
  junction (its smallest OSM node) and `w<id>~n<id>~n<id>` for a street; an end that is no OSM
  node is `x` plus a compass word (`city/CLAUDE.md`, Keys). Records are matched by key, else by the
  OSM ways and nodes they were made from. So a whole-area import does not orphan them. The
  acceptance test is the existing one in `osm_import/tests/real_city.rs`: at least 95% of
  same-made places follow when the tile is shifted, none takes another's edit. It moves with the
  importer and runs against the whole-area import.
- **Deploy.** `scripts/deploy.sh` ships `web/data/metro` (the tiles `just prepare` writes) inside
  the web bundle. It does not ship the `.citymodel`; the server unpacks one only when
  `CITYLOOM_AREAS` is set, which the E2E tier does. So there is no live v1 data to keep reading:
  a deploy replaces the tiles. `prepare` must run again before the next deploy.

## Calls I made (mine, not the user's; check them)

- **`IMPORTER_VERSION` moves to `cityloom_model`.** Both the builder and the editor depend on that
  crate, so one constant serves both. Nothing is "stamped" into the index: the tile checksum in
  `index.json` already changes with the tile's bytes, and the editor appends `+i<version>` as
  today. It is raised once for this change, and `world.spec.ts` follows.
- **A place is cut at its box, as today.** The editor keeps the roads of the tile that lie in the
  place's box and cuts those that cross it, leaving a street running off the map with an `x` end,
  so that keys, widths and what the editor shows for the same place stay as they are. The cut is
  a pure function on `Network` with native tests, comparing its result with what the clipped
  import gives for Plateau and Kreuzberg.
- **A place keeps its own frame.** The tile carries the longitude and latitude of its frame's
  origin; the editor re-expresses the place's roads in the frame the clipped import would have
  used (its box's south-west corner). Small differences from the projection's scale at another
  latitude are accepted if the same-place tests above still pass.
- **Version 1 archives are refused**, with a plain message; no compatibility is kept.
- **"The area" is the Montreal metro's roads**, the ones `prepare` already cuts from the Quebec
  extract into `web/data/metro` (about 1,700 tiles, 54 MB), not the whole Quebec extract.

## Left to the builder

- The tile margin: keep today's. Measure on the Montreal extract how many places' boxes it does
  not cover, which is how often the multi-tile merge of decision 7 runs.
- The exact cut at a place's box and the exact re-framing of positions are decided in "Calls I
  made"; what is left is how to check them (the same-place tests against the clipped import) and
  what difference in the frame's scale is small enough to accept.
- The size of the Montreal tiles in JSON after tiling, and whether to compress at serve time.
- What the e2e UI tier uses in place of an OSM extract: a small packed `.citymodel`, or tile JSON
  built from `kreuzberg.osm` by the builder.
- The names of the new messages and the wording of the "not covered" text, in both languages.
- The name of the member kind and the subcommand.

## As built

Where the build settled what the spec left open or departed from it.

- **One-pass import.** The merge fix (`importer/merge.rs`) made the one-pass import of the whole area
  feasible: measured on the real Montreal data (a developer Mac, not a runner), import, tiles and pack
  after the osmium steps take 52.8 s with 7.31 GB peak memory.
- **`clip_to` as built.** Zones are re-based per cut piece. A road is cut from its junction's point where
  its trimmed line lies outside the box, and at the box edge where its junction is outside but its trimmed
  line inside. osm2streets' rule that a way with no node in the box is dropped is not reproduced: the
  network keeps too few of a way's nodes to apply it. Pieces made by cutting that are shorter than 1.0 m
  are dropped. The output is sorted by id.
- **Acceptance** (`clip_agrees.rs`: the Plateau box and four boxes shifted 150 m). Of the places the importer
  makes the same in both the clipped import and the cut, 98.6% (288 of 292) follow, and none takes another's
  edit. Of all records, 92.6% follow (the floor asserted is 90%); exact-key agreement is informational. Road
  counts in the cut are 100 to 107.7% of the clipped import's (the extra corner pieces carry no edit).
  Known residue: 3 records lost to `merge.rs` pairing two pieces of one way in the clipped import, and
  junction clusters within about 1 m of a box edge differ.
- **Size.** 1,718 tiles, 570.1 MB of JSON at rest (largest tile 3.06 MB, about 337 KB gzipped); archive
  571.5 MB. The server gzips full responses on the fly. Recommended follow-ups: compression at rest, and
  dropping the lane shapes, which are recomputable and about 30% of a tile.
- **Commands and CI.** `import <in> <out.json>` (the whole `Network`) and `fixture <in> <out.json>` (one
  `NetworkTile` of the input's own frame) are new; `tiles` takes `--out`; `index.json` lists names sorted as
  text. The e2e fixtures are generated (`just e2e-fixtures` into `e2e/fixtures/generated`, gitignored), not
  committed. The deploy job's tile cache key hashes `tiles.rs`, `metro_index.rs`, `network_tiles.rs`,
  `importer/**` and `cityloom_model/src/**`; the spec map sends `crates/citymodel_builder/**` to every spec.
- **`web/city.js`** is kept, for `placeParam` only.
- **Importer panic.** On OSM XML without `<bounds>` the importer panics inside osm2streets; the e2e
  `open-ends.osm` fixture gained a `<bounds>` element for this.
