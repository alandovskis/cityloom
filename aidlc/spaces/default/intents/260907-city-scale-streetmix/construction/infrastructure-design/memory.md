<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-09-14T21:26:31Z — two questions only for osm-extract-proxy: the platform, cadence, gates, runner and secrets were all fixed by team.md and the NFR stages, so only the two placement decisions no prior stage could take were asked (where the cell store lives and how a build reaches production; how availability is measured). Platform facts were checked against Railway's documentation the same day rather than assumed, and the checked ones are marked in the artifacts.
- 2026-09-14T21:26:31Z — settled the server's builder as a committed Dockerfile here, although team.md leaves the builder question to environment-provisioning: the chosen store placement (inside the image) is only possible with a Dockerfile, so the decision falls out of the answer; the client bundle's build path stays U6's.
- 2026-09-20T16:05:00Z — accounts-sharing (U11) reuses design-storage's (U10) existing Postgres plugin rather than provisioning a new one: nfr-design's scalability-design.md SC-3 already fixed "no per-Unit pool," and the account-deletion cascade (reliability-design.md RD-2) needs single-database transactional atomicity that a second database would break outright. Result: this Unit is the first to add zero new fixed infrastructure cost — five new tables and a second independent migration set, no new Railway resource.


## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-09-14T21:26:31Z — the weekly data build commits a one-line pointer directly to main under the workflow's own token, which is a bot commit on the trunk team.md says all work merges to by pull request; chosen because it keeps every credential out of this Unit (the alternatives needed a Railway token or bucket key), and recorded as a practice question rather than silently absorbed.
- 2026-09-14T21:35:12Z — the consolidated summary the human confirmed stated the Unit's cost as memory $1.30 + CPU $1.00 + egress ≤ $0.50 'inside ≤ $2.50', which sums to $2.80; the review caught the sum. The artifacts carry the corrected arithmetic ($2.25 fixed, 5 GB/month egress headroom) and name a measured breach as the constraint change NFR5.1.5 requires; the questions file is left as confirmed. Lesson: add the line items up before writing 'inside'.


## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-09-14T21:26:31Z — availability is measured by an external free-tier uptime monitor rather than the platform: Railway's health check runs only at deploy time, so the probe history the reliability requirements assumed does not exist; a third-party account was judged cheaper than an unverifiable SLO or a log-gap script with no alert.


## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-09-14T21:26:31Z — whether GitHub branch rules can scope a bypass to the workflow actor and the pointer path alone; if not, the data path needs a token beyond the workflow's own, which reopens the Q1 choice.
- 2026-09-14T21:26:31Z — whether Railway accepts a ~600 MB image and reuses the data layer across code-only deploys; confirmed by the first deploy at environment-provisioning.
