# Functional Design — Questions — `street-import` (U4)

No open design question remains for this Unit: `unit-of-work.md`,
`components.md`'s `ExtractFetcher`/`StreetImportAdapter`/`CorrectionOverlay`
entries, `contract-summary.md` Contract 1 and Contract 8, and the assigned
stories (US3.1, US7.1-US7.5) together fully determine the entity model, the
business rules, and the workflows.

## Consolidated Summary Confirmation

Does this all look correct before I generate the artifacts?

Produces: `entities.md` (5 types — `ImportFailure`, `ImportedStreet`,
`Correction`, `ImportFingerprint`, `CorrectionReconciliationOutcome`, plus
external references to `street-core`'s `Street`/`Lane`/`StreetNetworkGraph`),
`rules.md` (9 rules — timeout, retry policy, typed failure surface,
provenance at the boundary, correction permanence, correction keying, never
write to OSM, three-outcome re-import reconciliation, blank-cross-section
provenance), `functional-spec.md` (4 workflows: import a street, handle an
import failure, correct what the import got wrong, re-import
reconciliation), and `traceability.json` (24 ACs across US3.1 and
US7.1-US7.5; 3 deferred to nfr-requirements/code-generation/
observability-setup per those stories' own stated deferrals, 1 marked N/A
as owned by `client-surfaces`).

- Looks correct
- Request changes

[Answer]: Looks correct
