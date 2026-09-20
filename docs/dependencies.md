# Dependency and asset manifest

The committed manifest `team.md` (Code Style) names: origin and licence for
every direct dependency and every third-party asset, updated whenever one is
added. `scripts/verify.sh` fails the gate if a direct dependency of any
workspace crate has no row here, if a vendored asset is missing, or if
`Cargo.lock` or this file names the AGPL street-editor `project.md` forbids as a dependency (its name is deliberately not written here, so the gate can grep for it).
Transitive crates are listed by `Cargo.lock` itself and audited by
`cargo audit` (slow CI tier).

Versions are exact pins (`=x.y.z`); every one was the newest stable release
on crates.io on 2026-09-15 (the versions `tech-stack-decisions.md` read on
2026-09-12 were the floor). Origin is crates.io unless stated.

## Direct dependencies

| Crate | Version | Licence | Used by | Purpose |
|---|---|---|---|---|
| `cityloom-api-types` | 0.1.0 (workspace path) | — (this repository) | osm-extract-proxy | Contract 1 wire types (`FailureReason`, `ApiError`, `ReloadRequired`), owned by this Unit |
| `cityloom-design-payload` | 0.1.0 (workspace path) | — (this repository) | (leaf crate, not yet consumed) | The wire/storage shape for a design payload — the one serialised structure crossing the device/server, Rust/database, and export boundaries (Contract 3, Unit U3) |
| `axum` | 0.8.9 | MIT | osm-extract-proxy; cityloom-design-storage | HTTP router, handlers, typed responses (TS-1). Default features off except `http1`, `tokio`, `json` and `query` (the `bbox` query parameter is read through `Query<HashMap<..>>`; no `form` extractor). `cityloom-design-storage` mounts `/api/designs*` and `/readyz` (Unit U10) |
| `tokio` | 1.53.1 | MIT | osm-extract-proxy; cityloom-design-storage | async runtime, `timeout` for the request budget, `Semaphore` for the cutting slots, signals; `cityloom-design-storage` also drives its in-process expiry-sweep background task (Unit U10) |
| `tower` | 0.5.3 | MIT | osm-extract-proxy; cityloom-design-storage (dev) | `Service`/`Layer`; `ServiceExt::oneshot` drives the router in tests |
| `tower-http` | 0.7.1 | MIT | osm-extract-proxy; cityloom-design-storage | `set-header` (the fixed response header set, SD-6) and `catch-panic` (BR10.4); its trace layer is **not** used (NFR6.3.1). `cityloom-design-storage` uses only `catch-panic`, to answer a handler panic as `internal` rather than tearing down the shared process (Unit U10) |
| `hyper` | 1.11.1 | MIT | osm-extract-proxy | HTTP/1 server connection with header size and read-timeout limits (NFR6.4.4) |
| `hyper-util` | 0.1.20 | MIT | osm-extract-proxy | the server connection builder, graceful shutdown (RD-7), tokio adapters |
| `http` | 1.5.0 | MIT OR Apache-2.0 | osm-extract-proxy | request/response/header types |
| `serde` | 1.0.229 | MIT OR Apache-2.0 | cityloom-api-types, osm-extract-proxy, cityloom-design-payload, cityloom-design-storage | Contract 1 types, manifest and counters JSON, design-payload derives (the only runtime dependency of `cityloom-design-payload`; its own hand-rolled `json_reader` module implements `serde::Deserializer` for the fail-closed version check, `NEVER` taking `serde_json` as a runtime dependency of that crate); `cityloom-design-storage`'s Contract 2 request/response types (Unit U10) |
| `serde_json` | 1.0.151 | MIT OR Apache-2.0 | osm-extract-proxy; cityloom-api-types (tests); cityloom-design-payload (tests only); cityloom-design-storage | JSON rows and manifest; design-payload round-trip tests (dev-dependency only — `tech-stack-decisions.md` defers the actual wire encoding to `u10-design-storage`); `cityloom-design-storage`'s upload-envelope parsing and JSON log rows (Unit U10, now consuming this deferral) |
| `osmpbf` | 0.3.8 | MIT OR Apache-2.0 | osm-extract-proxy | reading regional extracts and cells (TS-2); round-trip check of the project encoder |
| `protobuf` | 3.7.2 | MIT | osm-extract-proxy | runtime for the generated PBF types (TS-3). The `protobuf` crate name also carries Google's 4.x line (pre-release-tagged, `protoc`-based, BSD-3); 3.7.2 is rust-protobuf, the newest stable and the one with the pure-Rust parser TS-3 requires |
| `protobuf-codegen` | 3.7.2 | MIT | osm-extract-proxy (build) | generates the PBF types from `crates/osm-extract-proxy/proto/` at build time with the pure-Rust parser — no `protoc` |
| `flate2` | 1.1.10 | MIT OR Apache-2.0 | osm-extract-proxy | zlib blobs in the PBF encoder (TS-3); pure-Rust backend (`miniz_oxide`), shared with `osmpbf` |
| `moka` | 0.12.16 | (MIT OR Apache-2.0) AND Apache-2.0 | osm-extract-proxy | the extract cache: LRU by byte weight, single-flight `try_get_with` (TS-4) |
| `blake3` | 1.8.7 | CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception | osm-extract-proxy | cell digests, `buildId`, extract key (TS-5) |
| `md-5` | 0.11.0 | MIT OR Apache-2.0 | osm-extract-proxy (build tool) | the publisher's `.md5` checksum, and nothing else (TS-5) |
| `tracing` | 0.1.44 | MIT | osm-extract-proxy; cityloom-design-storage | the catalogued events (OD-1..OD-5); `cityloom-design-storage`'s `record_failure`/`record_sweep_cycle` rows (Unit U10) |
| `tracing-subscriber` | 0.3.23 | MIT | osm-extract-proxy; cityloom-design-storage | JSON rows to stdout, span events off (TS-6); same shape reused for `cityloom-design-storage`'s own subscriber (Unit U10) |
| `reqwest` | 0.13.5 | MIT OR Apache-2.0 | osm-extract-proxy (build tool) | region download over TLS with certificate verification (TS-7); `rustls` (aws-lc-rs provider, platform root store), blocking client, streamed to disk |
| `thiserror` | 2.0.20 | MIT OR Apache-2.0 | osm-extract-proxy; cityloom-design-storage | typed error enums at every boundary (`team.md`, typed results); `StorageFailure`'s typed reason enum (Unit U10) |
| `toml` | 1.1.6 | MIT OR Apache-2.0 | osm-extract-proxy | `data/regions.toml` (build configuration) and `data/current-build.toml` (the pointer) |
| `tempfile` | 3.27.0 | MIT OR Apache-2.0 | osm-extract-proxy | the build's temporary output directory (RD-8); test directories |
| `http-body-util` | 0.1.5 | MIT | osm-extract-proxy (tests) | collecting response bodies in the `oneshot` tests |
| `futures` | 0.3.34 | MIT OR Apache-2.0 | osm-extract-proxy (tests); cityloom-street-import | joining concurrent requests in the slot tests; `ExtractFetcher`'s fetch-vs-timeout race (`futures::future::select`, SD-4) |
| `cityloom-street-import` | 0.1.0 (workspace path) | — (this repository) | (leaf crate, not yet consumed) | The request-to-corrected-street pipeline: fetch, osm2streets conversion, correction reconciliation (Unit U4) |
| `street-core` | 0.1.0 (workspace path) | — (this repository) | cityloom-street-import | The imported street/lane baseline domain model — `Street`, `Lane`, `Dimension`, `Provenance`, `StreetNetworkGraph`, the `StreetSource` port (Unit U2). (Named `cityloom-street-core` in `code-generation-plan.md` Step 1 — the actual crate `u2-street-domain-model` published is `street-core`; this row uses the real name.) |
| `osm2streets` | 0.1.0 (git, pinned to commit `fc119c47dac567d030c6ce7c24a48896f58ed906`) | Apache-2.0 | cityloom-street-import | The street/lane import model this project's editor is built around; converts raw OSM ways into roads, lanes and intersections. The **only** component permitted to depend on it directly is `StreetImportAdapter` (`security-design.md` SD-3); see `docs/osm2streets-pin.md` for the full pin record |
| `streets_reader` | 0.1.0 (git, same repository and pinned commit as `osm2streets` above) | Apache-2.0 | cityloom-street-import | OSM XML/PBF parsing into a `StreetNetwork` (`osm_to_street_network`) — lives in the same upstream repository as `osm2streets` itself, which operates only on an already-built `StreetNetwork` and has no parser of its own. Not anticipated by `docs/osm2streets-pin.md` (written before this Unit existed); recorded here as this Unit's own addition, pinned identically |
| `abstutil` | 0.1.0 (git, `https://github.com/a-b-street/abstreet`, **deliberately declared without a `rev =`** — see below) | Apache-2.0 | cityloom-street-import, transitively via `osm2streets`/`streets_reader` | `Timer`, used to drive `osm_to_street_network`. `osm2streets`'s own transitive dependency; upstream's own `Cargo.toml` leaves it unpinned. **Deviation, verified during this Unit's build:** declaring this crate's own dependency with an explicit `rev = "0964f29315820c91b171b585eb51e300164e9197"` (matching `docs/osm2streets-pin.md`'s stated pin) resolves to a *second*, independent `abstutil` package — a different Cargo `SourceId` from the one `osm2streets`/`streets_reader` themselves pull in unpinned — so a `Timer` built from one does not type-check where the other expects it (`E0308`, reproduced and confirmed against this exact dependency set). This crate's `abstutil` line is therefore bare `git = "..."`, matching upstream's own unpinned form exactly so Cargo resolves a single package; the pin itself is enforced one layer down, by the committed `Cargo.lock` (which does record the resolved commit, `0964f29315820c91b171b585eb51e300164e9197` — today's live upstream HEAD, matching the intended pin) and `cargo build --locked` (`scripts/verify.sh`), which fails hard rather than silently re-resolving to a later commit. See `docs/osm2streets-pin.md` for the full record and follow-up |
| `thiserror` | 2.0.20 | MIT OR Apache-2.0 | osm-extract-proxy; cityloom-street-import | typed error enums at every boundary (`team.md`, typed results); `ImportFailure`'s `Display`/`std::error::Error` impl |
| `gloo-net` | 0.6.0 | MIT OR Apache-2.0 | cityloom-street-import | `GlooExtractTransport`'s browser `fetch` call (Contract 1, GET only — BR7.1) |
| `gloo-timers` | 0.3.0 | MIT OR Apache-2.0 | cityloom-street-import | `GlooTimeoutSource`'s timeout-budget future (`security-design.md` SD-4) |
| `sqlx` | 0.9.0 | MIT OR Apache-2.0 | cityloom-design-storage | Compile-time-checked Postgres queries (`security-design.md` SD-5), `postgres`/`runtime-tokio`/`tls-rustls`/`macros`/`migrate`/`time` features only, no ORM (Unit U10) |
| `time` | 0.3.55 | MIT OR Apache-2.0 | cityloom-design-storage | Timestamp handling for `stored_designs` (`created_at`/`expires_at`) and RFC 3339 response formatting (Unit U10) |

## osm2streets/abstutil pin — now consumed

`osm2streets-build` (U1) recorded the pin in `docs/osm2streets-pin.md` ahead
of any crate consuming it; `u4-street-import` (`cityloom-street-import`,
above) is that crate. `docs/osm2streets-pin.md` is updated with this Unit's
build record, including the `abstutil`-pinning deviation described in that
crate's row above. Both `osm2streets` and `streets_reader` are consumed from
`a-b-street/osm2streets` directly as an **interim measure** — the project's
own fork (`security-design.md` SD-1) does not exist yet; see
`docs/osm2streets-pin.md` "Fork status" for the exact, named follow-up.
Neither dependency's Apache-2.0 licence is the AGPL street-editor
`project.md` forbids; osm2streets was never that licence risk
(`code-generation-plan.md`, this Unit).

## Vendored assets

| Asset | Origin | Licence | Recorded in |
|---|---|---|---|
| `fileformat.proto` | github.com/openstreetmap/OSM-binary, `osmpbf/fileformat.proto`, commit `9f01ad3ece9737503a059d58749e02c9008d5699` | MIT (the licence in the file's own header; the repository's LGPL-3.0 covers its C++ library, none of which is vendored) | `crates/osm-extract-proxy/proto/UPSTREAM`, `LICENSE` |
| `osmformat.proto` | same repository, `osmpbf/osmformat.proto`, same commit | MIT (same) | same |

No icon, image or font is part of this Unit.

## Not dependencies, by decision

- Nothing from the AGPL street-editor `project.md` forbids
  (`constraint-register.md` LC-3) — `scripts/verify.sh` greps this file and
  `Cargo.lock` for its name and fails the gate on a hit.
- No osm2streets crate: the proxy cuts the bytes U1's fixtures are made
  from; it does not read them (`tech-stack-decisions.md`).
- No `tower_governor`/`governor` for limiting (TS-8), no `prost` (TS-3), no
  `osm-io` writer (TS-3), no metrics endpoint crate (TS-6).
