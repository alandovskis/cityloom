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
3. **The app**: what the plan's `## Change` says a person sees, exercised as the triage's `howToExercise` says, by `run`, captured as `evidence` says: for a browser behaviour, run its spec (`just e2e tests/<spec>.spec.ts`) and keep its screenshots and trace, or drive the page and take screenshots of each step and its result. A change with nothing to see (server, model) is evidenced by its tests' output alone, and the PR says so.

## 2. Check

Dispatch the `evidence-check` agent with the key, title, description, the plan's `## Change` and the ordered steps with their local paths. Do not judge the evidence yourself. `gaps`: capture what is missing, or, if the change itself is short of a behaviour, send the gaps to `factory-build` step 3 as findings; then capture and check again, at most `rounds.evidence` rounds, then hand back with the gaps. Rejected evidence is never published.

## 3. Publish

As the App (README, **Acting on GitHub as the App**), add the files under `clm-<n>/<head sha>/` of the branch `github.evidenceBranch`, never the PR's branch, in a scratch worktree of that branch:

```sh
git -C <main checkout> fetch -q origin <evidenceBranch> || true
git -C <main checkout> worktree add <scratch>/evidence origin/<evidenceBranch>     # or, the first time: --detach, then checkout --orphan <evidenceBranch> and rm -rf .
# copy the files, then:
git -C <scratch>/evidence add clm-<n> && git -C <scratch>/evidence commit -m "Evidence for CLM-<n> at <head>"
git -C <scratch>/evidence push origin HEAD:<evidenceBranch>      # rejected: fetch, rebase onto origin/<evidenceBranch>, push again, once
git -C <main checkout> worktree remove --force <scratch>/evidence
```

A file's link is `https://github.com/<github.repo>/raw/<evidenceBranch>/clm-<n>/<head>/<file>`.

## 4. The pull request

Open it as the App from the issue's branch, against `github.base`, titled as the repository's merged PRs are (`<what it does> (<version>) (CLM-<n>)`), with this body:

```
Part of CLM-<n>: <issue title>

## Behavior
One line per line of the plan's Change.

## Proof
For each behaviour, its evidence steps: the step's number and caption, then the capture (an image, or the text in a code block with a link to its file). The red report shows each new test failing and the green report the same test passing: say so, test by test.

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
- Intent and code: the person who approves. Built by the software factory: test-writer, implement, reviewer and evidence-check agents; the red and green gates passed.
```

ending with the attribution lines the session gives for pull requests. Then post `**Factory · PR** <url>` on the issue and move it to `linear.states.review`.

## 5. Ready for approval

Dispatch the `merge` agent with the PR's number and `<wt>`: it brings the PR up to date with `main`, fixes failing checks and SonarCloud issues on the branch, and hands it back green and `CLEAN`; it never merges. If it changed the head, run the green gate on the new head (`factory-build` step 4, which posts a new green record and pushes nothing new) and capture, check and publish again (steps 1 to 3), then replace the PR's Proof with `gh pr edit <pr> --body-file <file>`, as the App. A STOP from the merge agent is a hand-back, quoting its report.

The PR now waits for a person's approval on its head.

**Hand-back**: `labels.needsReview`, `**Factory · handed back** (evidence)` with the gaps or the merge agent's report.

## Report

The evidence steps and the check's verdict, the published links, the PR's URL, the merge agent's report, and every command run.
