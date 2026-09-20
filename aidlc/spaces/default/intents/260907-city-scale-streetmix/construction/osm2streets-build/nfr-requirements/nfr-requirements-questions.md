# NFR Requirements — Questions — `osm2streets-build` (U1)

This Unit is `packaging` kind (per `unit-of-work.md`): the forked, pinned
`osm2streets` dependency and its provenance record. It has no runtime
surface, so performance, scalability, reliability and observability
requirements do not apply (consistent with this stage's `produces_kinds`,
which restrict those four artifacts to `service`/`ui` units). Only security
requirements and tech-stack decisions apply, plus traceability.

Two points were flagged as open rather than assumed silently:

## Q1 — Fork branch protection

Should the forked repository's branch protection (no force-push once the
pinned commit is recorded) be locked down as part of this stage's
requirement, with the actual GitHub setting applied later at
`environment-provisioning` alongside the rest of the repository hardening
`team.md` already lists there (push protection, CodeQL, Dependabot)?

- A. Yes — record it as a requirement here (`NFR7.2.5`), apply the setting at `environment-provisioning`
- B. No — this is out of scope for a packaging Unit; drop it
- X. Other (please specify)

[Answer]: A. Yes — record it as a requirement here (`NFR7.2.5`), apply the setting at `environment-provisioning`

## Q2 — Cargo workspace structure

Should `osm2streets-build`'s pinned dependency and the client crates it
feeds live in the *same* Cargo workspace as `osm-extract-proxy` (the
server proxy Unit already built), or a separate client-only workspace?

- A. Same workspace, one root `Cargo.toml`/`rust-toolchain.toml` — reuse what `osm-extract-proxy` already established
- B. Separate workspace for the client
- X. Other (please specify)

[Answer]: A. Same workspace, one root `Cargo.toml`/`rust-toolchain.toml` — reuse what `osm-extract-proxy` already established

## Consolidated Summary Confirmation

Does this all look correct before I generate the artifact?

Produces (only what this Unit's `packaging` kind requires):
`security-requirements.md` (STRIDE-scoped threat model narrowed to
supply-chain integrity of the pinned fork; NFR7.2.1-NFR7.2.5,
NFR-SUPPLY-1/2), `tech-stack-decisions.md` (reuses the existing Rust
1.97.1 toolchain and Cargo workspace from `osm-extract-proxy`; defers the
WASM build-tool choice to whichever Unit picks the DOM framework), and
`traceability.json` (NFR6 N/A — no runtime secrets needed; NFR7 OK,
covered by the security requirements above).

- Looks correct
- Request changes

[Answer]: Looks correct
