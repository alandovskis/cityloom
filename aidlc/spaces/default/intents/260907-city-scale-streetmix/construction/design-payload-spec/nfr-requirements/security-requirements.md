# Security Requirements — `design-payload-spec` (U3)

_Confirmed._

Upstream inputs: `functional-spec.md`, `rules.md` (functional-design,
this Unit); `requirements.md` NFR6 (requirements-analysis).

This Unit is a pure data-shape spec — no network, no I/O, no runtime
component. Its threat surface is the shape itself: whether a malicious
or malformed payload can be deserialized into something that violates
one of this project's own invariants (e.g. a `Correction` presented with
`Mapped` provenance, which BR4.1 says must be structurally impossible).

## Threat model (STRIDE, scoped to this Unit)

| Category | Applicable? | Reasoning |
|---|---|---|
| Spoofing | N/A | No identity concept in the shape itself |
| **Tampering** | **Yes (narrow)** | A payload read from untrusted storage (a device's local storage, or the server database) must deserialize into valid Rust values or fail closed — never partially construct an invalid `DesignPayload` |
| Repudiation | N/A | No user-attributable action at this layer |
| **Information Disclosure** | **Yes (narrow)** | The payload itself carries no PII beyond what the design legitimately needs (street identifiers, dimensions) — no free-text field should silently become a place secrets or unrelated personal data get stored |
| Denial of Service | N/A | No request path this Unit controls |
| Elevation of Privilege | N/A | No authorization surface — object-level authorization on who may read/write a given payload is `u10-design-storage`'s and `u11-accounts-sharing`'s concern, not this shape's |

## Requirements

- **NFR6.4.4** Deserializing a payload with an unrecognised
  `payloadVersion` shall fail closed (`unsupported_payload_version`)
  rather than attempting a best-effort partial read. Source: `rules.md`
  BR1.1.
- **NFR6.4.5** The shape shall make an invalid provenance state
  unrepresentable where the type system can enforce it — a `Correction`
  has no field through which it could carry `Mapped` provenance,
  matching `street-core`'s own type-level enforcement of the same
  invariant. Source: `rules.md` BR4.1.
- **NFR6.1.3** (inherits `requirements.md` NFR6.1 — no secret in the
  repository) N/A for this Unit: it defines a data shape, not
  configuration or credentials.
- **NFR6.3.1** (inherits `requirements.md` NFR6.3 — no payment data and
  no special-category personal data shall be collected or stored) The
  shape's free-text fields (`design.name`, `LaneEdit.value`,
  `Correction.value`) hold only street-design content by construction —
  this Unit does not add any field intended for personal data, and any
  consumer that starts writing personal data into these fields would be
  violating NFR6.3 independently of this shape.

## Out of scope for this Unit

Authentication/authorization on who may read or write a stored payload
(`u10-design-storage`, `u11-accounts-sharing`), encryption at rest/in
transit for the storage layer (`u10-design-storage`,
`environment-provisioning`), and rate limiting (no request path here).

## Assumptions & Open Questions

None.

## Traceability

See `traceability.json` in this directory.
