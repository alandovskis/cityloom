# Security Design — `street-core` (U2)

_Confirmed._

Upstream inputs: `security-requirements.md`, `tech-stack-decisions.md`
(nfr-requirements, this Unit). Design elements are numbered `SD-n`.

## Design elements

- **SD-1 — No mutable public API into the baseline.** `Street`, `Lane`,
  and `StreetNetworkGraph` expose only `&self` methods and `Arc<...>` or
  `&...`-typed accessors; the crate declares no `&mut self` method on any
  of the three, and no public constructor accepts a mutable borrow of an
  already-constructed instance. Meets NFR6.4.1.
- **SD-2 — Crate-level dependency allowlist.** This crate's `Cargo.toml`
  declares zero dependencies beyond the Rust standard library (confirmed
  in `tech-stack-decisions.md`); `cargo tree -p street-core` run in CI (as
  part of the existing dependency-manifest check `osm-extract-proxy`
  established) would show a single-node tree, making an accidental
  osm2streets/UI-framework dependency immediately visible in any future
  diff. Meets NFR6.4.2.
- **SD-3 — No `unsafe`.** `#![forbid(unsafe_code)]` at the crate root.
  This is a compile-time-enforced guarantee, not a review-time
  convention — a later PR cannot silently introduce `unsafe` even by
  accident. Meets NFR6.4.3.
- **SD-4 — Dependency-free by construction.** No design element is
  needed beyond SD-2 itself: there is no dependency to pin because there
  is no dependency. Meets NFR7.2.6.

## Threats not applicable here

See `security-requirements.md`'s STRIDE table — Spoofing, Repudiation,
Denial of Service and Elevation of Privilege remain N/A for the same
reasons stated there (no identity, no user action, no request path, no
authorization surface).

## Assumptions & Open Questions

None.

## Traceability

See `traceability.json` in this directory.
