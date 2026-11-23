---
name: factory
description: Use when asked to run the software factory, to work CityLoom's Linear issues through to merged and deployed changes unattended, to resume the factory after it stopped, or to see what the factory has in flight. One pass per run; /loop /factory keeps it going.
---

# The software factory: one pass of the line

Runs one pass of CityLoom's software factory: every issue in flight moves on by as many stations as it can, side by side (up to `caps.inFlight` issues building and `caps.awaitingApproval` PRs open), approved PRs are merged and deployed, and new issues are taken from Linear while there is room, from the intake first and, when nothing there is ready, from the states of `linear.fallback`. Read `.claude/factory/README.md` first: the line, the records, where an issue is, side by side, acting as the App, the hand-back. The line keeps no state: each pass reads Linear, Git and GitHub again, so stopping anywhere and running `/factory` again resumes every issue where it was.

**Unattended**: `/loop /factory` runs a pass, waits, and runs the next; between passes with nothing to do but wait for a person or CI, wait about ten minutes. A session left running this way needs its tool permissions settled beforehand (auto mode, or an allow list), since no one is there to answer a prompt, and the workspace trusted, since the guard is a project hook.

## 0. Before anything

From the main checkout (`$CLAUDE_PROJECT_DIR`):

1. `python3 .claude/factory/factory.py config`: every problem it lists stops the pass. Report them; do not fill in a value yourself.
2. `export GH_TOKEN=$(python3 .claude/factory/factory.py token)`, noting the time; mint again after 55 minutes or on an authentication error. A failure stops the pass.
3. `git fetch -q origin` and the database for the gates: `just db`, exporting the `DATABASE_URL` it prints.
4. The Linear tools answer (`list_issue_statuses` for `linear.team`, and every state of `linear.states`, `linear.intake` and `linear.fallback` is among them). If not, stop and say so.

## 1. Read where everything is

List the team's issues in `linear.states.working` and `linear.states.review`, and the issues with an open or merged factory PR whose deploy has no `**Factory · deployed**` record. For each, read its labels and comments and its worktree, and place it by README **Where an issue is**. Write the list down before acting: key, place, what it waits for.

## 2. Approved PRs

For each open factory PR whose issue carries no waiting label (all of them, side by side, up to `caps.awaitingApproval`):

1. **Approved?** Read the reviews over REST (`gh api --paginate repos/<github.repo>/pulls/<pr>/reviews`) and the head (`gh pr view <pr> --json headRefOid`). Take each reviewer's latest review by `submitted_at`, ignoring `COMMENTED`. It is approved only when at least one latest review is `APPROVED` with `commit_id` equal to the head (full sha), from an `author_association` `OWNER`, `MEMBER` or `COLLABORATOR`, a `user.type` other than `Bot` and a `user.login` other than the PR's author, and no reviewer's latest is `CHANGES_REQUESTED`. Otherwise it waits: leave it.
2. **Merge and deploy.** Dispatch the `deploy` agent with this one PR and its worktree: the person's approval of the head is the person's word to merge it. It rebases on the version line only, re-runs the tests, waits for CI, merges, watches the deploy and checks the site, and stops on anything else.
   - `Merged: #<pr> (<version>, deploy success)`: post `**Factory · deployed** <run url>`, move the issue to `linear.states.done`, remove the label `labels.feasible`, and remove the worktree and local branch (`git worktree remove <wt>`, `git branch -D <branch>`; the merge deleted the remote one).
   - A STOP after the merge, in the deploy: add `labels.deployFailed`, post `**Factory · handed back** (deploy)` with the agent's report, and move the issue back to `linear.states.working`. Never retry a deploy or fix it here.
   - A STOP before the merge (a conflict, a red check): hand back with the agent's report. A PR that only went `BEHIND` goes to the `merge` agent first (`factory-evidence` step 5), and waits for approval again if its head moved.

**Side by side, then one at a time.** Read every PR's approval first. The PRs that need the `merge` agent (behind `main`, a red check) get one each, all dispatched at once in the background, each with its own worktree and port (README, **Side by side**). The approved ones are merged by the `deploy` agent **one at a time**, the next only after it has reported on the one before: each merge moves `main` and starts a deploy to production, so a second merge started meanwhile would be tested against a `main` that no longer exists, and two deploys would race. The approved PRs wait their turn in the order they were approved.

## 3. Issues in flight

Every issue in flight that waits for no person runs down the line at once, each on its own (README, **Side by side**): start the station its place names for each of them, and keep each one going down the line while its stations pass:

| Place | Station |
| --- | --- |
| feasible, no plan | `factory-plan` |
| planned, worktree or not | `factory-plan` step 4 if no worktree, then `factory-build` (from the test-writer, or the writer in the docs flow) |
| red, not green | `factory-build` step 3 |
| docs flow, commits but no `docs` record at the head | `factory-build` step D2 |
| green (docs flow: `docs` record), not reviewed at the head | `factory-review` |
| reviewed and passed at the head | `factory-evidence` |

A station that hands back ends that issue's turn; the others go on. Do not do a station's work yourself: each station's skill dispatches its own agent.

**How to run them side by side.** Subagents cannot dispatch subagents, so the issues are not handed to one agent each: you run every issue's stations yourself, interleaved. Dispatch each station's agent in the background and run each gate (`gate.py`, `just e2e`, captures) as a background command, so that no issue waits on another's agent or gate. When one finishes, take that issue's next step and dispatch it the same way. Keep a line per issue (key, worktree, port, station, round) and update it at each step: the rounds a station counts are per issue. The section ends when every issue has stopped: handed back, waiting for a person or CI, or its PR ready for approval.

## 4. New issues

`factory-queue` returns as many issues as there is room for, from `linear.intake`, and when nothing there is ready, from the states of `linear.fallback` (Triage, then Backlog). Run `factory-triage` on all of them at once, the `triage` agents in the background, and send each feasible one down the line beside the others (section 3) as soon as its triage is recorded. A feasible issue taken from a `linear.fallback` state goes on exactly as one from the intake: triage moves it to `linear.states.working`. When a triage hands an issue back, room is freed: ask the queue again, and stop when it returns none.

## 5. Report

At the end of the pass, the table of every issue: key, title, where it is, what it waits for (a person's answer, an approval on PR #<n>, CI) and what this pass did to it; the PRs merged with their deploy result; the hand-backs with their reasons. Then, when nothing moved this pass and nothing is running, say the line is idle.

## Red flags

| Thought | Reality |
| --- | --- |
| "The gate is wrong here, I'll pass it on" | Only `PASS` goes on. A gate you believe is wrong is a hand-back with its output. |
| "It's a one-line fix, I'll make it myself" | Stations dispatch their agents. The orchestrator writes no test and no code. |
| "The test is clearly wrong, let the implementer fix it" | A dispute goes to a person. Nobody but the test-writer changes a test. |
| "It's documentation, it needs no gate" | The docs flow has its own: only the docs gate's `PASS` goes on. |
| "This doc issue also needs a small code change" | Then it is the code flow's. The docs flow changes only documentation. |
| "It was approved before the last push" | Only an approval of the current head counts. |
| "The deploy failed, a quick fix will do" | Label, hand back, move on. |
| "The issue in Backlog looks easy, I'll take it now" | `linear.fallback` is read only when nothing in `linear.intake` is ready, in its order, through the queue. |
| "Two PRs are approved, I'll merge both at once" | Merges and deploys are one at a time. Only the `merge` agent's preparation runs side by side. |
| "This issue's e2e can use the usual port" | Every issue has its own `E2E_PORT`: a server on another's port would be reused, serving the wrong build. |
