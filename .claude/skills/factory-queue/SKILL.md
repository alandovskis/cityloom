---
name: factory-queue
description: Use when the software factory needs its next Linear issues, or when asked whether the factory has room for more work or which CityLoom issues it would take next. Read-only.
---

# Factory queue: the next issues from Linear

Picks the next CityLoom issues for the software factory from Linear, as many as there is room for, or says why there are none. It changes nothing. Read `.claude/factory/README.md` first: it defines the records, the factory PR and where an issue is. Take every name from `python3 "$CLAUDE_PROJECT_DIR/.claude/factory/factory.py" config` (below, `linear.*`, `caps.*`, `github.*`); never write one out.

## Room

The line takes new issues only while both hold:

- fewer than `caps.inFlight` issues **in flight**: issues of `linear.team` in `linear.states.working` that carry none of `labels.needsReview`, `labels.needsSetup`, `labels.deployFailed`, `labels.blocked` and have no open factory PR;
- fewer than `caps.awaitingApproval` **open factory PRs** (`gh pr list --repo <github.repo> --state open --json number,headRefName,author`, then the REST author type, `gh api repos/<github.repo>/pulls/<pr> --jq .user.type`, is `Bot`).

When either is reached, say which, with the count, and return none. Otherwise the **room** is the smaller of `caps.inFlight` less the issues in flight and `caps.awaitingApproval` less the open factory PRs: the most issues this call returns.

## Pick

1. List the candidates: `list_issues` with `team: linear.team`, `state: linear.intake`, every page (follow `cursor` while `hasNextPage`), with the fields `title, priority, labels, createdAt, gitBranchName`.
2. An issue's priority is its Linear priority; when that is none (0), it is its `P-` label instead, which issues migrated from GitHub carry (`P-critical` Urgent, `P-high` High, `P-medium` Medium, `P-low` Low). Skip an issue when:
   - it is in `linear.intake` and has no priority either way: a person has not said how much it matters (an issue of a `linear.fallback` state is not skipped for this: see step 4);
   - it carries `labels.needsReview`, `labels.needsSetup`, `labels.deployFailed`, `labels.blocked` or `labels.feasible` (waiting for a person, or already taken);
   - it has sub-issues (`list_issues` with `parentId`): it is a tracking issue, and its sub-issues are the work;
   - it is blocked by an issue that is not completed (`get_issue` with `includeRelations: true`: every `blockedBy` issue's `statusType` must be `completed`);
   - it already has a worktree `<worktrees>/clm-<n>`, or a branch of its own on `origin` (`git ls-remote --heads origin '<gitBranchName>*'`) with a PR of any author on it: someone is working on it, or did.
3. Rank the rest by priority, Urgent (1) first, then High (2), Medium (3), Low (4), then those with none; among equals, the oldest `createdAt` first.
4. **Nothing ready in the intake.** When steps 1 to 3 leave fewer issues than the room, take the rest from the states of `linear.fallback`, one state at a time in its order (the next state only while there is room left): steps 1 to 3 again with `state` that state. These are issues no person has moved to the intake yet; the line takes them all the same, and triage judges each.
5. Return the first ones, at most the room: each one's key (`CLM-<n>`), title, `gitBranchName` and the state it was in, or none with the reason ("no issue in Todo, Triage or Backlog", "CLM-31 is blocked by CLM-30", ...).

The issues are not judged yet: the caller sends each to `factory-triage` next, side by side.

## Report

The picks, each with its state, or none and why; the room counts; each skipped candidate with its reason; every call made, in order.
