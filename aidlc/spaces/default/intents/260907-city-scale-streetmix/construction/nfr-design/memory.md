<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-09-12T21:58:17Z — one question only for osm-extract-proxy (cell store layout): every resilience, scaling, security and observability pattern the stage lists is already fixed by the numbers, the stack and the rules, and the Unit has no downstream call to design a breaker or retry policy toward. The layout is asked because it is a real gap no prior stage covered and it fixes the read path, start-up verification and what infrastructure-design has to place.
- 2026-09-14T19:01:19Z — traceability.json for osm-extract-proxy carries both id levels; the traceability check reads a three-level id such as NFR1.1.1 as its two-level prefix NFR1.1 and expects that prefix declared, so the two-level inception sub-requirements were added as rows mapping each to the design elements meeting its refinements, and the three-level rows were kept. Two prefixes the check picked up are not requirements at all (NFR4.6 cited in an assumption, NFR6.5 named only in a review appendix) and are recorded N/A with the reason.
- 2026-09-20T — accounts-sharing (U11): NFR3.5.1 (account-deletion cascade) had no functional-design BR behind it — a gap `nfr-requirements`'s review flagged rather than silently patching the closed functional-design artifacts. Designed the capability (RD-2, one DB transaction) directly against the NFR/Contract 6 rather than inventing a BR citation; traceability.json's target field says so explicitly instead of pointing at a nonexistent rule.


## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-09-14T19:01:19Z — resumed this Unit from a prior session with the summary confirmed and all seven artifacts written; the ambiguity re-check, the stale traceability finding (fired under the wrong stage slug during unit-major work) and the review are what remained, so the question flow was not re-run.


## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
