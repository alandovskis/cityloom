# Business Rules — `street-import` (U4)

_Confirmed._

Upstream inputs: `entities.md` (this stage), `components.md`, `stories.md`
US3.1, US7.1-US7.5, `contract-summary.md` Contract 1.

```yaml
rules:
  # BR1 — Timeout, not an indefinite wait
  - id: BR1.1
    statement: "An import that has not completed within the configured budget is abandoned and treated as a failure, never left as an indefinite spinner."
    category: constraint
    applies_to: ExtractFetcher
    trigger: "Fetch or conversion exceeding the timeout budget"
    logic: "IF the elapsed time since the import was triggered exceeds the timeout budget THEN the import fails with reason=timeout, routed into the failure path (US7.1, US7.2)."
    violation_behaviour: "AC3.1.6 names an indefinite spinner an unacceptable outcome. The specific budget value is owed by nfr-requirements (AC3.1.2's own deferral discipline); this rule fixes the behaviour, not the number."
    source: "AC3.1.6; Contract 1"

  # BR2 — Retry only transient failures
  - id: BR2.1
    statement: "Retry is offered only for the transient failure classes (upstream_unavailable, upstream_rate_limited, timeout); the non-transient classes (area_not_found, area_too_large, malformed_extract) are never retried automatically or offered a retry action, because retrying a badly tagged street returns the same bytes."
    category: policy
    applies_to: ImportFailure
    trigger: "An import fails"
    logic: "IF failure.reason is in {upstream_unavailable, upstream_rate_limited, timeout} THEN failure.retryable = true and retry is offered as the first action (AC7.2.1); IF failure.reason is in {area_not_found, area_too_large, malformed_extract, internal} THEN failure.retryable = false."
    violation_behaviour: "Offering retry for a non-transient failure wastes the user's time on a failure that cannot change (AC7.2.1, AC7.2.2's own contrast between retry-succeeds and accept-blank paths)."
    source: "Contract 1 retryable classes; AC7.2.1, AC7.2.2"

  # BR3 — Typed failure surface, never native error text
  - id: BR3.1
    statement: "Every failure this Unit produces names a member of the closed FailureReason set and the affected street by its selection-time name; it never contains a stack trace, library error text, or internal identifier."
    category: validation
    applies_to: ImportFailure
    trigger: "Any failure produced by ExtractFetcher or StreetImportAdapter"
    logic: "IF a failure is constructed THEN it carries exactly one FailureReason value and a street_name matching the one shown at selection; the underlying dependency error (osm2streets panic, network error) is logged once locally with the street identifier and never included in the value returned to the caller."
    violation_behaviour: "A leaked stack trace or library string is exactly what AC7.1.2 forbids, and it defeats US7.1's purpose — an explanation in terms of the street, not the software."
    source: "AC7.1.1, AC7.1.2; team.md Code Style (typed results, not exceptions)"

  # BR4 — Provenance at the type boundary
  - id: BR4.1
    statement: "Every Dimension this Unit constructs on a Lane is assigned Mapped or Inferred provenance at construction time, read from osm2streets' tagged-versus-defaulted width signal; a Dimension without a provenance value cannot be constructed."
    category: constraint
    applies_to: StreetImportAdapter
    trigger: "Constructing a Lane's Dimension from osm2streets output"
    logic: "IF osm2streets reports the value as tagged THEN provenance = Mapped; IF osm2streets reports the value as defaulted THEN provenance = Inferred; there is no third outcome at this boundary (UserSet is only ever assigned by a Correction, never by the adapter)."
    violation_behaviour: "FR3.1 and street-core's own Dimension type (team.md Code Style) make the un-annotated form unconstructable — this rule is what the adapter must uphold to keep that type-level guarantee true at the one place values cross into it."
    source: "FR3.1; components.md StreetImportAdapter behaviour"

  # BR5 — Correction permanence
  - id: BR5.1
    statement: "A Correction's provenance is always UserSet and never returns to Mapped, because a Correction states the import itself was wrong — distinct from a design edit, which proposes a change to a street the import described correctly."
    category: policy
    applies_to: Correction
    trigger: "Correction construction, and every re-import reconciliation"
    logic: "IF a Correction exists THEN its value's provenance is UserSet for the entire lifetime of the Correction, including across every ReconciliationOutcome; no code path reverts a Correction's provenance to Mapped or Inferred."
    violation_behaviour: "AC7.3.1 states this directly. Confusing a correction with a measurement is the exact harm raid-log.md R-2 calls most damaging."
    source: "FR3.4; AC7.3.1"

  # BR6 — Correction keying
  - id: BR6.1
    statement: "A Correction is keyed by the OSM way id, the direction-normalised bounding node-id pair of its segment, and the derived per-lane key (lane type, direction, ordinal from the kerb) — never by position in the lane list."
    category: constraint
    applies_to: Correction
    trigger: "Correction storage or lookup"
    logic: "IF a Correction is stored or resolved against a fresh import THEN the key used is (osm_way_id, bounding_node_ids, lane_discriminator); positional (usize) indices are never used as any part of the key."
    violation_behaviour: "The corrected practice in team.md's Corrections (learned 2026-09-10): osm2streets' split_ways makes way id alone insufficient, and positional indices are unstable across a version bump or re-fetch. This is the same keying scheme street-core defines for the overlay, applied here to corrections specifically."
    source: "AC7.3.4; team.md Corrections"

  # BR7 — Never write to OpenStreetMap
  - id: BR7.1
    statement: "No operation this Unit performs — an import, a correction, a re-import reconciliation — ever issues a write to any OpenStreetMap endpoint."
    category: constraint
    applies_to: ExtractFetcher, StreetImportAdapter, CorrectionOverlay
    trigger: "Every operation this Unit performs"
    logic: "IF this Unit calls an external endpoint THEN the call is read-only (GET, or the equivalent read-only osm2streets crate operation); no code path in this Unit constructs a write request to OpenStreetMap or a mirror of it."
    violation_behaviour: "AC7.3.5 asserts this as a global invariant, and project.md's Forbidden section names it directly: never edit the underlying OpenStreetMap data from within this product."
    source: "AC7.3.5; project.md Forbidden"

  # BR8 — Re-import reconciliation, exactly one of three outcomes
  - id: BR8.1
    statement: "On every re-import, each stored Correction whose targetStreet matches resolves to exactly one of three outcomes: fingerprint matches (reapply silently), fingerprint changed but the lane still resolves by its key (reapply and notify), or the lane no longer resolves (retain as unresolved, neither applied nor discarded)."
    category: policy
    applies_to: Correction, CorrectionReconciliationOutcome
    trigger: "A street matching a stored Correction's targetStreet is re-imported"
    logic: "IF fresh_fingerprint == stored_fingerprint THEN outcome = reapplied_silently; ELSE IF the lane still resolves by target_lane_discriminator THEN outcome = reapplied_with_notice; ELSE outcome = unresolved (state = unresolved, still visible, still editable, never deleted)."
    violation_behaviour: "AC7.5.2-AC7.5.4 name these three outcomes exhaustively. Re-applying a correction unconditionally when the underlying lane changed would push a stale user value onto a lane OpenStreetMap has since re-tagged — the same class of harm as presenting an inference as a measurement (US7.5's own stated rationale: 'a correction can always be preserved; it cannot always be re-applied')."
    source: "AC7.5.1-AC7.5.4; stories.md US7.5"

  # BR9 — Blank cross-section after exhausted retry carries only UserSet
  - id: BR9.1
    statement: "A cross-section built by accepting the blank alternative after a failed import carries UserSet provenance on every attribute the user sets, and nothing in it is ever presented as measured."
    category: constraint
    applies_to: ImportFailure
    trigger: "The user accepts the blank cross-section offered alongside retry"
    logic: "IF the blank cross-section path is taken THEN no Dimension in the resulting design carries Mapped or Inferred provenance; every value set on it is UserSet from the moment it is set, because it never touched an import at all."
    violation_behaviour: "AC7.2.3, AC7.2.4. This Unit is responsible for offering the blank alternative alongside retry (BR2.1) and for guaranteeing it carries no import-provenance shape into design-editing (U5), which actually constructs the blank editing state; the guarantee that U5's blank state honours this rule is verified at U5's own functional-design and code-generation, not re-derived here."
    source: "AC7.2.3, AC7.2.4"
```

## Summary

| Rule | Category | What it protects |
|---|---|---|
| BR1.1 | Timeout | No indefinite spinner |
| BR2.1 | Retry policy | Retry only offered where it can help |
| BR3.1 | Typed failure surface | No leaked stack trace or library text |
| BR4.1 | Provenance at the boundary | Mapped/Inferred assigned correctly, never omitted |
| BR5.1 | Correction permanence | A correction can never be mistaken for a measurement |
| BR6.1 | Correction keying | Corrections survive a version bump or re-fetch |
| BR7.1 | Never write to OSM | The product never edits the underlying map |
| BR8.1 | Re-import reconciliation | A correction is preserved even when it cannot be re-applied |
| BR9.1 | Blank cross-section provenance | A user's own values are never presented as measured |

## Traceability

See `traceability.json` in this directory.
