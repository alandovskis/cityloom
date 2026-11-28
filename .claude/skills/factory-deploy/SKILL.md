---
name: factory-deploy
description: Use when asked to run the software factory's deploy loop - to watch the deploys of the CityLoom factory PRs the merge loop merged, check the live site and close their issues - or to see which factory merges wait for their deploy. One pass per run; /loop /factory-deploy keeps it going.
---

# Factory deploy: merged PRs, deployed and closed

One pass of the factory's deploy loop (README, **Four loops**): for every factory PR the merge loop merged and whose deploy is not yet recorded, it has the `deploy` agent watch the deploy and check the live site, and closes the issue, or hands it back when the deploy failed. It merges nothing and changes no branch: merging is the merge loop's (`factory-merge`), which merges the next approved PR only once this loop has recorded the deploy of the one before. Read `.claude/factory/README.md` first. It keeps no state: each pass reads Linear and GitHub again.

**Unattended**: `/loop /factory-deploy`, about every five minutes (a deploy takes minutes, and the merge loop waits on its record), in its own session beside the other loops, on the machine that holds the worktrees (it removes them).

## 0. Before anything

README, **Before a pass**, steps 1, 2 and 4: the configuration, the App's token (to read the PRs and runs) and Linear. It runs no test, so it needs no database; `git fetch -q origin` is needed only for the worktree's removal.

## 1. Read where everything is

List the team's issues in `linear.states.review` and `linear.states.working`, and those in `linear.states.done` updated in the last seven days (Linear may close an issue itself when its PR merges), that carry a `**Factory · merged**` record, or a `**Factory · PR**` record whose PR is merged (one merged before the merge loop wrote `merged` records: its merge commit is read from the PR, `gh pr view <pr> --json mergeCommit,mergedAt`), no `**Factory · deployed**` record and no waiting label (README, **Where an issue is**, steps 1 and 2). Write the list down before acting: key, PR, merge commit.

## 2. Watch the deploys

Dispatch the `deploy` agent in the background with every merge of the list, oldest first: the PR and its merge commit, and **after a cleared failure** for an issue that has a `**Factory · handed back** (deploy)` record and no longer carries `labels.deployFailed` (a person made `main` green again and gave it back). It finds each merge's Postsubmit run (and Documentation run, for `website/**`), watches it, follows a deploy that a later run replaced, and checks the site; it merges, re-runs and changes nothing. Do not watch the runs yourself. It reports one line per merge. While a `deploy` agent this session dispatched is still running, dispatch no other: take its report when it ends.

## 3. Record

For each line of the agent's report:

- `deployed, <run url>` (also `deployed with #<m>'s run`): post `**Factory · deployed** <run url>`, move the issue to `linear.states.done` (if it is not there already), remove the label `labels.feasible`, and remove the worktree and local branch (`git worktree remove <wt>`, `git branch -D <branch>`; the merge deleted the remote one), taking their turn on the main checkout's git (README, **Side by side**).
- `FAILED`: add `labels.deployFailed`, post `**Factory · handed back** (deploy)` with the agent's line and the output it quoted, and move the issue back to `linear.states.working`. While the issue carries the label, the merge loop merges nothing (`factory-merge`, **The deploy is clear**), so the line waits for a person: once `main` is green again, they remove the label, and the next pass gives the merge to the agent again, after a cleared failure. Never re-run a job, retry the deploy or fix it here.
- A merge the report leaves out, or a report not in the agent's shape: record nothing for it; the next pass gives it to the agent again.

## 4. Report

Every merge given to the `deploy` agent: PR, issue, merge commit, its line of the report, and what this pass recorded; the hand-backs with their reasons; whether the agent is still running. When there was nothing to watch, say the deploy loop is idle. Then print the status (README, **Status**) with `deploy` as its loop, as the last thing of the run, also when the pass stopped before it began.

## Red flags

| Thought | Reality |
| --- | --- |
| "The deploy job failed on a flaky step, I'll re-run it" | Never re-run, cancel or retry. Label, hand back. |
| "The agent is slow, I'll watch the run myself" | The `deploy` agent watches; the loop records its report. |
| "The run succeeded, the site check is a formality" | The issue is deployed only once the agent's line says `deployed`, site checked. |
| "An approved PR is waiting and nothing is deploying, I'll merge it" | Merging is the merge loop's: this loop merges nothing. |
