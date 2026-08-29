---
id: 025
title: "Implement the re-import reconciliation engine"
depends_on: [024, 006]
features: [RECON-001, RECON-002, RECON-003, RECON-004, RECON-005, RECON-006, RECON-007, RECON-008, RECON-013]
status: todo
acceptance:
  - "just verify"
  - "cargo nextest run -p cityloom-core recon::"
model_hint: opus
# Justification for opus: this is a three-way merge (last-imported, current
# local, new upstream) over a graph, and it is the task where a plausible-
# looking implementation silently destroys user work. The rules interact:
# "locally edited" has to be defined against the last import rather than
# against defaults, hand-drawn features must be invisible to the diff, and the
# five output categories must stay disjoint. Errors here surface much later as
# lost edits with no audit trail.
---

# Context
Because hand-drawn and imported geometry are peers, re-importing a bbox is a
merge, not a load. The engine must never overwrite a local edit and must never
touch hand-drawn features.

# Scope
- Store the last-imported snapshot per city so "locally edited" is decidable.
- An edge counts as locally edited only if its cross-section or geometry
  differs from the value last imported.
- Diff upstream against last-imported against local, producing added, updated,
  unchanged, conflicted, and removed edge id lists.
- Update in place only when there are no local edits.
- Record a conflict when both sides changed; never overwrite.
- Retain an upstream-deleted edge that has local edits, reported as
  removed-with-local-edits.
- Remove an upstream-deleted edge that has no local edits.
- Preserve node ids for OSM nodes persisting across imports.
- Leave hand-drawn features entirely out of the report.

# Out of scope
- Resolving conflicts. Task 026.
- Transactional integration with save. Task 026.
- The conflict UI. Task 048.
- Merging concurrent *live* edits from two browsers — that is task 015's
  version check, a different problem.

# Acceptance criteria (beyond the acceptance commands)
- The five id lists are asserted pairwise disjoint on every fixture.
- A fixture where a hand-drawn edge sits geometrically on top of an upstream
  edge asserts the hand-drawn one is untouched and unreported.
- An edge whose cross-section was edited to a value that happens to equal the
  new upstream value is reported as unchanged, not conflicted.
