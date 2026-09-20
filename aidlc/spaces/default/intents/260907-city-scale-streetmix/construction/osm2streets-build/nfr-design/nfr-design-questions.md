# NFR Design — Questions — `osm2streets-build` (U1)

No open design question remains for this Unit: both points flagged at
`nfr-requirements` (fork branch protection, workspace layout) were
resolved there, and every remaining design element follows directly from
`security-requirements.md` and `tech-stack-decisions.md`. The one human
checkpoint this stage requires is confirming the resulting design.

## Consolidated Summary Confirmation

Does this all look correct before I generate the artifact?

Produces (only what this Unit's `packaging` kind requires):
`security-design.md` — seven design elements (SD-1 fork identity, SD-2
two-point commit pin, SD-3 locked resolution via committed `Cargo.lock`,
SD-4 branch protection on the fork applied at `environment-provisioning`,
SD-5 provenance record in the existing `docs/dependencies.md`, SD-6
`cargo audit` coverage with no new tool, SD-7 the golden-fixture suite as
tamper-detection cross-reference) plus `traceability.json` mapping every
`NFR7.2.x`/`NFR-SUPPLY-n` requirement to its design element.

- Looks correct
- Request changes

[Answer]: Looks correct
