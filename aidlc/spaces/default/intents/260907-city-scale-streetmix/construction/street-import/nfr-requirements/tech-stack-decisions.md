# Tech Stack Decisions — `street-import` (U4)

_Confirmed._

Upstream inputs: `functional-spec.md`, `rules.md` (functional-design, this
Unit); `components.md`; `unit-of-work.md` U4.

## Decisions

- **Language/crate**: Rust, a new workspace crate (`cityloom-street-import`),
  compiled to `wasm32-unknown-unknown` as part of the client bundle (this
  Unit's deployment model is "embedded — a crate in the client Cargo
  workspace", per `unit-of-work.md`). Reuses the workspace's existing
  `rust-toolchain.toml` (rustc/cargo 1.97.1).
- **osm2streets**: consumed via the existing pinned git dependency U1
  (`u1-osm2streets-build`) already added to the workspace — this crate adds
  `osm2streets = { workspace = true }` (or the equivalent path reference to
  the already-pinned dependency), never a second independent pin. This is
  the only crate in the workspace permitted to depend on it directly
  (`team.md` Code Style's three-inward-pointing-layers rule) — `street-core`
  (U2) has zero osm2streets dependency, and this Unit is the adapter layer
  that rule names.
- **`street-core` (U2)**: a normal workspace dependency — this Unit
  constructs `Street`/`Lane`/`StreetNetworkGraph` values (owned by U2) and
  implements U2's `StreetSource` port (Contract 8).
- **HTTP fetch — `gloo-net`, not a native async HTTP client.** This crate
  runs inside a browser tab under `wasm32-unknown-unknown`; a
  native-runtime HTTP client (`reqwest` with its default Tokio-based
  transport, `ureq`) does not compile to that target without a browser-fetch
  backend. `gloo-net`'s `http` module wraps the browser's native `fetch()`
  via `web-sys`, is actively maintained, and is the de facto standard for
  WASM-target HTTP in the Rust ecosystem — it adds no native-runtime
  dependency and no bundle-size cost beyond a thin `web-sys` binding.
  `ExtractFetcher` uses it for the single `GET /api/extract` call (Contract
  1).
- **Serialization — `serde` + `serde_json`, both real (runtime)
  dependencies here, unlike `design-payload-spec`'s test-only use.** This
  Unit parses Contract 1's JSON error bodies (`ApiError`, `FailureReason`,
  `ReloadRequired`) and the raw extract bytes are handed to osm2streets
  as-is (not JSON) — `serde`/`serde_json` here is for the HTTP contract's
  error shapes, reusing the generated types from `cityloom-api-types`
  (the shared crate `osm-extract-proxy`, U9, already publishes — see below)
  rather than hand-rolling a second copy of `FailureReason`.
- **`cityloom-api-types`**: a normal workspace dependency. `osm-extract-proxy`
  (U9) already generates this crate from Contract 1's OpenAPI spec
  (`docs/dependencies.md` records it); this Unit reuses its `FailureReason`
  and `ApiError` types directly rather than redefining them, so the two
  ends of Contract 1 cannot drift.
- **No dependency on `design-payload-spec` (U3).** This Unit's `Correction`
  and `ImportFingerprint` (per `entities.md`, this stage) are this Unit's
  own live, in-memory types — U3's types of the same name are the wire
  shape a different Unit (`local-persistence`, U7) serializes through at
  the persistence boundary. `unit-of-work-dependency.md`'s graph does not
  place an edge from U4 to U3.
- **Error handling**: a closed `ImportFailure` enum (this crate's own type,
  distinct from `cityloom-api-types::FailureReason` which it wraps/reuses
  for the fetch-classified reasons) — `thiserror` is used for the
  `std::error::Error` impl boilerplate, keeping the dependency minimal
  (already a common pattern available in this workspace; confirmed
  reasonable given the closed, typed-failure discipline `team.md` Code
  Style requires).

## Rationale

Reusing `cityloom-api-types` and the already-pinned `osm2streets` dependency
means this Unit adds exactly two new runtime dependencies beyond the
existing workspace (`gloo-net` for fetch, `thiserror` for ergonomic error
types) plus `street-core` and `cityloom-api-types` as workspace-internal
dependencies — no new third-party surface for the adapter boundary itself
beyond what U1 already pinned.

## Assumptions & Open Questions

- **[assumption]** `cityloom-api-types` (U9's generated crate) already
  exposes `FailureReason` and `ApiError` in a form this Unit can import
  directly; if U9's actual generated shape differs from Contract 1's
  documented shape, code-generation reconciles against the real crate
  rather than the contract summary, per the project's own "read the
  source, not the summary" practice (`project.md` Corrections).

## Traceability

See `traceability.json` in this directory.
