---
name: factory
description: Use when asked to run the software factory, to work CityLoom's Linear issues through to merged and deployed changes unattended, to resume the factory after it stopped, or to see what the factory has in flight. Runs one pass of each of its four loops (deploy, merge, implement, intake); /loop /factory keeps it going, or each loop runs on its own.
---

# The software factory: one pass of each loop

The factory is four loops, each owning its own part of the line (README, **Four loops**): `factory-intake` takes new issues from Linear, triages and plans them; `factory-implement` builds each planned issue to a pull request ready for approval; `factory-merge` merges the approved PRs, one at a time; `factory-deploy` watches each merge's deploy and closes its issue. `/factory` runs one pass of each, in this order:

1. **`factory-deploy`**: a recorded deploy clears the way for the next merge and closes an issue, so it reads first.
2. **`factory-merge`**: a merge frees room for a PR and for an issue, and the deploy it starts is the longest wait, so it starts next. A deploy still being watched holds it (`factory-merge`, **The deploy is clear**): it merges nothing until a later pass.
3. **`factory-implement`**: every issue in flight moves on as far as it can.
4. **`factory-intake`**: new issues fill the room that is left.

Read `.claude/factory/README.md` first. Run **Before a pass** once (README), then each loop's skill from its section 1: their steps 0 are that same check. Their passes may overlap: start the deploy loop's `deploy` agent, the merge loop's `integrate` agent and the implement loop's agents in the background, and go on to the next loop while they run; the loops own different issues (README, **Four loops**), so nothing they do meets. The pass ends when the four have.

**Unattended**: either `/loop /factory`, which waits about ten minutes between passes with nothing to do but wait for a person or CI, or each loop in a session of its own on the same machine, at its own pace: `/loop /factory-intake` (about every 30 minutes), `/loop /factory-implement` (self-paced), `/loop /factory-merge` (about every ten minutes), `/loop /factory-deploy` (about every five minutes). Never both: one session per loop at most, since a loop's two passes at once would run one issue's station twice. A session left running needs its tool permissions settled beforehand (auto mode, or an allow list), since no one is there to answer a prompt, and the workspace trusted, since the guard is a project hook.

## Report

Each loop's report, in the order run, then one table of every issue: key, title, where it is, which loop owns it, what it waits for (a person's answer, an approval on PR #<n>, CI) and what this pass did to it. When none of the four moved anything and nothing is running, say the line is idle. Then print the status once (README, **Status**) with `all` as its loop, as the last thing of the run, instead of each loop's own: also when a loop stopped before it began.
