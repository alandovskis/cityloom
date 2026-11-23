---
name: factory-triage
description: Use when the software factory has picked a Linear issue and must decide whether its agents can build it, or when asked to triage, judge or check the feasibility of a CityLoom issue for the factory.
---

# Factory triage: can the line build this issue?

Decides whether one CityLoom issue can go down the factory's line without a person, and records the verdict on the Linear issue. Read `.claude/factory/README.md` first. Take label and state names from `python3 "$CLAUDE_PROJECT_DIR/.claude/factory/factory.py" config`; never write one out.

## Steps

1. Read the issue: `get_issue` (title, description, labels, `gitBranchName`) and `list_comments` (every page). Read `run` and `evidence` from the configuration.
2. Dispatch the `triage` agent with the key, title, description, the comments (author, date and body each) and `run` and `evidence`. Do not judge the issue yourself. It returns `{ verdict, flow, howToBuild, howToTest, howToExercise, question? }`, `flow` being `code` or `docs` (README, **Two flows**). A reply that is not that JSON: dispatch it once more; a second bad reply is handed back (below) as "triage gave no verdict".
3. Record the verdict on **this issue only**:

| Verdict | Label added | Comment |
| --- | --- | --- |
| `feasible` | `labels.feasible` | `**Factory · triage: feasible**`, then on its own line `Flow: <flow>`, then `howToBuild`, `howToTest` and `howToExercise` under their own headings |
| `needs-review` | `labels.needsReview` | `**Factory · handed back** (triage)`, then the question for a person to answer |
| `needs-setup` | `labels.needsSetup` | `**Factory · handed back** (triage)`, then what must be set up |

Add the label with `save_issue` and `addLabels` (never `labels`, which would replace the others), and the comment with `save_comment`. A feasible issue is also moved to `linear.states.working` (`save_issue` with `state`), so the queue's room counts it from now on. A handed-back issue stays in the intake state: the person answers on the issue and removes the label, and the queue picks it again.

## Report

The verdict and the flow, and for `feasible` the three texts; for the others the question posted. List every call made, in order, and say that triage changed no file.
