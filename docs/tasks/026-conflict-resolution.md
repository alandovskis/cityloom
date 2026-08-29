---
id: 026
title: "Add conflict resolution and transactional reconcile-on-save"
depends_on: [025, 014]
features: [RECON-009, RECON-010, RECON-011, RECON-012, OSM-021]
status: todo
acceptance:
  - "just verify"
  - "uv run --frozen pytest -m integration -k reconcile"
model_hint: sonnet
---

# Context
A reconciliation report is only useful if the user can act on it, and only safe
if applying it is atomic.

# Scope
- `resolve(conflict, KeepLocal)` leaving the local cross-section byte-identical
  and clearing the conflict.
- `resolve(conflict, TakeUpstream)` replacing the cross-section and clearing the
  conflict.
- Route imports into a non-empty city through reconciliation.
- Run reconciliation inside the save transaction; a mid-run failure leaves the
  city untouched.
- Idempotence: an immediate second reconcile reports everything unchanged.

# Out of scope
- Per-segment conflict resolution. v1 resolves a whole cross-section at a time.
- Undoing a resolution through the authoring undo stack.
- The UI. Task 048.

# Acceptance criteria (beyond the acceptance commands)
- After `KeepLocal`, the stored cross-section is compared byte-for-byte with
  the pre-resolution value.
- A failure injected between the edge and metadata writes leaves the city at
  its pre-reconcile state, asserted by reloading.
