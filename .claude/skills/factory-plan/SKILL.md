---
name: factory-plan
description: Use when the software factory has a CityLoom issue triaged as feasible and needs its plan, or when asked to plan a Linear issue for the factory before its tests are written.
---

# Factory plan: the plan the tests are written from

Writes and posts the plan for one feasible CityLoom issue, then makes the issue's worktree. Read `.claude/factory/README.md` first. Take names from `python3 "$CLAUDE_PROJECT_DIR/.claude/factory/factory.py" config`.

## Steps

1. Read the issue (`get_issue`, `list_comments` every page) and its `**Factory · triage: feasible**` record, and the flow from its `Flow:` line (none: `code`; README, **Two flows**). If a `**Factory · plan**` comment is already there (a person may have written it), skip to step 4.
2. Dispatch the `planner` agent with the key, title, description, comments, the flow and the triage's three texts. Do not plan the issue yourself.
3. Check its reply before posting it. It is a `QUESTION:` line: hand the issue back (below) with the question. Otherwise:
   - **code** flow: it must have the seven headings `## Change`, `## Where`, `## Slice dependencies`, `## Tests`, `## Strings`, `## Version`, `## Out of scope`, in that order, `## Slice dependencies` must say "None" or give each dependency its user, its module path, its table and its reason (a new slice, its name), `## Tests` must list at least one test and every line of `## Change` must be proved by one, and `## Version` must say minor or patch.
   - **docs** flow: it must have the five headings `## Change`, `## Where`, `## Facts`, `## Translation`, `## Out of scope`, in that order, `## Where` must name at least one page, each page of the site with both its English and its fr-CA path, and every line of `## Change` must rest on a line of `## Facts` that names a file. There is no `## Version`: a documentation change raises none.

   A plan that fails a check goes back to the planner once, with what failed; failing again is a hand-back. Then post it: `save_comment` whose body is `**Factory · plan**`, a blank line, and the plan.
4. Make the worktree, from the main checkout, unless it exists; other issues may be making theirs, so these git commands take their turn (README, **Side by side**):

   ```sh
   git fetch -q origin
   git worktree add "<worktrees>/clm-<n>" -b "<gitBranchName>" origin/<github.base>
   ```

   Then set it up once, as any new checkout: `just setup` in it (`npm ci` at its root and in `e2e/`, `wasm-bindgen` into its `.tools`, Playwright's Chromium); for the docs flow, `npm ci` in `e2e/` (the Prettier the check uses) and `just setup-docs` instead. Without it the gates refuse to run (`could not run a tool`).

   `<worktrees>` is `config.worktrees` against the main checkout's folder (`<parent>/main` gives `<parent>/worktrees`); `<gitBranchName>` is Linear's for the issue. A branch of that name already on `origin` or in another worktree: hand back, naming it (someone else's work). Nothing is pushed: the branch goes to `origin` once green.

**Hand-back**: add `labels.needsReview` (`save_issue`, `addLabels`), post `**Factory · handed back** (plan)` with the question or what failed, and stop. The person answers, or writes the plan as a `**Factory · plan**` comment, and removes the label.

## Report

The plan posted (or the hand-back), the worktree's path and branch, and every call and command made, in order.
