---
name: factory-build
description: Use when the software factory has a planned CityLoom issue and its worktree and must turn the plan into tests and code - the test-writer's red commit, the red gate, the implementer, the green gate and the push - or, for a documentation issue, into pages - the writer, the docs gate and the push - or, for an issue that asks only for tests, into tests proved by faults - the test-writer, the tests gate and the push - or when the review sends findings back to the implementer, the writer or the test-writer.
---

# Factory build: tests first, then the code, each through its gate

Takes one planned CityLoom issue from its plan to a green branch on `origin`. The tests are written first by the `test-writer`, who never sees an implementation; the code by the `implement` agent, who may not change a test. A script decides each hand-over: `gate.py red` and `gate.py green`. Read `.claude/factory/README.md` first. Below, `$F` is `$CLAUDE_PROJECT_DIR/.claude/factory`, `<wt>` the issue's worktree, and every round limit is `rounds.*` of `python3 "$F/factory.py" config`.

The flow is the `Flow:` line of the issue's `**Factory · triage: feasible**` record (none: `code`; README, **Three flows**). The steps 1 to 4 below are the code flow's; an issue of the docs flow goes to **The docs flow** instead, and one of the tests flow to **The tests flow**.

Run the gates from the main checkout's copy of `gate.py` (`$F`), never from the worktree's: the worktree is what is judged. They need the database: `just db` prints the `DATABASE_URL` to export. The base is the commit the branch started from: `BASE=$(git -C <wt> merge-base HEAD origin/main)`. Other issues build beside this one (README, **Side by side**): run every gate with the issue's port, `E2E_PORT=$((20000 + <n>))`, in the environment, and give that port to every agent you dispatch for it.

## 1. The red commit

Skip to step 3 when a `**Factory · red**` record's sha is in `<wt>`'s history (`git -C <wt> merge-base --is-ancestor <sha> HEAD`). When `<wt>` already has commits after `BASE` but no such record (the line stopped mid-station), go to step 2 with them.

Dispatch the `test-writer` agent with `<wt>`, the issue's key, title, description and comments, and the `**Factory · plan**` record's plan. Do not write tests yourself. It returns `{ red, tests, stubs, notes }`, or `{ question }`: hand the question back (below).

## 2. The red gate

```sh
python3 "$F/gate.py" red --repo <wt> --base "$BASE"
```

- `PASS`: post `**Factory · red** \`<HEAD sha>\``, then the gate's report in a code block and the test-writer's `notes`. Go on.
- `FAIL`: dispatch the `test-writer` again with the gate's problems, quoted, and gate again; at most `rounds.red` rounds in all, then hand back with the last report.
- `REFUSED` (a dirty worktree, no database, HEAD not after the base): fix what it names if it is the environment (start the database), otherwise hand back with it.

## 3. The implementer

Skip to step 4 when a `**Factory · green**` record names `<wt>`'s HEAD.

Dispatch the `implement` agent with `<wt>`, the issue's key, title, description and comments, the plan, the red record's sha and its gate report, and on a later round the green gate's problems or the review's findings (the `factory-review` skill sends you back here with them). Do not write the code yourself. The guard of `.claude/settings.json` refuses its edits to the tests; it may still try, and the green gate catches whatever got through.

It returns `{ head, commits, version, checks, notes }`, or `{ dispute }`: a test it believes is wrong, or a dependency between slices the plan did not name, which the dependency guard refuses. A dispute is handed back, quoting it whole: a person decides whether the test or the implementer is right, and changes the test (or the guard's table) in `<wt>` if it is the test (a new commit with the `Factory-Red` trailer, then a new red record by hand or by running step 2).

## 4. The green gate

```sh
python3 "$F/gate.py" green --repo <wt> --base "$BASE" --red <red sha>
```

`<red sha>` is the red record's sha while it is in `<wt>`'s history. After a rebase (the `merge` agent brings the PR up to date with `main`) the red commit has a new sha: it is then the last commit of the branch carrying the trailer, `git -C <wt> log -1 --format=%H --grep '^Factory-Red: CLM-<n>$' "$BASE"..HEAD`; say so in the green record. No such commit means the history lost the red commit: hand back.

- `PASS`: push the branch as the App (README, **Acting on GitHub as the App**): `git -C <wt> push -u origin HEAD` with the App's credential helper. The red commit goes up with the green ones, so the branch shows the tests failing and then passing. Then post `**Factory · green** \`<HEAD sha>\``, the gate's report and the implementer's `notes`. The issue goes on to `factory-review`.
- `FAIL`: back to step 3 with the gate's problems, quoted; at most `rounds.green` rounds in all, then hand back with the last report. A problem that says a test or a guarded file changed since the red commit is never fixed by the implementer changing it back by hand: send it back, and if it comes again, hand back.
- `REFUSED`: as for the red gate. A red commit not in the history means the history was rewritten: hand back.

A push that git rejects (someone else pushed to the branch) is a hand-back: never force-push.

## The docs flow

Its build is one agent and one gate. Skip to step D2 when `<wt>` already has commits after `BASE` and no `**Factory · docs**` record names its HEAD (the line stopped mid-station); skip the station when one does.

**D1. The writer.** Dispatch the `writer` agent with `<wt>`, the issue's key, title, description and comments, the `**Factory · plan**` record's plan, and on a later round the docs gate's problems or the review's findings (the `factory-review` skill sends you back here with them). Do not write the pages yourself. It returns `{ head, commits, pages, checks, notes }`, or `{ question }`: hand the question back (below).

**D2. The docs gate.** It needs no database.

```sh
python3 "$F/gate.py" docs --repo <wt> --base "$BASE"
```

- `PASS`: push the branch as the App (README, **Acting on GitHub as the App**): `git -C <wt> push -u origin HEAD` with the App's credential helper. Then post `**Factory · docs** \`<HEAD sha>\``, the gate's report in a code block and the writer's `notes`. The issue goes on to `factory-review`.
- `FAIL`: back to D1 with the gate's problems, quoted; at most `rounds.green` rounds in all, then hand back with the last report. A problem that names a file that is not documentation, or a guarded one, is never fixed by you: send it back, and if it comes again, hand back.
- `REFUSED`: as for the red gate (a site not set up: `npm ci` in `e2e/` and `just setup-docs` in `<wt>`, then gate again).

A push that git rejects is a hand-back: never force-push.

## The tests flow

Its build is one agent and one gate, and no implementer: the code already works. The plan is the latest `**Factory · plan**` record; write it to a scratch file outside the worktree, `<plan file>`, for the gate, which reads its `## Faults`. Skip to step T2 when `<wt>` already has commits after `BASE` and no `**Factory · tests**` record names its HEAD; skip the station when one does.

**T1. The test-writer.** Dispatch the `test-writer` agent with `<wt>`, the flow (`tests`), the issue's key, title, description and comments, the plan, the port, and on a later round the tests gate's brief report (never the full one: the test-writer does not see the faults) or the review's findings. It returns `{ head, tests, bugs, notes }`, or `{ question }`: hand the question back (below).

Each of `bugs` is a test that fails on today's code, which the test-writer left out: the code has a bug. File each as a new issue in Linear (`save_issue`: `linear.team`, state `Triage`, `relatedTo` this issue, a title saying what is wrong, and a description with the test's name and code and its failure, quoted), and keep their keys for the tests record and the PR's Unknowns. If `tests` is empty, nothing is left to prove: hand back, naming the issues filed.

**T2. The tests gate.** It needs the database and the main checkout's `cargo-mutants` (README, **Three flows**).

```sh
python3 "$F/gate.py" tests --repo <wt> --base "$BASE" --faults <plan file> --brief --report <scratch>/tests-report.txt
```

It prints the brief report, for the test-writer, and writes the full one, for everyone else. It takes minutes: a build of the tests, then a rebuild and a run for each fault.

- `PASS`: push the branch as the App (README, **Acting on GitHub as the App**): `git -C <wt> push -u origin HEAD` with the App's credential helper. Then post `**Factory · tests** \`<HEAD sha>\``, the full report in a code block, the test-writer's `notes` and the issues filed from its `bugs`. The issue goes on to `factory-review`.
- `FAIL`, every problem but a test that catches nothing: back to T1 with the brief report's problems, quoted.
- `FAIL` because a new test catches none of the faults run: back to T1 once, with the brief report. If the same tests still catch nothing, send the planner (`factory-plan`'s `planner` agent) the plan and the full report, never a test's source: it may replace `## Faults` once. Check its reply as `factory-plan` step 3 does (`gate.py faults` included), post it as a new `**Factory · plan**` record saying the faults were revised, write it to `<plan file>`, and run the gate again; a `QUESTION:` from it is a hand-back. A test that still catches nothing after that, or one that caught nothing because the limit left faults out (the full report's `left out by the limit`), is a hand-back with the full report.
- These rounds, and every other `FAIL`, count in `rounds.green`; past it, hand back with the full report.
- `REFUSED`: as for the red gate. `cargo-mutants` missing: hand back with `labels.needsSetup` and the command the gate gives. A note left by a gate that was stopped while it held a fault: do what the refusal says (put the named file back with `git -C <wt> checkout -- <file>`, `just build` in `<wt>`, delete the note), then run the gate again.

A push that git rejects is a hand-back: never force-push.

**Hand-back**: `labels.needsReview` (`save_issue`, `addLabels`), `**Factory · handed back** (build)` with the station, the round, and the exact output that blocked it. The worktree and its commits stay as they are.

## Report

Each step taken, each gate's verdict with its report, the red and green shas (docs flow: the docs record's sha; tests flow: the tests record's sha), the push, the records posted, the issues filed for bugs the tests found, and any hand-back with its reason.
