# Infrastructure Design — Questions — `street-core` (U2)

No open question remains: this Unit has no deployment, compute, storage,
or monitoring footprint of its own (a pure library crate), so this
stage's only applicable artifact for a `library`-kind unit is
`cicd-pipeline.md`, which extends the existing CI tiers rather than
defining anything new.

## Consolidated Summary Confirmation

Does this all look correct before I generate the artifact?

Produces: `cicd-pipeline.md` (adds to `osm-extract-proxy`'s existing
build/lint/test/coverage tiers; no deployment/rollback/secrets since
nothing here deploys) and `traceability.json` (most NFRs N/A — they are
compile-time guarantees, not CI-observable ones — except NFR6.4.3, which
clippy/fmt do cover).

- Looks correct
- Request changes

[Answer]: Looks correct
