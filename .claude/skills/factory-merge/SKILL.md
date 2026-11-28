---
name: factory-merge
description: Use when asked to run the software factory's merge loop - to merge the CityLoom factory PRs a person has approved, one at a time, each only once the deploy before it is done - or to see which factory PRs wait for approval. One pass per run; /loop /factory-merge keeps it going.
---

# Factory merge: approved PRs, merged

One pass of the factory's merge loop (README, **Four loops**): it reads every factory PR whose issue is in `linear.states.review`, and merges the approved ones with the `deploy` agent, one at a time and each only once the deploy of the merge before it is done. It watches no deploy: once a PR is merged, its deploy is the deploy loop's (`factory-deploy`). It changes no branch itself: a PR that needs preparing again goes back to the implement loop. Read `.claude/factory/README.md` first. It keeps no state: each pass reads Linear and GitHub again.

**Unattended**: `/loop /factory-merge`, about every ten minutes (it waits on a person's approval, on CI and on the deploy loop), in its own session beside the other loops.

## 0. Before anything

README, **Before a pass**, steps 1 to 4: the configuration, the App's token (to read the PRs), `git fetch` and the database (the `deploy` agent runs the tests again), Linear.

## 1. Read where everything is

List the team's issues in `linear.states.review` with an open factory PR (README, **Where an issue is**, step 3). Leave alone those waiting for a person (step 2). Write the list down before acting: key, PR, head, approved or not, mergeable state.

## 2. The deploy is clear

A merge starts a deploy to production, and the deploys go out one at a time: the merge loop merges only when the one before is done. The deploy is clear when both hold:

- every merged factory PR has its `**Factory · deployed**` record: none is the deploy loop's (README, **Where an issue is**, step 1), and none has a failed deploy waiting for a person (`labels.deployFailed`);
- the newest Postsubmit run on `github.base` (`gh run list --repo <github.repo> --branch <github.base> --workflow postsubmit.yml --limit 1 --json status,conclusion,url`) is `completed` with `success`. A run still going (a person's merge) waits; a failure means `main` is red or its deploy failed, and nothing is merged until a person has made it green again.

Not clear: merge nothing this pass. Read the approvals all the same (section 3, step 1), so that a PR that needs preparing again goes back to the implement loop.

## 3. Open PRs

Read every open factory PR whose issue is in `linear.states.review` and carries no waiting label first, then act on each:

1. **Approved?** Read the reviews over REST (`gh api --paginate repos/<github.repo>/pulls/<pr>/reviews`) and the head (`gh pr view <pr> --json headRefOid,mergeStateStatus,statusCheckRollup`). Take each reviewer's latest review by `submitted_at`, ignoring `COMMENTED`. It is approved only when at least one latest review is `APPROVED` with `commit_id` equal to the head (full sha), from an `author_association` `OWNER`, `MEMBER` or `COLLABORATOR`, a `user.type` other than `Bot` and a `user.login` other than the PR's author, and no reviewer's latest is `CHANGES_REQUESTED`. Not approved:
   - it went `BEHIND` or `DIRTY`, or a check failed: it needs preparing again. Move the issue back to `linear.states.working` and leave it; the implement loop's next pass runs `factory-evidence` step 5 on it (the `merge` agent, then the gate and the evidence again) and moves it back here, where it waits for an approval of its new head;
   - otherwise (clean, or a check still pending) it waits: leave it.
2. **Merge**, when the deploy is clear (section 2). Dispatch the `deploy` agent with this one approved PR and its worktree, for the merge only: the person's approval of the head is the person's word to merge it. It rebases on the version line only (a PR only behind `main` needs nothing more), re-runs the tests, waits for CI and merges (its steps **Look** to **Merge**), and stops on anything else.
   - `Merged: #<pr> (<version>, merge <sha>)`: post `**Factory · merged** <pr url> \`<sha>\`` on the issue. Leave the issue in `linear.states.review` and its worktree as they are: the deploy loop owns it now, and closes it once its deploy is done.
   - A STOP (a conflict beyond the version line, a red check, a refused merge): hand back (`labels.needsReview`, `**Factory · handed back** (merge)`) with the agent's report.

**One at a time.** At most one PR is merged per pass, the first approved: the next waits until the deploy loop has recorded this one's deploy, which clears the deploy again (section 2). Each merge moves `main` and deploys it, so a second merge started meanwhile would be tested against a `main` that no longer exists, and a deploy that failed would have another merge on top of it. The next approved PR is then behind the new `main`; the `deploy` agent rebases it on the version line as it does any.

## 4. Report

Every factory PR: number, issue, head, approved or waiting (for whom), and what this pass did; whether the deploy was clear, and if not, what it waits for; the merge with its version and merge commit; the PRs sent back to the implement loop and why; the hand-backs with their reasons. When nothing moved, say the merge loop is idle. Then print the status (README, **Status**) with `merge` as its loop, as the last thing of the run, also when the pass stopped before it began.

## Red flags

| Thought | Reality |
| --- | --- |
| "It was approved before the last push" | Only an approval of the current head counts. |
| "Two PRs are approved, I'll merge both" | One merge per pass, and only while the deploy is clear. |
| "The last deploy is nearly done, I'll merge now" | The deploy is clear only once the deploy loop has recorded it and `main`'s Postsubmit run succeeded. |
| "It's behind and not approved, I'll dispatch the merge agent here" | Preparing a PR is the implement loop's: move the issue back to `linear.states.working`. Two loops on one branch would race. |
| "It's merged, I'll watch its deploy while I'm here" | The deploy is the deploy loop's (`factory-deploy`). |
