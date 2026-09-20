# Monitoring Design — `osm2streets-build` (U1)

_Confirmed._

This Unit has no runtime, so the usual runtime observability pillars
(request metrics, live logs, distributed tracing) do not apply — there is
nothing running to observe. What is monitored instead is the **integrity
of the pin over time**, which is a build-time/CI concern, not a
production one.

## Metrics & KPIs

| Metric | Source | Threshold | Why it matters |
|---|---|---|---|
| `cargo audit` findings against the pinned dependency tree | CI (slow tier, per `team.md`) | Any Critical/High finding on `osm2streets` or `abstutil` fails the build | The pin exists specifically so a moving upstream can't surprise the project; a real advisory against the pinned commit is exactly the case worth blocking on |
| Golden-fixture suite pass/fail (asserted from `u4-street-import`) | CI (fast tier) | Any failure fails the build | This is this Unit's own trip-wire (`security-design.md` SD-7): if the fixture suite ever goes red against the pinned commit's output, the pin has effectively changed behaviour |

## Alerts

| Alert | Condition | Severity | Routes to |
|---|---|---|---|
| Pinned dependency audit finding | `cargo audit` reports a finding against `osm2streets`/`abstutil` in CI | High | The CI failure itself is the alert — a solo-builder project has no separate paging channel (`team.md`: "every security control here either blocks at the moment of the mistake ... or it is off") |

## SLIs / SLOs

Not applicable — no runtime service, no availability or latency target
for this Unit.

## Logs & Tracing

Not applicable. There is no request path to trace and no application log
to aggregate. The only "log" this Unit produces is the CI build output
itself, which is retained by GitHub Actions per its own default retention.

## Dashboards

None — a solo-builder project with no runtime surface for this Unit has
no dashboard to maintain (`team.md`'s stated preference against
"dashboard sprawl without clear ownership").

## Traceability

See `traceability.json` in this directory.
