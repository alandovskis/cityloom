# Security Design — `osm2streets-build` (U1)

_Confirmed._

Upstream inputs: `security-requirements.md`, `tech-stack-decisions.md`
(nfr-requirements, this Unit). Design elements are numbered `SD-n`;
`traceability.json` maps each `NFRx.y`/`NFR-SUPPLY-n` to the elements
that meet it. No `performance-design.md`, `scalability-design.md`,
`reliability-design.md`, `observability-design.md`, or
`logical-components.md` are produced for this Unit: its `kind` is
`packaging`, absent from every one of those artifacts' `produces_kinds`
— this Unit has no runtime component for them to describe.

## Design elements

- **SD-1 — Fork identity.** The dependency is consumed from
  `github.com/<project-org>/osm2streets` (the project's own fork), never
  from `a-b-street/osm2streets` directly. This is a repository-selection
  decision recorded in the pin itself (`Cargo.toml`'s `git =` URL points at
  the fork), not a separate mechanism. Meets NFR7.2.3.

- **SD-2 — Two-point pin.** Both `git` dependencies this Unit owns —
  `osm2streets` and its transitive `abstutil` — declare `rev =
  "<40-hex-char sha>"` in the client workspace's `Cargo.toml`(s). No
  `branch` or `tag` key is ever present alongside `rev` (Cargo treats
  their combination as an error, which is itself a compile-time guard
  against a future edit accidentally loosening the pin). Meets NFR7.2.1,
  NFR7.2.2.

- **SD-3 — Locked resolution.** `Cargo.lock` is committed at the workspace
  root (the same lockfile `osm-extract-proxy` already committed — one
  workspace, one lockfile). Every build command in CI and at deploy time
  passes `--locked`, which makes an out-of-date or manually-edited lockfile
  a hard build failure rather than a silent re-resolution. Meets NFR7.2.4.

- **SD-4 — Branch protection on the fork.** The fork's default branch has
  GitHub branch protection with force-push disabled, applied at
  `environment-provisioning` alongside the project's other GitHub
  hardening steps (push protection, CodeQL, Dependabot — already listed
  in `team.md`). This design stage records the requirement and the
  mechanism (a GitHub branch protection rule); it does not itself apply
  the setting, consistent with `environment-provisioning` owning
  GitHub-repository configuration project-wide. Meets NFR7.2.5.

- **SD-5 — Provenance record.** One entry in the existing
  `docs/dependencies.md` (not a new file) records: the pinned commit SHA,
  the upstream repository and commit it was forked from, the exact build
  command, and the toolchain versions (`rustc`, the WASM build tool once
  Domain Design/`u6-client-surfaces` selects it). This is a documentation
  control, not a technical one — its value is that a maintainer six months
  from now can verify what actually shipped without reconstructing it from
  memory. Meets NFR-SUPPLY-1.

- **SD-6 — Dependency scanning coverage.** No new tool: `cargo audit`
  (already a CI gate per `team.md`) scans the full resolved dependency
  graph including this Unit's pinned crates, once `Cargo.lock` exists.
  Meets NFR-SUPPLY-2.

- **SD-7 — Tamper detection via the golden-fixture suite.** `unit-of-work.md`
  assigns this Unit's golden-fixture suite (real OSM extracts with committed
  expected output) as the practical trip-wire for the pin actually changing
  behaviour: if the pinned commit is ever silently re-pointed (a git
  history rewrite that keeps the same short-hash prefix visible in a diff,
  for instance), the fixture suite — asserted from `u4-street-import`, not
  re-implemented here — fails the moment osm2streets' output no longer
  matches the committed golden files. This is a design cross-reference,
  not a mechanism this Unit implements: SD-1 through SD-6 prevent the pin
  from moving unnoticed; SD-7 is the safety net if it moves anyway.

## Threats not applicable here (see security-requirements.md's STRIDE table)

Spoofing, Repudiation, and Elevation of Privilege remain N/A — this Unit
introduces no identity, no user-attributable action, and no authorization
surface. Information Disclosure is narrowed to "a leaked write credential
to the fork" (SD-4's branch protection is the primary control; no such
credential should routinely exist since the fork is fetched over public
HTTPS with no auth). Denial of Service is narrowed to "the pinned commit
becomes unreachable" (SD-1 and SD-4 together: forking removes dependence
on upstream's own repository lifecycle, and branch protection prevents the
project's own fork from losing the commit).

## Assumptions & Open Questions

None. Both open questions from `nfr-requirements` (fork branch protection,
workspace layout) were resolved before this stage began.

## Traceability

See `traceability.json` in this directory.
