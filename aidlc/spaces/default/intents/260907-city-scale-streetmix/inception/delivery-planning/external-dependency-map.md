# External Dependency Map — Streetmix at City Scale

Upstream inputs: `requirements.md` (requirements-analysis), `stories.md`
(user-stories), `mockups.md` (refined-mockups), `components.md`
(domain-design), `unit-of-work.md`, `unit-of-work-dependency.md` and
`unit-of-work-story-map.md` (units-generation), `contract-summary.md`
(contract-design), `team-practices.md` (practices-discovery).

Things outside this project that can hold a **Bolt** up — a Bolt being one
build pass over a piece of the work, ending in something that runs.
`bolt-plan.md` has the sequence; this file says what each one waits on that
the builder does not control.

## Nothing here is another team

There are no hand-offs, no approval queues, and no external team commitments,
because there is no other team — Team Formation was skipped and
`team-practices.md` records that there is no second human. Every item below is
a third-party service or an upstream project. That makes this map shorter than
it would be on a staffed project, and it also means **no mitigation here is
"escalate"**; each one is either a fallback the builder can execute alone or an
accepted exposure.

## The map

| Dependency | What it is | Blocks | Lead time | If it slips |
|---|---|---|---|---|
| **osm2streets** (upstream project) | The source of lane geometry and network topology. `raid-log.md` D-1 records it as the project's single Critical dependency | B-0, B-1, B-4 — and transitively everything, since nothing imports without it | None once forked. The fork is the project's own from B-1 onward | The fallback is already in the plan rather than on paper: B-1 forks it into the project's own account and pins it to a commit SHA. After that, upstream moving is a decision, not an event. `raid-log.md` R-3 named forking as the treatment; B-1 makes it a fact |
| **Public OpenStreetMap API** | Source of extract bytes, fetched through the proxy | B-0, B-3 | None, but it is a free public service with rate limits and no contract | The proxy rate-limits its own traffic toward it (`decisions.md` ADR-004) and caches extracts keyed on the extract. Committed fixtures mean the test suite never calls it, so a slow or unavailable API blocks new imports, not the build |
| **Basemap tile service** (unchosen) | Background imagery and the coarse street network | B-0's spike; B-11 `client-surfaces` in full | Unknown — no service has been selected | **This is the one genuine unknown.** It must supply stable OpenStreetMap way identities for street selection (AC2.2.4), which not every tile service does. Q6 pulled the spike into B-0 for exactly this reason: a no changes the map design and possibly the selection model, and finding out at B-11 would be the expensive version. If no service meets the constraint, the selection mechanism for US2.2 has to be redesigned |
| **Railway** (hosting) | One service serving both the WASM bundle and the API | B-3 onward for anything deployed; the deploy-on-merge path for every Bolt | None to start | Two open items rather than a slip risk. Which builder can build a Rust workspace to a WASM artifact — a Nixpacks Rust provider versus a project-supplied `Dockerfile` — is confirmed at environment-provisioning, per `team-practices.md`, not assumed. Rollback uses Railway's own deployment history |
| **GitHub Actions, CodeQL, secret scanning, Dependabot** | The CI tiers and the free security controls a public repository gets | Every Bolt's merge gate | None | All free on a public repository, which is why `team-practices.md` made the repository public from the first commit. CodeQL is enabled at the start of B-10, named there as a line item rather than "sometime" |
| **SonarQube Cloud** | The blocking quality gate on new code, in the slow CI tier | Every Bolt's merge gate | Account setup, once | Free for public repositories with no line-of-code cap. Rust is not supported in its Automatic Analysis mode, so it runs as an explicit Actions step — a build-toolchain fact settled at practices-discovery, not a surprise waiting in CI |
| **PostgreSQL** (Railway service) | Design, account, session, grant and link storage | B-7 onward | None | A separate service from the application, so the application holds no attached volume — which is what makes NFR3.2's zero-downtime-deploy commitment achievable |

## Cost is the dependency to watch

Not a schedule risk, but the one external factor that can force a design
change. `project.md` forbids letting hosting and tooling spend grow past the
~$5/month working budget without treating that growth as a constraint change
requiring explicit justification — not something silently absorbed.

Two of the items above meter: Railway's compute and egress, and the proxy's
outbound traffic to the OpenStreetMap API. **The proxy's real egress is
unmeasured**, which `contract-summary.md` carries as an open question and B-3
closes with a figure for one real street. `team-practices.md` requires that
measurement at B-0.

If the figure breaks the budget, the affected decision is the proxy itself —
`decisions.md` ADR-004 chose it over a direct browser fetch — and reopening it
is a constraint change, not a tuning exercise.

## Gated items with lead times

**None.** No approval, procurement, contract, data-availability window or
external sign-off gates any Bolt in this plan. Every account needed is
free-tier and self-service, and every dependency is either open source or a
public API.

This section exists to say that positively rather than by omission: a reader
looking for the thing that will hold the project up will not find it here. On
this project the schedule risk is not external.
