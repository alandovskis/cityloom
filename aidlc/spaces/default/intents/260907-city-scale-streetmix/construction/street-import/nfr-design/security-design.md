# Security Design — `street-import` (U4)

_Confirmed._

Upstream inputs: `security-requirements.md`, `tech-stack-decisions.md`
(nfr-requirements, this Unit); `functional-spec.md` (functional-design,
this Unit).

## Design elements

- **SD-1 — Closed `ImportFailure` mapping.** `StreetImportAdapter` wraps
  every call into the osm2streets crate in `std::panic::catch_unwind`
  (osm2streets is a third-party crate this Unit does not control, and Rust
  panics unwinding across the WASM/JS boundary abort the whole module
  unless caught first — a DoS-adjacent failure mode worse than one failed
  import). A single translation function maps every `Result::Err` the
  crate returns, and every caught panic, into one of the closed
  `ImportFailure` reasons; no branch constructs an `ImportFailure` from
  raw dependency text. The underlying `std::panic::PanicHookInfo` message
  or `Result::Err` `Display` output is captured once into the local log
  (SD-2) and discarded from every return value. Meets NFR6.4.1.
- **SD-2 — Local-only failure log, no transmission.** The local log this
  Unit writes (street identifier, `FailureReason`, captured underlying
  error text) is an in-memory or browser-console sink scoped to this
  crate — it is never serialized, never sent over `gloo-net`, and never
  written to any storage this Unit or any other Unit reads back. This is
  what makes "local to this component" (`components.md`) a structural
  guarantee rather than a documentation claim: there is no code path in
  this crate that transmits it. Meets NFR6.4.2.
- **SD-3 — osm2streets consumed only through the existing pin.** This
  crate's `Cargo.toml` references the workspace's already-pinned
  `osm2streets` dependency (established by `u1-osm2streets-build`) via a
  workspace dependency entry, never a second `[dependencies.osm2streets]`
  table with its own `git`/`rev`. A `cargo metadata` check in CI (owned by
  this Unit's own `code-generation` verification step, not a new
  infrastructure component) asserts exactly one resolved `osm2streets`
  package in the dependency graph. Meets NFR6.4.3.
- **SD-4 — Untrusted extract bytes, bounded before conversion.**
  `ExtractFetcher` enforces the timeout budget (BR1.1) using a
  WASM-compatible timer (`gloo-timers`, consistent with the `gloo-net`
  choice in `tech-stack-decisions.md`) wrapping the fetch future; a
  response exceeding the budget is abandoned and classified `timeout`
  before osm2streets ever sees the bytes. `StreetImportAdapter` additionally
  bounds the byte length it will attempt to convert — Contract 1's
  `area_too_large` failure reason exists precisely so an oversized
  extract is rejected before conversion, not mid-conversion. Meets
  NFR6.4.4.
- **SD-5 — No secret to manage.** Confirmed by inspection of Contract 1:
  the only request header (`x-client-build`) is a build identifier, not a
  credential, and this Unit's HTTP call carries no `Authorization` header
  or API key. Nothing to design here beyond the confirmation itself.
  Meets NFR6.1.3.

## Threats not applicable here

See `security-requirements.md`'s STRIDE table — Spoofing, Repudiation, and
Elevation of Privilege remain N/A for the reasons stated there; Tampering's
transport-level half (HTTPS) is out of this Unit's scope
(`environment-provisioning`'s concern).

## Assumptions & Open Questions

None.

## Traceability

See `traceability.json` in this directory.
