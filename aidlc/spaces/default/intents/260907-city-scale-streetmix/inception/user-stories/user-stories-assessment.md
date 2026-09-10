# User Stories — Stage Assessment

## Decision

**Execute.**

## Rationale

This stage is conditional, and the condition is met on three of its four
triggers rather than one.

- **User-facing features.** Every capability in `scope-document.md` is something
  a person does: find a street, import it, edit its lanes, extend a design along
  a corridor, sign in, share, export for a meeting. There is no headless surface
  and no developer-tooling component.
- **Multiple personas with different needs.** `stakeholder-map.md` names three
  distinct target groups whose interests are not the same — city agency staff
  (standards compliance, feasibility, defensibility), elected officials and the
  public (legibility and being able to weigh in), and planning consultants
  (avoiding repeated manual corridor work) — plus the project owner, who is the
  only actual stakeholder today. A requirement written once for "the user"
  hides the fact that the same screen serves someone defending a proposal and
  someone trying to understand it.
- **Complex business logic.** The provenance model (FR3) is the clearest case:
  three states, a rule about which transitions are legal (FR3.4), a rule about
  what may never be displayed as a measurement (FR3.3), and a user-correction
  path (FR6.4) that has to respect the standing prohibition on editing
  OpenStreetMap. That is behaviour with edge cases, not a form.

The fourth trigger, cross-team coordination, does not apply — there is one
builder (`constraint-register.md` OC-1). It is not needed; the first three are
independently sufficient.

## Factors considered

| Factor | Finding |
|---|---|
| Project type | Greenfield product with a real end user, not a refactor or a fix |
| User-facing scope | Total — the whole deliverable is an interactive editor |
| Complexity signals | Provenance state machine, corridor extension with per-street feasibility warnings, import-failure recovery, access-controlled sharing |
| Persona count | Three target groups plus the owner; their goals conflict in places |
| Team size | One builder — the only trigger that does not apply |

## Where stories add the most value here

1. **The provenance rules.** FR3.1–FR3.4 and FR6.4 read as a consistent set on
   paper. Written as scenarios with a sad path each, they expose whether
   correcting an inferred width, then reloading, then extending along a
   corridor, keeps the correction distinguishable from a measurement. The
   walking skeleton's own pass criterion (`team-practices.md`, Q3) depends on
   that being checkable.
2. **Import failure.** FR6 is four requirements describing what happens when the
   product cannot do its main job. This is the area least likely to be built
   properly from requirements alone, and the one where a dead end costs a user
   permanently.
3. **Corridor extension.** FR5.2 says the system warns where a street cannot
   accommodate a design. What "cannot accommodate" means, and what the user does
   next, is not settled by the requirement and needs acceptance criteria.
4. **The keyboard path.** FR1.3, FR4.3 and NFR4.2 commit to a fully
   keyboard-operable editor. `team-practices.md` makes keyboard interaction
   tests a merge gate for any Bolt touching the editing surface, so those tests
   need stories with stated expected model state and announced accessible name
   — not a general accessibility aspiration.

## Traceability

Assessment derived from: `scope-document.md` (capabilities C1–C9, the three-stage
split, the public-release gate), `stakeholder-map.md` (persona groups and their
standing), `requirements.md` (FR1–FR11, NFR1–NFR7), `team-practices.md` (testing
posture and the accessibility merge gate), `constraint-register.md` (OC-1, TC-5,
RC-1).

## What the executed stage produced

Forty stories across thirteen groups, four personas, and element-level
traceability over all 61 requirement leaves. The four areas named above as
highest-value did hold up: the provenance rules turned out to contradict
themselves between two Must Have criteria and were resolved at Q10; import
failure was missing the retry that two approved artifacts already promised;
corridor extension's "cannot accommodate" needed both a rule (Q4) and an
operational definition the dependency does not supply; and the keyboard path
produced the finding that no status in the product was announced at all.

The decision to execute is retrospectively confirmed by what the stage found
rather than by what it produced: five defects that would each have surfaced
first as rework — a broken overlay key in an affirmed practice, an undefined
undo, two untestable performance budgets, a fixture suite blind to its own
purpose, and an unbuildable precondition in US8.2.
