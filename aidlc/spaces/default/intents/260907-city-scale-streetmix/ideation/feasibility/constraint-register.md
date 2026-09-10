# Constraint Register — Streetmix at City Scale

Constraints are non-negotiable boundaries. Each entry records what is fixed, why,
and what it forecloses. Items that are uncertain rather than fixed belong in
`raid-log.md`, not here.

Upstream inputs: `intent-statement.md`, `competitive-analysis.md`,
`market-trends.md`, `build-vs-buy.md`, and `feasibility-assessment.md`.

## Technical Constraints

| ID | Constraint | Consequence | Source |
|----|-----------|-------------|--------|
| TC-1 | The lane schema comes from osm2streets: each road carries lanes left-to-right with type, direction and width; intersections are polygons | The project's street model is shaped by osm2streets rather than chosen freely. Anything osm2streets cannot express is not directly representable | Q10; `feasibility-assessment.md` |
| TC-2 | OpenStreetMap is the base data | Coverage and quality vary by area, and the tool inherits that variation. Areas with thin tagging yield inferred rather than mapped cross-sections | Q7 |
| TC-3 | OSM does not carry reliable cross-section detail: `width` is defined kerb-to-kerb excluding sidewalks, and sidewalk width is largely unmapped under competing schemes | Cross-sections are partly derived rather than read. The tool cannot present derived values as surveyed fact | `feasibility-assessment.md` sources 1-3 |
| TC-4 | osm2streets' core is Rust compiled to WebAssembly, consumed through its JavaScript bindings | The dependency is a compiled artifact rather than readable JavaScript, so debugging into it is harder and the build has a WebAssembly step | `feasibility-assessment.md` source 5 |
| TC-5 | The street model must be jurisdiction-neutral, with standards supplied as a pluggable layer | No jurisdiction's classifications, widths or terminology may be baked into the core model | Q2, Q8 |
| TC-6 | City GIS import and export is required | The design must accommodate formats not yet examined | Q7 |

## Organisational Constraints

| ID | Constraint | Consequence | Source |
|----|-----------|-------------|--------|
| OC-1 | One person builds this, with AI assistance | Total capacity is one person. No parallel workstreams, no specialist cover, no redundancy | Q3 |
| OC-2 | Sole decision authority rests with the project owner; no external approval | Decisions can be made immediately, and there is no second opinion by default | `intent-statement.md`; Q6 of intent capture |
| OC-3 | No external deadline | Schedule pressure is self-imposed. Scope discipline cannot be delegated to a date | Q4; `intent-statement.md` |
| OC-4 | Hosting and tooling budget is approximately $5 per month | Anything requiring more must be justified as a change to this constraint rather than absorbed | Q4, Q9 |

## Commercial and Licensing Constraints

| ID | Constraint | Consequence | Source |
|----|-----------|-------------|--------|
| LC-1 | The product is open source and free | No revenue mechanism is available to fund infrastructure growth | Market research Q7 |
| LC-2 | osm2streets is Apache-2.0 | Permissive: it may be used under this project's own licence choice, with attribution | `feasibility-assessment.md` source 5 |
| LC-3 | Streetmix is AGPL-3.0-or-later and is NOT a code dependency of this project | Streetmix code and assets must not be copied in. Its interaction design may inform this project's own; its artwork must not be reused without establishing separate permission | Q10; `feasibility-assessment.md` sources 7-8 |
| LC-4 | This project's own licence remains its own choice | Preserved deliberately by not extending Streetmix. An open-core model stays available if the commercial model is ever revisited | Q10 |

## Regulatory Constraints

| ID | Constraint | Consequence | Source |
|----|-----------|-------------|--------|
| RC-1 | User accounts and saved designs tied to people place this in scope for GDPR and comparable privacy regimes from the first public release | Data subject rights — access, rectification, erasure, portability — are functional requirements, not operational work | Q6; `feasibility-assessment.md` |
| RC-2 | The audience is the general public with no geographic restriction | EU/EEA residents can be expected among users, so GDPR applies regardless of where the service is operated | Q2; `intent-statement.md` |
| RC-3 | Personal data must be minimised and retention bounded | Account data and saved designs need a stated retention position before launch | RC-1 |
| RC-4 | No payment data and no special-category data are handled | PCI-DSS and HIPAA are out of scope. This constraint holds only while the product remains free | LC-1; Q6 |

## Constraints Deliberately Not Recorded

- **No street-design standard is fixed.** TC-5 requires neutrality, so no
  jurisdiction's standards are a constraint on the core. A first supported
  jurisdiction is a future scope decision, not a present constraint. [assumption]
- **No availability or performance target is fixed.** None has been established,
  and inventing one here would create a constraint nobody agreed to. [assumption]
- **Railway is the intended host (Q5) but is not recorded as a constraint.**
  Nothing yet depends on Railway-specific behaviour, and OC-4's budget is the
  binding limit rather than the provider. [assumption]

## Assumptions & Open Questions

- TC-1 assumes osm2streets' schema is expressive enough for the editing the
  product needs; this rests on its documented schema rather than on use.
  [assumption]
- TC-6 records city GIS as required without any format having been examined, so
  its real constraint surface is unknown. [assumption]
- LC-3's boundary between "interaction design as reference" and "derivative work"
  has not been established with legal precision and should not be treated as
  settled. [assumption]
- RC-1 through RC-4 are a first-pass regulatory scan, not a privacy impact
  assessment or legal advice. [assumption]
- OC-4's $5 per month is a stated working budget, not a measured cost. [assumption]
