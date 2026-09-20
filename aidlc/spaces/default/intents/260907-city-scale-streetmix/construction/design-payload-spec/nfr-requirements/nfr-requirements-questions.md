# NFR Requirements — Questions — `design-payload-spec` (U3)

No open question remains: this Unit is a pure data-shape spec with no
runtime surface, so performance/scalability/reliability/observability
requirements do not apply (matching `produces_kinds`, which restrict
those artifacts to `service`/`ui` units). Only security requirements and
tech-stack decisions apply, plus traceability.

## Consolidated Summary Confirmation

Does this all look correct before I generate the artifact?

Produces: `security-requirements.md` (STRIDE narrowed to
deserialization-safety and type-level invariant enforcement;
NFR6.4.4, NFR6.4.5, NFR6.3.1), `tech-stack-decisions.md` (a standalone
`cityloom-design-payload` crate depending only on `serde`, no dependency
on `street-core` or osm2streets), and `traceability.json`.

- Looks correct
- Request changes

[Answer]: Looks correct
