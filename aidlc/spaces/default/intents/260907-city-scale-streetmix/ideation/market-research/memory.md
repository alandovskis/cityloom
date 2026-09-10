<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->

- 2026-09-07T08:20:00Z — read Q3 (city-acceptable output is the differentiator) together with Q6 (audience is primarily advocates and the public) as coherent rather than contradictory: the advocate pain recorded at intent capture is having no credible way to propose a street change a city will take seriously, so city-acceptable output is exactly what that audience needs. Stated the reading explicitly in the artifacts so it can be corrected rather than resolved silently.

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

- 2026-09-07T08:20:00Z — the stage lists a differentiation-strategy brief among Step 4's outputs, but the frontmatter `produces` declares only competitive-analysis, market-trends, build-vs-buy, and the questions file. Folded differentiation into competitive-analysis rather than writing an undeclared fifth artifact, since the declared contract is what the completion check reads.

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

- 2026-09-07T08:20:00Z — checked Streetmix's licence directly through the GitHub API rather than trusting a search summary. The repo's licence metadata reports only NOASSERTION, and the landscape write-up turns on whether extending Streetmix is legally open to an open-source-free product, so the licence file itself was worth reading: AGPL-3.0-or-later, relicensed from BSD 3-clause.

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->

- 2026-09-07T08:20:00Z — the AGPL copyleft obligation on any networked derivative of Streetmix is compatible with the stated open-source-free model but forecloses an open-core model later without a relicensing conversation. Feasibility should confirm this is understood before the build-vs-extend decision hardens.
