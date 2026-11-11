# The `.citymodel` holds tiled street networks: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `citymodel_builder` imports the whole Montreal area once, cuts the `Network` into the tile grid and packs those JSON tiles into the `.citymodel`; the editor reads and cuts a place from them; Overpass and the browser importer are gone, and `osm_import` lives inside `citymodel_builder`.

**Architecture:** A pure `Frame` (the lon/lat box a `Network`'s local metres are measured in) and tile/cut functions live in `cityloom_model`, shared by the builder (which cuts tiles) and the editor (which cuts a place out of tiles). The builder's pipeline becomes osmium filter (as now) → one import → tile cut → index → pack. The editor's `place::Loader` fetches the tile(s), merges by id, cuts to the place's box and keeps the result.

**Tech Stack:** Rust 2024, Leptos 0.8 (wasm), osm2streets (native only, after this change), `tar`/`sha2`, Playwright e2e, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-11-10-citymodel-network-tiles-design.md` (read it: its "Calls I made" and "Settled by the project" sections bind this plan).

## Global Constraints

- One pull request; `version` in `crates/cityloom_editor/Cargo.toml` goes 0.67.0 → **0.68.0**; run `cargo update -p cityloom_editor`.
- Test first; each task's commit leaves `cargo test --workspace` and the wasm32 build (`just test`) green under `RUSTFLAGS=-D warnings`. Run `just format` before each commit.
- `cityloom_model` stays serde-only: no `geom`, no I/O. The server (`citymodel_builder` with default features off) must not link osm2streets; `just check-no-fake` unchanged.
- Network types keep their shape; a tile is `NetworkTile { frame, network }` JSON, member `networks/<x>_<y>.json`, unpacked as `metro/<x>_<y>.json`. Archive index version 1 → **2**, refused with a plain message if 1.
- Tile = every road reaching into the core widened by `MARGIN_LON`/`MARGIN_LAT` (0.0064, 0.0045), whole, same ids in every tile it touches; **no corridors stored** (derived after the cut).
- A place is cut at its box as before: crossing roads end in a junction with empty provenance (`junction: false`, `Control::None`); lanes laid out and corridors derived after the cut.
- `IMPORTER_VERSION` lives in `cityloom_model`, becomes **10**; `world.spec.ts` follows.
- New messages in `en.ftl` and `fr.ftl` with the same id and variables; fr-CA per `docs/glossary-fr-ca.md`; update `i18n_guard.rs` lists.
- The import budget on a runner: peak memory < 10 GB, whole-area import measured ≈ 93 s / 7.2 GB on a developer machine.
- Commit messages end with the two attribution lines from the session reminder.

## Review Focus

- A place whose box straddles a tile edge opens whole (roads from both tiles, none doubled): Task 4 (`absorb`), Task 8.
- A road leaving and re-entering the box becomes two roads, neither joined across the gap: Task 4.
- A road lying wholly outside the box, or touching it at one point, is not in the place: Task 4.
- A tile that is missing, empty of roads in the box, or not JSON gives its own message and leaves kept roads alone: Task 8.
- Priority controls never name a road that is not in the tile or the place: Tasks 4, 5.

---

### Task 1: Make the dual-carriageway merge scale

**Files:** Modify `crates/osm_import/src/merge.rs` (`merge_dual_carriageways`, the `at` closure, the tail loops); Test: same file's `tests` module.

**Interfaces:** Produces: `merge_dual_carriageways(net: &mut Network)` unchanged signature and output.

- [ ] **Step 1: Failing test** `merging_twenty_thousand_roads_with_their_own_names_takes_under_two_seconds`: a `Network` of 20,000 disjoint one-way roads (each its own name, two junctions each, built with the module's `node`/`oneway` helpers); assert `merge_dual_carriageways` returns in < 2 s and leaves all 20,000 roads. (Quadratic in debug takes far longer.)
- [ ] **Step 2: Run** `cargo test -p osm_import --lib merge` — the new test FAILS (time), the other ten pass.
- [ ] **Step 3: Implement.** Pair search: group `oriented` indexes by `name` in a `HashMap<&str, Vec<usize>>` and visit only `a`'s group in ascending order (same tie-breaking); read junction positions from a `HashMap<NodeId, Point<Local>>` built once. Tail: apply node moves through an id→index map and a union-by-redirect map, rewriting road ends once at the end instead of per merged pair; remove merged junctions in one `retain`. Keep every output byte the same.
- [ ] **Step 4: Run** `cargo test -p osm_import` — all pass (the ten merge tests, `import.rs`, `real_city.rs`).
- [ ] **Step 5: Commit** "Make the dual-carriageway merge scale with the number of roads".

### Task 2: `IMPORTER_VERSION` moves to `cityloom_model`

**Files:** Create `crates/cityloom_model/src/importer.rs`; Modify `cityloom_model/src/lib.rs` (re-export), `cityloom_editor/src/place/tiles.rs`, `cityloom_editor/src/archive/model.rs:43,186`, `archive/tests.rs:128`, `e2e/tests/world.spec.ts`.

**Interfaces:** Produces: `cityloom_model::IMPORTER_VERSION: u32 = 10` and `cityloom_model::with_importer(checksum: &str) -> String` (`"{checksum}+i{IMPORTER_VERSION}"`). `place::tiles` keeps no copy.

- [ ] **Step 1: Failing test** in `importer.rs`: `with_importer("ab12")` equals `"ab12+i10"`.
- [ ] **Step 2: Run** `cargo test -p cityloom_model importer` — FAIL.
- [ ] **Step 3: Implement** the const (with the version history comment moved from `tiles.rs`, plus "10: the importer runs once over the whole area at pack time") and the function; delete them from `tiles.rs`, point the editor's uses at `cityloom_model`; update `world.spec.ts`'s expected `+i` value.
- [ ] **Step 4: Run** `cargo test --workspace` — pass.
- [ ] **Step 5: Commit.**

### Task 3: `Frame` in `cityloom_model`

**Files:** Create `crates/cityloom_model/src/frame.rs`; Modify `lib.rs`; Modify `cityloom_editor/src/city/projection.rs` so `Projection` is built on `Frame`.

**Interfaces:** Produces:
```rust
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame { pub south: f64, pub west: f64, pub north: f64, pub east: f64 }
impl Frame {
    pub fn width_m(&self) -> f64;   // haversine along the south edge, rounded to 0.1 mm as geom does
    pub fn height_m(&self) -> f64;  // along the west edge
    pub fn lon_lat(&self, x_m: f64, y_m: f64) -> [f64; 2];
    pub fn local(&self, lon: f64, lat: f64) -> [f64; 2];   // inverse of lon_lat
}
```
and, in the editor, `impl place::Bounds { pub fn frame(&self) -> Frame }` (Tasks 5 and 8 use it).
- [ ] **Step 1: Failing tests** (`frame.rs`): `local` inverts `lon_lat` for five points of a Plateau-sized frame to 1e-9; `lon_lat(0,0)` is `[west, south]`; a frame 0.0257° × 0.018° at lat 45.5 has `width_m ≈ 2005 ± 5`, `height_m ≈ 2002 ± 5`.
- [ ] **Step 2: Run** — FAIL. **Step 3: Implement**, moving `haversine_m`/`EARTH_RADIUS_M` from the editor's `projection.rs` (which now calls `Frame`; its public API and tests unchanged). **Step 4: Run** `cargo test --workspace` (including `osm_import/tests/projection.rs`) — pass. **Step 5: Commit.**

### Task 4: Cutting and merging networks

**Files:** Create `crates/cityloom_model/src/cut.rs`; Modify `lib.rs`, `network.rs` (if a helper is needed).

**Interfaces:** Produces, all pure, on `Network`:
```rust
pub struct NetworkTile { pub frame: Frame, pub network: Network }   // serde
impl Network {
    /// Roads whose centre line reaches into `rect` (a Frame, as lon/lat), whole, with the junctions they use;
    /// priority controls keep only roads present; corridors cleared. `from` is this network's frame.
    pub fn reaching(&self, from: &Frame, rect: &Frame) -> Network;
    /// Union by id: roads and junctions of `other` this network lacks are added (same frame assumed).
    pub fn absorb(&mut self, other: &Network);
    /// What lies in `to`, re-expressed in `to`'s frame: roads wholly inside kept, crossing roads cut at the
    /// border (one new road id per inside piece, a new empty-provenance end junction), lanes laid out,
    /// corridors derived, priorities pruned. `from` is this network's frame.
    pub fn clip_to(&self, from: &Frame, to: &Frame) -> Network;
}
```
New ids for cut pieces/ends are above the network's largest road/junction id.

- [ ] **Step 1: Failing tests** (`cut.rs`, tiny hand-built networks, a 1 km frame): `reaching` keeps a road with one point inside and a road whose segment crosses `rect` with no point inside, drops a road wholly outside, keeps ids; `reaching` prunes a `Priority` naming an absent road and clears corridors; `absorb` called with a network sharing two roads adds nothing twice and keeps ids; `clip_to` keeps an inside road with the same ends, cuts a crossing road at the border (its end junction has empty provenance, `junction == false`, position on the box edge ±1 mm), turns a road that leaves and re-enters into two roads, drops a road touching the box at a single point, re-expresses positions in `to` (a point at `to`'s south-west corner is `[0,0]`), and a `NetworkTile` round-trips through `serde_json`.
- [ ] **Step 2: Run** `cargo test -p cityloom_model cut` — FAIL.
- [ ] **Step 3: Implement.** Work in lon/lat via `Frame::lon_lat`/`local` per point; clip polylines to the rectangle segment by segment (Liang–Barsky), splitting at exits; after clipping call `Road::lay_out_lanes` on each changed road and `derive_corridors()`; re-index junctions used.
- [ ] **Step 4: Run** `cargo test -p cityloom_model` — pass. **Step 5: Commit.**

### Task 5: The importer returns its frame; the Plateau agrees with the clipped import

**Files:** Modify `crates/osm_import/src/lib.rs`; Create `crates/osm_import/tests/clip_agrees.rs`.

**Interfaces:** Produces `pub fn import_whole(osm: &[u8]) -> Result<(Network, Frame), String>` — `import_in(osm, None)` plus the frame from `streets.gps_bounds` (`min_lat, min_lon, max_lat, max_lon`). Consumes `Network::clip_to` (Task 4).

- [ ] **Step 1: Failing test** `clipping_the_whole_plateau_gives_the_places_the_clipped_import_gives`: `import_whole(PLATEAU)`; pick the box `Area::new("Plateau", lat, lon).bounds()` shrunk to 250 m half-width around the data's centre; `whole.clip_to(&frame, &box_frame)` against `import_in(PLATEAU, Some(box))`; build both `City::from_network`; assert at least 95% of street keys of the clipped import are keys of the cut one, none of the cut one's keys names a way the other lacks, and road counts differ by ≤ 5%. Also `import_whole` of `kreuzberg.osm` returns a frame containing the area's centre.
- [ ] **Step 2: Run** `cargo test -p osm_import --test clip_agrees` — FAIL (no `import_whole`). **Step 3: Implement.** If the test shows a gap, the fix belongs in Task 4's `clip_to` (report it there, do not weaken the 95%). **Step 4: Run** — pass. **Step 5: Commit.**

### Task 6: Archive carries network tiles (index version 2)

**Files:** Modify `crates/citymodel_builder/src/archive/{index.rs,pack.rs,unpack.rs,mod.rs}`, `crates/cityloom_server/tests/{areas.rs,data_mount.rs}` and their data, `crates/citymodel_builder/tests/archive.rs`.

**Interfaces:** `Kind::Road` → `Kind::Network`; member path `networks/<x>_<y>.json`; `destination(entry)` = `metro/<x>_<y>.json`; `VERSION = 2`; `PackInputs.roads` → `networks: &[(u16, u16, PathBuf)]`; `IndexError::Version(u16)` message says "reads version 2".

- [ ] **Step 1: Failing tests:** archive round-trip with two `.json` members unpacks to `metro/0_0.json` and an `index.json` listing them; a hand-made version-1 index is refused with `Version(1)`; the server's `data_mount` test serves `/data/metro/0_0.json`.
- [ ] **Step 2: Run** `cargo test -p citymodel_builder -p cityloom_server` — FAIL. **Step 3: Implement** the renames and version. **Step 4: Run** — pass. **Step 5: Commit.**

### Task 7: The builder imports, tiles and packs networks

**Files:** Create `crates/citymodel_builder/src/network_tiles.rs`; Modify `tiles.rs`, `prepare.rs`, `commands.rs`, `Cargo.toml` (feature `import` = `dep:osm_import`, `dep:cityloom_model`, included in `cli`; remove `rlimit`), `lib.rs`, `.github/workflows/ci.yml`, `justfile`.

**Interfaces:**
```rust
// network_tiles.rs
pub fn cut(network: &Network, frame: &Frame, grid: &Grid) -> Vec<(u16, u16, NetworkTile)>; // non-empty tiles, x outer y inner
pub fn write(out: &Path, tiles: &[(u16, u16, NetworkTile)]) -> Result<Vec<(String, String)>, StepError>; // name, short checksum
```
`tiles::run`: steps `area, highways, tracks, merge` (osmium, as now), then `import` (`osm_import::import_whole` on `roads.osm.pbf`), `tiles` (`cut` + `write`), `index` (`index_json`). `--batch`, `MAX_EXTRACTS`, `extracts()`, `raise_open_files` removed. `pack`'s `read_inputs` reads `<x>_<y>.json`. New command `citymodel_builder fixture <in.osm|pbf> <out.json>`: one `NetworkTile` of the whole input (frame = its data's).

- [ ] **Step 1: Failing tests:** `cut` puts a road in every tile whose widened core it reaches (two tiles for a road across an edge, same ids), leaves empty tiles out, and writes no corridors; `write` returns names and checksums that `index_json` accepts and the files parse as `NetworkTile`; `tiles::steps()` lists `import` between `merge` and `tiles`; `fixture` on `e2e/fixtures/plateau.osm.pbf` writes a tile whose network has more than 150 roads; `pack` of a folder of `.json` tiles round-trips.
- [ ] **Step 2: Run** `cargo test -p citymodel_builder` — FAIL. **Step 3: Implement** (select with `Network::reaching` using each tile's widened box as a `Frame`). Update the CI cache key to hash `tiles.rs`, `metro_index.rs`, `network_tiles.rs`, `crates/osm_import/src/**` and `crates/cityloom_model/src/**`; `just prepare --batch 2` → `just prepare`; recipe comments.
- [ ] **Step 4: Run** `cargo test --workspace`; then on the real data `just prepare` (extract kept) and record time and peak RSS (`/usr/bin/time -l`) against the budget. **Step 5: Commit.**

### Task 8: The editor reads network tiles; Overpass and the browser importer go

**Files:** Modify `place/tiles.rs`, `place/loader.rs`, `place/area.rs`, `place/mod.rs`, `shared/ports.rs` (remove `Importer`, `FakeImporter`, `Ports.importer`, test helpers' importer returns), `shared/platform.rs` (`BrowserImporter`), `i18n_guard.rs`, `place/i18n/{en,fr}.ftl`, `scripts/build.sh`, `web/credits.html`, every caller of `test_ports_with_fetcher`/`_services`; Delete `place/overpass.rs`, `web/city.js`; Modify `e2e/tests/delivery.spec.ts` (drop the importer-retry test, lines ~175–200).

**Interfaces:**
```rust
// tiles.rs
impl Index { pub fn tile_at(&self, lat: f64, lon: f64) -> Option<String>;            // "data/metro/<x>_<y>.json"
             pub fn tiles_touching(&self, b: Bounds) -> Vec<String>;                 // every listed tile whose core meets b, the one holding the centre first
             pub fn checksum_at(&self, lat: f64, lon: f64) -> Option<String>; }     // with cityloom_model::with_importer
// loader.rs
impl Loader { pub fn load(&self, store: &CityStore, done: impl FnOnce(Result<(), Said>) + 'static); } // same signature
```
New message `place-not-covered` (variable `area`) in both languages.

- [ ] **Step 1: Failing tests** (`loader.rs` with `FakeFetcher`; `tiles.rs`): a place in tile `12_9` fetches index then `data/metro/12_9.json`, parses a `NetworkTile`, keeps `clip_to` of it with the checksum; a place whose box reaches `12_10` fetches both and keeps a union with no doubled road; no index, no listed tile, or an unreadable tile → `place-not-covered` / `place-fetch-failed` / `place-roads-not-understood` and nothing asked of Overpass; a tile with no roads in the box → `place-no-streets`; a kept area whose checksum is unchanged is left alone with only the index asked; a kept area whose tile cannot be had stays as it is. Message parity test passes for `place-not-covered`.
- [ ] **Step 2: Run** `cargo test -p cityloom_editor place` — FAIL. **Step 3: Implement**; delete the importer port and Overpass; `scripts/build.sh` stops building and copying `osm_import.wasm`; credits drop "the roads are fetched with the Overpass API" (keep the Overpass font credit). **Step 4: Run** `just test` — pass (native and wasm32). **Step 5: Commit.**

### Task 9: e2e fixtures and specs use network tiles

**Files:** Modify `e2e/tests/{fixtures.ts,home.spec.ts,home-search.spec.ts,archive.spec.ts,map.spec.ts,junction.spec.ts}`, `scripts/e2e-archive.sh`, `justfile` (`e2e-ui`, `e2e-integration`, `e2e-archive`), `e2e/tiers.json` and `scripts/ci/spec-map.json` as `just test-scripts` requires; Create gitignored `e2e/fixtures/generated/`.

**Interfaces:** A script step runs `citymodel_builder fixture` for `plateau`, `turbo`, `open-ends` into `e2e/fixtures/generated/<name>.json` before the UI tier; the `PLATEAU_PBF`-style constants become `PLATEAU_TILE` etc.; stubs serve `**/data/metro/0_0.json`; `e2e-archive.sh` packs `0_0.json` with `{"lon0":-73.6078,"lat0":45.5161,…,"tiles":["0_0"]}`.

- [ ] **Step 1: Update** the stubs and expectations first; remove the Overpass stubs and the home spec's Kreuzberg-through-Overpass cases, replacing them with: a place no tile holds shows the not-covered message (English and fr-CA).
- [ ] **Step 2: Run** `just e2e-ui` — FAIL before the fixture step exists. **Step 3: Implement** the generation step and the not-covered spec. **Step 4: Run** `just e2e-ui`, then `just e2e-archive` (E2E tier) — pass; `just e2e-integration` needs `DATABASE_URL`. **Step 5: Commit.**

### Task 10: Merge `osm_import` into `citymodel_builder`

**Files:** Move `crates/osm_import/src/{lib.rs,merge.rs}` → `crates/citymodel_builder/src/importer/{mod.rs,merge.rs}`, `src/bin/osm2network.rs` → the `import` subcommand in `commands.rs`, `crates/osm_import/tests/**` → `crates/citymodel_builder/tests/importer/**` (data included); Modify `Cargo.toml` files, `Cargo.lock`, `justfile` (`default-area`), `scripts/ci/spec-map.json`, `.github/workflows/ci.yml`, `e2e/tests/home.spec.ts` (path), `place/tiles.rs`/`city/*` comments naming `osm_import`; Delete `crates/osm_import`.

**Interfaces:** `citymodel_builder::importer::{import, import_in, import_whole}` (behind feature `import`, on by default through `cli`); the `cdylib` type and `osm_to_network` are not carried over; `citymodel_builder import <in.osm|pbf> <out.json>` replaces `osm2network`.

- [ ] **Step 1: Move** with `git mv`, fix paths, `osm_import::` → `crate::importer::`; the moved tests are the failing tests until paths resolve. **Step 2: Run** `cargo test -p citymodel_builder` — pass, including `real_city`, `clip_agrees`, `import`. **Step 3: Run** `cargo build -p cityloom_server --release` and `just check-no-fake`; confirm `cargo tree -p cityloom_server | grep -c osm2streets` prints 0. **Step 4: Run** `just test` and `just test-scripts`. **Step 5: Commit.**

### Task 11: Docs, version, final verification

**Files:** Modify root `CLAUDE.md`, `place/CLAUDE.md`, `city/CLAUDE.md`, `e2e/CLAUDE.md`, `citymodel_builder/CLAUDE.md`, `cityloom_server/CLAUDE.md`, `README.md`, `website/docs/getting-started.md` and its fr-CA twin, `crates/cityloom_editor/Cargo.toml` (0.68.0), `Cargo.lock`.

- [ ] **Step 1: Update** every sentence that says places can come from Overpass, that tiles are PBF, that `osm_import` is a crate or a wasm module, or that `prepare` takes `--batch`; describe the new archive, `import`/`fixture` commands, the tile cache key, and the frame/cut. **Step 2: Run** `just docs-build` (no broken link). **Step 3: Bump** version; `cargo update -p cityloom_editor`. **Step 4: Run** `just format-check`, `just test`, `just check-no-fake`, `just test-scripts`, `just e2e` (needs `DATABASE_URL`, `just db`); then `just prepare` on the real extract and record time and peak memory in the PR. **Step 5: Commit**, then open the PR with the `create-pr` skill.
