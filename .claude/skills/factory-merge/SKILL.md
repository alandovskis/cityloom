---
name: factory-merge
description: Use when asked to run the software factory's merge loop - to merge and deploy the CityLoom factory PRs a person has approved, one at a time, and to watch their deploys - or to see which factory PRs wait for approval. One pass per run; /loop /factory-merge keeps it going.
---

# Factory merge: approved PRs, merged and deployed

One pass of the factory's merge loop (README, **Three loops**): it reads every factory PR whose issue is in `linear.states.review`, merges and deploys the approved ones one at a time with the `deploy` agent, and finishes the issues whose deploy succeeded. It changes no branch itself: a PR that needs preparing again goes back to the implement loop. Read `.claude/factory/README.md` first. It keeps no state: each pass reads Linear and GitHub again.

**Unattended**: `/loop /factory-merge`, about every ten minutes (it waits on a person's approval and on CI), in its own session beside the other two loops.

## 0. Before anything

README, **Before a pass**, steps 1 to 4: the configuration, the App's token (to read the PRs), `git fetch` and the database (the `deploy` agent runs the tests again), Linear.

## 1. Read where everything is

List the team's issues in `linear.states.review`, and the issues whose factory PR is merged without a `**Factory · deployed**` record (README, **Where an issue is**, steps 1 and 3). Leave alone those waiting for a person (step 2). Write the list down before acting: key, PR, head, approved or not, mergeable state.

## 2. Merged, not deployed

A pass stopped while a deploy ran leaves a merged factory PR with no `deployed` record. The `deploy` agent stops on a PR that is no longer open, so watch it here, read-only, as the `deploy` agent's steps 7 and 8 do: the Postsubmit run whose `headSha` is the PR's merge commit (`gh pr view <pr> --json mergeCommit`; the Documentation workflow's run too when the PR changes `website/**`), its jobs' conclusions, then the site's two checks. Then record as in section 3, step 2: success, or a failure after the merge.

## 3. Open PRs

Read every open factory PR whose issue is in `linear.states.review` and carries no waiting label first, then act on each:

1. **Approved?** Read the reviews over REST (`gh api --paginate repos/<github.repo>/pulls/<pr>/reviews`) and the head (`gh pr view <pr> --json headRefOid,mergeStateStatus,statusCheckRollup`). Take each reviewer's latest review by `submitted_at`, ignoring `COMMENTED`. It is approved only when at least one latest review is `APPROVED` with `commit_id` equal to the head (full sha), from an `author_association` `OWNER`, `MEMBER` or `COLLABORATOR`, a `user.type` other than `Bot` and a `user.login` other than the PR's author, and no reviewer's latest is `CHANGES_REQUESTED`. Not approved:
   - it went `BEHIND` or `DIRTY`, or a check failed: it needs preparing again. Move the issue back to `linear.states.working` and leave it; the implement loop's next pass runs `factory-evidence` step 5 on it (the `merge` agent, then the gate and the evidence again) and moves it back here, where it waits for an approval of its new head;
   - otherwise (clean, or a check still pending) it waits: leave it.
2. **Merge and deploy.** Dispatch the `deploy` agent with this one approved PR and its worktree: the person's approval of the head is the person's word to merge it. It rebases on the version line only (a PR only behind `main` needs nothing more), re-runs the tests, waits for CI, merges, watches the deploy and checks the site, and stops on anything else.
   - `Merged: #<pr> (<version>, deploy success)`: post `**Factory · deployed** <run url>`, move the issue to `linear.states.done`, remove the label `labels.feasible`, and remove the worktree and local branch (`git worktree remove <wt>`, `git branch -D <branch>`; the merge deleted the remote one), taking their turn on the main checkout's git (README, **Side by side**).
   - A STOP after the merge, in the deploy: add `labels.deployFailed`, post `**Factory · handed back** (deploy)` with the agent's report, and move the issue back to `linear.states.working`. Never retry a deploy or fix it here.
   - A STOP before the merge (a conflict beyond the version line, a red check): hand back (`labels.needsReview`, `**Factory · handed back** (merge)`) with the agent's report.

**One at a time.** The approved PRs are merged **one at a time**, the next only after the `deploy` agent has reported on the one before, in the order they were approved: each merge moves `main` and starts a deploy to production, so a second merge started meanwhile would be tested against a `main` that no longer exists, and two deploys would race. The next approved PR is then behind the new `main`; the `deploy` agent rebases it on the version line as it does any.

## 4. Report

Every factory PR: number, issue, head, approved or waiting (for whom), and what this pass did; the merges with their deploy result; the PRs sent back to the implement loop and why; the hand-backs with their reasons. When nothing moved, say the merge loop is idle.

## Red flags

| Thought | Reality |
| --- | --- |
| "It was approved before the last push" | Only an approval of the current head counts. |
| "Two PRs are approved, I'll merge both at once" | Merges and deploys are one at a time. |
| "It's behind and not approved, I'll dispatch the merge agent here" | Preparing a PR is the implement loop's: move the issue back to `linear.states.working`. Two loops on one branch would race. |
| "The deploy failed, a quick fix will do" | Label, hand back, move on. |
