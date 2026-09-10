# Practices Discovery — Interview

## Sources

- [desc] Initial description: "Streetmix at city scale"
- [scope] Workflow-selected scope: `feature`.

Inputs: the lead draft (`team-practices.md`, `discovered-rules.md`,
`evidence.md`) and three independent reviews under `contributions/`, written
blind to one another. Approved Ideation artifacts:
`initiative-brief.md`, `constraint-register.md`, `raid-log.md`,
`scope-document.md`, `wireframes.md`.

These questions settle the five practice areas that get promoted into
`aidlc/spaces/default/memory/team.md`: Way of Working, Walking Skeleton, Testing
Posture, Deployment, and Code Style.

## What the three reviews agreed on, without seeing each other

Three points of independent convergence, which is stronger evidence than any one
review:

1. **The CI-cost tension in the lead draft is a false premise.** All three
   checked and found GitHub Actions is free and unmetered on public
   repositories, and the ~$5/month constraint binds Railway hosting only. The
   draft asks whether CI can be afforded; that question is struck.
2. **Repository visibility is the question that actually matters**, because it
   gates free CI, secret scanning, push protection and code scanning — during
   exactly the window when secrets get committed by accident.
3. **The draft's "keep naming distance from Streetmix" rule should go.** Two
   reviews independently called it unenforceable: no pass/fail criterion, and it
   collides with the osm2streets vocabulary the project is obliged to use.

## Verified findings that change what to decide

- **osm2streets has an unpinned git dependency.** Its `osm2streets-js/Cargo.toml`
  carries `abstutil = { git = "https://github.com/a-b-street/abstreet" }` with no
  `rev`, `tag`, or `branch`. Building from source pulls the moving tip of the
  repository the RAID log already calls dormant. Confirmed by reading the file.
- **The published npm package is old and the API is unstable.**
  `osm2streets-js@0.1.4` was published 2023-06-04, has zero runtime dependencies
  and no npm provenance attestation, against a repository last pushed
  2025-10-02.
- **Automated accessibility tools cannot see inside a raster canvas.** axe-core
  treats `<canvas>` as an opaque node, so a green run on a canvas-based editor
  asserts nothing about the editor.
- **This workspace is not empty.** `CLAUDE.md` declares `just build`,
  `just format` and `just verify`, and instructs that
  `./scripts/state-summary.sh` runs before anything else. There is no
  `justfile`, no `scripts/state-summary.sh`, no `scripts/next-tasks.sh` and no
  `docs/tasks/`. `scripts/verify.sh` exists as a stub with no checks wired in.

---

## Q1. Is the repository public from the first commit?

Way of Working. This is the highest-leverage answer in the interview: on a public
repository, continuous integration, secret scanning, push protection and code
scanning are all free. Private means paying for CI minutes or going without, and
none of the free security controls run during the period when a secret is most
likely to be committed by accident.

- A. Public from the first commit — the project is open source anyway, so start
  that way and get the controls.
- B. Private during the walking skeleton, public before stage 1 ships.
- C. Private until it is presentable, with no fixed date.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: A

## Q2. `CLAUDE.md` describes commands that do not exist. What should it say?

Way of Working. Your own conventions outrank framework defaults as evidence of
how you work — but three of the commands it names are missing, so every session
opens with a failing instruction. This is a small recurring cost paid on every
round.

- A. Build the missing pieces — add a `justfile` wrapping `scripts/verify.sh`,
  and create the state and task scripts it references.
- B. Correct `CLAUDE.md` to describe what exists — `scripts/verify.sh` as the
  single gate, no `just`, no state scripts — and let it grow as the project does.
- C. Both: correct it now, and add a `justfile` when there is something to build.
- D. Leave it; it is aspirational and does no harm.
- X. Other (please specify)

[Answer]: A

## Q3. What counts as the walking skeleton succeeding?

Walking Skeleton. A walking skeleton is a minimal version that runs the whole way
through, built first to prove the pieces connect before the real features go in.
You already chose to build one: B-0, one street imported, edited and saved.

What is missing is a pass/fail line. `initiative-brief.md` says B-0 failing
reverses the decision to proceed, so "it worked" needs to mean something
checkable.

- A. It renders — a named real street imports through osm2streets and its lanes
  display with the right count and order.
- B. It round-trips — that, plus an edit is made, saved, reloaded, and comes back
  identical.
- C. It round-trips and the provenance holds — that, plus mapped values are
  visibly distinguished from inferred ones, since presenting a guess as a
  measurement is the failure that matters most.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: C

## Q4. How should tests relate to the code being written?

Testing Posture. The framework default when nothing is affirmed is test-after:
build a layer, then write and run its tests. The quality review disputes that for
this project, because with no second person the tests are the only reviewer, and
writing them after one pass tends to lock in whatever the implementation already
does.

- A. Test-after throughout — the framework default; simplest to hold to.
- B. Mixed, by layer — tests first for the street model, editing state and
  storage where correctness is subtle; explore-then-pin-down at the osm2streets
  boundary; tests after for interface work.
- C. Tests first throughout — strictest, and the most expensive per change.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: C

## Q5. Is the editing surface built from real page elements, or drawn on a canvas?

Testing Posture, and load-bearing beyond it. A canvas is easier to draw a street
on. But automated accessibility checking cannot see inside one, and you have
committed to WCAG 2.1 AA including the editing canvas — so with a raster canvas
that commitment can only be checked by hand, every time, forever.

Real page elements make each lane focusable and announceable, which is what the
select-then-act interaction already needs.

- A. Real page elements (DOM or SVG) — accessibility is verifiable automatically,
  and it fits the interaction already chosen.
- B. Canvas — accept manual-only accessibility checking as the cost.
- C. Not yet defined — settle it at Domain Design.
- X. Other (please specify)

[Answer]: C

## Q6. How is osm2streets brought into the build?

Deployment and supply chain. Two routes, and the choice is inherited by every
build afterwards. If left unsettled it gets decided by accident during B-0.

- A. Use the published package — `osm2streets-js@0.1.4` from 2023. Reproducible
  and simple, but three years behind the repository and possibly missing what is
  needed.
- B. Build from source, pinned — vendor or fork it and pin `abstutil` to a
  specific commit, so the unpinned dependency on a dormant repository cannot
  shift underneath you.
- C. Build from source as-is — accept the unpinned dependency.
- D. Not yet defined — let B-0 establish which is necessary, then decide.
- X. Other (please specify)

[Answer]: B

## Q7. How does deploying work?

Deployment. The framework default is deploy-on-merge to staging, with production
gated on a second person's sign-off. Neither part fits: there is no budget for a
second environment and no second person to sign off.

- A. One environment, deploy on merge — merging to `main` deploys. Simplest, and
  matches the budget.
- B. One environment, deploy on merge, with the accounts stage behind an access
  gate until the privacy work is done — deployed is not the same as public.
- C. Two environments — find the budget for staging.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: A

## Q8. Follow-up — deploy-on-merge against the public-release gate

Q7 chose one environment with deploy on merge: merging to `main` puts it live.
That works cleanly for stage 1, which has no accounts and no personal data.

Stage 2 is the problem. `constraint-register.md` RC-1 and `scope-document.md`
both record that accounts and saved designs tied to people put this in scope for
GDPR and comparable regimes **from the first public release**, and that data
subject rights are a gate on public availability. With deploy-on-merge and
nothing else, the moment stage 2 merges, accounts are live to the public — which
breaches that gate unless something holds access back.

This is not a change to Q7's answer; it is what has to be true alongside it.

- A. Accounts stay behind an access gate — a feature flag or invite list — until
  the erasure and export work exists. Deploying is not the same as opening.
- B. Data subject rights ship in the same merge as accounts, so there is no
  window where accounts are public without them.
- C. Stage 2 is not merged to `main` at all until the rights work is done — it
  waits on a branch, accepting the merge debt.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

**Note on ordering.** This checkpoint should have run before the artifacts were
written and did not. The four artifacts already exist. Confirming here therefore
confirms what was produced rather than authorising its production. Nothing has
been written to `memory/team.md` or `memory/project.md`; that happens only at the
affirmation gate after this.

Your eight answers:

- Repository is public from the first commit (Q1).
- Build the missing pieces: a `justfile` wrapping `scripts/verify.sh`, plus the
  state and task scripts `CLAUDE.md` references (Q2).
- The walking skeleton succeeds when it round-trips and provenance holds: a named
  real street imports with correct lane count and order; an edit is saved,
  reloaded and identical; mapped values are visibly distinguished from inferred
  ones (Q3).
- Tests first throughout (Q4).
- The editing surface — real page elements versus raster canvas — is deferred to
  Domain Design (Q5).
- osm2streets is built from source with `abstutil` pinned to a specific commit
  (Q6).
- One environment, deploy on merge to `main` (Q7).
- Stage 2's accounts stay behind an access gate until erasure and export exist
  (Q8).

What was produced from them:

- `team-practices.md` — five sections. Testing Posture carries
  `**Methodology**: tdd` and an ordering that requires the osm2streets fixture to
  be committed before the adapter code written against it. The 80% coverage floor
  has a stated denominator, with rendering and WASM glue excluded by listed config
  rather than by lowering the number.
- `discovered-rules.md` — six `ALWAYS` mandates and five `NEVER` prohibitions,
  held to the bar of a stated constraint with a named consequence. New this round:
  pin any osm2streets git dependency; access-gate stage 2 accounts; never commit a
  secret.
- `evidence.md` — what each participant inspected, the interview decisions, and
  the three corrections: the CI-cost tension struck as a false premise, the
  Streetmix naming rule dropped as unenforceable, and the greenfield framing
  corrected.
- `practices-discovery-timestamp.md` — finalised.

Three things recorded rather than smoothed away:

- The quality review proposed mixed-by-layer testing; you chose stricter. The
  dissent is preserved in `evidence.md` with the reasoning on both sides.
- Deferring the editing surface (Q5) interacts with tests-first and the WCAG 2.1
  AA commitment. If Domain Design picks a raster canvas, automated accessibility
  verification is not available. That is carried as a constraint on Domain
  Design's choice.
- The "week to under a day" measure must not become an acceptance criterion until
  a real baseline exists. Carried to Requirements Analysis.

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
