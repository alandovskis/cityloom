# Phase Check — Inception → Construction

**Verdict: PASS.** No `GAP`, no `ORPHAN`, no missing upstream ID, and no
`OK`/`Deferred`/`N/A` entry without a target or justification, across all three
Inception traceability files. The transition to Construction is not blocked.

Run at the close of Delivery Planning, the capstone Inception stage, per that
stage's Step 5.

## What was checked

Every `traceability.json` produced by an Inception stage that executed:

- `<record>/inception/user-stories/traceability.json`
- `<record>/inception/domain-design/traceability.json`
- `<record>/inception/units-generation/traceability.json`

Contract Design produces no `traceability.json` — it owns formal contracts
rather than requirement coverage — so it does not contribute to this check.
Reverse Engineering was SKIP (greenfield), and Practices Discovery and
Requirements Analysis produce no traceability artifact.

The files were parsed and their entries counted by status, rather than read.
Three things were asserted per file: that no entry carries `GAP` or `ORPHAN`
in either the forward or reverse direction; that every entry's `id` list equals
the file's own `upstream_ids` list, so nothing was declared upstream and then
left uncovered; and that every non-`GAP` entry carries a non-empty target or
justification, since `OK`, `Deferred` and `N/A` all require one.

## Results

| Stage | Upstream IDs | Coverage entries | Coverage statuses | Reverse entries | Reverse statuses | IDs match | GAP / ORPHAN | Missing target |
|---|---|---|---|---|---|---|---|---|
| user-stories | 61 | 61 | 47 OK, 8 Deferred, 6 N/A | 8 | 4 OK, 4 N/A | yes | none | none |
| domain-design | 40 | 40 | 39 OK, 1 Deferred | 4 | 1 OK, 3 N/A | yes | none | none |
| units-generation | 40 | 40 | 39 OK, 1 Deferred | 4 | 4 N/A | yes | none | none |

## The non-OK entries, and why each is legitimate

A `Deferred` or `N/A` status is not a gap, but it is a claim, and a claim with
no owner is a gap wearing a different label. Each is listed with the stage that
owns it.

### Deferred — owned by a named later stage

| ID | Owner | What it is |
|---|---|---|
| NFR1.1 | `nfr-requirements` | The 10-second import budget. AC3.1.2 and AC3.1.6 defer it explicitly: no device, no network and no reference street are established |
| NFR1.2 | `nfr-requirements` | The 100ms editing-feedback budget |
| NFR1.3 | `nfr-requirements` | The coarse viewport pass, which has no stated budget yet |
| NFR2.2 | `nfr-requirements` | Memory within a browser tab's working set. `requirements.md` itself states this is a constraint on design, not a testable threshold |
| NFR3.1 | `nfr-requirements` | 99.5% monthly availability |
| NFR3.2 | `deployment-pipeline` | Health-checked deploys not counting against availability |
| NFR3.3 | `infrastructure-design` | The application service holding no attached volume |
| NFR4.7 | `nfr-requirements` | Desktop browser support, which has no matrix yet — `requirements.md` OQ6 carries the gap |
| US7.4 (domain-design, units-generation) | `observability-setup` | The maintainer-facing half of "see which streets are failing". The local-logging half is delivered by U4 `street-import`; the split is recorded identically in both files |

Every one of these names a real stage in this workflow's remaining path.

### N/A — not user-observable behaviour, and owned elsewhere

| ID | Owner | Why no story asserts it |
|---|---|---|
| NFR5.1 | `project.md` mandate, checked at infrastructure and deployment stages | An operating-cost ceiling, not behaviour a user can observe |
| NFR6.1 | `team-practices.md` — push protection plus the CI secret scan | A repository practice with no user-facing surface |
| NFR6.3 | Satisfied by absence | No story collects payment or special-category data |
| NFR7.1 | `build-and-test` | A coverage floor measured in CI |
| NFR7.2 | `ci-pipeline`, and the `project.md` pin mandate | A dependency-pinning build practice. AC3.1.5 asserts its observable consequence |
| NFR7.3 | `build-and-test` | A defect-handling practice |

The four reverse-direction `N/A` entries in `units-generation` are the four
foundation and infrastructure Units — `osm2streets-build`, `street-core`,
`design-payload-spec` and `osm-extract-proxy` — which carry no primary story
because they are shared by everything and owned by no feature. Each names the
acceptance criteria it is responsible for. `unit-of-work.md` records that as a
property of a decomposition mixing feature slices with foundation Units, not as
an omission.

## Consistency across the three files

The same story set flows through all three without drift. `stories.md` declares
40 stories; `domain-design` and `units-generation` each enumerate exactly those
40, and both record US7.4 as `Deferred` to `observability-setup` with the same
split. `user-stories` enumerates 61 upstream IDs because its upstream is
`requirements.md`'s functional and non-functional requirements rather than the
story set.

## What this check does not cover

Stated so the PASS verdict is not read as broader than it is.

- **It checks coverage, not correctness.** That every story maps to a Unit, and
  that the Unit's described behaviour can deliver it, was verified at each
  owning stage's review. This check confirms the mapping exists and resolves.
- **It does not cover the outstanding amendments.** Ten divergences between the
  approved artifacts and the real design are recorded — seven in
  `decisions.md` ADR-001 and three in `contract-summary.md`. None of them
  produces a `GAP` in a traceability file, because each one is a statement in
  `requirements.md`, `scope-document.md` or a dependency graph that the design
  contradicts, not a story left uncovered. `bolt-plan.md` § "Before
  Construction starts" records which are fixed before B-0 and which are carried
  to B-10.
- **Contract completeness is not in scope here.** `contract-summary.md`
  carries six open questions of its own, including the deferred timeout values
  and the unchosen basemap tile service.

## Verdict

**PASS.** No finding blocks the Inception → Construction transition.

- [ ] Human approval — recorded at the Delivery Planning approval gate.
