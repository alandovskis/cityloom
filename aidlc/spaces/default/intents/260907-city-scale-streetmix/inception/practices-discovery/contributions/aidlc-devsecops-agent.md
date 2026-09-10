**Collaborator:** aidlc-devsecops-agent

## Contribution

Security review of the lead draft, scoped to lint/format rules, SAST/DAST,
secret and dependency scanning, and supply-chain controls.

The draft is honest, well-sourced, and disciplined about not promoting
proposals to mandates. Its Way of Working, Walking Skeleton and Deployment
reasoning are sound and I endorse them. But on my remit it is close to silent:
there is no mention of secret handling, dependency intake, the build path, or
what the lint pass is actually expected to catch. For a project whose source
will be public, whose single critical dependency is a compiled binary, and
which enters GDPR scope at Stage 2, that is a real hole — and it contains one
factual error that would send the interview to a wrong answer.

Everything below fits inside the five existing sections. **Do not add a sixth
heading**: `practices-promote` replaces exactly the five sections that match
`memory/team.md`, so a `## Supply Chain` section would be written to the draft
and then silently dropped at promotion. Section placement is given per item.

---

### 1. Correction — the CI cost tension does not exist. Strike it.

`team-practices.md` Testing Posture says "for a $5/month host, CI minutes are
also a cost constraint the interview should weigh," and `evidence.md` escalates
it to "Whether CI should run at all given the $5/month constraint... a real
tension between org.md's 'CI execution before merge' default and OC-4 that the
interview must resolve explicitly."

This conflates hosting spend with CI spend. They are unrelated bills.
**GitHub Actions is free and unmetered on standard runners for public
repositories**, on every plan including Free. LC-1 fixes this product as open
source, so the repository is public and CI minutes cost nothing. The March 2026
platform charge applies to self-hosted runners and explicitly exempts public
repositories.

This matters beyond tidiness. Framed as written, the interview may reasonably
answer "skip CI to protect the budget" — which would remove the only place any
security control can run, on a false premise, and would also drop the org
default's own coverage gate. It also runs against this project's affirmed
correction: *"Before recommending a dependency or foundation, check its live
repository and pricing state rather than reasoning from reputation or a search
summary"* (project.md, learned 2026-09-07). The pricing was not checked.

**Change to make:** replace both passages with the stated fact — CI on GitHub
Actions is free for this repository and is not charged against OC-4; OC-4 binds
Railway hosting only. Remove "whether CI should run at all" from
`evidence.md`'s "What Could Not Be Established" and from the interview list.
The remaining CI question is what the pipeline *does*, not whether it exists.

### 2. The interview must settle repository visibility, and when. (Way of Working)

This is the question that gates most of my other recommendations, and the draft
does not ask it. On GitHub, secret scanning, **push protection**, code
scanning/CodeQL and dependency review are free **on public repositories** and
require a paid Code Security licence on private ones. Dependabot alerts and
security updates are free everywhere.

So if the builder develops privately and opens the repository at Stage 1
launch, none of the scanning controls exist during exactly the window when
secrets are most likely to be committed — scaffolding, first Railway deploy,
first database URL. The security-cheapest posture is unambiguous:

> **Recommendation: the repository is public from the first commit.** It costs
> nothing, it is consistent with LC-1, and it turns on push protection, secret
> scanning, code scanning and dependency review for free on day one.

If the owner wants a private period, say so at the interview and the controls
change shape (a local `gitleaks` hook becomes mandatory rather than optional,
and CodeQL is unavailable). Do not leave this implicit.

**Add to interview:** "Is the repository public from the first commit, or
private until a later stage? This decides whether GitHub's free scanning
controls are available while you build."

### 3. Supply chain — the live issue, and the draft misses it (Walking Skeleton)

I checked the current state directly rather than reasoning from the brief.
Verified today:

- `osm2streets-js` on npm is at **0.1.4, published 2023-06-04**, by a single
  maintainer account (`dabreegster`). It ships **4 files, zero runtime
  dependencies**, and carries **no npm provenance attestation** — nothing
  cryptographically links that tarball to a source commit.
- `a-b-street/osm2streets` was last pushed **2025-10-02**, is Apache-2.0, and
  is **not archived** (34 open issues). Its `osm2streets-js/Cargo.toml`
  currently depends on `chrono 0.4.35`, released well after the npm publish —
  confirming the published package is far behind `main`.
- Building the binding from source means `wasm-pack` plus a Rust toolchain, and
  that Cargo tree contains a **git dependency with no pin**:
  `abstutil = { git = "https://github.com/a-b-street/abstreet" }` — no `rev`,
  no `tag`, no `branch`. The repository ships a root `Cargo.lock`, so a
  lock-respecting build is reproducible; any `cargo update`, or a resolve
  without that lock, floats that dependency to the **HEAD of a repository the
  RAID log records as dormant for a year** (R-3, D-1).
- Useful positive finding the register does not record: `a-b-street/abstreet`
  is **Apache-2.0** (read from its LICENSE file, per the project's own
  correction about not trusting repository metadata). The transitive
  dependency the build path actually reaches introduces no copyleft
  obligation, so LC-2's permissive position survives option (b) below.

There is also a counter-intuitive result worth stating plainly: **on the npm
side, the frightening dependency is the safest thing in the tree.**
`osm2streets-js` has no transitive dependencies at all. The project's real npm
supply-chain exposure will come from the map, framework and build tooling —
hundreds of transitive packages nobody will read — not from osm2streets.

**The decision this forces, which belongs to B-0 and not to a later stage:**

| Option | What you get | What you take on |
|---|---|---|
| (a) Consume prebuilt `osm2streets-js@0.1.4` from npm | No Rust toolchain in the build path; zero transitive npm deps; fastest route through B-0 | A 3-year-old, unattested `.wasm` binary you cannot audit or diff against source, that will never receive a security fix |
| (b) Build the binding from source at a pinned commit | Current code; ability to patch; the R-3 fork contingency becomes real instead of theoretical | Rust toolchain plus a Cargo tree in the build path, including an unpinned git dependency on a dormant repository |

B-0 already exists to retire R-1 by running osm2streets against a real area.
That same slice necessarily answers "prebuilt or from source," and the answer
is inherited by every later build. **Make it an explicit B-0 exit
criterion rather than something settled by accident at integration time.**

**If (b) is chosen, one control makes it safe and costs nothing:** fork
`a-b-street/osm2streets` into the project's own GitHub account, build from a
**pinned commit SHA** of that fork, commit `Cargo.lock`, and change the
`abstutil` dependency in the fork to `{ git = "...", rev = "<sha>" }`. That
converts "HEAD of a dormant third-party repository" into "a commit I chose,"
and it pre-positions R-3's stated fallback — the RAID log already names forking
as the contingency, so doing it up front turns a paper plan into a fact for the
price of a fork and two lines of TOML.

`cargo audit` in the same CI job is worth adding **only under option (b)**.
Under (a) there is no Rust in the build and it would be pure ceremony.

### 4. Secrets (Deployment)

The repository being public inverts the usual severity: a leaked secret is not
an internal incident, it is immediately world-readable and must be treated as
compromised the moment it lands. The controls:

- **Push protection on** (free, default on public repos). This is the highest-
  value control available to this project, because it blocks at the moment of
  the mistake rather than reporting it afterwards — no queue, no triage.
- **Push protection is not sufficient on its own.** It matches known provider
  token patterns (AWS, Stripe, GitHub). It will *not* catch this project's
  actual secrets: a session-signing key, a `DATABASE_URL` with an embedded
  password, a basemap API token from an unrecognised provider (D-4). Add **one
  `gitleaks` step in CI** to cover the generic-entropy and connection-string
  classes. It runs in seconds, is free, and fails with a file and line number —
  actionable, not a dashboard.
- **Prefer gitleaks in CI over a pre-commit hook.** A hook that must be
  installed per machine is a control that silently stops working; the same
  check at the server boundary cannot be skipped or forgotten. If the owner
  wants the faster local feedback too, that is fine as an addition, never as
  the substitute.
- **All secrets via Railway environment variables.** No `.env` in the
  repository; `.env` and `.env.*` in `.gitignore` from the first commit. If a
  secret is ever pushed, the response is rotate first, then clean history —
  history rewriting alone is not remediation on a repository that others may
  have already cloned.
- **`SECURITY.md` with a disclosure contact, and GitHub private vulnerability
  reporting enabled.** I would rank this second only to push protection, and it
  is the one control that directly answers OC-1. This project has no security
  reviewer and no second pair of eyes; a public repository means the realistic
  discoverer of a vulnerability is a stranger. Without a stated private
  channel they either open a public issue (an instant zero-day on a live
  service holding user data) or say nothing at all. One file, no recurring
  cost, and it recruits the reviewer the constraint register says does not
  exist.

### 5. Dependency scanning — enable it, but tuned so it stays quiet (Way of Working)

The failure mode the brief warns about is real and Dependabot is where it
usually happens. The distinction that decides it:

- **Enable Dependabot security updates** — a PR only when a CVE actually
  affects a dependency you use. Group them into a single PR, weekly.
- **Do not enable Dependabot version updates.** Scheduled bump-everything PRs
  produce a treadmill a solo builder will not merge, and an ignored PR queue
  trains you to ignore the security PRs sitting in the same list. This is the
  single most important noise decision in this contribution.
- **Do not add `npm audit --audit-level=high` as a build gate.** It reads the
  same advisory database Dependabot does, but fails the build on transitive
  dev-dependency findings that frequently have no available patch — the
  textbook un-actionable alert. Pick one source; Dependabot is the better one
  here because it delivers the same information without blocking work.
- **Commit the lockfile, and install from it in CI and at deploy.** Cheapest
  supply-chain control there is, and the draft does not mention it. Note for
  environment-provisioning rather than an assumption to make now: Railway's
  Nixpacks builder runs `npm ci`, but the newer Railpack builder may need
  `RAILPACK_INSTALL_CMD` set to install frozen — **verify which builder is in
  use rather than assuming the lockfile is honoured.**
- **Pin `osm2streets-js` to an exact version, no caret range.** It has not
  moved since June 2023; a range buys nothing, and any future release from a
  project this quiet should be a deliberate reviewed act. Ranges are fine for
  everything else.

### 6. SAST — CodeQL, once there is something for it to find (Testing Posture)

CodeQL default setup is free on public repositories, runs on GitHub's runners
at no cost, and its default query pack is tuned for precision over recall — it
is specifically built not to be the thing nobody triages. On a small, clean
TypeScript codebase it typically reports nothing.

But turning it on for B-0 and Stage 1 would scan a client-side map editor with
no authentication, no server-side persistence and no personal data. It would
cost setup and return nothing, which is how a control loses credibility before
it is ever needed.

**Recommendation: enable CodeQL default setup at the start of Stage 2**, when
accounts, sessions, access-controlled sharing and user-owned data appear — and
**name it as a line item on the public-release gate that already exists**, so
it cannot be quietly skipped. Attaching it to a gate the project already
enforces is stronger than "do it sometime."

I would **not** add a separate SAST product (Semgrep, Sonar, Snyk Code) on top.
CodeQL plus the lint pass covers this stack, and a second scanner is a second
findings list for one person to reconcile.

### 7. DAST — no. Replace it with assertable header tests. (Testing Posture)

This is my clearest "cargo cult" call, and I am rejecting a control my own
knowledge base recommends by default.

The draft proposes (and I agree with) a single Railway environment and no
staging tier. That leaves nowhere safe to point a scanner. An active OWASP ZAP
scan against production would write junk into the live database and drive
exactly the metered memory/vCPU/egress usage R-8 warns about; standing up a
scan target instead costs money OC-4 does not have. And a ZAP baseline scan on
a client-heavy map editor reports mostly missing-header findings — a checklist
you satisfy once, not a scan worth running forever.

**Replace DAST with a small set of security assertions in the normal test
suite**, each with a clear pass/fail criterion as the inception phase rules
require:

- Response carries `Content-Security-Policy`, `Strict-Transport-Security`,
  `X-Content-Type-Options: nosniff`, and a `Referrer-Policy`.
- Session cookies set `HttpOnly`, `Secure`, and `SameSite` (Stage 2).
- An unauthenticated request for another user's saved design returns 403/404,
  not the design (Stage 2 — the object-level authorization check, which is the
  vulnerability class access-controlled sharing per Q5 actually turns on).

That converts a recurring noisy scan into deterministic tests the org's own
testing posture already runs for free on every merge. Revisit real DAST only if
a staging environment ever gets funded.

### 8. Lint and format (Code Style)

The org default — Prettier and ESLint, run in CI, failure blocks the PR — is
right and I endorse the draft carrying it forward. Three additions:

- **Check formatting, don't just configure it.** `prettier --check` in CI. A
  formatter nobody runs is not a practice.
- **TypeScript `strict: true` from the first commit.** Retrofitting strict mode
  later is a large mechanical change nobody schedules; it is free on day one,
  and it eliminates a whole class of defect at the osm2streets adapter boundary
  the draft already proposes (TC-4).
- **For the injection class, use `eslint-plugin-no-unsanitized`, and do NOT
  add `eslint-plugin-security`.** The latter flags every dynamic object access
  and non-literal regex and will bury real findings under noise in a
  geometry-heavy codebase — precisely the control this project cannot afford.
  `no-unsanitized` targets DOM sink assignment, which is the one that actually
  bites an editor rendering user-authored street and lane labels.

### 9. Sharpen the Streetmix rule so it is checkable (Code Style / Forbidden)

The draft's proposed Code Style rule — don't use Streetmix's naming or file
layout — is well-intentioned but unenforceable. It has no pass/fail criterion,
which the inception phase rules require, and it does not address the actual
licence risk. LC-3 and R-10 are about *code entering the codebase*, and R-10's
own stated mitigation is behavioural: "do not read Streetmix source while
building the editor." A naming convention does not enforce that; not cloning
the repository does.

Note also that automated tooling will not save them here: I-1 records that
Streetmix's GitHub licence metadata reports NOASSERTION while its LICENSE file
states AGPL-3.0-or-later, so a licence scanner would misclassify it. The
control has to be "never take the dependency," enforced where it is visible.

**Replace the naming proposal with the checkable version:** Streetmix is never
added as a dependency and never cloned into the workspace — verifiable by
grepping the lockfile and the git remotes, and consistent with R-10's stated
avoidance treatment.

### 10. What I am deliberately NOT recommending

Stated so a later stage does not quietly add them back:

- **SBOM generation.** No customer is asking, and GDPR does not require one.
  GitHub's dependency graph gives the equivalent view free. Revisit only if a
  city procurement process asks — plausible given the C7 differentiator, but a
  Stage 3 concern at the earliest.
- **Container image scanning.** Nothing here builds or ships an image the
  project controls.
- **IaC scanning** (Checkov, cfn-nag, cdk-nag). There is no IaC — Railway
  config is not a Terraform/CloudFormation surface these tools read.
- **A second SAST product**, `npm audit` as a gate, `eslint-plugin-security`,
  and scheduled Dependabot version updates — each argued against above.
- **A threat model document at this stage.** The attack surface at B-0 and
  Stage 1 is a client-side editor over public map data. A STRIDE pass earns its
  keep at Stage 2, against accounts, sessions and access-controlled sharing —
  and should be run then, before that code is written, not as ceremony now.

**The organizing rule I would put in the practice draft**, so it constrains
future stages: *every security control this project adopts either blocks at the
moment of the mistake with a specific actionable message, or it is off. No
control produces a dashboard, a periodic report, or a queue for one person to
work through.* That is the honest reading of OC-1, and it is what separates the
list above from the list I just rejected.

### 11. Proposed additions to `discovered-rules.md`

I have respected the draft's own bar — a mandate needs a stated constraint with
a named consequence — and it excludes most of what I proposed above. Two meet
it; the rest are practice bullets, not rules.

**Forbidden (proposed):**

- NEVER add Streetmix as a dependency or clone its repository into the
  workspace. Source: `constraint-register.md` LC-3 (AGPL-3.0-or-later, "NOT a
  code dependency of this project") together with `raid-log.md` R-10, whose
  treatment is Avoid and whose stated action is "do not read Streetmix source
  while building the editor." This sharpens the existing "NEVER copy" rule into
  something checkable at the lockfile and the git remotes.

- NEVER commit a secret, credential or connection string to the repository;
  secrets are supplied only as Railway environment variables, and any secret
  that reaches a commit is rotated before history is touched. Source:
  `constraint-register.md` LC-1 (the product is open source) — the consequence
  is that the source is public, so leakage is immediate and global rather than
  internal. **Conditional on the visibility answer in §2**; if the repository
  is private for a period, the rule still holds but the rationale narrows.

**Not proposed as rules**, deliberately: dependency pinning, the fork-and-pin
control, CodeQL timing, the DAST rejection, and the lint additions are all
engineering judgment fitted to stated constraints, not constraints themselves.
They belong in `team-practices.md` where the interview can accept or reject
them.

### 12. Interview questions to add

1. **Repository visibility and timing** — public from the first commit, or
   private until a later stage? (Decides whether the free scanning controls
   exist while the code is being written. §2)
2. **osm2streets build path** — does B-0 settle "prebuilt npm package" versus
   "build from source at a pinned commit," and if from source, is the
   fork-and-pin done up front? (§3)
3. **Dependabot posture** — security updates only, or also scheduled version
   updates? (Recommend security-only. §5)
4. **CodeQL timing** — accept enabling it at Stage 2 as a line item on the
   existing public-release gate? (§6)
5. **DAST** — accept replacing it with security-header and object-level
   authorization tests in the normal suite? (§7)

### 13. Additions for `evidence.md`

Under "What Was Inspected", the devsecops review checked live external state
rather than the brief's summary of it, per the project's affirmed correction:
the npm registry metadata for `osm2streets-js` (0.1.4, 2023-06-04, four files,
no dependencies, no provenance attestation); the GitHub API state of
`a-b-street/osm2streets` (last pushed 2025-10-02, Apache-2.0, not archived);
`osm2streets-js/Cargo.toml` on `main` (unpinned `abstutil` git dependency;
`chrono 0.4.35` postdating the npm publish); the presence of a root
`Cargo.lock`; the `a-b-street/abstreet` LICENSE file (Apache-2.0, read
directly, not from metadata); GitHub's published pricing for Actions on public
repositories and its security-feature availability matrix; and Railway's
builder install behaviour (Nixpacks `npm ci`; Railpack may need
`RAILPACK_INSTALL_CMD`).

Under "What Could Not Be Established": whether the repository is public from
the first commit; whether B-0 will consume the prebuilt package or build from
source; and which Railway builder will be in use, which decides whether the
lockfile is actually honoured at deploy.

Remove from "What Could Not Be Established": the CI-cost tension (§1) — it is
established, and the answer is that CI is free here.

## Positions

- AGREE: The single-Railway-environment deployment proposal with no staging tier — it is the right call at this budget, and it is also why I reject DAST rather than asking for a scan target OC-4 cannot fund.
- AGREE: The draft's discipline that a mandate requires a stated constraint with a named consequence — I have held my own proposals to it, and it is why only two of my twelve recommendations are proposed as rules.
- AGREE: Promoting data subject rights to a Mandated rule gating Stage 2's public release — retrofitting erasure and export is materially harder than building them in, exactly as R-6 states.
- AGREE: The osm2streets adapter-module proposal in Testing Posture — a single boundary is also the only place a future fork or version pin can be swapped without touching the editor.
- OBJECT: The CI-cost tension in Testing Posture and `evidence.md` is factually wrong — GitHub Actions is free and unmetered for public repositories, OC-4 binds Railway hosting only, and leaving it in risks the interview answering "no CI" on a false premise.
- OBJECT: The draft is silent on supply chain, which the RAID log names as this project's critical dependency risk — the unpinned `abstutil` git dependency on a dormant repository and the unattested three-year-old `.wasm` binary are decisions B-0 makes by accident unless the practice draft forces them.
- OBJECT: Nothing in the draft covers secret handling on a repository that will be public, where a leaked credential is world-readable on push rather than an internal incident.
- OBJECT: The proposed Code Style rule about avoiding Streetmix's naming has no pass/fail criterion and does not address the actual licence risk — replace it with the checkable rule that Streetmix is never a dependency and never cloned into the workspace.
- OBJECT: `SECURITY.md` plus private vulnerability reporting is missing, and it is the one control that directly answers OC-1's "no second pair of eyes" — without a private channel, a stranger finding a flaw in a public repo either files a public zero-day or stays silent.
