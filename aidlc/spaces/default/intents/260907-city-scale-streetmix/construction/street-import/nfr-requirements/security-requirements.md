# Security Requirements — `street-import` (U4)

_Confirmed._

Upstream inputs: `functional-spec.md`, `rules.md` (functional-design, this
Unit); `requirements.md` NFR6; `contract-summary.md` Contract 1.

## STRIDE pass

| Threat | Applicable? | Reasoning |
|---|---|---|
| Spoofing | N/A | This Unit performs no authentication of its own; it is a client-side library with no identity concept. |
| Tampering | Partial | Extract bytes travel over the extract-fetch call (Contract 1); transport-level protection (HTTPS) is the deployment's concern (`environment-provisioning`), not this Unit's. What this Unit owns is treating the bytes as untrusted once received — see NFR6.4.4. |
| Repudiation | N/A | No user-attributable action this Unit performs requires non-repudiation; corrections are attributed to no account (Stage 1 has none) and OSM writes are forbidden outright (BR7.1). |
| Information Disclosure | **Yes** | The adapter runs a third-party crate (osm2streets) whose native errors and any panic message could leak internal paths, dependency versions, or implementation detail if allowed to reach a caller. See NFR6.4.1. |
| Denial of Service | **Yes** | A malformed or adversarially large extract could make osm2streets' conversion pathologically slow or cause unbounded memory use inside the browser tab. See NFR6.4.4 and BR1.1 (timeout). |
| Elevation of Privilege | N/A | No privilege boundary exists inside this Unit. |

## Detailed requirements

- **NFR6.4.1 — No dependency-native error text reaches a caller.** Every
  `ImportFailure` this Unit constructs carries only the closed `FailureReason`
  enum and a project-authored `detail` string; the underlying osm2streets
  error (including any panic message this Unit catches via
  `std::panic::catch_unwind` or an equivalent boundary) is logged once
  locally with the street identifier and never included in the value
  returned to a caller. Meets BR3.1, AC7.1.2.
- **NFR6.4.2 — The local failure log stores nothing that identifies a
  person.** The log entry (street identifier, `FailureReason`, underlying
  error text) contains no browser fingerprint, IP address, or account
  identifier — this Unit has no account concept in Stage 1, and the log is
  local to the browser tab, never transmitted. Meets AC7.4.3, NFR6.3.
- **NFR6.4.3 — osm2streets is consumed only through the pin `u1-osm2streets-build`
  establishes.** This Unit's `Cargo.toml` depends on the `osm2streets` crate
  via the workspace's existing pinned git dependency (`rev = "<sha>"`,
  established by U1) — it never adds a second, independent dependency
  declaration for osm2streets or `abstutil`. Meets project.md's pin mandate
  and NFR7.2 (which the pin itself already satisfies at U1; this requirement
  is that U4 does not accidentally reopen it).
- **NFR6.4.4 — Extract bytes are untrusted input at the adapter boundary.**
  `StreetImportAdapter` treats every byte sequence `ExtractFetcher` hands it
  as untrusted: malformed input is classified `malformed_extract` rather
  than passed through, and the timeout budget (BR1.1) bounds how long a
  pathological extract can occupy the browser tab before the import is
  abandoned as a failure. Meets AC3.1.6 and the DoS row above.
- **NFR6.1.3 — No secret, credential, or connection string in this Unit.**
  This Unit makes an unauthenticated GET request (Contract 1 defines no
  auth header beyond `x-client-build`, a build identifier, not a secret)
  and holds no credential of any kind. N/A beyond restating org
  policy — there is nothing here to secure.

## Data protection

No payment data, no special-category personal data, no PII of any kind
flows through this Unit (NFR6.3) — extract bytes describe map geometry and
tags, never a person. `ImportFingerprint.wayTags` and `.mapConfig` are OSM
map metadata, not user data.

## Assumptions & Open Questions

None.

## Traceability

See `traceability.json` in this directory.
