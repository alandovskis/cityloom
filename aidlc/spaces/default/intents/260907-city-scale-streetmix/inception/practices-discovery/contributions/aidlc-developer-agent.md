**Collaborator:** aidlc-developer-agent

## Contribution

Remit: naming, layer boundaries, error handling, file organisation, code style.
Greenfield, so this is what should be adopted rather than what is in place.
Testing *volume* and coverage tooling are the quality agent's; I only cover the
places where a test convention is a consequence of a code convention, and I mark
those.

### 0. Evidence the draft did not have

I checked five things directly rather than reasoning from reputation, per
`project.md` `## Corrections`. Three change what the practice sections should say.

**(a) The workspace is not as empty as `evidence.md` says.** `evidence.md`
records "no CI configuration file, no deployment configuration, and no
dependency manifest to inspect" and concludes "Nothing here was inferred from
code, because there is no code." True about code, but the workspace root already
carries the owner's own written conventions, and they were not inspected:

- `CLAUDE.md` declares `Build: just build`, `Format: just format`,
  `Single gate: just verify (exit 0 = done)`, a `docs/state/` +
  `docs/tasks/` layout, and `ALWAYS run ./scripts/state-summary.sh before
  anything else`.
- `scripts/verify.sh` exists and is executable. Its own comment reads: "The
  single gate: run this before claiming any work is complete. Add formatting,
  linting, and test steps here as the project grows."
- `docs/state/features.md` and `docs/state/handoff.md` exist as empty templates.

These are stated practice by the one person who will build this, so they
outrank `org.md` defaults as evidence of how this project actually works. They
also contain a live contradiction the Code Style / Way of Working sections must
resolve rather than layer a second convention on top of: **there is no
`justfile`, and `docs/tasks/`, `scripts/next-tasks.sh` and
`scripts/state-summary.sh` do not exist.** So `just verify` fails today and
`./scripts/state-summary.sh` fails at every session start. For a solo builder
working with AI assistance, a stale command in `CLAUDE.md` is not cosmetic: the
assistant will run it, fail, and burn a round, every session. Recommend the
practice states one gate and one spelling of it — either add a `justfile` that
delegates to `scripts/verify.sh`, or drop `just` from `CLAUDE.md` and name the
script — and that the missing referenced paths are created or removed.

**(b) The osm2streets boundary is a JSON boundary, not an object boundary.** I
read `osm2streets-js/src/lib.rs` on the repository's `main`. `JsStreetNetwork`
is constructed as `new(osm_input: &[u8], clip_pts_geojson: &str, input: JsValue)
-> Result<JsStreetNetwork, JsValue>`, and every query method returns a JSON
`String`: `toGeojsonPlain()`, `toJson()`, `toLanePolygonsGeojson()`,
`toLaneMarkingsGeojson()`, `getGeometryForWay(id: i64)`,
`getOsmTagsForWay(id: i64)`, `findBlock(...)`, `findAllBlocks(...)`. This is
better news than the draft assumes and it makes the adapter proposal concrete
rather than merely hygienic — see §3.

It also surfaces a trap. The same class exposes **mutation** methods that change
the network in place and return nothing: `overwriteOsmTagsForWay(id, tags)`,
`collapseShortRoad(road)`, `collapseIntersection(intersection)`,
`zipSidepath(road)`. They are right there and they look like the obvious way to
implement editing. Using them would make the WASM object the mutable source of
truth, which collides with two already-approved rules at once: the NEVER in
`discovered-rules.md` about not editing OSM data, and TC-3's requirement that
derived values are not presented as surveyed fact. See §2.

And the addressing is positional: `debugMovementsFromLaneGeojson(road: usize,
index: usize)`, `findBlock(road: usize, left: bool, sidewalks: bool)`. Array
indices are not stable across a library version bump or a re-fetch of the OSM
extract. Anything this project persists must not be keyed on them.

**(c) The npm/`main` gap is confirmed and wider than "roughly three years".**
The npm registry has `osm2streets-js` at latest `0.1.4`, published
**2023-06-04**, Apache-2.0 (consistent with LC-2). `main`'s `lib.rs` carries
methods that post-date it. So building from source is not merely possible, it is
the likely path, and a from-source WebAssembly build is **not** reproducible
from `package.json` alone. Practice consequence in §7.

**(d) The CI-cost tension in `evidence.md` is largely false.** GitHub's billing
documentation states that use of standard GitHub-hosted runners is free in
public repositories, with no minute cap. The product is open source and free
(LC-1), so a public repository is consistent with the plan and CI before merge
costs nothing against the ~$5/month budget (OC-4). Caveat honestly: this holds
for *standard* hosted runners on *public* repositories; a private repository
during early development falls under the capped free tier. Recommend the
interview is told the tension is resolved rather than asked to trade it off,
with the one real decision being "public from the first commit, or accept the
cap until it goes public".

**(e) Mechanisms I name below exist.** `import/no-restricted-paths` takes
`zones` of `{ target, from, except, message }` and can forbid one directory from
importing another. TypeScript's `noUncheckedIndexedAccess` adds `undefined` to
index-signature access; it is **not** part of `strict` and must be enabled
explicitly.

### 1. Code Style — what to add

The draft's Code Style section is two lines of org default, one stack fact, and
one proposed rule I dispute (§8). The section is the thinnest part of the draft
and it is the one where this project's constraints bite hardest. Suggested
replacement content, in team voice:

- **Formatter and linter.** Prettier + ESLint per the org default, configured in
  the repo root. The deciding criterion if the interview prefers a
  single-binary toolchain instead is whether it can express import-boundary
  zones, because the boundary rules in §2 are the point of having a linter here
  at all; verify that before adopting one.
- **TypeScript is strict, plus `noUncheckedIndexedAccess`.** `strict: true` is
  table stakes; `noUncheckedIndexedAccess` is the one worth arguing for. The
  central data shape is an ordered lane list addressed by position — "Lane 2 of
  5", insert, remove, reorder, `[+ Add lane]`, `[Remove lane]` (wireframes S3).
  Off-by-one and `undefined`-at-the-edge is precisely the bug class that
  AI-generated code produces and that a solo builder with no second reviewer
  ships. This flag converts that class into a compile error. It costs some
  `?? throw` noise at array accesses; that is the trade and it is worth it here.
- **Metres are the only unit in the core.** osm2streets and OSM are metric; the
  product must serve jurisdictions that are not (TC-5). Store metres, convert at
  the presentation boundary only. A width that has crossed a conversion twice is
  a credibility bug in a product whose differentiator is institutional
  acceptance.
- **Naming authority is osm2streets' schema.** TC-1 fixes the model's shape;
  the vocabulary should follow it (road, lane, intersection, and its lane-type
  and direction terms) so that the adapter's mapping is near-identity and
  readable. One naming authority, stated positively. This also disposes of the
  draft's Streetmix-distance rule (§8).
- **Test hooks: accessible name and role first, `data-testid` second.** The
  framework's own developer knowledge (`code-generation-guide.md`) says
  `data-testid` is *required on all interactive elements*. Adopted verbatim that
  encourages testid-first querying, which silently hides a missing accessible
  name — directly against the WCAG 2.1 AA commitment. Recommend a deliberate
  project-level narrowing: query by role and accessible name as the primary
  selector so every interaction test doubles as partial AA evidence, and reserve
  `data-testid` for elements that legitimately have no accessible name (the map
  canvas, individual lane blocks). At one person's capacity, making the
  interaction tests carry the accessibility assertions is the difference between
  one test suite and two.

### 2. The rule I would most want in this project

**Every dimension carried in the imported model states where it came from, at
the type level, and the un-annotated form is unconstructable.**

TC-3 and the wireframes' Partial state say the product must not present an
inferred lane width as a measured one, and `wireframes.md` calls this "the one
that matters most for credibility". If provenance is a rendering-time
decoration, it is lost the first time a value passes through a function that
takes a `number`, and with one builder and no reviewer that loss is silent. The
practice that survives contact with the codebase is a type:

```ts
type Provenance = 'mapped' | 'inferred';
interface Dimension { readonly metres: number; readonly provenance: Provenance; }
```

No field of the imported street model is a bare `number` for a physical
quantity. The enforcement is not a review or a lint rule — it is that a
`Dimension` cannot be produced without naming its provenance, so code physically
cannot fabricate one. Pass/fail criterion for the interview: *does any width,
count or offset in the imported model have type `number`?*

Paired with it, one structural rule:

**The imported baseline is immutable; a design is an overlay on top of it,
never a mutation of it.** `scope-document.md` puts it exactly — "Designs are
proposals over the map, never changes to it" — and the same sentence is already
a NEVER in `discovered-rules.md`. In code that means the osm2streets-derived
baseline is `readonly` and never written, and edits live in a separate
project-owned design layer. Three things fall out for free: the before/after
export (C7, S8) is just rendering two layers; undo is a stack of overlay
operations; and S4's "Review after applying" is a diff. It is also the rule that
forbids reaching for `overwriteOsmTagsForWay` — the convenient wrong answer
that the API puts within reach.

**The overlay is keyed on OSM way id plus a project-owned lane discriminator,
never on osm2streets' positional indices.** Those indices are `usize` array
positions (§0b) and will shift under a version bump or a re-fetch. A saved
design keyed on them silently corrupts. This belongs in `CLAUDE.md`'s
`## Known traps`, which is currently empty by design and is the right home.

### 3. Layer boundaries and file organisation

The draft has nothing on this, and it is the decision that is expensive to
reverse. Three zones, dependencies pointing inward only, enforced by
`import/no-restricted-paths` zones so the boundary fails on save rather than in
a review that has no reviewer:

1. **The osm2streets adapter.** The *only* module in the repo permitted to
   import `osm2streets-js`. One `no-restricted-paths` zone: `from` the binding,
   `except` the adapter directory. This is the enforceable version of the
   draft's proposal.
2. **The core street model and editing operations.** Plain TypeScript. No WASM,
   no map library, no UI framework, no jurisdiction constants (TC-5's
   jurisdiction-neutrality is a *dependency direction*, not a promise). This is
   the layer that is fast to test and where the interesting logic lives.
3. **The outer ring — UI, persistence, jurisdiction packs, export.** All depend
   inward; none is imported by the core.

A note so the lead does not write a rule that contradicts the framework's own
guidance: the developer knowledge base says to organise by feature rather than
by layer, and I agree — inside the outer ring, group by feature. The three zones
above are dependency boundaries, not a layer-cake taxonomy replacing
feature-first organisation.

The adapter's shape, made concrete by §0b: because every osm2streets query
returns a JSON string, the adapter is a **parse-and-validate boundary**, which
is the textbook trust boundary from the framework's own patterns. It parses,
validates against a schema, attaches provenance (§2), and returns project-owned
plain data. Nothing above it holds a WASM handle.

That has a large testing consequence, which I flag for the quality agent rather
than claim: **capture real osm2streets output for a handful of real streets as
JSON fixtures, commit them, and test everything above the adapter against the
fixtures with no WASM in the test process.** It makes the whole core testable
without a WebAssembly build, keeps the suite fast enough that a solo builder
actually runs it, and — the point for debugging — when something breaks you
immediately know whether it broke above or below the fixture line. This is the
substantive answer to TC-4's "debugging into it is harder"; a wrapper alone is
not.

### 4. Error handling

Two boundaries dominate and they want different treatments. The org default
("fail fast, fail loud") needs shaping here, not overriding.

**The import boundary is not exceptional, it is expected.** Thin tagging, an
unparseable street, a WASM module that fails to load — `wireframes.md` already
specifies the behaviour: the drawer "states plainly that this street's data
could not be read, offers retry, and offers starting from a blank cross-section
instead". That is three distinct recoveries, so the UI needs a *typed reason*,
not an exception. The constructor and several methods return `Result<_, JsValue>`
on the Rust side, which surfaces in JavaScript as a thrown value carrying no
useful stack. So: **the adapter catches, maps to a small closed set of typed
failure reasons, and returns a result value; it never lets a raw WASM error
reach a caller.** Log the raw text once with the OSM way id, at the adapter, and
never render it to a user.

**Editing operations return outcome plus findings.** S4's "Oak Street is
narrower than Elm. Lanes will be scaled to fit. Review after applying" is error
*prevention*: a validation result the UI renders before anything is applied, not
an exception thrown after. Same shape as the adapter.

One rule covers both, and it is checkable: **the adapter and every core editing
operation return typed results; exceptions are reserved for programmer error.**

### 5. Way of Working — one sharpening

I agree with the draft's refusal to keep a review step that implies a reviewer
who does not exist. I would sharpen what replaces it. The draft substitutes "the
AI support-agent passes... plus the solo owner's own approval". Agent passes are
non-deterministic and unrepeatable; they are worth having and they are not a
gate. The durable substitute for a second pair of eyes is the **machine gate**:
type check, lint (including the boundary zones in §3), and tests, in one
command, exit 0 or the work is not done. `scripts/verify.sh` already exists and
already says exactly this about itself. Recommend the practice names that script
as the gate, and positions AI review as additive on top of it rather than as the
thing standing in for review.

### 6. Deployment — one code-level consequence

The draft is right that "deployed" must be distinguishable from "publicly
released" for the Stage 2 gate. From the code side the cheap, checkable version
is: the account and sharing surface sits behind a single server-side flag read
from configuration, defaulting to off, with no client-side-only gating. One
flag, one place, no per-feature toggles to forget.

### 7. Dependency practice for a from-source WASM build

Given §0c, one practice belongs somewhere in Code Style or Way of Working: **if
the binding is built from source, record how.** The upstream commit SHA, the
build command, and the toolchain versions, in one committed file, plus the built
artifact treated as a vendored dependency. `package.json` cannot express any of
it. For a solo builder the pass/fail criterion is "can I rebuild this in six
months without reconstructing what I did", and today the answer would be no.

### 8. The Streetmix naming rule should be dropped

The draft's proposed hard rule — "naming and modules must not use Streetmix's
own naming or file layout as a starting point" — is theatre, and I think mildly
harmful. Four reasons:

1. **It is unenforceable.** No test fails, no lint rule fires, and there is no
   reviewer. A rule with no failure mode is decoration.
2. **It collides with TC-1.** The constraint register fixes the model's shape as
   osm2streets', and osm2streets' vocabulary overlaps Streetmix's heavily,
   because both describe streets: lane, width, sidewalk, cross-section. A rule
   to avoid Streetmix's names pushes the code away from the schema it is
   required to mirror, and puts two approved constraints in conflict.
3. **It targets the wrong risk.** LC-3's exposure is copied code and copied
   assets. Naming distance provides no protection against either and produces no
   evidence of compliance — it provides the *feeling* of a managed boundary,
   which is worse than an unmanaged one.
4. **It costs most where the project is weakest.** An AI-assisted solo builder
   generates a lot of names. A vague "don't sound like Streetmix" either
   paralyses that or, more likely, is quietly ignored — which teaches that the
   rules file contains rules nobody follows.

Replace it with three things that are actually checkable:

- The existing `NEVER copy Streetmix code or assets into this project` stands
  unchanged. It is a real constraint with a real source.
- A **positive naming authority**: domain vocabulary follows osm2streets' schema
  terms (§1). This is the rule that was reaching for expression.
- A **mechanical check for the real risk**: one committed manifest recording
  origin and licence for every third-party asset (icon, image, font) and every
  dependency, updated when one is added, and a line in `scripts/verify.sh` that
  fails if a Streetmix package appears in the dependency manifest. The asset half
  of LC-3 is the half most likely to be violated by accident — grabbing an icon
  that looks right — and a manifest is the only thing that catches it.

### 9. What I would put to the interview that the draft does not

1. Resolve `CLAUDE.md` against reality: one gate, one spelling, missing paths
   created or removed (§0a).
2. Public repository from the first commit, which makes CI free and dissolves the
   cost tension (§0d)?
3. Adopt the provenance type and the immutable-baseline-plus-overlay structure
   as project rules, or leave them to Domain Design (§2)? My view: they are
   practices, not design details — they constrain every unit written afterwards.
4. Accept dropping the Streetmix naming rule in favour of the asset/dependency
   manifest (§8)?
5. Enable `noUncheckedIndexedAccess`, accepting the extra guard clauses (§1)?

## Positions

- AGREE: Marking every bullet `[org default]` / `[ideation]` / `[proposed]` — it makes the draft honestly answerable at the interview instead of presenting framework defaults as this team's practice.
- AGREE: Refusing to promote any proposal into `discovered-rules.md` — mandates should require a stated constraint with a named consequence, and the draft holds that line cleanly.
- AGREE: Wrapping osm2streets behind an adapter — I verified the boundary is a JSON-string boundary, which makes the adapter cheaper and more effective than the draft assumed.
- AGREE: Restating "review" rather than keeping a step that implies a reviewer who does not exist — though the substitute should be the machine gate, with AI passes additive on top (§5).
- AGREE: Gating every Bolt through Stage 1 as the ladder default — gates cost the owner's own time and there is no external deadline (OC-3), so the cheap conservative default is right.
- OBJECT: The proposed Streetmix naming-and-module-distance rule — unenforceable, collides with TC-1's osm2streets vocabulary, and gives false comfort against a risk (copied code and assets) that naming does not touch; replace it with a positive naming authority plus an asset/dependency manifest checked in `verify.sh` (§8).
- OBJECT: `evidence.md`'s claim that there is nothing in the workspace to inspect — `CLAUDE.md` and `scripts/verify.sh` are the owner's own stated conventions, they outrank org defaults as evidence of practice, and they currently contradict each other (`just verify` with no `justfile`; three referenced paths missing) (§0a).
- OBJECT: Code Style has no file-organisation or module-boundary practice at all — TC-1 and TC-5 make dependency direction the expensive-to-reverse decision, and it should be three inward-pointing zones enforced by `import/no-restricted-paths`, not left to emerge (§3).
- OBJECT: Nothing in the draft carries TC-3's inferred-versus-mapped distinction into code — it is the product's stated credibility requirement and it will not survive as a rendering-time decoration; it needs to be a type whose un-annotated form is unconstructable (§2).
- OBJECT: The adapter proposal stops at "a thin adapter with its own test suite" — without single-importer enforcement, committed JSON fixtures, and a read-only rule that forbids osm2streets' in-place mutation methods, it is a wrapper rather than a boundary (§2, §3).
- OBJECT: Treating CI-before-merge as a genuine trade-off against the $5/month budget — GitHub's standard hosted runners are free and uncapped for public repositories, and the product is open source, so the question is only whether the repository is public from the start (§0d).
- OBJECT: Nothing addresses reproducing a from-source WebAssembly build — npm's latest `osm2streets-js` is `0.1.4` from June 2023 while `main` has moved on, so building from source is the likely path and `package.json` cannot record it (§0c, §7).
