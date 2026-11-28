---
name: factory-evidence
description: Use when a CityLoom issue's branch in the software factory has passed review and needs its evidence captured and its pull request opened, or when a factory PR's head moved and its evidence must be captured again.
---

# Factory evidence: proof that it works, and the pull request

Captures the evidence for one reviewed CityLoom branch, has the `evidence-check` agent hold it against the plan, publishes it, and opens the pull request as the App, ready for a person's approval. Read `.claude/factory/README.md` first. Below, `$F` is `$CLAUDE_PROJECT_DIR/.claude/factory`, and `run`, `evidence`, `github.*` and `rounds.evidence` come from `python3 "$F/factory.py" config`.

## 1. Capture

The head must be the one the latest `**Factory · review <k>: pass**` record names; otherwise go back to `factory-review`. Write every capture to a new scratch folder outside the worktree, numbered in order, each with a caption saying the action and the result seen:

1. **Red**: the `**Factory · red**` record's gate report, as a text file: the new tests, each failing.
2. **Green**: the `**Factory · green**` record's gate report for the head, as a text file: every test passing. When the change has new Rust tests, add `cargo test <each new test>` run at the head, its output as a text file.
3. **The app**: what the plan's `## Change` says a person sees, exercised as the triage's `howToExercise` says, by `run`, captured as `evidence` says: for a browser behaviour, run its spec (`E2E_PORT=<port> just e2e tests/<spec>.spec.ts`, the issue's port: README, **Side by side**) and keep its screenshots and trace, or drive the page and take screenshots of each step and its result. A change with nothing to see (server, model) is evidenced by its tests' output alone, and the PR says so.

**The tests flow** (README, **Three flows**) has no red report and nothing new to see in the app: what it proves is that each new test passes and can fail. Its steps are:

1. **The gate**: the `**Factory · tests**` record's full report for the head, as a text file: each new test, the faults it caught, and the faults no new test caught.
2. **The tests at the head**, each new test's own output: for a Rust test, `cargo test <its name> -- --exact` in `<wt>`, as a text file; for a browser test, `E2E_PORT=<port> npx playwright test tests/<spec>.spec.ts:<line> --trace on` from `<wt>/tests/e2e`, keeping its output and the screenshots its trace holds (`npx playwright show-trace` is not needed: unzip the trace and keep its screenshots), as files.

**The docs flow** has no red and no green report. Its steps are:

1. **The gate**: the `**Factory · docs**` record's gate report for the head, as a text file: the pages changed, nothing else, and the site built.
2. **The pages**, as a reader sees them: in `<wt>`, `just docs-build`, then serve the build on the issue's port, since other issues may be serving theirs (`cd website && npx docusaurus serve --no-open --port <port>`), and for each page of the site the branch adds or changes, a full-page screenshot in English and in fr-CA (`cd tests/e2e && npx playwright screenshot --full-page http://localhost:<port>/cityloom/<page> <file>`, and `.../cityloom/fr-CA/<page>`), one capture per page and language; stop the server after. A changed file outside the site (`docs/`, a `CLAUDE.md`, a README) is evidenced by its text at the head, as a file.

## 2. Check

Dispatch the `evidence-check` agent with the key, title, description, the flow, the plan's `## Change` and the ordered steps with their local paths. Do not judge the evidence yourself. `gaps`: capture what is missing, or, if the change itself is short of a behaviour, send the gaps to `factory-build` step 3 (docs flow: step D1) as findings; then capture and check again, at most `rounds.evidence` rounds, then hand back with the gaps. Rejected evidence is never published.

## 3. Publish

As the App (README, **Acting on GitHub as the App**), add the files under `clm-<n>/<head sha>/` of the branch `github.evidenceBranch`, never the PR's branch, in a scratch worktree of that branch:

```sh
git -C <main checkout> fetch -q origin <evidenceBranch> || true
git -C <main checkout> worktree add <scratch>/evidence origin/<evidenceBranch>     # or, the first time: --detach, then checkout --orphan <evidenceBranch> and rm -rf .
# copy the files, then:
git -C <scratch>/evidence add clm-<n> && git -C <scratch>/evidence commit -m "Evidence for CLM-<n> at <head>"
git -C <scratch>/evidence push origin HEAD:<evidenceBranch>      # rejected: fetch, rebase onto origin/<evidenceBranch>, push again, up to five times
git -C <main checkout> worktree remove --force <scratch>/evidence
```

A file's link is `https://github.com/<github.repo>/raw/<evidenceBranch>/clm-<n>/<head>/<file>`.

## 4. The pull request

Open it as the App from the issue's branch, against `github.base`, titled as the repository's merged PRs are (`<what it does> (<version>) (CLM-<n>)`, a tests-flow PR saying what its tests check; docs flow, which raises no version: `<what it documents> (CLM-<n>)`), with this body:

```
Part of CLM-<n>: <issue title>

## Behavior
One line per line of the plan's Change.

## Proof
For each behaviour, its evidence steps: the step's number and caption, then the capture (an image, or the text in a code block with a link to its file). The red report shows each new test failing and the green report the same test passing: say so, test by test. Docs flow: the docs gate's report, then each page in English and fr-CA. Tests flow: a table of each new test and one fault it caught (from the gate's report), then the gate's report and each test's own output at the head; and under Unknowns, the faults no new test caught and the issues filed for bugs the tests found.

## Risk
- Surface: what the diff touches (the pages, the stored data and its version, the server, the build).
- Level: low, ordinary or high, judged from the diff.

## Unknowns
What was not checked and why, and the reviewer's testNotes. If nothing: "no unknowns identified within the checks performed".

## Rollback plan
- How to roll back: revert the squashed commit (and anything stored data needs).
- When to roll back: what, seen on the live site, would force it.

## Out of scope for this PR
The plan's Out of scope.

## Reviewers
- Intent and code: the person who approves. Built by the software factory: test-writer, implement, reviewer and evidence-check agents; the red and green gates passed. (Docs flow: writer, reviewer and evidence-check agents; the docs gate passed. Tests flow: test-writer, reviewer and evidence-check agents; the tests gate passed, each new test catching a fault.)
```

ending with the attribution lines the session gives for pull requests. Then post `**Factory · PR** <url>` on the issue. The issue stays in `linear.states.working` until step 5 is done: its state is what tells the merge loop the PR is its (README, **Four loops**).

## 5. Ready for approval

Dispatch the `merge` agent with the PR's number, `<wt>` and the issue's `E2E_PORT` (README, **Side by side**), in the background: it brings the PR up to date with `main`, fixes failing checks and SonarCloud issues on the branch, and hands it back green and `CLEAN`; it never merges. If it changed the head, run the green gate on the new head (`factory-build` step 4, which posts a new green record and pushes nothing new; docs flow: the docs gate, step D2; tests flow: the tests gate, step T2, with `--proved <sha>`, the sha of the last `**Factory · tests**` record, which keeps that proof when the rebase changed nothing it rests on and runs the faults again otherwise) and capture, check and publish again (steps 1 to 3), then replace the PR's Proof with `gh pr edit <pr> --body-file <file>`, as the App. A STOP from the merge agent is a hand-back, quoting its report.

Once the merge agent has handed it back green and `CLEAN` (and the evidence is up to date with its head), move the issue to `linear.states.review`. The PR now waits for a person's approval on its head, and the merge loop (`factory-merge`) owns it; if it later falls behind `main` unapproved or a check goes red, the merge loop moves the issue back to `linear.states.working` and the implement loop runs this step again.

**Hand-back**: `labels.needsReview`, `**Factory · handed back** (evidence)` with the gaps or the merge agent's report.

## Report

The evidence steps and the check's verdict, the published links, the PR's URL, the merge agent's report, and every command run.
