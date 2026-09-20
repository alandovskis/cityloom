# Functional Design — Questions — `street-core` (U2)

No open design question remains for this Unit: its entities, rules, and
workflow are all already specified by `components.md`'s `StreetModel`
description, `requirements.md` FR3.1-FR3.4, and `team.md`'s Code Style
corrections (overlay keying, bounds-checked access, three-layer crate
boundaries). The one human checkpoint this stage requires is confirming
the resulting design.

## Consolidated Summary Confirmation

Does this all look correct before I generate the artifact?

Produces: `entities.md` (4 entities — Provenance, Dimension, Lane, Street,
StreetNetworkGraph — with the derived lane key and way-id-plus-node-pair
Street identity), `rules.md` (7 business rules — BR1 provenance, BR2 lane
identity/access, BR3 Street identity, BR4 carriageway width, BR5
immutability, BR6 metric units, BR7 the StreetSource port), `functional-spec.md`
(the construct-from-import workflow, a Provenance state machine, and
derived ER/rules-summary views), and `traceability.json` mapping this
Unit's assigned acceptance criteria to the rules above (with N/A
justifications for criteria that belong to `u5-design-editing` or
`u4-street-import`).

- Looks correct
- Request changes

[Answer]: Looks correct
