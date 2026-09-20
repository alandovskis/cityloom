<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->

## Deviations
- 2026-09-19T00:00:00Z — design-storage (U10): first-generation pass missed
  Step 3's schema-shape test (NFR6.3.1); found on self-review before the
  formal reviewer dispatch and added
  (`repository::tests::the_migrated_table_has_exactly_the_columns_entities_md_names`).
  Required re-fingerprinting the plan; restored the original approved bytes
  first, made the code fix, then re-marked completion — see the plan's own
  approval-fingerprint history for the byte-exact trail.
- 2026-09-19T00:00:00Z — design-storage (U10): the architecture reviewer's
  adversarial review (iteration 1) returned NOT-READY with 2 Major findings
  (R-01 rate-limiter sweep never invoked in production; R-02 NFR7.4.2
  aggregate-counters requirement unimplemented and missing from
  `traceability.json`, since corrected to a `GAP` row). The required
  developer-agent repair dispatch is blocked: the reviewer's own sanctioned
  `## Review` appendix to `code-generation-plan.md` (the file the Plan
  Approval fingerprint covers byte-for-byte) permanently broke that unit's
  fingerprint match, the fingerprint-refresh path refuses once application
  code exists in the workspace, and the documented
  `AIDLC_DISABLE_PLAN_APPROVAL_GUARD=1` escape hatch requires writing
  `.claude/settings.local.json`, which is itself outside the unit's record
  directory and blocked by the very guard it would disable. Consulted the
  human at each step; they chose to stop rather than have the settings file
  edited on their behalf.

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
- 2026-09-19T00:00:00Z — design-storage (U10) is NOT-READY and blocked on a
  tooling catch-22 (see Deviations). Resume requires either the human
  creating `.claude/settings.local.json` with
  `{"env": {"AIDLC_DISABLE_PLAN_APPROVAL_GUARD": "1"}}` outside this
  session, or another sanctioned way to refresh Plan Approval evidence
  after a reviewer appendix without a pre-generation workspace state to
  fall back to. Once unblocked: re-invoke the developer agent for R-01/R-02
  only, then re-review (iteration 2 of the adversarial budget), then
  complete the unit and remove the override.
