# Contract Design — Questions

Upstream inputs: `unit-of-work.md` and `unit-of-work-dependency.md`
(units-generation), `components.md` (domain-design), `requirements.md`
(requirements-analysis).

**What this stage decides:** the formal agreements across boundaries — what
data crosses, in what shape, over what protocol, and what happens when it goes
wrong. It does not decide build order or implementation detail.

## The boundaries on the table

From `unit-of-work-dependency.md` § "Integration points between Units", plus
two external surfaces that document did not have to name:

| # | Provider | Consumer | Crosses |
|---|---|---|---|
| 1 | U9 `osm-extract-proxy` | U4 `street-import` | Browser to server |
| 2 | U10 `design-storage` | U7 `local-persistence` | Browser to server |
| 3 | U11 `accounts-sharing` | U6 `client-surfaces` | Browser to server (product Stage 2) |
| 4 | U3 `design-payload-spec` | U7, U10, U12 | Both tiers and the database |
| 5 | U10/U11 | **External: public web** | An unauthenticated reader opening a shared link (US10.2) |
| 6 | U12 `data-rights` | U10, U11 | Inside one deployable |
| 7 | U2 `street-core` `StreetSource` port | U4 `street-import` | Inside the client Cargo workspace |

Boundaries 6 and 7 are in-process Rust: the compiler is the contract and no
separate specification document is proposed for them. The questions below are
about 1 through 5.

Two further boundaries are ones this system **consumes** rather than exposes,
so their shape is not ours to design: the public OpenStreetMap API behind U9,
and the basemap tile service behind `MapView`. The tile service is still
unchosen — `components.md` records that it must supply stable OpenStreetMap way
identities for selection (AC2.2.4), which not every tile service does, and
leaves the choice to Infrastructure Design.

**What is already settled and not re-asked:** one Railway service serves both
the WASM bundle and the API (Q7 at units-generation); the four server Units are
four work boundaries inside that one deployment boundary; the stored-design
payload is its own Unit (Q5 at units-generation).

---

## Q1. Which boundaries get a written specification now?

Boundaries 3 and 5 are product Stage 2. `project.md` mandates that the accounts
and sharing surface stays behind a server-side access gate until erasure and
export exist, so nothing there is reachable for some time. Boundaries 1, 2 and 4
are product Stage 1 and needed for the walking skeleton and the first release.

Against deferring: `stories.md` places US8.3 (upload, boundary 2) in Stage 1,
and `unit-of-work.md` records that U10 `design-storage` is therefore a Stage 1
Unit. A contract written late is the rework disaster this stage exists to
prevent.

A. **All five now.** Every boundary gets a spec in this stage, including the
   Stage 2 ones. One document, one decision, nothing revisited.

B. **Stage 1 boundaries in full; Stage 2 boundaries as a stated shape without
   a full spec.** Boundaries 1, 2 and 4 get complete specs. Boundaries 3 and 5
   get their mechanism, ownership, error shape and versioning policy recorded,
   with the endpoint-level detail left to be filled in when Stage 2 work
   starts.

C. **Stage 1 boundaries only.** Boundaries 3 and 5 are named as known future
   contracts and nothing more is written now.

X. Other (please specify)

[Answer]: B

---

## Q2. How is a client/server contract expressed, given that both ends are Rust?

This project is unusual in a way worth exploiting: the client compiles to
WebAssembly from Rust, and the server is Rust. Both ends of every HTTP boundary
are compiled by the same toolchain, so the request and response types can be
one shared crate rather than two hand-kept-in-sync definitions.

`team.md` states the principle this bears on directly: boundaries are enforced
"by crate boundaries and module visibility, not by a linter plugin, so a
boundary violation fails the build itself rather than a review that has no
reviewer."

A. **A shared Rust types crate, with JSON on the wire.** One crate defines
   every request and response type; client and server both depend on it and
   serialise with `serde`. A drifted contract is a compile error rather than a
   runtime surprise. An OpenAPI document is generated from those types for
   readability rather than hand-written.

B. **Hand-written OpenAPI as the source of truth.** The specification is the
   document; both ends implement against it. Conventional, tool-friendly, and
   language-neutral if a future consumer is not Rust — but nothing fails the
   build when an implementation drifts from the document.

C. **Plain REST with no formal specification.** The endpoints are documented in
   prose in `contract-summary.md` and nothing generates or checks them.

X. Other (please specify)

[Answer]: A

---

## Q3. How are wire contracts versioned between client and server?

One Railway service serves both the WASM bundle and the API, deployed together
on merge to `main`. So there is no version skew between a fresh client and the
server — except for a browser holding a cached bundle from before a deploy, or
a tab left open across one.

A. **No API versioning; a build-stamp check instead.** Client and server always
   ship together. The client sends its build identifier; a server that does not
   recognise it answers with a "reload required" response the client surfaces
   through `AppShell`'s live region. Simplest, and honest about a single
   deployable.

B. **URL-prefixed versioning (`/v1/...`).** Conventional and cheap to add now,
   expensive to add later. Costs nothing while there is one version, and means
   a future public consumer is not broken by a change.

C. **Additive-only with unknown-field tolerance, no version marker.** Consumers
   ignore fields they do not recognise; a field is never removed or repurposed.
   No version negotiation anywhere.

X. Other (please specify)

[Answer]: A

---

## Q4. How is the **stored** design payload versioned?

This is a different question from Q3 and a longer-lived one. A design saved in
a browser's IndexedDB, or a row in Postgres, outlives any number of deploys.
AC8.1.1 and AC8.1.2 require a reload to come back identical in lane order,
types, widths and every provenance state — which is a promise about data
written by an older version of the code.

There is no migration story yet, and `unit-of-work.md` gives U3
`design-payload-spec` one owner precisely so that this has one place to be
decided.

A. **A schema version field on every stored payload, with forward
   migrations.** Each payload records the version that wrote it; the reader
   migrates older shapes forward on load. The only option that survives a
   breaking change to the model without data loss.

B. **Additive-only, no version field.** Fields may be added and must be
   optional; nothing is ever removed or repurposed. Simpler, and enough while
   the model only grows — but a genuine restructure has no path.

C. **A version field, but no migrations until one is actually needed.** The
   field is written from the first release so the option exists later; reading
   an unknown version is a stated, handled failure rather than a crash.

X. Other (please specify)

[Answer]: C

---

## Q5. What shape does an error take at an HTTP boundary?

`team.md`'s Code Style requires that errors at integration boundaries are typed
results rather than exceptions, that the adapter maps every underlying error
into a small closed set of project-owned failure reasons, and that no
dependency's native error type reaches a caller. `stories.md` AC7.1.1 and
AC7.1.2 require the user to be told why an import failed without library text
leaking through.

A. **A project-owned typed error enum, serialised as JSON.** One closed set of
   failure reasons shared by both ends — the same crate as Q2's types if that
   is chosen — so the client matches on a variant rather than parsing a string.

B. **RFC 9457 `application/problem+json`.** The standard shape: `type`,
   `title`, `status`, `detail`, `instance`. Conventional and self-describing
   for any future non-Rust consumer, at the cost of the client reading a URI
   string rather than matching on a type.

C. **HTTP status codes alone, with a human-readable message body.** Least
   machinery; the client maps status codes to its own reasons.

X. Other (please specify)

[Answer]: A

---

## Q6. What timeout and retry behaviour does the proxy boundary commit to now?

`components.md` gives `ExtractFetcher` a timeout treated as failure rather than
an indefinite wait (AC3.1.6), and retries only on transient classes — never on
malformed data, because retrying a badly tagged street returns the same bytes.
What it does not give is numbers.

NFR1.1 sets 10 seconds for a full import, but AC3.1.2 and AC3.1.6 both
explicitly defer the real budget to `nfr-requirements`, on the stated grounds
that no device, no network and no reference street are established.

A. **Record the obligations, defer the numbers to `nfr-requirements`.** The
   contract states that a timeout exists, that exceeding it is a failure, and
   which failure classes are retryable — with the values marked as owed by a
   later stage. Consistent with how the same figures are already deferred.

B. **Set provisional values now and mark them provisional.** A 10-second
   timeout from NFR1.1, three retries with exponential backoff and jitter on
   transient classes only. Something concrete to build against, revised when
   `nfr-requirements` measures.

C. **Set them now as binding.** Treat NFR1.1's 10 seconds as the contract and
   do not reopen it.

X. Other (please specify)

[Answer]: A

---

## Q7. Nothing in the design calls the erasure and export service

`components.md` gives `DataRightsService` an empty `dependents` list — no
component calls it. But US11.1 ("Delete my account and everything in it") and
US11.2 ("Take my designs with me") are user-facing Must Have stories, and
`project.md` mandates data subject rights as functional requirements gating
product Stage 2's public release.

So there is a boundary the design needs and does not declare: something the
user touches has to reach U12. This stage is where that gap becomes visible,
because a contract cannot be written for an edge that does not exist.

A. **Add it here as a contract, and record the component-catalogue gap.**
   `contract-summary.md` declares the boundary — `AppShell` (U6) to U12 — with
   its spec, and records that `components.md` needs `AppShell` to gain the
   dependency and `DataRightsService` the matching dependent. The amendment
   joins the seven already outstanding in `decisions.md` ADR-001.

B. **Treat it as out of scope until product Stage 2 and record it as an open
   question.** No contract is written now; the gap is logged against U12 and
   the Stage 2 access gate cannot open until it is closed.

C. **Go back and fix `components.md` first.** Reopen domain design so the
   catalogue is correct before any contract is written against it.

X. Other (please specify)

[Answer]: A

---

## Consolidated Summary Confirmation

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
