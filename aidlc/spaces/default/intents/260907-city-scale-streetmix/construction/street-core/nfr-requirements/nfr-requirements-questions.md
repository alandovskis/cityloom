# NFR Requirements — Questions — `street-core` (U2)

No open question remains for this Unit: it is a pure in-memory domain
model with no runtime surface, so performance/scalability/reliability/observability
requirements do not apply (matching this stage's `produces_kinds`, which
restrict those artifacts to `service`/`ui` units). Only security
requirements and tech-stack decisions apply, plus traceability.

## Consolidated Summary Confirmation

Does this all look correct before I generate the artifact?

Produces: `security-requirements.md` (STRIDE narrowed to crate-boundary
and mutability concerns; NFR6.4.1-6.4.3, NFR7.2.6), `tech-stack-decisions.md`
(no dependency beyond the Rust standard library, reusing the existing
workspace toolchain), and `traceability.json`.

- Looks correct
- Request changes

[Answer]: Looks correct
