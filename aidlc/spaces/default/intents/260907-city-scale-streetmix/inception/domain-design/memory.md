<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
- 2026-09-10T04:10:00Z — Read the "before Domain Design" amendments table in `stories.md` as a claim to verify rather than a status to trust, and checked all five against the files. None had landed, which turned a footnote into this stage's first question.
- 2026-09-10T04:35:00Z — Read the Railway fetch decision as touching privacy, not only cost. A proxy sees the requester's address and the place they are looking at, and a log pairing those is location data — so the no-logging rule became part of the component's definition rather than an operational note.

- 2026-09-10T15:40:00Z — Read the review's R-05 fix suggestion ("a core-owned port the adapter implements") as insufficient on its own. A trait narrows what crosses a boundary but does not remove the Cargo edge, because something must still construct the concrete adapter. The edge only disappears when construction moves to a component that does nothing else, so the port and the composition root are one fix rather than two.

## Deviations
- 2026-09-10T05:05:00Z — Derived every `dependents` list from `depends_on` in the generator rather than writing both by hand, so the symmetry well-formedness rule holds by construction. Same reasoning as generating the ASCII diagrams at refined-mockups: the rule is mechanical, so checking it by eye is the wrong tool.
- 2026-09-10T05:20:00Z — Validated the catalogue by parsing its own YAML block and re-checking every well-formedness rule against the parsed structure, rather than against the generator's inputs. The first attempt did not parse — an unquoted colon in a summary — which the input-side check would never have caught.

- 2026-09-10T15:45:00Z — Extended the catalogue's layer check to enforce the rule as actually written, not a paraphrase of it: only the composition root may cross from the outer ring into the adapter, and the server tier is partitioned separately because a rule about Cargo crate boundaries has nothing to say about a different deployable. Two of the first run's three failures were bugs in the check rather than in the artifact, which is its own argument for running it — an unexercised check tells you nothing about either.
- 2026-09-10T15:47:00Z — Made the layer table enumerate every component by name instead of ending in "everything else", and cross-checked that its partition equals the checker's. A prose bucket that says "everything else" cannot be compared against anything, so it drifts silently; an enumerated one fails loudly the moment a component is added.

## Tradeoffs
- 2026-09-10T04:50:00Z — Chose 18 components over a merged 14, accepting three thin ones, because the two overlays differ in what they claim rather than in what they store and the reversibility is asymmetric: merging later is local, splitting after data exists is a migration.
- 2026-09-10T04:55:00Z — Accepted that corridor matching by type and ordinal produces an unmatched change on a structurally different street rather than a guessed one. Reporting the failure is worse for the user in the moment and better for the product's central claim.

- 2026-09-10T15:50:00Z — Chose to add a component and an indirection rather than widen `team.md`'s outer-ring wording to fit the draft. The cheaper option was to record the direct shell-to-adapter edge as a deliberate composition-root pattern; it was declined because widening an affirmed practice to match a draft spends the practice's credibility, and because it would have left the coupling in the component with the most behaviour worth testing.

## Open questions
- 2026-09-10T05:30:00Z — What the OSM proxy actually costs in egress. Deferred to a real measurement at B-0; until then this design does not assert the ~$5 budget holds, and the affirmed prohibition makes that measurement a gate rather than a curiosity.
- 2026-09-10T05:31:00Z — Whether the proxy needs its own rate limiting toward the upstream public API, and how much. The obligation is stated; sizing it needs the same measurement.
- 2026-09-10T05:32:00Z — Which basemap tile service. It must supply stable OpenStreetMap way identities for selection (AC2.2.4), which not every tile service does — a selection constraint rather than a preference, and Infrastructure Design's to settle.
