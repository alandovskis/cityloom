---
name: factory-deploy
description: Use when asked to run the software factory's deploy loop - to watch the deploys of the CityLoom factory PRs the merge loop merged, check the live site and close their issues - or to see which factory merges wait for their deploy. One pass per run; /loop /factory-deploy keeps it going.
---

# Factory deploy: merged PRs, deployed and closed

One pass of the factory's deploy loop (README, **Four loops**): for every factory PR the merge loop merged and whose deploy is not yet recorded, it watches the deploy, checks the live site and closes the issue, or hands it back when the deploy failed. It merges nothing and changes no branch: merging is the merge loop's (`factory-merge`), which merges the next approved PR only once this loop has recorded the deploy of the one before. Read `.claude/factory/README.md` first. It keeps no state: each pass reads Linear and GitHub again.

**Unattended**: `/loop /factory-deploy`, about every five minutes (a deploy takes minutes, and the merge loop waits on its record), in its own session beside the other loops, on the machine that holds the worktrees (it removes them).

## 0. Before anything

README, **Before a pass**, steps 1, 2 and 4: the configuration, the App's token (to read the PRs and runs) and Linear. It runs no test, so it needs no database; `git fetch -q origin` is needed only for the worktree's removal.

## 1. Read where everything is

List the team's issues in `linear.states.review` and `linear.states.working`, and those in `linear.states.done` updated in the last seven days (Linear may close an issue itself when its PR merges), that carry a `**Factory · merged**` record, or a `**Factory · PR**` record whose PR is merged (one merged before the merge loop wrote `merged` records: its merge commit is read from the PR, `gh pr view <pr> --json mergeCommit,mergedAt`), no `**Factory · deployed**` record and no waiting label (README, **Where an issue is**, steps 1 and 2). Write the list down before acting: key, PR, merge commit, whether it changes `website/**`.

## 2. Watch each deploy

For each, oldest merge first, do as the `deploy` agent's last steps say (`.claude/agents/deploy.md`, **Watch the deploy** and **Check the site**), read-only:

1. **The runs.** The Postsubmit run (`deploy.workflow` of `.github/issue-flow.json`) on `github.base` whose `headSha` is the merge commit: `gh run list --repo <github.repo> --branch <github.base> --workflow postsubmit.yml --limit 20 --json databaseId,headSha,status,conclusion,url`. When the PR changes `website/**`, the Documentation workflow's run too (`--workflow docs.yml`), whose `Deploy the documentation` job must be `success`. A PR of documentation alone outside `website/` starts neither: there is nothing to watch, and it is deployed as soon as it is merged.
   - No run for the merge commit yet: it waits for the next pass. Still none `deploy.timeoutMinutes` after the merge (`mergedAt`): a failure.
   - A run not `completed`: `gh run watch <id> --repo <github.repo>` in the background, and take its result when it ends; watches of several merges run side by side. Still running `deploy.timeoutMinutes` after the merge: a failure.
   - `Deploy to Railway` `cancelled` while a later merge's run deploys it: normal (the `deploy-production` group keeps only the newest pending deploy). This merge goes out with that later run's deploy: take that run's result for it, success or failure, and its URL.
   - A run that failed, on an issue a person has given back by removing `labels.deployFailed` (after making `main` green again): the newest Postsubmit run on `github.base` started after the merge carries this merge too. Take its result and URL instead.
2. **The site**, once the deploy succeeded: `curl -s -A cityloom-deploy -o /dev/null -w '%{http_code}' https://cityloom.app/` is `200`, and `curl -s -A cityloom-deploy -w ' %{http_code}' https://cityloom.app/api/me` answers `401`, or `503` with `account-unavailable`. Anything else: a failure.
3. **Record.**
   - Every job `success` and the site answers: post `**Factory · deployed** <run url>`, move the issue to `linear.states.done` (if it is not there already), remove the label `labels.feasible`, and remove the worktree and local branch (`git worktree remove <wt>`, `git branch -D <branch>`; the merge deleted the remote one), taking their turn on the main checkout's git (README, **Side by side**).
   - A failure: add `labels.deployFailed`, post `**Factory · handed back** (deploy)` with the failing job's name and link, or the site's answer, quoted as it was, and move the issue back to `linear.states.working`. While the issue carries the label, the merge loop merges nothing (`factory-merge`, **The deploy is clear**), so the line waits for a person: once `main` is green again, they remove the label, and the next pass reads the issue again. Never re-run a job, retry the deploy or fix it here.

## 3. Report

Every merge watched: PR, issue, merge commit, the run and its result, the site's answers, and what this pass recorded; the hand-backs with their reasons; the watches still running. When there was nothing to watch, say the deploy loop is idle. Then print the status (README, **Status**) with `deploy` as its loop, as the last thing of the run, also when the pass stopped before it began.

## Red flags

| Thought | Reality |
| --- | --- |
| "The deploy job failed on a flaky step, I'll re-run it" | Never re-run, cancel or retry. Label, hand back. |
| "The run succeeded, the site check is a formality" | The issue is deployed only once the site answers as it should. |
| "Its run was cancelled, so it never deployed" | A later merge's deploy carries it: record it when that one succeeds. |
| "An approved PR is waiting and nothing is deploying, I'll merge it" | Merging is the merge loop's: this loop merges nothing. |
