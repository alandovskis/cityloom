# NFR Design — Questions — `street-import` (U4)

No open design question remains for this Unit: every design element
follows directly from `security-requirements.md` and `tech-stack-decisions.md`.

## Consolidated Summary Confirmation

Does this all look correct before I generate the artifacts?

Produces: `security-design.md` (5 design elements — closed ImportFailure
mapping via `catch_unwind`, local-only failure log, osm2streets consumed
only through the existing pin, untrusted extract bytes bounded before
conversion, no secret to manage) and `logical-components.md` (in-process
component inventory and blast-radius mapping — the `catch_unwind` boundary
is what keeps one failed import from aborting the whole WASM module) and
`traceability.json`. No `performance-design.md`/`scalability-design.md`/
`reliability-design.md`/`observability-design.md` — this Unit's `library`
kind excludes them from `produces_kinds`.

- Looks correct
- Request changes

[Answer]: Looks correct
