# Story-to-Unit Map — Streetmix at City Scale

Upstream inputs: `stories.md` (user-stories), `requirements.md`
(requirements-analysis), `components.md` and `decisions.md` (domain-design),
`unit-of-work.md` and `unit-of-work-dependency.md` (this stage).

All 40 user stories, each mapped to the Unit that owns delivering it. The
`USx.y` identifiers are permanent traceability keys carried forward from
`stories.md` exactly, never renumbered.

**Primary Unit versus "also touches".** A story's Unit is the one whose
Definition of Done is not met until that story works. "Also touches" names the
other Units that contribute — usually a surface rendering what a model Unit
computes, or a foundation Unit supplying the shape. Only the primary Unit
appears as the story's target in `traceability.json`, because a target that
names several Units names none.

## The map

| Story | Title | Stage | Unit | Directory | Also touches |
|---|---|---|---|---|---|
| US1.1 | Understand the tool from a worked example | 1 | U6 | `u6-client-surfaces` | — |
| US1.2 | Start designing without being taught first | 1 | U6 | `u6-client-surfaces` | — |
| US2.1 | Navigate to a place | 1 | U6 | `u6-client-surfaces` | — |
| US2.2 | Select a street | 1 | U6 | `u6-client-surfaces` | U4 |
| US2.3 | Select a street without a pointing device | 1 | U6 | `u6-client-surfaces` | — |
| US3.1 | Turn a real street into an editable cross-section | 1 | U4 | `u4-street-import` | U1, U2, U6, U9 |
| US3.2 | See the surrounding network without importing all of it | 1 | U6 | `u6-client-surfaces` | U4 |
| US4.1 | See which values are measured and which are inferred | 1 | U6 | `u6-client-surfaces` | U2 |
| US4.2 | Never be shown a guess as a measurement | 1 | U6 | `u6-client-surfaces` | U2 |
| US5.1 | Change a lane's type and width | 1 | U5 | `u5-design-editing` | U6 |
| US5.2 | Add and remove lanes | 1 | U5 | `u5-design-editing` | U6 |
| US5.3 | Do every edit without dragging | 1 | U6 | `u6-client-surfaces` | U5 |
| US5.4 | See what I changed | 1 | U5 | `u5-design-editing` | U6 |
| US5.5 | Undo the last thing I did | 1 | U5 | `u5-design-editing` | U6 |
| US6.1 | Extend a design to one adjacent street | 1 | U5 | `u5-design-editing` | U6 |
| US6.2 | Be warned before applying a design that does not fit | 1 | U5 | `u5-design-editing` | U6 |
| US6.3 | Choose a corridor of connected streets | 1 | U5 | `u5-design-editing` | U6 |
| US6.4 | Apply a design across the corridor I chose | 1 | U5 | `u5-design-editing` | U6 |
| US7.1 | Understand why a street could not be imported | 1 | U4 | `u4-street-import` | U6 |
| US7.2 | Never hit a dead end | 1 | U4 | `u4-street-import` | U5, U6 |
| US7.3 | Correct what the import got wrong | 1 | U4 | `u4-street-import` | U6 |
| US7.4 | See which streets are failing | 1 | U4 *(deferred half)* | `u4-street-import` | — |
| US7.5 | My corrections survive a later import | 1 | U4 | `u4-street-import` | U7 |
| US8.1 | Return to my design without an account | 1 | U7 | `u7-local-persistence` | U3, U6 |
| US8.2 | Understand when my work is gone | 1 | U7 | `u7-local-persistence` | U6 |
| US8.3 | Keep a design that is not tied to one device | 1 | U7 | `u7-local-persistence` | U3, U10 |
| US9.1 | Create an account and sign in | 2 | U11 | `u11-accounts-sharing` | U6 |
| US9.2 | Keep what I made before signing in | 2 | U11 | `u11-accounts-sharing` | U3, U7 |
| US9.3 | Save, name and reopen designs | 2 | U11 | `u11-accounts-sharing` | U10 |
| US10.1 | Designs are private until I say otherwise | 2 | U11 | `u11-accounts-sharing` | U10 |
| US10.2 | Choose who can see a design | 2 | U11 | `u11-accounts-sharing` | — |
| US10.3 | See what is shared at a glance | 2 | U11 | `u11-accounts-sharing` | U6 |
| US11.1 | Delete my account and everything in it | 2 | U12 | `u12-data-rights` | U10, U11 |
| US11.2 | Take my designs with me | 2 | U12 | `u12-data-rights` | U3 |
| US11.3 | Uploaded anonymous designs do not persist forever | 2 | U10 | `u10-design-storage` | — |
| US12.1 | Produce something for a public meeting | 3 | U8 | `u8-meeting-output` | U6 |
| US12.2 | Output keeps the provenance distinction | 3 | U8 | `u8-meeting-output` | U2 |
| US12.3 | See the output before producing it | 3 | U8 | `u8-meeting-output` | U6 |
| US13.1 | The product says out loud what it just did | 1 | U6 | `u6-client-surfaces` | every Unit that reports a status |
| US13.2 | The editing surface works on a phone | 1 | U6 | `u6-client-surfaces` | U5 |

## Stories that span more than one Unit

Every story with an entry in the "also touches" column above is cross-cutting
to some degree. Six are cross-cutting in a way that changes how they are
verified, rather than merely in which files change:

| Story | Why it spans Units | How it is verified |
|---|---|---|
| US3.1 — Turn a real street into an editable cross-section | The single highest-risk story in the set. Its fetch is U9, its pinned-build fixture assertions are U1, its types are U2, its conversion is U4, and the panel that opens immediately (AC3.1.3) is U6 | AC3.1.4 and AC3.1.5 assert against committed fixtures on the CI runner, independent of the surface; AC3.1.1 and AC3.1.3 assert against the rendered cross-section |
| US7.4 — See which streets are failing | Split deliberately. The local half — logging a failure once with the source street identifier — is U4. The maintainer-facing half, aggregating repeated failures so a pattern is visible, is not realised in any Unit here | Recorded as `Deferred` with target `observability-setup` in `traceability.json`, matching the same split already recorded at domain-design. It is not counted as delivered |
| US7.5 — My corrections survive a later import | The fingerprint comparison and the three re-import outcomes are U4; the corrections have to have been persisted and reloaded first, which is U7 | The three outcomes (AC7.5.2–AC7.5.4) are asserted against committed fixtures in U4; the round-trip through storage is asserted in U7 |
| US8.3 — Keep a design that is not tied to one device | The upload affordance and the anonymous identifier are U7; the shape that crosses the wire is U3; the storage and its retention are U10 | The three upload states (AC8.3.1–AC8.3.3) are asserted client-side; AC8.3.4's account-free removal is asserted as a server response |
| US9.2 — Keep what I made before signing in | Migration reads a device design (U7) in the shape U3 defines and writes it against an account (U11) | AC9.2.1 requires every edit and provenance state preserved across the migration — an assertion over the payload shape, not over either endpoint alone |
| US13.1 — The product says out loud what it just did | The live region and all 23 committed status messages are U6, but each message is produced by whichever Unit detects the condition | `stories.md` binds this to an enumerated list of 23 messages rather than a universal quantifier. Adding a member without its test fails the build; the Unit that introduces a new member owns adding it to that list |

## Story order within each Unit

Derived from the story-level dependency graph in `stories.md`, restricted to
each Unit. This is the order in which stories inside a Unit unblock each
other — it is **not** a build order for the Units themselves, which is
Delivery Planning's decision.

| Unit | Order within the Unit |
|---|---|
| U4 `street-import` | US3.1 → US7.1 → US7.2; US7.3 → US7.5; US7.4 stands alone (deferred half) |
| U5 `design-editing` | US5.1 → US5.2 → US5.5; US5.1 → US5.4; US6.3 → US6.1 → US6.2 → US6.4 |
| U6 `client-surfaces` | US2.1 → US2.2 → US2.3; US2.1 → US1.2; US2.1 → US3.2; US4.1 → US4.2; US5.3 and US13.2 follow the editing controls they exercise; US1.1 and US13.1 are independent |
| U7 `local-persistence` | US8.1 → US8.3 → US8.2 |
| U8 `meeting-output` | US12.1 → US12.2; US12.1 → US12.3 |
| U10 `design-storage` | US11.3 stands alone |
| U11 `accounts-sharing` | US9.1 → US9.2; US9.1 → US9.3 → US10.1 → US10.2 → US10.3 |
| U12 `data-rights` | US11.1 and US11.2 are independent of each other; both follow US9.1 and US9.3 in U11 |

## Coverage verification

**Every story is assigned.** 40 stories in `stories.md`, 40 rows in the map
above, each with exactly one primary Unit. Counted by Unit: U4 has 6, U5 has
8, U6 has 11, U7 has 3, U8 has 3, U10 has 1, U11 has 6, U12 has 2 — summing to
40. The check is an equality against the source document's story set, not a
spot check: the map was generated by enumerating every `### USx.y` heading in
`stories.md`, so a story added there and missing here fails loudly rather than
drifting.

**Every traceability target is on its story's row here.** `traceability.json`'s
40 coverage entries were checked against this table by matching each entry's
target against the Unit column of the row bearing the same `USx.y` id — not by
confirming the Unit exists somewhere in the document. A target that names a
real Unit which does not appear on that story's row would pass a
name-existence check and fail this one.

**Not every Unit has a story, and that is deliberate.** Four of the twelve —
U1 `osm2streets-build`, U2 `street-core`, U3 `design-payload-spec` and U9
`osm-extract-proxy` — carry no primary story. They are foundations and
infrastructure: shared by everything, owned by no feature. `unit-of-work.md` §
"Units that carry no story of their own" names the specific acceptance
criteria each is responsible for. This is a property of a decomposition that
mixes feature slices with foundation Units, which is what Q1, Q5, Q6 and Q10
together chose; recording it is better than inventing a story to fill a column.

**One story is not delivered by this plan.** US7.4's maintainer-facing half is
`Deferred` to `observability-setup`, carried forward unchanged from
domain-design rather than quietly promoted to covered.

## Known divergences carried forward

These are recorded in `decisions.md` ADR-001 as outstanding amendments to
approved artifacts. They are repeated here because this file maps stories, and
three of the divergences are about stories having no requirement behind them:

| Divergence | Affects |
|---|---|
| US5.5 (undo) traces to no requirement in `requirements.md` | U5 |
| US8.2 (work gone from this device) traces to no requirement | U7 |
| US8.3 (keep a design off this device) traces to no requirement | U7 |
| FR9.3 asserts an expiry over designs the product never receives | U10 |
| `scope-document.md` places all server-side storage in product Stage 2, but US8.3 puts anonymous upload storage in Stage 1 | U7, U10 |
| AC3.1.7 ("no request is made to the application's own server" during import) is false by design under the proxy | U4, U9 |
| NFR5.2 needs restating: per-user computation still runs in the browser; the fetch does not | U9 |

A later stage that reads `requirements.md` or `scope-document.md` without
reading ADR-001 will design against a stale picture. The amendments remain
outstanding work.
