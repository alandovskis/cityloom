---
name: factory-queue
description: Use when the software factory needs its next Linear issue, or when asked whether the factory has room for more work or which CityLoom issue it would take next. Read-only.
---

# Factory queue: the next issue from Linear

Picks the next CityLoom issue for the software factory from Linear, or says why there is none. It changes nothing. Read `.claude/factory/README.md` first: it defines the records, the factory PR and where an issue is. Take every name from `python3 "$CLAUDE_PROJECT_DIR/.claude/factory/factory.py" config` (below, `linear.*`, `caps.*`, `github.*`); never write one out.

## Room

The line takes a new issue only while both hold:

- fewer than `caps.inFlight` issues **in flight**: issues of `linear.team` in `linear.states.working` that carry none of `labels.needsReview`, `labels.needsSetup`, `labels.deployFailed`, `labels.blocked` and have no open factory PR;
- fewer than `caps.awaitingApproval` **open factory PRs** (`gh pr list --repo <github.repo> --state open --json number,headRefName,author`, then the REST author type, `gh api repos/<github.repo>/pulls/<pr> --jq .user.type`, is `Bot`).

When either is reached, say which, with the count, and return none.

## Pick

1. List the candidates: `list_issues` with `team: linear.team`, `state: linear.intake`, every page (follow `cursor` while `hasNextPage`), with the fields `title, priority, labels, createdAt, gitBranchName`.
2. Skip an issue when:
   - its priority is none (0): a person has not said how much it matters;
   - it carries `labels.needsReview`, `labels.needsSetup`, `labels.deployFailed`, `labels.blocked` or `labels.feasible` (waiting for a person, or already taken);
   - it has sub-issues (`list_issues` with `parentId`): it is a tracking issue, and its sub-issues are the work;
   - it is blocked by an issue that is not completed (`get_issue` with `includeRelations: true`: every `blockedBy` issue's `statusType` must be `completed`);
   - it already has a worktree `<worktrees>/clm-<n>`, or a branch of its own on `origin` (`git ls-remote --heads origin '<gitBranchName>*'`) with a PR of any author on it: someone is working on it, or did.
3. Rank the rest by priority, Urgent (1) first, then High (2), Medium (3), Low (4); among equals, the oldest `createdAt` first.
4. Return the first one's key (`CLM-<n>`), title and `gitBranchName`, or none with the reason ("no issue in Todo with a priority", "CLM-31 is blocked by CLM-30", ...).

The issue is not judged yet: the caller sends it to `factory-triage` next.

## Report

The pick or none and why; the room counts; each skipped candidate with its reason; every call made, in order.
