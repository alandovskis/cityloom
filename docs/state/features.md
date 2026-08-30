# CityLoom — Feature Inventory

Every requirement as one atomic, verifiable line. Status marker is the first
token: `[FAILING]`, `[PASSING]`, or `[BLOCKED:<reason>]`. A line may only be
moved to `[PASSING]` by a test that actually asserts it and runs under
`just verify`.

Decisions that shaped this list (2026-08-29):
- Product code is Rust only: `cityloom-server` (axum) + `cityloom-client`
  (wasm). Python (uv/pytest) owns integration + E2E harness and tooling.
- v1 is network-first: pannable city map, select a street, edit its
  cross-section, whole city persists as one document.
- Hand-drawn authoring and OSM import are peer entry paths in v1, which forces
  a shared canonical model plus re-import reconciliation.
- Rendering is WASM to WebGL2, so assertions hang off an exported
  `scene_state()` hook rather than the DOM.

---

## OPS — environment, gate, packaging

- [PASSING] OPS-001 — `just verify` exits 0 on a clean checkout with no product code changes.
- [PASSING] OPS-002 — `just verify` exits non-zero when any Rust test fails.
- [PASSING] OPS-003 — `just verify` exits non-zero when clippy emits any warning (`-D warnings`).
- [PASSING] OPS-004 — `just verify` exits non-zero when `cargo fmt --check` finds unformatted Rust.
- [PASSING] OPS-005 — `just verify` exits non-zero when `ruff check` finds a Python lint error.
- [PASSING] OPS-006 — `just verify` prints at most 40 lines of output per stage on success.
- [PASSING] OPS-007 — `just verify` on failure prints the failing stage name and the last 60 lines of that stage's log, and nothing from passing stages.
- [PASSING] OPS-008 — `just verify` names every stage it ran and its pass/fail state in a final summary block.
- [FAILING] OPS-009 — `just setup` installs the `wasm32-unknown-unknown` target and `wasm-pack`, and is idempotent.
- [FAILING] OPS-010 — `just build` produces a wasm artifact at `web/pkg/cityloom_client_bg.wasm`.
- [FAILING] OPS-011 — `just format` rewrites unformatted Rust and Python in place and exits 0.
- [FAILING] OPS-012 — `docker compose up -d db` yields a Postgres accepting connections within 30 s.
- [FAILING] OPS-013 — `just migrate` applies all migrations to an empty database and exits 0.
- [FAILING] OPS-014 — `just migrate` is idempotent: a second run exits 0 and applies zero new migrations.
- [FAILING] OPS-015 — `uv sync` provisions the Python environment and `uv run pytest --collect-only` exits 0.
- [FAILING] OPS-016 — The production Docker image builds and `docker run` serves HTTP 200 on `/healthz`.
- [PASSING] OPS-017 — `just verify` completes in under 5 minutes on a warm cache.

## CORE — canonical network model

- [FAILING] CORE-001 — A `Network` holds nodes and edges and survives a JSON round-trip with structural equality.
- [FAILING] CORE-002 — A `Node` carries a stable `NodeId`, a position in city-local metres, and its incident edge ids.
- [FAILING] CORE-003 — An `Edge` carries a stable `EdgeId`, `from`/`to` node ids, an ordered centreline polyline, and a cross-section id.
- [FAILING] CORE-004 — Every edge's centreline first point equals its `from` node position and last point equals its `to` node position, within 1e-9 m.
- [FAILING] CORE-005 — `Network::validate()` reports edges whose endpoints reference missing node ids.
- [FAILING] CORE-006 — `Network::validate()` reports duplicate node ids and duplicate edge ids.
- [FAILING] CORE-007 — `Network::validate()` reports zero-length edges (endpoints under 0.01 m apart).
- [FAILING] CORE-008 — `Network::validate()` reports edges referencing a missing cross-section id.
- [FAILING] CORE-009 — `Network::validate()` returns every violation found, not just the first.
- [FAILING] CORE-010 — Removing a node also removes every edge incident to it.
- [FAILING] CORE-011 — Node degree is derived from incident edges and stays correct after any topology mutation.
- [FAILING] CORE-012 — Splitting an edge at parameter t in (0,1) yields two edges sharing one new node, preserving total centreline length within 1e-6 m.
- [FAILING] CORE-013 — Splitting an edge gives both children the parent's cross-section id.
- [FAILING] CORE-014 — Splitting at t outside (0,1) is refused with a typed error and leaves the network unchanged.
- [FAILING] CORE-015 — Merging two edges across a shared degree-2 node yields one edge whose polyline is the ordered concatenation of the originals.
- [FAILING] CORE-016 — Merging is refused when the shared node has degree other than 2.
- [FAILING] CORE-017 — Merging is refused when the two edges carry different cross-section ids, unless `force` is set.
- [FAILING] CORE-018 — Ids record provenance: either `Osm { id }` for imported features or `Authored` for in-app ones.
- [FAILING] CORE-019 — Ids are stable across a save/load round-trip.
- [FAILING] CORE-020 — Newly allocated ids never collide with ids already present in the document.
- [FAILING] CORE-021 — A `CrossSection` is an ordered list of `Segment`s; its total width is the sum of segment widths.
- [FAILING] CORE-022 — A `Segment` carries a kind, a width in metres, and a direction of inbound, outbound, or none.
- [FAILING] CORE-023 — Segment kinds include at least: sidewalk, bike lane, drive lane, parking lane, transit lane, turn lane, median, planting strip, buffer.
- [FAILING] CORE-024 — Each segment kind declares whether a direction is meaningful for it.
- [FAILING] CORE-025 — Reordering segments preserves total width exactly.
- [FAILING] CORE-026 — Inserting a segment increases total width by exactly that segment's width.
- [FAILING] CORE-027 — Removing a segment decreases total width by exactly that segment's width.
- [FAILING] CORE-028 — A segment's cumulative offset from the left kerb is the sum of preceding segment widths.
- [FAILING] CORE-029 — Segment widths are rejected if negative or non-finite.
- [FAILING] CORE-030 — Cross-sections are shared by reference: editing one referenced by N edges is observable from all N.
- [FAILING] CORE-031 — `detach_cross_section(edge)` produces an independent copy and leaves other referencing edges unchanged.
- [FAILING] CORE-032 — A `City` bundles a network, a named cross-section library, a display name, a slug, and a schema version.
- [FAILING] CORE-033 — Loading a city whose schema version is newer than the binary's fails with a typed error rather than panicking.
- [FAILING] CORE-034 — A city with zero nodes and zero edges is valid and round-trips.
- [FAILING] CORE-035 — Edge metadata holds arbitrary string key/values (e.g. `maxspeed`) and round-trips.

## GEO — geometry and projection

- [FAILING] GEO-001 — WGS84 lon/lat converts to city-local metres about a per-city origin and back within 0.01 m over a 20 km extent.
- [FAILING] GEO-002 — The projection is exactly invertible at the origin itself.
- [FAILING] GEO-003 — Polyline length in metres matches a hand-computed fixture within 1e-6.
- [FAILING] GEO-004 — Nearest-point-on-polyline returns the correct segment index, parameter t, and distance for a fixture.
- [FAILING] GEO-005 — Nearest-point handles a query exactly on a vertex without NaN.
- [FAILING] GEO-006 — Offsetting a polyline by w produces a result everywhere w from the original along straight runs, within 1e-6 m.
- [FAILING] GEO-007 — Offsetting miters convex corners without self-intersection for interior angles above 30 degrees.
- [FAILING] GEO-008 — Offsetting by a negative width offsets to the opposite side.
- [FAILING] GEO-009 — Douglas-Peucker simplification with tolerance 0 is the identity.
- [FAILING] GEO-010 — Simplification never displaces the polyline by more than the tolerance.
- [FAILING] GEO-011 — Simplification always retains the first and last points.
- [FAILING] GEO-012 — A network's axis-aligned bounding box contains every node and every centreline vertex.
- [FAILING] GEO-013 — The bounding box of an empty network is reported as `None`, not a degenerate box.
- [FAILING] GEO-014 — Intersection points of two crossing polylines are found for a fixture within 1e-6 m.
- [FAILING] GEO-015 — A node of degree 3 or more yields an intersection polygon spanning all incident street widths.
- [FAILING] GEO-016 — A degree-2 node whose incident cross-sections have equal width yields no intersection polygon.
- [FAILING] GEO-017 — A degree-1 node yields a street end cap, not an intersection polygon.
- [FAILING] GEO-018 — Street ribbon generation emits one quad strip per segment, in left-to-right cross-section order.

## PERSIST — Postgres persistence

- [FAILING] PERSIST-001 — A migration creates `cities` with id, slug, name, schema_version, version, created_at, updated_at.
- [FAILING] PERSIST-002 — A migration enables PostGIS and creates `nodes` with a `geometry(Point,4326)` column and a GIST index.
- [FAILING] PERSIST-003 — A migration creates `edges` with a `geometry(LineString,4326)` column, from/to node FKs, and a cross-section FK.
- [FAILING] PERSIST-004 — A migration creates `cross_sections` and `segments`, with `segments` unique on (cross_section_id, position).
- [FAILING] PERSIST-005 — A migration creates `edge_metadata` keyed by (edge_id, key) with a unique constraint.
- [FAILING] PERSIST-006 — Deleting a city cascades to its nodes, edges, cross-sections, segments, and metadata.
- [FAILING] PERSIST-007 — Deleting a node referenced by an edge is refused by the FK and surfaced as a typed error.
- [FAILING] PERSIST-008 — `save_city` then `load_city` returns a `City` equal to the original.
- [FAILING] PERSIST-009 — Saving an existing slug updates in place rather than inserting a duplicate.
- [FAILING] PERSIST-010 — City slugs are unique; a conflicting insert returns a typed `SlugTaken` error, not a raw driver error.
- [FAILING] PERSIST-011 — `load_city` on an unknown slug returns `None` rather than an error.
- [FAILING] PERSIST-012 — Saving increments the city's `version` column by exactly 1.
- [FAILING] PERSIST-013 — Saving with a stale expected version returns a typed `Conflict` and writes nothing.
- [FAILING] PERSIST-014 — OSM provenance ids survive a save/load round-trip.
- [FAILING] PERSIST-015 — A city of 50,000 edges saves and loads in under 10 s against a local database.
- [FAILING] PERSIST-016 — Every save runs in a single transaction; an error mid-save leaves the prior state intact.

## API — HTTP surface

- [FAILING] API-001 — `GET /healthz` returns 200 with body `ok`.
- [FAILING] API-002 — `GET /healthz` returns 503 when the database is unreachable.
- [FAILING] API-003 — `POST /api/cities` with a name creates a city and returns 201 with its slug.
- [FAILING] API-004 — `POST /api/cities` derives a URL-safe slug from the name and de-duplicates with a numeric suffix.
- [FAILING] API-005 — `GET /api/cities/{slug}` returns 200 and the city document as JSON.
- [FAILING] API-006 — `GET /api/cities/{slug}` for an unknown slug returns 404.
- [FAILING] API-007 — `PUT /api/cities/{slug}` replaces the document and returns 200 with the new version.
- [FAILING] API-008 — `PUT /api/cities/{slug}` with a stale version returns 409.
- [FAILING] API-009 — `PATCH /api/cities/{slug}` renames a city without touching its network.
- [FAILING] API-010 — `DELETE /api/cities/{slug}` returns 204 and a subsequent GET returns 404.
- [FAILING] API-011 — `GET /api/cities` lists slug, name, and updated_at, newest first.
- [FAILING] API-012 — Request bodies over 32 MB are rejected with 413.
- [FAILING] API-013 — Malformed JSON returns 400 with a machine-readable error code.
- [FAILING] API-014 — A document failing `Network::validate()` is rejected with 422 listing the violations.
- [FAILING] API-015 — Every error response has the shape `{"error":{"code":...,"message":...}}`.
- [FAILING] API-016 — `POST /api/cities/{slug}/import/osm` accepts a bbox and returns 202 with a job id.
- [FAILING] API-017 — `POST .../import/osm` with an inverted or oversized bbox returns 400.
- [FAILING] API-018 — `GET /api/jobs/{id}` reports status pending, running, succeeded, or failed.
- [FAILING] API-019 — A failed job exposes its error message via `GET /api/jobs/{id}`.
- [FAILING] API-020 — A succeeded import job exposes its reconciliation report.
- [FAILING] API-021 — Static assets are served from `/` with `application/wasm` for `.wasm` files.
- [FAILING] API-022 — Structured JSON logs carry a request id that is echoed in the `x-request-id` response header.
- [FAILING] API-023 — The server drains in-flight requests on SIGTERM and exits 0.

## OSM — import pipeline

- [FAILING] OSM-001 — An OSM XML fixture parses into nodes and ways with no error.
- [FAILING] OSM-002 — A malformed XML document is rejected with a typed parse error.
- [FAILING] OSM-003 — Only ways whose `highway` tag is in the routable allow-list become edges.
- [FAILING] OSM-004 — Ways tagged `area=yes` are skipped.
- [FAILING] OSM-005 — A way with N node refs becomes an edge with an N-point centreline.
- [FAILING] OSM-006 — A way is split at every node it shares with another retained way, producing a connected topology.
- [FAILING] OSM-007 — Nodes used by exactly one way and not at its ends remain shape points, not network nodes.
- [FAILING] OSM-008 — A way referencing a node absent from the fixture is skipped with a warning, without aborting the import.
- [FAILING] OSM-009 — `oneway=yes` yields a cross-section whose drive lanes are all outbound.
- [FAILING] OSM-010 — `oneway=-1` yields outbound lanes with the centreline reversed.
- [FAILING] OSM-011 — `lanes=N` yields N drive lanes.
- [FAILING] OSM-012 — Absent `lanes`, a per-highway-class default lane count is used.
- [FAILING] OSM-013 — `width=X` sets total right-of-way width; absent it, a per-highway-class default is used.
- [FAILING] OSM-014 — `sidewalk=both|left|right|no` adds the corresponding sidewalk segments and no others.
- [FAILING] OSM-015 — `cycleway=lane` and `cycleway:left`/`cycleway:right` add bike lanes on the correct sides.
- [FAILING] OSM-016 — `parking:lane:*` tags add parking lane segments on the tagged side.
- [FAILING] OSM-017 — `maxspeed` is preserved as edge metadata and round-trips.
- [FAILING] OSM-018 — Import is deterministic: two runs over one fixture produce byte-identical city JSON.
- [FAILING] OSM-019 — Every imported node and edge carries provenance mapping back to its OSM id.
- [FAILING] OSM-020 — Imported geometry is projected into city-local metres using the city's origin.
- [FAILING] OSM-021 — Importing into a non-empty city routes through reconciliation rather than appending duplicates.
- [FAILING] OSM-022 — The import job reports progress as a fraction that increases monotonically to 1.0.

## RECON — re-import reconciliation

- [FAILING] RECON-001 — Re-importing an unchanged extract produces a report with zero added, updated, conflicted, and removed.
- [FAILING] RECON-002 — An OSM edge whose upstream geometry changed and which has no local edits is updated in place.
- [FAILING] RECON-003 — An OSM edge whose cross-section was locally edited is never overwritten; the upstream change is recorded as a conflict.
- [FAILING] RECON-004 — An OSM edge deleted upstream but locally edited is reported as removed-with-local-edits and retained.
- [FAILING] RECON-005 — An OSM edge deleted upstream with no local edits is removed.
- [FAILING] RECON-006 — Hand-drawn edges are never added to, updated in, or removed by a reconciliation report.
- [FAILING] RECON-007 — `reconcile()` returns lists of added, updated, unchanged, conflicted, and removed edge ids, and they are pairwise disjoint.
- [FAILING] RECON-008 — Node ids are preserved for OSM nodes that persist across imports.
- [FAILING] RECON-009 — Resolving a conflict as keep-local leaves the local cross-section byte-identical and clears the conflict.
- [FAILING] RECON-010 — Resolving a conflict as take-upstream replaces the cross-section and clears the conflict.
- [FAILING] RECON-011 — Reconciliation is idempotent: an immediate second run reports everything unchanged.
- [FAILING] RECON-012 — Reconciliation runs inside the save transaction; a mid-run failure leaves the city untouched.
- [FAILING] RECON-013 — An edge counts as locally edited only if its cross-section or geometry differs from the value last imported.

## WASM — client bindings and test seam

- [FAILING] WASM-001 — The module exports `init()` and it returns a handle without throwing.
- [FAILING] WASM-002 — `load_city_json(s)` accepts the API's city JSON and returns Ok.
- [FAILING] WASM-003 — `load_city_json` on malformed input returns a JS error with a readable message, leaving prior state intact.
- [FAILING] WASM-004 — `city_json()` returns a document that round-trips equal to what was loaded.
- [FAILING] WASM-005 — `scene_state()` returns camera, mode, selection, and per-visible-edge ids with screen-space bounds.
- [FAILING] WASM-006 — `scene_state()` is reachable from Playwright as `window.__cityloom.scene_state()`.
- [FAILING] WASM-007 — `scene_state()` reflects a selection made via `select_edge(id)`.
- [FAILING] WASM-008 — Rust panics surface as JS exceptions carrying the panic message, not bare `unreachable`.
- [FAILING] WASM-009 — `apply_op(json)` applies one authoring operation and returns the new document version.
- [FAILING] WASM-010 — `apply_op` with an unknown op kind returns an error and does not mutate the document.
- [FAILING] WASM-011 — The release wasm bundle is under 3 MB gzipped.
- [FAILING] WASM-012 — `init()` touches no DOM globals, so the module loads in a Web Worker.

## GL — WebGL2 renderer

- [FAILING] GL-001 — A WebGL2 context is acquired; failure renders a visible fallback message instead of a blank canvas.
- [FAILING] GL-002 — Every shader program compiles and links at init with no GL error.
- [FAILING] GL-003 — `gl.getError()` is 0 after a full frame.
- [FAILING] GL-004 — Street edges tessellate into triangle strips whose width comes from the cross-section total width.
- [FAILING] GL-005 — Each segment within a cross-section draws in its own kind-specific colour.
- [FAILING] GL-006 — Segment quads are positioned by cumulative offset, so they abut with no gap or overlap above 1e-4 m.
- [FAILING] GL-007 — Intersection polygons draw beneath street quads.
- [FAILING] GL-008 — Geometry buffers rebuild only for edges whose data changed since the last frame.
- [FAILING] GL-009 — An unchanged frame issues zero buffer uploads.
- [FAILING] GL-010 — The drawing buffer matches CSS size times device pixel ratio.
- [FAILING] GL-011 — Resizing the window updates viewport and projection with no aspect distortion.
- [FAILING] GL-012 — The selected edge draws with a highlight distinct from the unselected style.
- [FAILING] GL-013 — A hovered edge draws with a hover highlight distinct from both selected and unselected.
- [FAILING] GL-014 — Edges outside the viewport are culled and excluded from draw calls.
- [FAILING] GL-015 — `render_stats()` reports draw calls, triangle count, and culled edge count.
- [FAILING] GL-016 — 10,000 visible edges render at 60 fps on the reference machine.
- [FAILING] GL-017 — Street name labels render as glyph quads above a configured zoom threshold.
- [FAILING] GL-018 — Overlapping labels are culled so no two label boxes intersect.
- [FAILING] GL-019 — Teardown deletes every GL buffer, texture, and program it created.

## CAM — camera

- [FAILING] CAM-001 — Wheel zoom keeps the world point under the cursor fixed within 0.5 px.
- [FAILING] CAM-002 — Drag-pan moves the map by exactly the world delta under the cursor.
- [FAILING] CAM-003 — Zoom is clamped to the configured min and max scale.
- [FAILING] CAM-004 — Fit-to-bounds frames the whole network with the configured padding.
- [FAILING] CAM-005 — Fit-to-bounds on an empty network leaves the camera unchanged.
- [FAILING] CAM-006 — `scene_state()` reports camera centre and scale.
- [FAILING] CAM-007 — Camera state is encoded in the URL hash and restored on reload within 1e-6.
- [FAILING] CAM-008 — Arrow keys pan by a fixed screen-space step.
- [FAILING] CAM-009 — `+` and `-` zoom one step about the viewport centre.
- [FAILING] CAM-010 — Double-click zooms one step toward the clicked point.
- [FAILING] CAM-011 — No camera operation mutates the city document or its version.

## HIT — hit testing and selection

- [FAILING] HIT-001 — Clicking an edge selects it and `scene_state().selection` reports its id.
- [FAILING] HIT-002 — Clicking empty space clears the selection.
- [FAILING] HIT-003 — Clicking where geometry overlaps selects the topmost drawn feature.
- [FAILING] HIT-004 — Hit testing applies a 2 px screen-space tolerance converted to world units at the current zoom.
- [FAILING] HIT-005 — Shift-click adds an edge to the selection without removing existing members.
- [FAILING] HIT-006 — Shift-click on an already-selected edge removes it from the selection.
- [FAILING] HIT-007 — A rubber-band drag selects every edge intersecting the rectangle.
- [FAILING] HIT-008 — In draw mode, clicking near a node selects the node rather than the edge beneath it.
- [FAILING] HIT-009 — Selection survives pan and zoom unchanged.

## DRAW — hand-drawn authoring

- [FAILING] DRAW-001 — Clicking empty map in draw mode creates a node at the un-projected click position.
- [FAILING] DRAW-002 — A second click creates an edge joining the two nodes.
- [FAILING] DRAW-003 — Clicking an existing node while drawing connects to it instead of creating a duplicate.
- [FAILING] DRAW-004 — Clicking an existing edge while drawing splits it and connects to the new node.
- [FAILING] DRAW-005 — Escape during drawing cancels and leaves the network byte-identical.
- [FAILING] DRAW-006 — Dragging a node moves it and updates every incident edge's centreline endpoint.
- [FAILING] DRAW-007 — Dropping a node within the snap radius of another merges the two.
- [FAILING] DRAW-008 — A merge that would create a duplicate edge between the same pair collapses to one edge.
- [FAILING] DRAW-009 — Deleting a selected edge removes it and any node thereby left at degree 0.
- [FAILING] DRAW-010 — A newly drawn edge is assigned the configured default cross-section.
- [FAILING] DRAW-011 — Drawn positions snap to the configured grid when grid snap is on.
- [FAILING] DRAW-012 — Every hand-drawn node and edge carries `Authored` provenance.
- [FAILING] DRAW-013 — Undo reverts the last authoring operation, restoring the exact prior document.
- [FAILING] DRAW-014 — Redo reapplies the most recently undone operation.
- [FAILING] DRAW-015 — A new operation after an undo discards the redo stack.
- [FAILING] DRAW-016 — The undo stack retains at least 100 operations.
- [FAILING] DRAW-017 — Undo across a save does not resurrect deleted rows on the next save.

## XS — cross-section editor

- [FAILING] XS-001 — Selecting an edge opens the cross-section editor bound to that edge's cross-section.
- [FAILING] XS-002 — The editor lists segments left to right in model order.
- [FAILING] XS-003 — Dragging a segment to a new index reorders it, and `city_json()` reflects the new order.
- [FAILING] XS-004 — Editing a segment's width updates the displayed total width live.
- [FAILING] XS-005 — A negative or non-numeric width input is rejected and the model is unchanged.
- [FAILING] XS-006 — Deleting a segment reduces total width by exactly that segment's width.
- [FAILING] XS-007 — A segment can be added from a palette covering every supported kind.
- [FAILING] XS-008 — A segment's direction can be toggled where its kind allows a direction.
- [FAILING] XS-009 — The direction toggle is absent for kinds with no meaningful direction.
- [FAILING] XS-010 — The editor shows total width and its difference from the edge's right-of-way width.
- [FAILING] XS-011 — Total width exceeding the right-of-way raises a visible warning.
- [FAILING] XS-012 — A bike lane directly adjacent to a drive lane with no buffer raises a warning.
- [FAILING] XS-013 — A sidewalk narrower than the configured minimum raises a warning.
- [FAILING] XS-014 — A cross-section with no sidewalk on either side raises a warning.
- [FAILING] XS-015 — Warnings clear as soon as the condition that raised them is resolved.
- [FAILING] XS-016 — Editing a shared cross-section updates every referencing edge in the next rendered frame.
- [FAILING] XS-017 — "Make unique" detaches the cross-section for the selected edge only, leaving others unchanged.
- [FAILING] XS-018 — Cross-section edits push onto the same undo stack as authoring edits.
- [FAILING] XS-019 — A cross-section can be saved into the city library under a name and applied to another edge.
- [FAILING] XS-020 — Opening and closing the editor with no interaction leaves the document version unchanged.

## DOC — document lifecycle

- [FAILING] DOC-001 — "New city" creates a city and navigates to its `/c/{slug}` URL.
- [FAILING] DOC-002 — Loading `/c/{slug}` fetches and renders that city.
- [FAILING] DOC-003 — An unknown slug renders a not-found page, not a blank canvas.
- [FAILING] DOC-004 — Any edit marks the document dirty and shows an unsaved indicator.
- [FAILING] DOC-005 — Save issues a PUT and clears the dirty indicator on 200.
- [FAILING] DOC-006 — A failed save keeps the dirty state and surfaces a retryable error.
- [FAILING] DOC-007 — A 409 on save shows a conflict message offering reload.
- [FAILING] DOC-008 — Autosave fires at most once per 5 s while the document is dirty.
- [FAILING] DOC-009 — Autosave does not fire while the document is clean.
- [FAILING] DOC-010 — Navigating away with unsaved changes prompts for confirmation.
- [FAILING] DOC-011 — The share action copies the current URL including the camera hash.
- [FAILING] DOC-012 — A city can be renamed and the new name persists across reload.

## UI — application chrome

- [FAILING] UI-001 — Tailwind is compiled to one served stylesheet; no CDN script or stylesheet tag appears in the HTML.
- [FAILING] UI-002 — The layout is a full-viewport canvas with a left tool rail and a right inspector.
- [FAILING] UI-003 — The inspector shows an empty state when nothing is selected.
- [FAILING] UI-004 — The tool rail offers select, draw, and import modes with the active one visually marked.
- [FAILING] UI-005 — Pressing `v` switches to select mode and `d` to draw mode.
- [FAILING] UI-006 — The import dialog accepts a bbox and starts an import job.
- [FAILING] UI-007 — Import job progress is polled and displayed until a terminal state.
- [FAILING] UI-008 — A failed import surfaces its error message in the dialog.
- [FAILING] UI-009 — A reconciliation report is shown after importing into a non-empty city.
- [FAILING] UI-010 — Each conflict is listed with keep-local and take-upstream actions.
- [FAILING] UI-011 — Acting on a conflict removes it from the list.
- [FAILING] UI-012 — The app is usable at 1280x800 with no horizontal page scroll.
- [FAILING] UI-013 — Every interactive control is reachable via keyboard tab order.
- [FAILING] UI-014 — The canvas has an accessible label describing it as the city map.

## TEST — test infrastructure

- [PASSING] TEST-001 — `cargo nextest run` executes all Rust unit and integration tests.
- [FAILING] TEST-002 — A pytest fixture provisions an isolated Postgres database per test and drops it afterwards.
- [FAILING] TEST-003 — A pytest fixture starts the server on an ephemeral port and waits for `/healthz` before yielding.
- [FAILING] TEST-004 — The server fixture tears the process down even when a test fails.
- [FAILING] TEST-005 — Playwright fixtures launch headless chromium and expose a `page`.
- [FAILING] TEST-006 — A helper reads `window.__cityloom.scene_state()` and returns it as a dict.
- [FAILING] TEST-007 — A helper waits for the first rendered frame before assertions run.
- [FAILING] TEST-008 — OSM fixtures are checked into the repo; no test reaches the network.
- [FAILING] TEST-009 — The whole suite passes with outbound network access blocked.
- [FAILING] TEST-010 — Each test declares the feature ids it covers via a marker.
- [FAILING] TEST-011 — `just features` reports, per feature id, whether a test claims it.
- [FAILING] TEST-012 — `just verify` fails when a feature marked `[PASSING]` has no test claiming it.
