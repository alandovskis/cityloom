# Project-Level Rules

> Project-specific specialisation and corrections. Loaded after `org.md` and
> `team.md` as strict-additive guidance; contradictions with broader policy
> are rejected. Populated by practices-discovery and the self-learning loop.
>
> Use sparingly: most teams don't need a project layer. Reach for it
> only when this specific project needs stable, durable guidance beyond the
> team practice (for example, package-specific release checks or an additional
> regression suite for a legacy component).

## Way of Working

<!-- Project-specific specialisation. Example: -->
<!-- This monorepo requires package-scoped branch names and a package owner -->
<!-- review in addition to the team's normal merge policy. -->

## Walking Skeleton

<!-- Project-specific specialisation. Example: -->
<!-- The walking skeleton must exercise the legacy service adapter as well -->
<!-- as the new service boundary. -->

## Testing Posture

<!-- Project-specific specialisation. -->

## Deployment

<!-- Project-specific specialisation. -->

## Code Style

<!-- Project-specific specialisation. -->

## Tech Stack

<!-- Technology choices locked for this project. -->

## Decided

<!-- Decisions made in earlier stages that should not be re-asked. -->
<!-- Format: DECIDED: [decision] (Stage [slug], [date]) -->

## Scope Overrides

<!-- Custom scope rules for this project. -->

## Forbidden

<!-- Populated by practices-discovery affirmation gate. -->
<!-- Format: NEVER [behavior] (affirmed [date]) -->
<!-- Example: NEVER throw exceptions across service layer boundaries (affirmed 2026-05-17) -->

- NEVER copy Streetmix code or assets into this project, add Streetmix as a (affirmed 2026-09-08)
dependency, or clone its repository into the workspace. Source: (affirmed 2026-09-08)
`constraint-register.md` LC-3 — Streetmix is AGPL-3.0-or-later and is (affirmed 2026-09-08)
explicitly "NOT a code dependency of this project... Streetmix code and (affirmed 2026-09-08)
assets must not be copied in" — together with `raid-log.md` R-10, whose (affirmed 2026-09-08)
stated treatment is Avoid, with "do not read Streetmix source while (affirmed 2026-09-08)
building the editor" as its named action. (This sharpens and merges the (affirmed 2026-09-08)
lead draft's original "never copy" wording with the devsecops review's (affirmed 2026-09-08)
checkable form — see `evidence.md` for why the separate naming-distance (affirmed 2026-09-08)
rule was dropped instead of promoted.) Verifiable at the dependency (affirmed 2026-09-08)
manifest, the lockfile, and the git remotes. (affirmed 2026-09-08)
- NEVER let hosting/tooling spend grow past the stated ~$5/month working (affirmed 2026-09-08)
budget without treating that growth as a constraint change requiring (affirmed 2026-09-08)
explicit justification, not something silently absorbed. Source: (affirmed 2026-09-08)
`constraint-register.md` OC-4 — "Anything requiring more must be justified (affirmed 2026-09-08)
as a change to this constraint rather than absorbed." (affirmed 2026-09-08)
- NEVER edit the underlying OpenStreetMap data from within this product. (affirmed 2026-09-08)
Source: `scope-document.md` Out of Scope — "Designs are proposals over the (affirmed 2026-09-08)
map, never changes to it," explicitly to avoid the responsibilities of an (affirmed 2026-09-08)
OSM editing client. (affirmed 2026-09-08)
- NEVER commit a secret, credential, or connection string to the repository; (affirmed 2026-09-08)
secrets are supplied only as Railway environment variables, and any secret (affirmed 2026-09-08)
that reaches a commit is rotated before history is touched. Source: (affirmed 2026-09-08)
`constraint-register.md` LC-1 (the product is open source) together with (affirmed 2026-09-08)
the interview's Q1 answer (public from the first commit) — the consequence (affirmed 2026-09-08)
is that this repository's leakage is immediate and global rather than (affirmed 2026-09-08)
internal, from the very first commit, not from some later "presentable" (affirmed 2026-09-08)
point. (affirmed 2026-09-08)
## Mandated

<!-- Populated by practices-discovery affirmation gate. -->
<!-- Format: ALWAYS [behavior] (affirmed [date]) -->
<!-- Example: ALWAYS use Result<T,E> for fallible operations in service layer (affirmed 2026-05-17) -->

- ALWAYS treat data subject rights (at minimum access, rectification, (affirmed 2026-09-08)
erasure, and portability for an account holder's own data) as functional (affirmed 2026-09-08)
requirements gating Stage 2's public release, never as deferred (affirmed 2026-09-08)
operational work. Source: `constraint-register.md` RC-1, RC-2; the (affirmed 2026-09-08)
consequence is stated directly — "Data subject rights... are functional (affirmed 2026-09-08)
requirements, not operational work." (affirmed 2026-09-08)
- ALWAYS keep the street/lane model expressible without baking in any single (affirmed 2026-09-08)
jurisdiction's classifications, widths, or terminology. Source: (affirmed 2026-09-08)
`constraint-register.md` TC-5 — "No jurisdiction's classifications, widths (affirmed 2026-09-08)
or terminology may be baked into the core model." (affirmed 2026-09-08)
- ALWAYS keep Stage 2's accounts and sharing surface behind an access gate (affirmed 2026-09-08)
(a feature flag or invite list) until data subject erasure and export (affirmed 2026-09-08)
exist, and never rely on the deploy step itself as that gate. Source: the (affirmed 2026-09-08)
interview, Q7/Q8 — deploy-on-merge to the single Railway environment (Q7) (affirmed 2026-09-08)
means a Stage 2 merge is live to the public the moment it lands unless (affirmed 2026-09-08)
something else holds access back; RC-1/RC-2 make data subject rights a (affirmed 2026-09-08)
condition of public release, and the human's answer (Q8) is explicit that (affirmed 2026-09-08)
"deploying is not the same as opening." The consequence of skipping this is (affirmed 2026-09-08)
a live breach of the RC-1/RC-2 gate at the moment Stage 2 merges. (affirmed 2026-09-08)
- ALWAYS pin any osm2streets-related Cargo dependency built from source to a (affirmed 2026-09-08)
specific commit — never an unpinned branch, an untagged repository (affirmed 2026-09-08)
reference, or a floating `git` dependency with no `rev`. Source: the (affirmed 2026-09-08)
interview, Q6, against a verified fact — `osm2streets-js/Cargo.toml` on (affirmed 2026-09-08)
`a-b-street/osm2streets` `main` carries (affirmed 2026-09-08)
`abstutil = { git = "https://github.com/a-b-street/abstreet" }` with no (affirmed 2026-09-08)
`rev`, `tag`, or `branch`, so an unpinned build resolves to the moving tip (affirmed 2026-09-08)
of a repository `raid-log.md` already records as dormant. The human chose (affirmed 2026-09-08)
"build from source, pinned" explicitly over "build from source as-is." The (affirmed 2026-09-08)
affirmation-gate decision to build the client in Rust/WebAssembly changes (affirmed 2026-09-08)
the mechanism, not the rule: osm2streets is now added to the client's own (affirmed 2026-09-08)
Cargo workspace as a git dependency pinned with `rev = "<sha>"` (rather (affirmed 2026-09-08)
than pinned inside a separate `osm2streets-js` binding package), and the (affirmed 2026-09-08)
`abstutil` transitive dependency is pinned the same way. This one rule (affirmed 2026-09-08)
covers both pins; it is not multiplied into a separate rule for the crate (affirmed 2026-09-08)
dependency. The consequence of not pinning is unchanged: the project's (affirmed 2026-09-08)
single Critical dependency (`raid-log.md` D-1) shifting underneath it (affirmed 2026-09-08)
without warning. (affirmed 2026-09-08)
## Corrections

<!-- Project-specific corrections from human feedback. -->
<!-- Format: NEVER/ALWAYS [behavior] (learned [date]) -->
- When answers contradict each other literally, or a success measure has no checkable threshold, ask a targeted follow-up rather than recording the gap as an assumption. One extra human turn buys artifacts that are not self-contradictory and metrics that can be verified later. (learned 2026-09-07) <!-- cid:260907-city-scale-streetmix:intent-capture:cecdbdf8a51fb1f2e9b3c2c37020d3947c3e6ffab823579bf513894e8be26ba2 -->
- "Streetmix at city scale" refers to the open-source street cross-section editor (streetmix.net) scaled beyond a single slice to corridors and networks on a real map. Treat that reading as project context, not as a confirmed claim to assert in artifacts. (learned 2026-09-07) <!-- cid:260907-city-scale-streetmix:intent-capture:6833af729aef9c7905c0717f99e9c9d062d60877baf8df8ad11f88cd20a5c591 -->
- When a licence, contract, or legal term is load-bearing for a decision, read the source file rather than trusting a search summary or repository metadata. Streetmix's GitHub licence metadata reports only NOASSERTION while the LICENSE file states AGPL-3.0-or-later, relicensed from BSD 3-clause. (learned 2026-09-07) <!-- cid:260907-city-scale-streetmix:market-research:175f9b651efe560aa0e874acbb4b1ff67a9d9c81e0341c437888088628498cfc -->
- Before recommending a dependency or foundation, check its live repository and pricing state rather than reasoning from reputation or a search summary. Checking activity dates reversed this project's foundation recommendation: A/B Street was last pushed a year ago in Rust, while Streetmix was pushed three days ago in TypeScript. (learned 2026-09-07) <!-- cid:260907-city-scale-streetmix:feasibility:3482e57d3379a395c9d0b9171a27befdf3f999713780880be83b8b8849ff34fc -->
- Validate against the mechanism that actually fails the build, not a proxy for it, and validate the artifact you ship rather than the inputs you generated it from. Three misses in one stage: a component graph was confirmed acyclic while describing a circular Cargo crate dependency that could not build, because acyclicity at the component level says nothing about crate boundaries; a generated YAML block did not parse, because only the generator's inputs had been checked; and a layer table had silently absorbed five server-side components because its last bucket read "everything else". Corollary: never let a checked partition end in an open-ended bucket. Enumerate every member by name and assert the document's partition equals the checker's, so adding a component fails loudly instead of drifting. (learned 2026-09-10) <!-- cid:260907-city-scale-streetmix:domain-design:a04658034d986dc4649c268f8dadbb505fb3ed21bed7d96b6980b45fe4f5cdb2 -->
- Confirming that a traceability target exists as a name is not confirming that it delivers the story. Check the target's stated behaviour, not the catalogue's index. A story was mapped to a component whose behaviour could not deliver it, and the name-existence check passed clean; the error surfaced only when a reviewer read the component's own prose. (learned 2026-09-10) <!-- cid:260907-city-scale-streetmix:domain-design:d1a6a567190dfbd6939a107db9ddd204190b7890c308fca2fb6515a8e5180b14 -->
- A prior stage's table of amendments required "before stage X" records what was asked for, not what landed. Treat it as a claim to verify against the files, never as a status to trust. All five amendments an approved user-stories artifact marked as required before Domain Design were still unapplied when Domain Design opened, which turned a footnote into that stage's first question. (learned 2026-09-10) <!-- cid:260907-city-scale-streetmix:domain-design:5d2f72d0dcf9acee5cf6bdc5afc7bc1f18861ef8d2e9476e93be2f6d1a0c6026 -->
