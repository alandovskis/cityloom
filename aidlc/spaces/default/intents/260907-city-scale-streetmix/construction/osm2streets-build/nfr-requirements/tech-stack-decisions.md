# Tech Stack Decisions — `osm2streets-build` (U1)

_Confirmed._

Upstream inputs: `requirements.md` Constraints (Rust/WASM client); `team.md`
Deployment (osm2streets consumed as a direct Cargo dependency, built from
source, pinned); `unit-of-work.md` U1.

## Decisions

- **Language/build**: Rust, consumed as an ordinary Cargo workspace member
  dependency (not `osm2streets-js`/wasm-bindgen). Source: `team.md`
  Deployment — the affirmation-gate decision to build the client itself in
  Rust/WASM means osm2streets crosses no JS/WASM boundary; both compile into
  the same Rust binary.
- **Dependency mechanism**: `git` dependency in the client's root
  `Cargo.toml` (or a dedicated adapter crate's `Cargo.toml` — the exact
  crate that declares it is `u4-street-import`'s adapter, per
  `team.md`'s "only the adapter may depend on the osm2streets crate
  directly"; this Unit owns the *fork and pin*, not the dependency
  declaration site). `rev = "<sha>"`, no `branch`/`tag`.
- **Toolchain versions**: pinned via `rust-toolchain.toml` at the workspace
  root — the same file `osm-extract-proxy` (U9) already established at
  `rustc 1.97.1`/`cargo 1.97.1` (confirmed installed on the build machine,
  per that Unit's `code-generation-plan.md`). This Unit reuses that pin
  rather than introducing a second one: one Cargo workspace, one toolchain
  file, consistent with `unit-of-work.md`'s "embedded — compiled into the
  client artifact" deployment model (U1 is not a separate crate with its
  own toolchain; it *is* the pinned git dependency plus its provenance
  record, consumed by whichever workspace member needs it).
- **WASM build tool**: not yet selected — Domain Design's open constraint
  (`team.md` Testing Posture) defers the DOM-framework choice (Leptos, Yew,
  Dioxus) to whichever Unit selects the client's UI framework (`u6-client-surfaces`),
  and the WASM build tool (`wasm-pack` vs. `trunk`) follows that framework
  choice, not this Unit's. This Unit's own requirement (`unit-of-work.md`)
  is only to record whichever tool is chosen in the provenance file —
  it does not choose it.
- **Golden-fixture toolchain**: the committed OSM extract fixtures
  (`team.md` Testing Posture — "a well-tagged street, a thinly-tagged
  street, a one-way, a street with a separately mapped cycleway, one
  intersection") are authored in this Unit and consumed by `u4-street-import`.
  No new tooling beyond `cargo test` is needed to run them; they are plain
  committed data files exercised by ordinary Rust tests in the consuming
  Unit.
- **Dependency scanning**: `cargo audit` (already adopted project-wide,
  `team.md` Deployment) covers this Unit's pinned dependency tree once it
  is in the lockfile — no separate tool.
- **Provenance record format**: a single committed Markdown file (not a
  machine-readable format) — `docs/dependencies.md` (already created by
  U9, per `team.md`'s "`CLAUDE.md` is brought in line with reality" item)
  is the natural home for the pinned-commit record, upstream commit,
  build command, and toolchain versions, rather than a new file. This
  keeps one asset-and-dependency manifest instead of two.

## Rationale

Every one of these decisions is already settled by `team.md`/`project.md`
Mandated rules or by the prior Unit's (`osm-extract-proxy`) established
workspace conventions; this stage records them against this Unit rather
than re-deciding them, per the strict-additive rule model — a narrower
stage does not re-litigate a broader affirmed practice.

## Assumptions & Open Questions

- **[Q1]** Confirm at code-generation time: reuse the existing root
  `Cargo.toml`/`rust-toolchain.toml` from `osm-extract-proxy`'s workspace
  bootstrap (recommended — one workspace, one toolchain), rather than a
  second, separate Cargo workspace for the client. `unit-of-work.md`
  describes U1 as "embedded — compiled into the client artifact", which
  implies a single client workspace; `osm-extract-proxy` is a *separate*
  server-side crate in the *same* Cargo workspace (per `team.md`'s
  three-ring layering applying "inside" the proxy too). Recommendation:
  one Cargo workspace at the repository root containing both the server
  (`osm-extract-proxy`, `cityloom-api-types`) and, once built, the client
  crates — confirmed at Domain Design's already-settled layering, not
  reopened here.

## Traceability

See `traceability.json` in this directory.
