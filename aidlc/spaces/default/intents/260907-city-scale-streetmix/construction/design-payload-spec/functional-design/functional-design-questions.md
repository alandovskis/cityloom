# Functional Design — Questions — `design-payload-spec` (U3)

No open design question remains for this Unit: `contract-design`'s
Contract 3 already fixed the complete shape (schema, versioning policy,
round-trip promise); this stage transcribes it into entities/rules
format and adds the workflow-level round-trip contract.

## Consolidated Summary Confirmation

Does this all look correct before I generate the artifact?

Produces: `entities.md` (6 types — DesignPayload, StreetKey, Dimension,
LaneEdit, Correction, ImportFingerprint — transcribed from Contract 3),
`rules.md` (6 rules — versioning, round-trip fidelity, wire-carried
provenance, correction permanence, import-fingerprint completeness,
additive-only evolution), `functional-spec.md` (the write-then-read and
export workflows), and `traceability.json` (11 assigned ACs, several
N/A where the behaviour belongs to a consuming Unit rather than this
shape).

- Looks correct
- Request changes

[Answer]: Looks correct
