---
name: factory-review
description: Use when a CityLoom issue's branch in the software factory has passed the green gate (or, for a documentation issue, the docs gate; for a tests-only issue, the tests gate) and must be reviewed before its pull request is opened, or when asked to review a factory branch.
---

# Factory review: a second reading by an agent that did not write it

Reviews one green CityLoom branch with the `reviewer` agent, which reads the diff, the issue and the plan but never the implementer's account of its work. Findings go back to the `implement` agent (docs flow: the `writer`; tests flow: the `test-writer`) through `factory-build`, and the branch is reviewed again. Below, "green record" is `**Factory · green**` in the code flow, `**Factory · docs**` in the docs flow and `**Factory · tests**` in the tests flow (README, **Three flows**). Read `.claude/factory/README.md` first; `rounds.review` is from `python3 "$CLAUDE_PROJECT_DIR/.claude/factory/factory.py" config`.

## Steps

1. Find the round: `k` is one more than the number of `**Factory · review <k>: …**` records on the issue (the first review is round 1). If `k` is above `rounds.review`, hand back: the review did not converge.
2. The head must be the one the latest green record names (`git -C <wt> rev-parse HEAD`); otherwise the branch is not green yet: go back to `factory-build` step 4 (docs flow: step D2; tests flow: step T2).
3. Dispatch the `reviewer` agent with `<wt>`, `BASE=$(git -C <wt> merge-base HEAD origin/main)`, the head, the flow, the issue's key, title, description and comments, and the plan; in the tests flow, also the tests record's full report (which faults each test caught). Do not pass anything the implementer, the test-writer or the writer said, and do not review the change yourself.
4. Record and act:
   - `pass`: post `**Factory · review <k>: pass** \`<head>\``, and the `testNotes` if there are any. The issue goes on to `factory-evidence`.
   - `findings`: post `**Factory · review <k>: findings** \`<head>\`` with the findings as a list (file and line, defect, why, fix) and the `testNotes`. Then run `factory-build` from step 3 (docs flow: step D1; tests flow: step T1) with the findings as the round's input: the implementer (or the writer, or the test-writer) fixes them, the gate runs again and pushes, and this skill runs again on the new head.
   - A reply that is not the JSON: dispatch once more; a second bad reply is handed back.

**Hand-back**: `labels.needsReview`, `**Factory · handed back** (review)` with the last round's findings. A person decides whether the findings stand.

`testNotes` never go to the implementer, who may not change a test: they are for the person who approves the PR, and the evidence station copies them into the PR's Unknowns. In the tests flow the tests are the change: a weak test is a finding, sent to the test-writer, not a `testNotes` line.

## Report

The round, the verdict, each finding, and what was dispatched next.
