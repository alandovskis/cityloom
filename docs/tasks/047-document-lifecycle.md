---
id: 047
title: "Implement the document lifecycle: dirty, save, autosave, conflict"
depends_on: [043, 018]
features: [DOC-001, DOC-002, DOC-003, DOC-004, DOC-005, DOC-006, DOC-007, DOC-008, DOC-009, DOC-010, DOC-011, DOC-012]
status: todo
acceptance:
  - "just verify"
  - "uv run --frozen pytest -m e2e -k document_lifecycle"
model_hint: sonnet
---

# Context
Everything that makes a city a document rather than a scratch buffer. The
version check from task 015 becomes a user-visible conflict here.

# Scope
- New city creating and navigating to `/c/{slug}`.
- Loading `/c/{slug}`; a not-found page for an unknown slug.
- Dirty tracking off the document version, with an unsaved indicator.
- Save via PUT clearing the indicator; failures staying dirty and retryable.
- 409 showing a conflict message offering reload.
- Autosave at most once per 5 s while dirty, never while clean.
- Confirmation prompt on navigating away dirty.
- Share copying the URL including the camera hash.
- Rename persisting.

# Out of scope
- Merging on conflict; v1 offers reload only.
- Offline editing or a local draft cache.
- Version history or restore.

# Acceptance criteria (beyond the acceptance commands)
- Autosave timing is asserted with a fake clock, not a real 5 s sleep.
- The conflict path is driven by a concurrent PUT from the test, asserting the
  message and that local edits are not silently discarded.
- Share is asserted to include the hash, using a stubbed clipboard.
