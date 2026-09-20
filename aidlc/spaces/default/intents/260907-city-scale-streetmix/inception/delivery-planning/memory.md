<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-09-11T10:05:00Z — Read the walking skeleton's value here as larger than the risk it was affirmed to retire. Because client-surfaces is the graph's single sink, every Bolt before the last completes a Unit with no surface to exercise it; without B-0 the plan would have one demo, at the end, after eleven gates with nothing to look at.

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-09-11T10:07:00Z — Recorded an unanswered question as unanswered rather than inferring it. Q7 was put twice and the reply was "continue"; the questions file says so in those words, and risk-and-sequencing-rationale.md names the two risks that would have moved earlier had it been answered. An inferred answer would have read as a decision nobody made.

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-09-11T10:09:00Z — Bundled accounts-sharing and data-rights into one Bolt against the one-Unit-per-Bolt rule chosen at Q3. Splitting them would produce a Bolt whose Definition of Done is "built, but unreachable", because project.md forbids opening the accounts surface until erasure and export exist. The rule was worth breaking once, in the open, rather than bent quietly.
- 2026-09-11T10:11:00Z — Let the dependency graph overrule value-first in two places rather than reordering the plan to fit the intent. Stage 3 output and the Stage 2 server work land before the Stage 1 surface that presents them, because AppShell presents both. Recording the two exceptions was cheaper than a decomposition that avoided them.

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-09-11T10:13:00Z — Whether the product being unusable until the last Bolt is acceptable, or whether it argues for revisiting the client-surfaces boundary. The decomposition made all three views one Unit for acyclicity; the delivery consequence is that the surface arrives last. B-0 mitigates it but does not remove it.
- 2026-09-11T10:14:00Z — Whether gating every Bolt still makes sense once B-0 has retired the osm2streets risk. team-practices.md sets the gate through product Stage 1 and names the ladder as revisitable once the core editor is proven, which is B-11 — the last Bolt. So the practice as written never reopens within this plan.
