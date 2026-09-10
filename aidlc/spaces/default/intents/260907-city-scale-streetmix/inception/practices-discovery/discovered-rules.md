# Discovered Rules — Streetmix at City Scale

Only hard constraints with a stated consequence — a human-stated constraint,
or an interview decision with a stated consequence — are promoted here. This
gets stamped into `project.md`, so the bar stays high: a good practice
belongs in `team-practices.md`, not here. The devsecops review held itself to
exactly two additions against this bar out of twelve recommendations; that
discipline is respected in what follows rather than loosened at integration.

## Mandated

- ALWAYS treat data subject rights (at minimum access, rectification,
  erasure, and portability for an account holder's own data) as functional
  requirements gating Stage 2's public release, never as deferred
  operational work. Source: `constraint-register.md` RC-1, RC-2; the
  consequence is stated directly — "Data subject rights... are functional
  requirements, not operational work."
- ALWAYS keep the street/lane model expressible without baking in any single
  jurisdiction's classifications, widths, or terminology. Source:
  `constraint-register.md` TC-5 — "No jurisdiction's classifications, widths
  or terminology may be baked into the core model."
- ALWAYS keep Stage 2's accounts and sharing surface behind an access gate
  (a feature flag or invite list) until data subject erasure and export
  exist, and never rely on the deploy step itself as that gate. Source: the
  interview, Q7/Q8 — deploy-on-merge to the single Railway environment (Q7)
  means a Stage 2 merge is live to the public the moment it lands unless
  something else holds access back; RC-1/RC-2 make data subject rights a
  condition of public release, and the human's answer (Q8) is explicit that
  "deploying is not the same as opening." The consequence of skipping this is
  a live breach of the RC-1/RC-2 gate at the moment Stage 2 merges.
- ALWAYS pin any osm2streets-related Cargo dependency built from source to a
  specific commit — never an unpinned branch, an untagged repository
  reference, or a floating `git` dependency with no `rev`. Source: the
  interview, Q6, against a verified fact — `osm2streets-js/Cargo.toml` on
  `a-b-street/osm2streets` `main` carries
  `abstutil = { git = "https://github.com/a-b-street/abstreet" }` with no
  `rev`, `tag`, or `branch`, so an unpinned build resolves to the moving tip
  of a repository `raid-log.md` already records as dormant. The human chose
  "build from source, pinned" explicitly over "build from source as-is." The
  affirmation-gate decision to build the client in Rust/WebAssembly changes
  the mechanism, not the rule: osm2streets is now added to the client's own
  Cargo workspace as a git dependency pinned with `rev = "<sha>"` (rather
  than pinned inside a separate `osm2streets-js` binding package), and the
  `abstutil` transitive dependency is pinned the same way. This one rule
  covers both pins; it is not multiplied into a separate rule for the crate
  dependency. The consequence of not pinning is unchanged: the project's
  single Critical dependency (`raid-log.md` D-1) shifting underneath it
  without warning.

## Forbidden

- NEVER copy Streetmix code or assets into this project, add Streetmix as a
  dependency, or clone its repository into the workspace. Source:
  `constraint-register.md` LC-3 — Streetmix is AGPL-3.0-or-later and is
  explicitly "NOT a code dependency of this project... Streetmix code and
  assets must not be copied in" — together with `raid-log.md` R-10, whose
  stated treatment is Avoid, with "do not read Streetmix source while
  building the editor" as its named action. (This sharpens and merges the
  lead draft's original "never copy" wording with the devsecops review's
  checkable form — see `evidence.md` for why the separate naming-distance
  rule was dropped instead of promoted.) Verifiable at the dependency
  manifest, the lockfile, and the git remotes.
- NEVER let hosting/tooling spend grow past the stated ~$5/month working
  budget without treating that growth as a constraint change requiring
  explicit justification, not something silently absorbed. Source:
  `constraint-register.md` OC-4 — "Anything requiring more must be justified
  as a change to this constraint rather than absorbed."
- NEVER edit the underlying OpenStreetMap data from within this product.
  Source: `scope-document.md` Out of Scope — "Designs are proposals over the
  map, never changes to it," explicitly to avoid the responsibilities of an
  OSM editing client.
- NEVER commit a secret, credential, or connection string to the repository;
  secrets are supplied only as Railway environment variables, and any secret
  that reaches a commit is rotated before history is touched. Source:
  `constraint-register.md` LC-1 (the product is open source) together with
  the interview's Q1 answer (public from the first commit) — the consequence
  is that this repository's leakage is immediate and global rather than
  internal, from the very first commit, not from some later "presentable"
  point.

## Not Included Here (and why)

Strong candidates deliberately **not** promoted, because they are practices
fitted to stated constraints rather than constraints themselves — they live
in `team-practices.md` where a later stage can still weigh them:

- The osm2streets adapter pattern, its single-importer enforcement, and its
  committed fixtures — engineering judgment applied to `TC-4`, not a
  stated rule.
- The provenance type (`mapped` / `inferred` / `user-set`) and the
  immutable-baseline-plus-overlay structure — these operationalise TC-3/R-2
  and the existing "never edit OSM data" rule above, but the type-level
  mechanism itself is a design choice, not the constraint.
- CodeQL timing, the DAST rejection, Dependabot's security-only posture, and
  the specific lint plugin choices — all engineering judgment fitted to
  OC-1/OC-4, not constraints in their own right.
- Accessibility testing method (axe-core as floor, keyboard-path tests,
  manual screen-reader pass) — the WCAG 2.1 AA commitment is stated in
  `initiative-brief.md`, but the enforcement method is this project's
  practice, not something a human has stated as a rule.

## Sources

- `aidlc/spaces/default/intents/260907-city-scale-streetmix/ideation/feasibility/constraint-register.md`
- `aidlc/spaces/default/intents/260907-city-scale-streetmix/ideation/scope-definition/scope-document.md`
- `aidlc/spaces/default/intents/260907-city-scale-streetmix/ideation/feasibility/raid-log.md`
- `practices-discovery-questions.md` — Q1, Q6, Q7, Q8 and their answers
- `osm2streets-js/Cargo.toml` on `a-b-street/osm2streets` `main` (read
  directly, confirming the unpinned `abstutil` git dependency)
- `contributions/aidlc-devsecops-agent.md` §11 (the two rules the security
  review itself proposed against this same bar)
