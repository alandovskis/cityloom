---
name: factory-intake
description: Use when asked to run the software factory's intake loop - to take new CityLoom issues from Linear, triage them and post their plans - or to see what the intake would take next. One pass per run; /loop /factory-intake keeps it going.
---

# Factory intake: new issues, judged and planned

One pass of the factory's intake loop (README, **Four loops**): while there is room, it takes new issues from Linear, triages them and posts their plans. It ends at the `**Factory · plan**` record: the worktree and everything after it are the implement loop's (`factory-implement`). Read `.claude/factory/README.md` first. It keeps no state: each pass reads Linear again.

**Unattended**: `/loop /factory-intake`, about every 30 minutes (Linear changes at a person's pace), in its own session beside the other loops (README, **Four loops**).

## 0. Before anything

README, **Before a pass**, steps 1 and 4 (the configuration and Linear). The intake builds nothing and touches no GitHub branch, so it needs no token and no database; `git fetch -q origin` is enough for the queue's look at branches.

## 1. Issues it owns

The issues of `linear.team` in `linear.states.working` with a `**Factory · triage: feasible**` record and no `**Factory · plan**` record, and no waiting label (README, **Where an issue is**, step 9): a pass stopped between triage and plan. Plan each (section 3) before taking new ones.

## 2. New issues

`factory-queue` returns as many issues as there is room for, from `linear.intake`, and when nothing there is ready, from the states of `linear.fallback`. Its room counts every issue in `linear.states.working`, planned or building, so the intake never takes more than the implement loop is allowed to build. Run `factory-triage` on all of them at once, the `triage` agents in the background. When a triage hands an issue back, room is freed: ask the queue again, and stop when it returns none.

## 3. Plans

Each feasible issue goes to `factory-plan`, steps 1 to 3 (the planner, the checks, the posted plan), all of them side by side, each `planner` in the background, as soon as its triage is recorded. Not step 4: the worktree is made by the implement loop, which owns the issue from its plan on. A `QUESTION:` or a plan failing its checks twice is the plan station's hand-back, as before.

## 4. Report

Each issue taken: key, title, the state it came from, the verdict and flow, and the plan posted or the hand-back with its reason; the room counts; and, when nothing was taken and nothing is running, that the intake is idle. Then print the status (README, **Status**) with `intake` as its loop, as the last thing of the run, also when the pass stopped before it began.

## Red flags

| Thought | Reality |
| --- | --- |
| "The plan is posted, I'll make the worktree too" | The worktree is the implement loop's (`factory-plan` step 4, from `factory-implement`). |
| "The issue in Backlog looks easy, I'll take it now" | `linear.fallback` is read only when nothing in `linear.intake` is ready, in its order, through the queue. |
| "There is room for one more if I don't count the planned ones" | The queue's room counts every issue in `linear.states.working`. |
