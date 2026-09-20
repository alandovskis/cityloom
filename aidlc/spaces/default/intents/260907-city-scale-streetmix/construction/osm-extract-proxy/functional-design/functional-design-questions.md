# Functional Design — Questions — `osm-extract-proxy` (U9)

Upstream inputs: `unit-of-work.md` and `unit-of-work-story-map.md`
(units-generation), `requirements.md` (requirements-analysis), `components.md`
and `decisions.md` (domain-design), `contract-summary.md` (contract-design),
`bolt-plan.md` (delivery-planning).

**What this stage decides for this Unit:** the proxy's entities, business
rules, workflows and failure behaviour at a design level — not code. Most of
it is already settled upstream and is not re-asked here:

- The fetch goes through the Railway service (domain-design Q3); the proxy is
  built now and its real egress is measured at B-0 / B-3 (Q8).
- It logs neither IP addresses nor requested locations, and never a pair of
  the two; its cache is keyed on the extract, never on the requester
  (`decisions.md` ADR-004).
- Contract 1 fixes the endpoint: `GET /api/extract?bbox=…` with an
  `x-client-build` header; extract bytes back with an `x-extract-key` header;
  a closed failure-reason set (`area_not_found`, `area_too_large`,
  `upstream_unavailable`, `upstream_rate_limited`, `malformed_extract`,
  `timeout`, `internal`); `426` when the client build is no longer served.
- Timeout and retry *numbers* are owed to `nfr-requirements` (contract-design
  Q6); the obligations exist now, the values do not.
- Whether `bbox` stays the request shape is an open question that depends on
  the tile service Infrastructure Design has not chosen; this design uses
  `bbox` as the contract states.

Four things are genuinely undecided and change what gets designed. They are
below.

---

## Q1. Which public OpenStreetMap service does the proxy fetch from?

`components.md` says "a public OpenStreetMap API" and `contract-summary.md`
leaves the upstream unnamed because its shape is not ours to design. But the
choice shapes this Unit's rules: what an upstream `429` or `406` means, how
large an area may be asked for, and what "being polite" concretely requires.

Two facts checked against the OpenStreetMap wiki on 2026-09-11:

- **The Overpass API's public instance (`overpass-api.de`)** states that you
  do not disturb other users below 10,000 queries and 1 GB downloaded per
  day, and asks a "regular application" to stay under a hundredth of that —
  fewer than 100 queries fetching less than 10 MB per day. It asks that the
  application identify itself with a `User-Agent` or `Referer`, that it pause
  30 seconds after a `429` or `406`, and says commercial use should go to
  self-hosted or paid servers. **A proxy makes every user's request look like
  one application from one address, so those figures bound the whole product,
  not each user.** That is what makes the cache and the upstream limiter
  load-bearing rather than polite extras.
- **The OpenStreetMap editing API's map call** (`/api/0.6/map`) caps a
  bounding box at 0.25 degrees on a side and 50,000 nodes, and its own
  documentation points anyone downloading data for a purpose other than
  editing to Overpass instead.

A. **The public Overpass instance, behind a narrow "extract source" seam.**
   Design against `overpass-api.de` now — identifying `User-Agent`, one
   upstream fetch per extract at a time, a global limiter toward upstream, a
   30-second back-off after `429`/`406` — with the upstream reached through
   one seam so a different instance or a paid service can replace it without
   touching the rest. Honest cost: the product shares one fair-use allowance;
   at any real usage the numbers above will be exceeded, and the measurement
   at B-3 is what tells us whether that is a problem.

B. **Design against "an Overpass-compatible service" and let Infrastructure
   Design pick the instance.** Same rules and seam as A, but the functional
   design names no host; the instance (public, another public mirror, or a
   paid service — the last is spend against the ~$5 budget and therefore a
   constraint change under `project.md`) is chosen at infrastructure-design,
   before any code is written.

C. **The OpenStreetMap editing API's map call.** Simpler query, but its own
   documentation directs non-editing downloads elsewhere, so the product
   would be using a free service against its stated purpose from day one.

X. Other (please specify)

[Answer]: X — Use geofabrik provincial PBFs (settled by Q1a and Q1b below)

---

## Q2. Where does the extract cache live?

NFR3.3 says the application service holds no attached storage volume and that
persistent state lives in a separate database service — which is what lets a
new version take over without downtime (NFR3.2). `components.md` gives the
proxy a `CachedExtract` entity and an "extract cache" dependency, but not where
it lives.

Two things to weigh:

- **A cache hit still sends the extract bytes to the browser.** The egress
  the budget worries about ($0.05/GB, `requirements.md` OQ1) is paid whether
  the bytes came from the cache or from upstream. The cache saves upstream
  queries — the shared fair-use allowance in Q1 — and latency. It does not
  save money.
- **Deploys happen on every merge to `main`** (`team.md`). An in-memory cache
  is empty after each one.

A. **In memory only.** Bounded by a total-bytes ceiling with least-recently-
   used eviction; empty after every deploy. Nothing to provision, nothing to
   pay for, and the cost of a cold cache is upstream queries, not money.
   The ceiling's number is owed to `nfr-requirements` like the timeout.

B. **In the separate database service**, so the cache survives deploys and is
   shared if there is ever more than one instance. Means the database exists
   at B-3 (`osm-extract-proxy`) rather than first appearing at B-6
   (`design-storage`), and extract bytes sit in a database table that grows
   with every area anyone looks at.

C. **Memory in front, database behind.** A's speed and B's persistence — and
   two layers to design, build and test in a Unit sized Small.

X. Other (please specify)

[Answer]: A

---

## Q3. How long is a cached extract served before it is fetched again?

`CachedExtract` carries `expiresAt`; nothing upstream says what it is set to.
The value is a configuration setting either way; this question is its default.

What it changes:

- A user who fixes the map in OpenStreetMap and re-imports the street sees the
  old extract until it expires (or the cache empties).
- Corrections (US7.5) carry an import fingerprint; a changed extract is what
  makes a correction "no longer resolve". A longer life means fewer of those
  moments; a shorter one means fresher data.
- Every expiry is one more upstream query against the allowance in Q1.

A. **24 hours.** "Yesterday's map": a fix in OpenStreetMap shows up the next
   day, and a street looked at repeatedly in one session is fetched once.

B. **7 days.** Fewer upstream queries for streets people keep returning to;
   a map fix waits up to a week.

C. **30 days.** Matches the anonymous-upload expiry already in the product;
   the map can be a month stale.

D. **No time expiry — size-based eviction only.** An extract stays until it
   is pushed out or the cache is emptied by a deploy. Freshest only by
   accident.

X. Other (please specify)

[Answer]: A

*Superseded in meaning by Q1b: with clips cut from regional files there is no per-extract expiry; freshness is the regional refresh cadence. The answer is kept as given.*

---

## Q4. Does the proxy limit individual requesters, and what may it hold to do so?

`decisions.md` ADR-004 forbids logging IP addresses, requested locations, or a
pair of the two. The rate limiting the design states is toward upstream —
global, for the whole service. Nothing bounds one client.

That matters more than it first looks, because of the point in Q2: egress
scales with requests *served*, cached or not. A single looping or abusive
client — the endpoint is unauthenticated by design — can run up the egress bill
and exhaust the shared upstream allowance for everyone. `DesignRepository`
(U10) rate-limits by identifier for the same reason; the proxy has no
identifier except the requester's address, which is exactly the thing ADR-004
is careful about.

A. **Per-requester limiting held in memory only.** A counter keyed on a keyed
   hash of the requester's address — the hash key minted at process start, so
   values cannot be matched across restarts — holding no location, never
   written to any log or store, discarded on restart. This is a stated
   boundary ADR-004 does not forbid: it forbids logs and pairs, and this is
   neither. Exceeding the limit answers `429 upstream_rate_limited`
   (retryable), so the client's retry offer (US7.2) behaves the same way as
   for an upstream limit.

B. **No per-requester state at all** — the strictest reading of ADR-004. Only
   the global upstream limiter and aggregate counters exist. Accepts that one
   client can consume the whole product's allowance and egress.

C. **A, plus a global daily egress ceiling** after which the proxy refuses
   every request with a retryable failure until the day rolls over. This goes
   beyond what domain-design Q8 chose (measure first, no ceiling yet); it is
   offered because caching turns out not to bound egress, and B-3 is the
   first time the figure exists.

X. Other (please specify)

[Answer]: A

---

## Q1 follow-up — Geofabrik regional extracts instead of a live query service

To Q1 you answered, verbatim: "Use geofabrik provincial PBFs". None of the
offered options covered it, so Q1's answer is held until the two questions
below settle what that means for the design.

Facts checked on 2026-09-11:

- Geofabrik publishes one `.osm.pbf` per sub-region, refreshed about daily.
  Canada's provinces range from 10.8 MB (Prince Edward Island) through
  333 MB (Alberta), 943 MB (Ontario), 1.1 GB (Quebec) to 1.2 GB (British
  Columbia). Licence ODbL 1.0; the public files omit user names and changeset
  ids.
- osm2streets' reader (`streets_reader::osm_to_street_network`) accepts
  either `.osm.xml` or `.pbf` bytes, so the proxy can serve PBF clips, which
  are several times smaller than XML — a direct saving on the $0.05/GB egress
  the budget worries about.

What changes if the proxy serves clips from regional files rather than
querying a live service:

- **There is no per-request upstream.** The Overpass fair-use allowance, the
  upstream limiter and the 30-second back-off all disappear. What replaces
  them is a periodic refresh of regional data, at most as often as Geofabrik
  publishes. `upstream_unavailable` comes to mean "the regional data is not
  loaded", and `area_not_found` also covers "outside the regions this
  deployment carries".
- **Coverage becomes a configured set of regions**, not the world. Fine for
  a city-scale tool; the product has to be able to say "this street is not in
  an area this deployment covers".
- **Q3 (cache lifetime) stops meaning what it asked.** Clips cut from a
  regional file are only as fresh as that file, so "24 hours" can only be
  realised as the refresh cadence of the regional data — Q1b below makes that
  explicit rather than reinterpreting your answer silently.
- **The hard part is NFR3.3 and memory.** The service holds no attached
  volume, and cutting a bounding box out of a ~1 GB province needs a lookup
  from node id to coordinate for every node the ways in that box reference.
  Holding that for a whole province in memory is far beyond what ~$5/month
  buys (Railway meters memory), so "load the province and clip per request"
  is not an option this design can take. Q1a is about what is.

---

## Q1a. How is the regional data prepared and held?

A. **Pre-cut offline, shipped with the deploy.** A build step — run by the
   maintainer, or by CI on a schedule — takes each configured regional file,
   keeps what osm2streets needs (the `highway` ways and the nodes they
   reference) and slices it into a fixed grid of small cells. The cells are
   baked into the deploy image (or fetched from static storage at start-up
   onto ephemeral disk). At request time the proxy picks the cells covering
   the bounding box, merges them and answers. No memory beyond the clip being
   built, no attached volume, deterministic, testable against committed
   cells. Freshness equals how often the build step runs and redeploys. U9
   gains a build-time pipeline as part of its scope.

B. **Downloaded and sliced at service start.** Each start pulls the regional
   files from Geofabrik onto ephemeral disk, filters and slices them there,
   then serves as in A. Simplest deploy, but every deploy — one per merge to
   `main` — downloads up to ~1 GB per region from a free service and spends
   minutes slicing before the service is healthy, and the slicing still needs
   a node lookup: on disk to stay within memory, which is real work inside
   this Unit.

C. **Clip directly from the regional file per request.** No preparation
   step, but every request scans or seeks across the whole file; the one
   option that is either too slow or needs the memory the budget cannot pay
   for. Listed so its rejection is on record.

X. Other (please specify)

[Answer]: A

---

## Q1b. How often is the regional data refreshed?

This replaces Q3's "24 hours" with the thing that now actually controls
freshness. Each refresh downloads the regional files again (hundreds of MB to
~1 GB per region) and redeploys.

A. **Weekly, on a schedule.** A map fix appears within a week; one download
   per region per week from Geofabrik.

B. **Daily, on a schedule** — Geofabrik's own cadence and the literal reading
   of your Q3 answer; a daily redeploy and a daily ~1 GB download per region.

C. **Only when the maintainer chooses.** No schedule; the data moves when the
   build step is run by hand. Cheapest, and the map is as stale as the last
   time anyone remembered.

X. Other (please specify)

[Answer]: A

---

## Consolidated Summary Confirmation

**Your answers**

- **Upstream** (Q1, settled by Q1a/Q1b): Geofabrik regional (provincial)
  `.osm.pbf` extracts, **pre-cut offline** into a fixed grid of cells holding
  the highway ways and the nodes they reference, shipped with the deploy;
  **refreshed weekly on a schedule**. No live query service at request time.
- **Cache** (Q2): in memory only, bounded by a total-bytes ceiling with
  least-recently-used eviction; empty after every deploy.
- **Lifetime** (Q3): 24 hours as answered — superseded in meaning by Q1b, since
  a clip cut from a regional file cannot go stale before the next weekly
  build; the cache evicts by size only.
- **Requester limiting** (Q4): a per-requester counter held in memory only,
  keyed on a keyed hash of the address with the key minted at process start,
  holding no location, never logged or stored, discarded on restart.

**Things I will write that follow from your answers rather than being stated
by them**

- **The proxy becomes a clip server with a build-time pipeline.** U9 gains
  an offline step — download the configured regions, keep what osm2streets
  needs, slice into cells, write a manifest — and the request path becomes
  pick-cells, merge, de-duplicate, complete every included way's nodes,
  encode, serve. The served bytes are PBF, since osm2streets reads it and it
  is several times smaller than XML.
- **Coverage is a configured set of regions.** A bounding box that touches no
  built cell answers `area_not_found` with a detail saying the area is not
  one this deployment covers; a box inside a region that contains no highway
  answers `area_not_found` with a detail saying nothing is mapped there.
- **The extract key carries the data build.** `x-extract-key` is derived from
  the canonical bounding box and the regional data build identifier, so the
  same street on the same build yields the same key, and a new weekly build
  yields a new one — which is what a correction's import fingerprint (US7.5)
  needs to be deterministic.
- **Contract 1's failure set needs amending, and I will record the
  amendments rather than edit the approved artifact:** `invalid_area` is
  added for a bounding box that cannot be read (the enum has no reason for a
  malformed request today); `upstream_rate_limited` is renamed
  `rate_limited` because there is no upstream and the proxy's own limiter is
  its only producer; `malformed_extract` has no producer on the server and is
  recorded as reserved; `internal` is classified retryable, which the
  contract leaves unclassified. Status mapping: 400 for `invalid_area` and
  `area_too_large`, 404 for `area_not_found`, 429 for `rate_limited`, 503 for
  `upstream_unavailable` (regional data not loaded), `timeout` and
  `internal`.
- **`CachedExtract` loses `expiresAt`** and gains the data build identifier;
  `fetchedAt` becomes the moment the clip was cut. Recorded as a deviation
  from `components.md`'s attribute list.
- **Privacy holds at the type level.** The requester counter and the service
  counters are the only requester-related state, both memory-only; the
  failure log carries the reason, the status and the time — never a bounding
  box, never an address.
- **Numbers are owed to `nfr-requirements`, not set here**, following
  contract-design Q6: the cell size, the cell-span bound behind
  `area_too_large`, the cache byte ceiling, the per-requester window and
  limit, and the request timeout.
- **Four upstream statements now describe a different design**, recorded as
  amendments in the spec rather than edited: `components.md`'s external
  dependency "Public OpenStreetMap API" and its upstream rate-limiting
  responsibility; `unit-of-work.md` U9's "fetching … on a browser's behalf
  … rate-limiting its own traffic toward the upstream public API";
  `bolt-plan.md` B-3's Definition of Done; and `decisions.md` ADR-004's
  alternatives, which never considered regional files.

**Things I will record as open rather than answer**

- The egress figure for one real street (B-3's Definition of Done) is still
  a measurement, now of a PBF clip rather than an XML response.
- Whether `bbox` remains the request shape once a tile service is chosen
  (contract-design open question) — unchanged by this design.
- Which regions the first deployment carries, and the first fixture streets —
  configuration and fixture choices for U1 and code generation, not design.

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
