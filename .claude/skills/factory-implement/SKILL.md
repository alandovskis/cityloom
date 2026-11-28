---
name: factory-implement
description: Use when asked to run the software factory's implement loop - to take planned CityLoom issues through their worktree, tests, code, review and evidence to a pull request ready for approval - or to see what the factory is building. One pass per run; /loop /factory-implement keeps it going.
---

# Factory implement: from the plan to a PR ready for approval

One pass of the factory's implement loop (README, **Three loops**): every planned issue moves on by as many stations as it can, side by side, from its worktree to a pull request that is green, `CLEAN` and waiting for a person's approval. It starts at the `**Factory · plan**` record (posted by `factory-intake`) and ends when the issue is in `linear.states.review`, where the merge loop (`factory-merge`) takes it. Read `.claude/factory/README.md` first. It keeps no state: each pass reads Linear, Git and GitHub again.

**Unattended**: `/loop /factory-implement`, in its own session beside the other two loops. While agents or gates run, the next pass starts when they finish; with nothing to do but wait for a person or CI, wait about ten minutes. Needs its tool permissions settled beforehand (auto mode, or an allow list) and the workspace trusted, since the guard is a project hook.

## 0. Before anything

README, **Before a pass**, steps 1 to 4: the configuration, the App's token, `git fetch` and the database, Linear.

## 1. Read where everything is

List the team's issues in `linear.states.working` that have a `**Factory · plan**` record. For each, read its labels and comments and its worktree, and place it by README **Where an issue is** (steps 2 to 8). Leave alone those waiting for a person (step 2). Write the list down before acting: key, worktree, port, place, what it waits for.

## 2. Issues in flight

Every issue that waits for no person runs down the line at once, each on its own (README, **Side by side**): start the station its place names for each of them, and keep each one going down the line while its stations pass:

| Place | Station |
| --- | --- |
| planned, no worktree | `factory-plan` step 4, then `factory-build` (from the test-writer, or the writer in the docs flow) |
| planned, worktree | `factory-build` (from the test-writer, or the writer in the docs flow) |
| red, not green | `factory-build` step 3 |
| docs flow, commits but no `docs` record at the head | `factory-build` step D2 |
| tests flow, commits but no `tests` record at the head | `factory-build` step T2 |
| green (docs flow: `docs` record; tests flow: `tests` record), not reviewed at the head | `factory-review` |
| reviewed and passed at the head, no PR | `factory-evidence` |
| factory PR open, issue still in `linear.states.working` | `factory-evidence` step 5 (the `merge` agent, then the gate and the evidence again if the head moved) |

The last row is also how a PR comes back from the merge loop: when `main` moved under it or a check went red, `factory-merge` moves its issue back to `linear.states.working`, and this loop prepares it again.

A station that hands back ends that issue's turn; the others go on. Do not do a station's work yourself: each station's skill dispatches its own agent.

**How to run them side by side.** Subagents cannot dispatch subagents, so the issues are not handed to one agent each: you run every issue's stations yourself, interleaved. Dispatch each station's agent in the background and run each gate (`gate.py`, `just e2e`, captures) as a background command, so that no issue waits on another's agent or gate. When one finishes, take that issue's next step and dispatch it the same way. Keep a line per issue (key, worktree, port, station, round) and update it at each step: the rounds a station counts are per issue. The pass ends when every issue has stopped: handed back, waiting for CI, or in `linear.states.review`.

**Opening PRs.** `factory-evidence` opens a PR only while fewer than `caps.awaitingApproval` factory PRs are open; an issue reviewed and passed beyond that waits at its place for the next pass.

## 3. Report

The table of every issue: key, title, where it is, what it waits for and what this pass did to it; the PRs opened or prepared again, each now in `linear.states.review`; the hand-backs with their reasons; the bugs filed by the tests flow. When nothing moved and nothing is running, say the implement loop is idle.

## Red flags

| Thought | Reality |
| --- | --- |
| "The gate is wrong here, I'll pass it on" | Only `PASS` goes on. A gate you believe is wrong is a hand-back with its output. |
| "It's a one-line fix, I'll make it myself" | Stations dispatch their agents. The orchestrator writes no test and no code. |
| "The test is clearly wrong, let the implementer fix it" | A dispute goes to a person. Nobody but the test-writer changes a test. |
| "It's documentation, it needs no gate" | The docs flow has its own: only the docs gate's `PASS` goes on. |
| "This doc issue also needs a small code change" | Then it is the code flow's. The docs flow changes only documentation. |
| "This test caught no fault; the faults are wrong, I'll pass it" | Only the tests gate's `PASS` goes on. The planner may revise the faults once; after that it is a hand-back. |
| "The PR is open, I'll move the issue to review now" | Only once the `merge` agent has handed it back green and `CLEAN` (`factory-evidence` step 5): the state tells the merge loop it may take it. |
| "This issue has no plan yet, I'll plan it here" | Triage and plan are the intake loop's. |
| "This issue's e2e can use the usual port" | Every issue has its own `E2E_PORT`: a server on another's port would be reused, serving the wrong build. |
