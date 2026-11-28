---
name: factory
description: Use when asked to run the software factory, to work CityLoom's Linear issues through to merged and deployed changes unattended, to resume the factory after it stopped, or to see what the factory has in flight. Runs one pass of each of its three loops (merge, implement, intake); /loop /factory keeps it going, or each loop runs on its own.
---

# The software factory: one pass of each loop

The factory is three loops, each owning its own part of the line (README, **Three loops**): `factory-intake` takes new issues from Linear, triages and plans them; `factory-implement` builds each planned issue to a pull request ready for approval; `factory-merge` merges and deploys the approved PRs. `/factory` runs one pass of each, in this order:

1. **`factory-merge`**: a merge frees room for a PR and for an issue, and a deploy is the longest wait, so it starts first.
2. **`factory-implement`**: every issue in flight moves on as far as it can.
3. **`factory-intake`**: new issues fill the room that is left.

Read `.claude/factory/README.md` first. Run **Before a pass** once (README), then each loop's skill from its section 1: their steps 0 are that same check. Their passes may overlap: start the merge loop's `deploy` agent and the implement loop's agents in the background, and go on to the next loop while they run; the loops own different issues (README, **Three loops**), so nothing they do meets. The pass ends when the three have.

**Unattended**: either `/loop /factory`, which waits about ten minutes between passes with nothing to do but wait for a person or CI, or each loop in a session of its own on the same machine, at its own pace: `/loop /factory-intake` (about every 30 minutes), `/loop /factory-implement` (self-paced), `/loop /factory-merge` (about every ten minutes). Never both: one session per loop at most, since a loop's two passes at once would run one issue's station twice. A session left running needs its tool permissions settled beforehand (auto mode, or an allow list), since no one is there to answer a prompt, and the workspace trusted, since the guard is a project hook.

## Report

Each loop's report, in the order run, then one table of every issue: key, title, where it is, which loop owns it, what it waits for (a person's answer, an approval on PR #<n>, CI) and what this pass did to it. When none of the three moved anything and nothing is running, say the line is idle.
