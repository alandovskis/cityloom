<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
- 2026-09-09 — Read the MoSCoW anchor (Q7, the advocate) as one class of Must Have rather than the only test, after the design contribution showed the single rule had been silently overridden three times in the draft. Stated three classes instead: persona, access, policy.
- 2026-09-09 — Read FR9.3's unqualified "anonymous designs shall expire after 30 days" as binding only uploaded designs, because the browser-first decision means the product never holds the others. Recorded as OQ-US1 against `requirements.md` rather than reinterpreting an approved requirement in place.

## Deviations
- 2026-09-09 — Appended a `## Mob triage` H2 to the questions file after the consolidated summary. The gate refused it: only `Q<n>`, `Requested Changes Feedback`, or one `Assumption Confirmation` section may follow the summary, and H3 headings inside the summary are refused too. Repaired by flattening to bold labels, moving the summary to the end of the file, and re-confirming.
- 2026-09-09 — The reviewer appended a `---` rule before its `## Review` heading; `REVIEW_COMPLETED` refused it because the appended bytes must begin with blank lines then the exact heading. Stripped the rule.

## Tradeoffs
- 2026-09-09 — Split US6.1 from "the whole corridor feature, not splittable further" into four stories (extend to one adjacent street, choose a corridor, apply across it, warn on fit). The draft's claim that it could not be split was wrong; the one-street slice retires the real risk first.
- 2026-09-09 — Declined to assert NFR1.1's 10-second and NFR1.2's 100-millisecond budgets as acceptance criteria. Replaced each with a deterministic CI assertion that can fail, and sent the user-facing figure to `nfr-requirements` with its missing conditions named. Chose a testable weaker claim over an untestable stronger one.
- 2026-09-09 — Kept US8.3 (upload off the device) as Must Have per Q12, accepting that it re-opens the anonymous-identifier surface Q9 had deliberately shrunk. The alternative was a mandatory apology message naming an optional capability.

## Open questions
- 2026-09-09 — `team.md`'s overlay-keying practice is still wrong on disk. Q11 settled the fix; the learnings ritual is the sanctioned path and has not run yet at the time of writing.
- 2026-09-09 — OQ-US7: whether B-0's third pass criterion is automatable depends on the deferred canvas/DOM choice. If canvas is chosen without an accessibility tree, the walking skeleton's provenance criterion falls to a manual walkthrough on the first Bolt, indefinitely.
