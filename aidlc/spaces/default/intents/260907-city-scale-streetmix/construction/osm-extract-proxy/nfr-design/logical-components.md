# Logical Components — `osm-extract-proxy` (U9)

Upstream inputs: `performance-requirements.md`, `security-requirements.md`,
`scalability-requirements.md`, `reliability-requirements.md`,
`observability-requirements.md` and `tech-stack-decisions.md`
(nfr-requirements, this Unit), `functional-spec.md` and `entities.md`
(functional-design, this Unit), `contract-summary.md` (contract-design),
`components.md` (domain-design), `unit-of-work.md` (units-generation),
`nfr-design-questions.md` (this stage).

Design elements are numbered `LC-n`. This is the bridge to Infrastructure
Design: the logical pieces this Unit consists of, where each NFR pattern
lives, what fails together, and what is shared. `components.md` names one
component for this Unit, `OsmExtractProxy`; this file is its inside and its
surroundings, not a new catalogue entry.

## Inventory

```
 +------------------------------------------------------------------+
 | LC-1  proxy process (one Rust binary, one Railway service)       |
 |                                                                  |
 |  +----------+  +---------+  +--------------+  +---------------+  |
 |  | listener |->| limiter |->| canonicaliser|->| store reader  |  |
 |  | + router |  | (SD-4)  |  | + span/cover |  | (PD-2, index) |  |
 |  +----------+  +---------+  +--------------+  +-------+-------+  |
 |       |                                               |          |
 |       v                                               v          |
 |  +----------+  +---------+  +--------------+  +---------------+  |
 |  | readiness|  | cache   |<-| cutter       |<-| encoder       |  |
 |  | (SD-8)   |  | (PD-5)  |  | (PD-3, slots)|  | (TS-3)        |  |
 |  +----------+  +---------+  +--------------+  +---------------+  |
 |       |                                                          |
 |  +----------+  +---------------------------------------------+   |
 |  | counters |  | emitter (OD-1..OD-4)  --> stdout             |   |
 |  | (OD-2)   |  +---------------------------------------------+   |
 |  +----------+                                                    |
 +------------------------------------------------------------------+
        ^ reads (read-only)                     ^ probes     ^ collects
        |                                       |            |
 +--------------------+              +----------------+  +-----------+
 | LC-2 cell store    |              | LC-4 platform  |  | LC-5 log  |
 | store + manifest   |              | edge + health  |  | sink      |
 +--------------------+              +----------------+  +-----------+
        ^ produces
        |
 +--------------------+   TLS + .md5   +--------------------+
 | LC-3 region-build  |<---------------| LC-6 publisher     |
 | tool (offline)     |                | (Geofabrik)        |
 +--------------------+                +--------------------+
```

<!-- Text fallback: LC-1 is the proxy process, one Rust binary in one Railway service, containing the listener and router, the limiter, the canonicaliser with span and coverage checks, the store reader with its index, the cutter behind the slots, the encoder, the cache, the readiness endpoint, the counters and the emitter writing to stdout. LC-2 is the cell store artifact (store file plus manifest) the process reads read-only. LC-3 is the offline region-build tool that produces LC-2 by downloading from LC-6, the publisher, over TLS with checksums. LC-4 is the platform edge with its health probe, and LC-5 the platform's log sink. -->

| # | Component | Kind | Owner | What it is |
|---|---|---|---|---|
| LC-1 | **Proxy process** | service (this Unit's runtime) | U9 | The `OsmExtractProxy` component of `components.md`, as one process: the request path of W3 and the start-up of W2, in the modules named in the diagram |
| LC-2 | **Cell store artifact** | read-only data | U9 (built), Infrastructure Design (placed) | One packed store file plus one manifest per `RegionalDataBuild` (Q1); the only data the process reads |
| LC-3 | **Region-build tool** | offline job (a second binary in the workspace, TS-9) | U9 (code), Infrastructure Design (schedule and runner) | W1: download, verify, filter, slice, write the store and manifest |
| LC-4 | **Platform edge and health probe** | external (Railway) | Infrastructure Design | TLS termination, the asserted client address, the readiness probe that gates rollover |
| LC-5 | **Platform log sink** | external (Railway) | observability-setup | Collects stdout; where the rows are read |
| LC-6 | **Publisher** | external (Geofabrik) | — | The source of regional extracts and their checksums, consumed weekly at build time only |

## Where each NFR pattern lives

| Pattern | Component | Design element |
|---|---|---|
| Request budget (deadline) | LC-1 listener/handler | PD-4, RD-3 |
| Admission control (per-requester windows) | LC-1 limiter | SD-4, SC-1 |
| Bulkhead (cutting slots) | LC-1 cutter | PD-3, RD-3, SC-2 |
| Cache and single-flight | LC-1 cache | PD-5, SC-3 |
| Integer addressing and read-only data | LC-1 canonicaliser, store reader; LC-2 | PD-1, PD-2, SD-5 |
| Verified start-up and Unready | LC-1 (start-up) reading LC-2 | RD-1, SD-10 |
| Readiness-gated rollover | LC-1 readiness; LC-4 probe | RD-2, SD-8, OD-6 |
| Panic boundary and failure mapping | LC-1 listener, emitter | RD-4, SD-2, OD-3 |
| Emission discipline | LC-1 emitter → LC-5 | SD-7, OD-1 to OD-4 |
| Build integrity and provenance | LC-3 ← LC-6 → LC-2 | SD-10, RD-8, OD-5 |
| Supply chain | the workspace | SD-11 |

## Failure domains and blast radius

| Domain | Fails as | Blast radius | Recovery |
|---|---|---|---|
| LC-1 process | crash, panic, exhaustion | Every street import in the product (U4 `street-import` depends on this Unit — `unit-of-work.md`); the client degrades to FR6.1's message and FR6.2's blank cross-section, never a wrong street | Platform restart (RD-6); a panic is contained per request (RD-4) |
| LC-2 artifact | missing, truncated, corrupted | The new deployment never takes traffic (Unready, RD-1); the previous one keeps serving (RD-2) | Redeploy the last good build (RD-9) |
| LC-3 build job | download, checksum or slicing failure | Nothing: no publish, the deployed build stays (RD-8); the map is a week staler than intended | Re-run the build |
| LC-4 platform edge | outage | The whole service, and every other Unit in the same Railway service | None in this Unit (`reliability-requirements.md` NFR3.1.11) |
| LC-5 log sink | outage | Observability only; requests are unaffected (stdout writes never block the request path) | None needed |
| LC-6 publisher | outage or a changed file layout | The weekly build fails and publishes nothing; the deployed build stays | Re-run later; a layout change is a build-tool change |

The domains are nested the way the recovery is: a build failure never
reaches the artifact, an artifact failure never reaches traffic, a process
failure never reaches data (there is none).

## Isolation and shared resources

- **Isolation inside LC-1** is by the slot (four cuts) and the deadline;
  hits, failures and readiness share the async runtime and never wait on a
  cut. There is no other contention point: the limiter's critical section
  is an increment, the cache is lock-free for readers, the counters are
  atomics.
- **The one shared resource is the Railway service itself.** `unit-of-work.md`
  makes U9–U12 four work Units in one deployable; when U10–U12 are built,
  they share LC-1's process, runtime, memory and CPU. This Unit's figures
  (128 MB typical, ≤ 0.05 vCPU average, `performance-requirements.md`,
  `scalability-requirements.md`) are its contribution; the process-wide
  targets are the sum the later Units add to, and Infrastructure Design
  sizes the service against the sum. The slots and the cache ceiling bound
  this Unit's share so a later Unit's load cannot be starved by cutting.
- **No shared state with any other Unit.** The proxy reads LC-2 only and
  holds nothing another Unit reads. Its one consumer, U4 `street-import`,
  runs in the browser and reaches it only through Contract 1 over HTTP;
  the server Units that will share its process (U10–U12) do not call it at
  all.

## Handed to Infrastructure Design

Named so the next stage has a list rather than a search:

1. Where LC-2's two files live in the deploy — inside the image, or fetched
   to ephemeral disk at start — and how LC-3's output reaches them
   (`functional-spec.md` assumption; NFR2.1.6's size targets).
2. LC-3's schedule and runner (weekly plus on demand, BR8.5; ≤ 30 minutes,
   NFR2.1.6; memory per SC-5).
3. LC-4's health probe: path, interval, timeout exceeding the measured
   Ready time (RD-2, NFR3.2.2); the asserted-address header and position
   (SD-1, NFR6.3.3); `SIGTERM` and drain behaviour (RD-7).
4. The service size for one shared vCPU and the memory sum of the four
   server Units (SC-2, SC-6).
5. The account usage limit (NFR5.1.6).

## Assumptions & Open Questions

- U10–U12 will share LC-1's process rather than being separate services;
  `unit-of-work.md` says one deployable, and one binary is the reading this
  design takes. If they become separate processes on the same service, the
  shared-resource note above weakens rather than breaks. [assumption]
- No later server Unit will call the proxy in-process; if one ever does,
  the boundary stays Contract 1's shape as a function signature, and the
  limiter and counters are not bypassed. [assumption]
