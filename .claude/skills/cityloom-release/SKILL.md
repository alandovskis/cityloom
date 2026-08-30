---
name: cityloom-release
description: >
  Use when the task involves publishing a version, tagging, cityloom's own
  changelog, or the make release command.
---

# Shiplog release

## Autonomy: the step
- You MAY, without asking: run the full verify, generate the changelog
  from commits (make changelog), assemble the tag locally.
- You MUST STOP and ask a human for: pushing the tag, make release-publish,
  any step that leaves the machine. No exceptions, even if the task says
  "do the full release" — the task does not revoke this step.

## Process up to the step
1. ./scripts/verify.sh (full, not --quick). RED = stop here.
2. make changelog — review entries against features flipped PASSING in
   this version; adjust wording for humans (the changelog is product!).
3. make tag-local VERSION=<semver> — the version follows semver by the
   feature diff: new PASSING = minor; fix without a feature = patch.
