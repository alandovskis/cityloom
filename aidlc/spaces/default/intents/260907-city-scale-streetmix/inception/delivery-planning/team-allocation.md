# Team Allocation — Streetmix at City Scale

Upstream inputs: `requirements.md` (requirements-analysis), `stories.md`
(user-stories), `mockups.md` (refined-mockups), `components.md`
(domain-design), `unit-of-work.md`, `unit-of-work-dependency.md` and
`unit-of-work-story-map.md` (units-generation), `contract-summary.md`
(contract-design), `team-practices.md` (practices-discovery).

Who builds each **Bolt** — a Bolt being one build pass over a piece of the
work, ending in something that runs. `bolt-plan.md` has the sequence; this
file says who owns each one.

## There is one person, and this document says so rather than implying otherwise

Team Formation (stage 1.5) was skipped, so no roster exists. There is no mob —
a mob being a group working together on one thing at one keyboard — and
therefore nothing to allocate between mobs. `team-practices.md` states the
situation plainly and this file does not dress it up:

> There is no second human, and "review" is named honestly rather than kept as
> a step that implies a reviewer who does not exist.

## Allocation

| Bolt | Completes | Owner |
|---|---|---|
| B-0 | *(walking skeleton — thin slice)* | The owner, with AI support |
| B-1 | U1 `osm2streets-build` | The owner, with AI support |
| B-2 | U2 `street-core` | The owner, with AI support |
| B-3 | U9 `osm-extract-proxy` | The owner, with AI support |
| B-4 | U4 `street-import` | The owner, with AI support |
| B-5 | U3 `design-payload-spec` | The owner, with AI support |
| B-6 | U5 `design-editing` | The owner, with AI support |
| B-7 | U10 `design-storage` | The owner, with AI support |
| B-8 | U7 `local-persistence` | The owner, with AI support |
| B-9 | U8 `meeting-output` | The owner, with AI support |
| B-10 | U11 `accounts-sharing`, then U12 `data-rights` | The owner, with AI support |
| B-11 | U6 `client-surfaces` | The owner, with AI support |

**No Program Board.** A Program Board is the cross-team view used when several
mobs run Bolts in parallel and their hand-offs need coordinating. With one
builder there are no hand-offs and nothing to coordinate, so none is produced.

## What stands in for a reviewer

This matters more than the allocation table, because the allocation is a single
row repeated twelve times and the review substitute is the actual quality
mechanism. From `team-practices.md`:

| Layer | What it is | Why it counts |
|---|---|---|
| **The machine gate** | `scripts/verify.sh`, wrapped by the `justfile`'s `verify` target, run to a clean exit before anything is called done | Deterministic and repeatable. This is the durable substitute for a second pair of eyes |
| **The merge gate in CI** | Build and check clean, `clippy` with warnings denied, `rustfmt` checked, tests green, the coverage floor met, the osm2streets fixture suite green, and the keyboard and accessibility tests green for any Bolt touching the editing surface | Blocks the merge rather than reporting afterwards |
| **AI support passes** | Quality, developer and security perspectives brought in during a Bolt | Additive on top of the machine gate, never a replacement: they are non-deterministic and unrepeatable, and the machine gate is not |
| **The human gate** | The owner approves every Bolt | `team-practices.md` sets this deliberately for all of product Stage 1 — the gates cost the owner's own time rather than a reviewer's, and no deadline pushes the other way |

## Bolt approval rhythm

Every Bolt is gated through product Stage 1. That is an affirmed practice, not
a default, and `team-practices.md` records when to revisit it: once the core
editor is proven through Stage 1 — which is B-11 in `bolt-plan.md` — the
question of running later Bolts without a gate at each one can be reopened.

Until then the walking skeleton is solo and gated, and the owner approves it
explicitly before any other Bolt runs.
