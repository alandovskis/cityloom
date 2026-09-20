# Security Requirements — `osm2streets-build` (U1)

_Confirmed._

Upstream inputs: `requirements.md` NFR6, NFR7.2 (requirements-analysis);
`unit-of-work.md` U1 (units-generation); `team.md` Deployment section
(osm2streets pinning practice) and `project.md` Mandated (pinning rule).
No `functional-design` artifacts exist for this Unit — its `packaging` kind
is outside functional-design's applicable kinds, so this stage derives
directly from requirements and the Unit definition, per this stage's Step 1.

This Unit owns no runtime surface: no request path, no user input, no data
store. Its whole security posture is **supply-chain integrity of a single
forked, pinned dependency** — the project's one Critical dependency
(`raid-log.md` D-1). Performance, scalability, reliability and observability
requirements do not apply to a build-time packaging unit (this Unit's `kind`
is absent from those artifacts' `produces_kinds`) and are not produced here.

## Threat model (STRIDE, scoped to this Unit)

| Category | Applicable? | Reasoning |
|---|---|---|
| Spoofing | N/A | No identity, no runtime actor |
| **Tampering** | **Yes** | The forked repository or its pinned commit could be altered after the pin is recorded, silently changing what every downstream Unit builds against |
| Repudiation | N/A | No user-attributable action |
| **Information Disclosure** | **Yes (narrow)** | A leaked write credential to the project's own fork would let an attacker rewrite pinned history |
| **Denial of Service** | **Yes (narrow)** | If the pin resolves to a commit that no longer exists (force-pushed away, or the fork is deleted), every downstream build fails |
| Elevation of Privilege | N/A | No authorization surface |

## Requirements

- **NFR7.2.1** The `osm2streets` crate dependency in the client Cargo
  workspace shall be a `git` dependency pinned with an explicit `rev =
  "<40-character commit SHA>"` — never a `branch`, `tag`, or unpinned `git`
  reference. Source: `requirements.md` NFR7.2; `project.md` Mandated.
- **NFR7.2.2** The `abstutil` transitive git dependency shall be pinned the
  same way (`rev = "<sha>"`), checked independently in `osm2streets`'s own
  `Cargo.toml` at implementation time rather than assumed identical to the
  known-unpinned pattern in `osm2streets-js`'s `Cargo.toml`. Source:
  `team.md` Deployment; `project.md` Mandated.
- **NFR7.2.3** `osm2streets` shall be forked into the project's own
  GitHub account (not consumed from `a-b-street/osm2streets` directly),
  so the pinned commit cannot disappear from under the project if upstream
  rewrites or deletes history. Source: `team.md` Deployment ("fork
  `a-b-street/osm2streets` into the project's own account").
- **NFR7.2.4** `Cargo.lock` shall be committed and every build (CI and
  local) shall run `cargo build --locked`, so a resolver change can never
  silently substitute a different transitive version. Source:
  `requirements.md` Constraints; `team.md` Deployment.
- **NFR7.2.5** The fork's default branch and any tags shall be
  branch-protected (no force-push) once the pinned commit is recorded, so
  `NFR7.2.1`'s pin cannot be invalidated by a rewrite of the very ref it
  points at. This is new: `team.md` mandates the pin but does not yet state
  a protection rule for the fork itself. Flagged for confirmation below.
- **NFR6.1.1** (inherits `requirements.md` NFR6.1 — no secret in the
  repository) No credential is needed to fetch a public GitHub fork over
  HTTPS; if the fork is ever made private or a deploy key is used to fetch
  it, that key shall be supplied only as a CI/Railway secret, never
  committed. Currently N/A: the plan (`team.md`) is a public fork with no
  auth required.
- **NFR-SUPPLY-1** The single committed provenance file (upstream commit
  SHA, build command, toolchain versions) required by `unit-of-work.md`'s
  Definition of Done ("can this be rebuilt in six months without
  reconstructing what was done") is itself a security control: it is the
  only record that lets a maintainer verify, after the fact, that the
  binary in production was actually built from the reviewed, pinned
  source — not a substituted one. Source: `unit-of-work.md` U1
  "Why it is its own Unit".
- **NFR-SUPPLY-2** `cargo audit` (already a CI gate per `team.md` Deployment)
  covers this Unit's pinned dependency tree along with every other crate;
  no additional scanning tool is introduced for this Unit specifically —
  RustSec's crates.io-scoped database already reaches a `git` dependency's
  declared `Cargo.toml` metadata once it resolves into the lockfile.

## Out of scope for this Unit

- Runtime authentication/authorization (no service).
- Data protection / encryption at rest or in transit (no data held).
- Rate limiting, DoS-of-a-live-endpoint (no endpoint; NFR-DoS above is about
  the *dependency disappearing*, not about traffic).

## Assumptions & Open Questions

- **[assumption]** The fork is created under the same GitHub account/org
  that owns the main `cityloom` repository, consistent with `team.md`'s
  "public repository" posture — no separate access model is introduced.
- **[Q1]** Should the forked repository's branch protection (`NFR7.2.5`)
  be a hard requirement of this stage, or confirmed later at
  `environment-provisioning` alongside the rest of the GitHub repository
  hardening (`team.md` already lists push protection, CodeQL, Dependabot
  there)? Recommendation: treat it as this Unit's requirement (it protects
  the pin itself, which is this Unit's whole job) but implement it
  alongside the other GitHub hardening steps at `environment-provisioning`
  rather than duplicating that work here — recorded as `NFR7.2.5`
  regardless of which stage flips the setting.

## Traceability

See `traceability.json` in this directory.
