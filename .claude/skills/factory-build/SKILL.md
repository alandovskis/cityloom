---
name: factory-build
description: Use when the software factory has a planned CityLoom issue and its worktree and must turn the plan into tests and code - the test-writer's red commit, the red gate, the implementer, the green gate and the push - or when the review sends findings back to the implementer.
---

# Factory build: tests first, then the code, each through its gate

Takes one planned CityLoom issue from its plan to a green branch on `origin`. The tests are written first by the `test-writer`, who never sees an implementation; the code by the `implement` agent, who may not change a test. A script decides each hand-over: `gate.py red` and `gate.py green`. Read `.claude/factory/README.md` first. Below, `$F` is `$CLAUDE_PROJECT_DIR/.claude/factory`, `<wt>` the issue's worktree, and every round limit is `rounds.*` of `python3 "$F/factory.py" config`.

Run the gates from the main checkout's copy of `gate.py` (`$F`), never from the worktree's: the worktree is what is judged. They need the database: `just db` prints the `DATABASE_URL` to export. The base is the commit the branch started from: `BASE=$(git -C <wt> merge-base HEAD origin/main)`.

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

It returns `{ head, commits, version, checks, notes }`, or `{ dispute }`: a test it believes is wrong. A dispute is handed back, quoting it whole: a person decides whether the test or the implementer is right, and changes the test in `<wt>` if it is the test (a new commit with the `Factory-Red` trailer, then a new red record by hand or by running step 2).

## 4. The green gate

```sh
python3 "$F/gate.py" green --repo <wt> --base "$BASE" --red <red sha>
```

`<red sha>` is the red record's sha while it is in `<wt>`'s history. After a rebase (the `merge` agent brings the PR up to date with `main`) the red commit has a new sha: it is then the last commit of the branch carrying the trailer, `git -C <wt> log -1 --format=%H --grep '^Factory-Red: CLM-<n>$' "$BASE"..HEAD`; say so in the green record. No such commit means the history lost the red commit: hand back.

- `PASS`: push the branch as the App (README, **Acting on GitHub as the App**): `git -C <wt> push -u origin HEAD` with the App's credential helper. The red commit goes up with the green ones, so the branch shows the tests failing and then passing. Then post `**Factory · green** \`<HEAD sha>\``, the gate's report and the implementer's `notes`. The issue goes on to `factory-review`.
- `FAIL`: back to step 3 with the gate's problems, quoted; at most `rounds.green` rounds in all, then hand back with the last report. A problem that says a test or a guarded file changed since the red commit is never fixed by the implementer changing it back by hand: send it back, and if it comes again, hand back.
- `REFUSED`: as for the red gate. A red commit not in the history means the history was rewritten: hand back.

A push that git rejects (someone else pushed to the branch) is a hand-back: never force-push.

**Hand-back**: `labels.needsReview` (`save_issue`, `addLabels`), `**Factory · handed back** (build)` with the station, the round, and the exact output that blocked it. The worktree and its commits stay as they are.

## Report

Each step taken, each gate's verdict with its report, the red and green shas, the push, the records posted, and any hand-back with its reason.
