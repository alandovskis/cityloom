# Security Requirements — `street-core` (U2)

_Confirmed._

Upstream inputs: `functional-spec.md`, `rules.md` (functional-design, this
Unit); `requirements.md` NFR6 (requirements-analysis).

This Unit is a pure domain-model library: no network, no I/O, no user
input parsing, no secrets, no persistence. Its threat surface is
correspondingly narrow — the risks that matter are **type-safety and
crate-boundary violations that would let a bug elsewhere corrupt the
imported baseline or leak an internal representation**, not the OWASP
Top 10 categories that apply to a networked service.

## Threat model (STRIDE, scoped to this Unit)

| Category | Applicable? | Reasoning |
|---|---|---|
| Spoofing | N/A | No identity, no caller authentication concept |
| **Tampering** | **Yes (narrow)** | Mutable access to the imported baseline from outside this Unit would let another component silently corrupt data any downstream reader trusts as "as imported" |
| Repudiation | N/A | No user-attributable action |
| **Information Disclosure** | **Yes (narrow)** | Leaking an osm2streets-native type past this Unit's boundary would defeat the whole adapter-isolation design (`team.md` three-ring layering) |
| Denial of Service | N/A | No request path, no resource exhaustion surface |
| Elevation of Privilege | N/A | No authorization surface |

## Requirements

- **NFR6.4.1** No component outside this Unit shall obtain a mutable
  reference into the imported baseline (`Street`, `Lane`,
  `StreetNetworkGraph`) — this Unit's public API exposes only shared
  references and owned copies. Source: `rules.md` BR5.1; `team.md` Code
  Style ("the osm2streets-derived baseline is held behind shared, never
  mutable, references").
- **NFR6.4.2** This Unit's crate shall have zero dependency on the
  osm2streets crate, any map-rendering crate, any UI-framework crate, or
  any jurisdiction-specific constant — enforced at the Cargo dependency-graph
  level, not by review. Source: `team.md` Code Style, "three inward-pointing
  layers"; `rules.md` BR7.1.
- **NFR6.4.3** No `unsafe` Rust code in this crate — a pure data model has
  no legitimate reason to bypass Rust's memory-safety guarantees, and
  permitting it here would be the one place a memory-safety bug could
  enter the entire client's core.
- **NFR7.2.6** (inherits `requirements.md` NFR7.2's pinning obligation by
  extension) — this Unit itself introduces no new dependency to pin; it
  depends on nothing beyond the Rust standard library. Recorded here so a
  later dependency addition is a conscious NFR-visible decision, not a
  silent one.
- **NFR6.1.2** (inherits `requirements.md` NFR6.1 — no secret in the
  repository) N/A for this Unit specifically: it holds no configuration,
  no credential, and no environment-derived value of any kind.

## Out of scope for this Unit

Authentication/authorization (no service), encryption at rest/in transit
(no data held or transmitted), rate limiting (no request path), and
injection classes (no external input parsed here — parsing untrusted OSM
data is `u4-street-import`'s adapter boundary, not this Unit's).

## Assumptions & Open Questions

None.

## Traceability

See `traceability.json` in this directory.
