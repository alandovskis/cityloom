# Junction signal plan: one fold per option, the phases first

CLM-49, step 2 of CLM-32 (`2026-11-22-junction-editor-layout-design.md`, "The steps", item 2).

## Why

The person asked for the junction editor to be user friendly, after the street editor's overhaul. The signal plan level
(`junction/view/inspector.rs`, the level `Signal plan` opens) stacks, top to bottom: time-of-day plans, the phases,
then pedestrian calls, vehicle actuation, bicycle detection, transit priority and pre-emption, each a long section
(settings per street or crossing, the hour's rows, totals, findings). The phases, the thing a person comes to change,
sit between six sections they rarely need.

## Goals

- Each of the six options is a fold, closed at first, as #215 did for a street's transit priority and position: its
  name, and on the same line what is set.
- A closed fold never hides a problem: its line also says how many warnings (the option's findings) it has, in the
  warning colour (the person's choice, option A).
- The phases are the first thing open on the level.

## Non-goals

- Changing any option's settings, rows, totals, findings or wording inside the fold.
- Remembering which folds are open (as #215's folds: not remembered).
- Moving the time-of-day plans below the phases.

## Anti-goals

- A warning that can only be found by opening every fold.
- A keyboard path that gets longer by more than the one press that opens a fold.
- A read-only page on which anything inside a fold can be changed.

## Approaches

- **#215's `fold` helper, given a summary that can carry a warning count (chosen).** One `details` per option, the
  section inside unchanged; the summary is a function of the view, as `transit_set` is.
- **A new fold component for the signal plan.** Rejected: two kinds of fold on one panel, and the same markup twice.
- **Open a fold on its own while it has a warning (option B).** Rejected by the person: a junction with several warnings
  gets the long level back.

## The design

The level, top to bottom: the plans fold, the phases (open, as today), then the calls, actuation, bicycles, transit
priority and pre-emption folds, in today's order.

Each fold is `fold(w, id, heading, set, body)` (`junction/view/inspector.rs`), with:

| Fold id | Name (existing heading) | What is set (new messages) |
| --- | --- | --- |
| `jn-fold-plans` | `jn-plans-heading` | `{ $n }` plans, `{ $name }` shown; one plan: the plan's name alone |
| `jn-fold-calls` | `jn-calls-heading` | `{ $n }` crossings on call, or "None on call" |
| `jn-fold-actuation` | `jn-act-heading` | `{ $n }` streets actuated, or "None actuated" |
| `jn-fold-bikes` | `jn-bikes-heading` | `{ $n }` movements on detection, or "None on detection" |
| `jn-fold-priority` | `jn-transit-heading` | `{ $n }` streets with priority, or "None with priority" |
| `jn-fold-preemption` | `jn-preempt-heading` | `{ $n }` streets with pre-emption, or "None with pre-emption" |

Counts come from the view: crossings with `on_call`; streets with `settings` Some; movements with `on_detection`;
transit streets with a priority; pre-emption streets with `rate` Some; the plans' count and the one `viewing`.

The warnings: the option's `findings` (for the plans, the sum of each plan's `findings`). When there are any, the
summary adds "· `{ $n }` warnings" (plural selectors) in `.jn-fold-warn`, whose colour is the existing warning token
(`--warn`, already measured in `shared/contrast.rs`). A screen reader hears the fold's name, what is set and the
warnings as one summary.

The section inside keeps its markup, ids and heading (its `h3` stays, so the panel's heading order and every spec's
selector holds); only the `details` around it is new. The summary is the fold's one control; the details' own
behaviour (Enter or Space opens it) is the keyboard path.

## Constraints

- MVVM: each summary is a pure function of the view (`JView`) and `I18n`, tested natively.
- English and fr-CA for every new message (plurals by selector; OQLF rules, `docs/glossary-fr-ca.md`).
- Read-only: unchanged inside; a fold opens and closes on a read-only page (looking is allowed).
- Tests that use a control inside one of these sections open its fold first (as the transit fold's tests do); no test
  loses what it checks.
- The version rises (minor) and `Cargo.lock` follows.

## Left to the builder

- Whether the counts are computed in the view-model or the view (a pure function either way, tested).
- The exact English and French wording, within the table's meaning.
