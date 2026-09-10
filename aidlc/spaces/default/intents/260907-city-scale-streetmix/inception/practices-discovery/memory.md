<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->

- 2026-09-08T03:15:00Z — did not ask the stage's scripted walking-skeleton question ("build a thin end-to-end slice first?") because Scope Definition already answered it and the initiative brief records B-0. Kept the required gloss but redirected the question to the gap the quality review found: B-0 had no pass/fail criterion, despite the brief saying B-0 failing reverses the decision to proceed. Re-asking a settled question to satisfy a script would have wasted the one turn available for that area.

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

- 2026-09-08T03:15:00Z — briefed all four dispatched agents as "greenfield, workspace empty". That was wrong. `CLAUDE.md`, `scripts/verify.sh`, `docs/state/` and `docs/templates/` exist and carry the owner's own conventions, which outrank org defaults as evidence of practice. The developer review caught it and I verified it directly. The correct pre-dispatch step was listing the workspace rather than inferring emptiness from the absence of application code.
- 2026-09-08T03:15:00Z — trimmed the pasted rule bundle in each agent brief to the operative sections rather than the full verbatim chain: the five practice sections, project.md Corrections, and the applicable inception guardrails, plus the mandatory `Conversation language:` line. The org.md conversation-language block runs to several thousand words whose entire operative effect on a delegated agent is carried by that one line, which org.md itself declares authoritative for delegated work. Recorded because the protocol says paste the bundle verbatim.

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

- 2026-09-08T03:15:00Z — three blind reviews independently found the same thing: the lead draft's central open question, whether CI is affordable at $5 a month, rests on a false premise, because GitHub Actions is free and unmetered on public repositories and the budget constraint binds Railway hosting only. Independent convergence from mutually blind reviewers is far stronger evidence than one reviewer asserting it, and it is the clearest argument for the hub-and-spoke topology's cost on this stage. Struck the question and replaced it with repository visibility, which is what actually gates the controls.
- 2026-09-08T03:15:00Z — verified the security review's supply-chain claim myself rather than relaying it. `osm2streets-js/Cargo.toml` does carry `abstutil = { git = "https://github.com/a-b-street/abstreet" }` with no rev, tag or branch. That single unpinned line is what made the build-path question worth asking as a practice rather than leaving it to be settled by accident during B-0.

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->

- 2026-09-08T03:15:00Z — the editing surface question (real page elements versus a raster canvas) was deferred to Domain Design, but tests-first was affirmed and WCAG 2.1 AA including the editing canvas is already committed. If Domain Design chooses a raster canvas, accessibility conformance becomes manually verifiable only, and the affirmed testing posture cannot deliver it automatically. Domain Design should treat that as a constraint on the choice, not a consequence discovered after it.
- 2026-09-08T03:15:00Z — the quality review's point that the "week to under a day" success measure must not become an acceptance criterion until a real baseline exists is unaddressed by any practice. Requirements Analysis should carry it.
