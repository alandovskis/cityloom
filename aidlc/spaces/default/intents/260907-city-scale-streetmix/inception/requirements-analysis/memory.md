<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->

- 2026-09-08T18:00:00Z — read Q7's "let the user fix the input by hand" as editing the project's own imported model, not the OpenStreetMap source, so it does not collide with the affirmed prohibition on editing OSM data from within the product. It resolves cleanly through the provenance model the quality review proposed: a corrected value is neither mapped nor inferred but user-set, a third state the requirements must carry explicitly.
- 2026-09-08T18:00:00Z — treated the six-dimension completeness pass as a filter rather than a script. Functional, business and technical context were settled by approved Ideation artifacts and the affirmed practices, so this stage asked only about non-functional targets and the two items earlier stages explicitly deferred here. Re-eliciting settled ground would have spent the interview on questions whose answers were already on disk.

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

- 2026-09-08T18:00:00Z — the stage lists six completeness dimensions and says to identify gaps in each. Recorded the four well-covered dimensions as settled with a pointer to where they were settled, rather than generating questions to demonstrate coverage. The gap analysis still ran across all six; only the questions were narrowed.

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

- 2026-09-08T18:00:00Z — the human asked what blue-green deployment would cost rather than answering the availability question. Checked Railway's health-check documentation instead of estimating, and it changed the question: Railway holds the old deployment active until the new one returns 2xx, so zero-downtime deploys are already included, with attached volumes the stated exception. The useful answer was that they largely have blue-green already and the real question is what target to state — not a price.
- 2026-09-08T18:00:00Z — asked one follow-up on what "loaded" means at city scale rather than writing requirements over the ambiguity. A per-street 10-second budget and a thousands-of-streets working scale are not contradictory but describe different loading models, and the choice between them shapes the data model. The answer was the layered option, which is more to build than either single model.

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->

- 2026-09-08T18:00:00Z — 99.5% monthly availability allows about 3.6 hours of downtime a month and is stated against a single Railway environment with no failover. Zero-downtime deploys make it plausible, but a platform incident or a bad migration has no fallback. Recorded as an assumption; Infrastructure Design should decide whether it stays realistic.
- 2026-09-08T18:00:00Z — where OpenStreetMap data is fetched from is unsettled and has a direct cost consequence: through the Railway service, egress bills at $0.05 per GB against a $5 budget; fetched by the browser directly from an OSM API, it costs nothing. At the layered city-scale loading model chosen at Q8, this is the difference between a negligible and a budget-breaking bill. Domain Design or Infrastructure Design must settle it.
- 2026-09-08T18:00:00Z — the 99.5% target and the free zero-downtime path both depend on the application service holding no attached volume, since Railway states that a service with a volume takes downtime on redeploy. That is a derived requirement nobody stated directly and it should not be lost.
