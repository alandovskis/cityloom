# Infrastructure Specification — `osm2streets-build` (U1)

_Confirmed._

Upstream inputs: `security-design.md` (nfr-design, this Unit),
`components.md` (domain-design), `contract-summary.md` (contract-design).
Design elements are numbered `ID-n`; `traceability.json` maps each
`NFR7.2.x`/`NFR-SUPPLY-n` to the resource or setting that meets it.

This Unit deploys nothing of its own: per `unit-of-work.md`, U1 is
"embedded — compiled into the client artifact." Its only infrastructure
footprint is the **forked GitHub repository** that holds the pinned
dependency, and the **CI additions** that keep the pin honest — the CI
mechanics themselves are `cicd-pipeline.md`, below.

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute model | None — no runtime service | This Unit is a pinned source dependency compiled into `u6-client-surfaces`'s WASM bundle at build time; it has no deployable artifact of its own |
| Networking topology | None | No runtime network surface |
| Storage strategy | Git repository only (the fork itself) | The pinned commit *is* the persisted artifact |
| Environments | None (single client build, not environment-specific) | The pin is the same in every environment; only the client bundle that embeds it varies by deploy |
| IaC approach | None | No cloud resource to provision |
| Resource sizing | N/A | No compute/storage resource sized |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| `github.com/<project-org>/osm2streets` | Source repository (the fork) | Public, branch protection on default branch (no force-push) — ID-1 | Created once, at this Unit's implementation; ownership and protection settings applied at `environment-provisioning` per `team.md`'s existing GitHub-hardening list |

## Shared Infrastructure

Not applicable — no resource here is shared with another Unit. `u4-street-import` *consumes* the pinned commit as a Cargo dependency, but that is a build-time dependency relationship recorded in `Cargo.toml`, not a shared runtime or storage resource.

## Design elements

- **ID-1 — Fork repository with branch protection.** A GitHub repository
  under the project's own account, forked from `a-b-street/osm2streets`,
  with branch protection (no force-push) enabled on its default branch
  once the pinned commit is recorded. This is the concrete infrastructure
  behind `security-design.md`'s SD-1 (fork identity) and SD-4 (branch
  protection). It is created and configured at `environment-provisioning`
  alongside the project's other GitHub-repository settings (push
  protection, CodeQL, Dependabot — already listed in `team.md`), not a
  separate infrastructure step for this Unit alone.

## Assumptions & Open Questions

None.

## Traceability

See `traceability.json` in this directory.
