---
id: 048
title: "Build the import dialog, job progress, and reconciliation UI"
depends_on: [043, 020, 026]
features: [UI-006, UI-007, UI-008, UI-009, UI-010, UI-011]
status: todo
acceptance:
  - "just verify"
  - "uv run --frozen pytest -m e2e -k import_ui"
model_hint: sonnet
---

# Context
The surface where the two authoring paths meet. Its job is to make
reconciliation legible: what changed, what conflicted, and what the user wants
done about it.

# Scope
- Import dialog accepting a bbox and starting a job.
- Polling and displaying progress until a terminal state.
- Surfacing a failed import's error message.
- Showing the reconciliation report after importing into a non-empty city.
- Listing each conflict with keep-local and take-upstream actions.
- Removing a conflict from the list once acted on.

# Out of scope
- Drawing the bbox on the map; v1 takes typed coordinates.
- Previewing changes before applying.
- Bulk resolve-all actions.

# Acceptance criteria (beyond the acceptance commands)
- Progress is asserted non-decreasing across polls.
- A conflict resolved as keep-local asserts, via `city_json()`, that the local
  cross-section is unchanged.
- The report distinguishes all five categories from task 025.
