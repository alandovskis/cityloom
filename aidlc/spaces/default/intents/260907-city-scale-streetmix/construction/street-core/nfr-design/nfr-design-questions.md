# NFR Design — Questions — `street-core` (U2)

No open design question remains for this Unit: every design element
follows directly from `security-requirements.md` (this Unit's own crate
boundary and mutability constraints).

## Consolidated Summary Confirmation

Does this all look correct before I generate the artifact?

Produces: `security-design.md` (4 design elements — no mutable public
API, crate-level dependency allowlist, `forbid(unsafe_code)`,
dependency-free by construction), `logical-components.md` (a single
in-process component with no runtime failure domain and no shared
resource), and `traceability.json`.

- Looks correct
- Request changes

[Answer]: Looks correct
