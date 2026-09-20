<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-09-11T00:05:00Z — Read the two in-process boundaries (the rights service to storage and accounts, and the core's StreetSource port to the adapter) as needing no specification document at all. The compiler already checks a trait signature and a function signature; a document beside them would add something to drift from without adding a check.
- 2026-09-11T00:07:00Z — Read "External: public web" as this project's own WASM client rather than a third-party caller. Someone opening a shared link loads the app, which then fetches the design. That reading is what makes the shared-crate and build-stamp answers safe, so it is stated in the artifact as the condition they rest on rather than left implicit.

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-09-11T00:10:00Z — Declared a contract for a boundary the component catalogue does not carry. DataRightsService has an empty dependents list, so nothing reaches it, yet erasure and export are user-facing Must Haves gating public release. Writing the contract and recording two upstream amendments was chosen over reopening two approved stages or leaving the release gate with no path to the user.

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-09-11T00:12:00Z — Chose a shared Rust types crate over hand-written OpenAPI as the source of truth, accepting that a future non-Rust consumer would have to be served differently. Both ends compile from the same toolchain today, so a drifted contract can be a compile error rather than a runtime surprise — the same reason team.md enforces the client's layering with crate boundaries rather than a lint.
- 2026-09-11T00:14:00Z — Chose a stored payloadVersion with no migration machinery over building migrations now or omitting the field. The field costs nothing at release one and is the only part that cannot be retrofitted; the machinery can wait until there is something to migrate.

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-09-11T00:16:00Z — Whether the extract fetch should take a bounding area or a street identity. An identity-based request would be narrower, but it depends on the basemap tile service supplying stable OpenStreetMap way identities, and that service is unchosen. For Infrastructure Design, then back to this contract.
- 2026-09-11T00:17:00Z — Whether an uploaded anonymous design is reachable by link before accounts exist. stories.md OQ-US2 raised it at user-stories and it is still open; a yes moves a Stage 2 contract into Stage 1 and pulls the privacy defaults a stage earlier with it.
