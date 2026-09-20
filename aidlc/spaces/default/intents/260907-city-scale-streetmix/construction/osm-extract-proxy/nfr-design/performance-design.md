# Performance Design — `osm-extract-proxy` (U9)

Upstream inputs: `performance-requirements.md`, `scalability-requirements.md`
and `tech-stack-decisions.md` (nfr-requirements, this Unit),
`functional-spec.md`, `rules.md` and `entities.md` (functional-design, this
Unit), `contract-summary.md` (contract-design), `nfr-design-questions.md`
(this stage).

Design elements are numbered `PD-n`; `traceability.json` maps each
`NFRx.y.z` from `performance-requirements.md` to the elements that meet it.
This is a design, not an implementation: the shapes and the reasons are
fixed here, the code is code generation's.

## The shape of a request

W3 in `functional-spec.md` fixes the phase order. This design fixes what each
phase costs and where it runs. Two paths exist and they are deliberately
unequal:

```
request --> [stamp] --> [limit] --> [validate+canonicalise] --> [span] --> [coverage]
                                                                           |
                                            +------------------------------+
                                            v
                                     [cache lookup] --hit--> [respond]   (async task, no slot)
                                            |
                                           miss
                                            v
                              [single-flight: try_get_with]
                                            |
                                            v
                             [acquire 1 of 4 slots] --> [cut on blocking pool]
                                            |            (read cells by offset,
                                            |             decode, filter, dedup,
                                            |             sort, encode)
                                            v
                                   [store in cache] --> [respond]
```

<!-- Text fallback: the phases before the cache lookup are cheap integer work on the async task. A hit responds from memory with no slot. A miss enters the cache's single-flight loader, acquires one of four slots, and runs the cut on the blocking thread pool: read the touched cells by offset, decode, filter, de-duplicate, sort, encode; the result is stored and served. -->

Everything above the cache lookup is a few microseconds of integer work on
the request's async task. The only expensive work is the cut, and it is
fenced by the slot, the deadline and the single-flight.

## Design elements

### PD-1 — Integer canonical box and cell addressing

The raw `bbox` is parsed as four `f64` values, validated (BR3.1), then
converted once to an integer box in units of 1e-5 degree: minimums are
floored, maximums are ceiled (the outward rounding of
`performance-requirements.md`). From that point no floating-point value
exists on the request path.

- 0.01° is exactly 1,000 units, so a cell's column is `floor(lon / 1000)`
  and its row `floor(lat / 1000)` in integer division — cell edges are exact,
  and a box touching a cell edge is unambiguous.
- The touched-cell set is the rectangle of columns and rows the box covers;
  its size is checked against the span bound of 12 before anything is read
  (BR3.3, NFR1.1.5).
- The extract key is BLAKE3 over the canonical box rendered as four decimal
  strings with exactly five decimals, joined with commas, then a newline,
  then the `buildId` (BR6.1). Rendering from integers makes the key text
  reproducible on every platform.

### PD-2 — The packed cell store and its in-memory index

Q1's answer. One store file per build; the manifest lists each populated
cell's `cellId`, `offset`, `length`, `digest` (BR8.4).

- At start the index is loaded as a table sorted by cell id — 24 bytes per
  entry (`u64` id, `u64` offset, `u32` length plus padding), under 5 MB for a
  province of 200,000 cells — and looked up by binary search. A `HashMap`
  would be faster per lookup and larger; twelve lookups per miss do not
  justify the memory.
- A cell is read with a positional read (`read_exact_at`) into a buffer
  sized from the index. No memory map: the operating system's page cache
  already keeps hot cells resident, and a map would add `unsafe` and a
  second view of the same bytes. `entities.md`'s "a cell file exists only
  for cells with at least one way" is read as "a cell has an entry in the
  store only if it has at least one way"; an absent entry is a covered,
  empty cell (NFR3.1.10).
- Cells are stored compressed exactly as the encoder writes them (PBF
  blobs with zlib), so the store is the cells' served bytes' source form,
  not a second encoding.

### PD-3 — The cut, on the blocking pool, behind four slots, with a deadline

The cut is CPU work — decoding twelve cells and encoding a clip — and must
not block the async runtime. Design:

1. The cache loader (PD-5) acquires one permit from a `Semaphore(4)`
   (NFR1.1.6). Waiting for a permit happens on the async task, inside the
   request deadline (PD-4).
2. With the permit, the cut runs on `tokio::task::spawn_blocking`. It
   receives the deadline and checks it **between cells** and before
   encoding: a blocking task cannot be cancelled from outside, so the cut
   cancels itself, within one cell's decode time of the deadline, and
   releases the permit (NFR3.1.7). One cell at 0.01° is tens to a hundred
   kilobytes, a few milliseconds to decode.
3. Per cell: decode with `osmpbf` from the buffer; collect nodes into a
   map from id to (lat, lon); for each way, compute its extent from its
   nodes (all present, BR8.3) and keep it if the extent intersects the
   box (BR5.2), marking its nodes as needed.
4. Across cells: de-duplicate ways and nodes by id (BR5.3) — a way in two
   touched cells arrives twice with identical content.
5. Sort ways and nodes by id, nodes then ways (BR5.3), encode once with the
   project encoder into a byte buffer (TS-3), release the permit, return
   the bytes. A clip with no ways returns the "nothing mapped" failure
   instead (BR4.2) so nothing is cached.

Per-cut memory: twelve decoded cells plus the clip is bounded at 8 MB by the
span bound and the cell size (NFR5.1.2); the buffers are freed when the
task ends.

### PD-4 — One deadline for the whole request

After the build-stamp check (which is answered before anything else, BR1.1,
and needs no budget), the remainder of the handler body runs inside a single
`tokio::time::timeout` of 3,000 ms (NFR1.1.1). The same deadline instant is
handed to the cut (PD-3) so both the async wait for a slot or for an
in-flight cut, and the blocking cut itself, observe one clock. Elapsing
maps to `503 timeout` through the ordinary failure mapping (BR10.3) — the
framework's generic timeout status is never reached.

### PD-5 — Cache and single-flight

`moka::future::Cache<ExtractKey, Arc<[u8]>>` with `EvictionPolicy::lru()`,
a weigher returning the byte length, and `max_capacity` of 32 MiB
(NFR5.1.1, BR6.2). A hit is one lookup and a shared pointer to the bytes;
the response streams from that buffer with no copy per response.

A miss goes through `try_get_with(key, loader)`: concurrent misses for one
key share one loader run (BR6.3), and a loader error (empty clip, fault,
deadline) is returned to every waiter and stored nowhere. A clip larger
than the ceiling is admitted and immediately evicted by weight; the
response is unaffected.

The cache is created empty at start and owned by the loaded build (BR6.4);
a new build is a new process.

### PD-6 — Latency measurement without a request log

Two arrays of nine `AtomicU64` bucket counters, hit and miss, with the
upper bounds of `observability-requirements.md` NFR3.1.14; the handler
measures its own elapsed time at the response and increments one bucket.
No per-request value leaves the process (BR7.4); percentiles are read from
the differenced buckets in the periodic counters row (`observability-
design.md` OD-2).

### PD-7 — Start-up time

Start-up is bounded by one sequential read of the store (PD-2) and a BLAKE3
pass over each cell's byte range in file order; with the index sorted by
cell id but the store written in the same order by the build, the read is
sequential. For a filtered province of ≤ 500 MB that is disk-bound, seconds
on the platform's storage, inside NFR1.1.4's 30 seconds. The manifest
(JSON, 15–20 MB for 200,000 cells) is parsed directly into the compact
index table, never into a generic document model.

### PD-8 — The budget allocation, kept honest

The proxy owns 3 s of NFR1.1's 10 s. This design spends almost none of it on
the common path: a hit is a lookup; a miss is at most twelve small reads and
decodes plus one encode. The 3-second hard budget is a guard against the
uncommon — a cold page cache, a full slot queue, a slow disk — not the
expected cost, which the p95 targets state.

## Performance budgets per phase (provisional)

| Phase | Where it runs | Expected | Bound |
|---|---|---|---|
| Stamp, limit, validate, canonicalise, span, coverage | async task | microseconds | — |
| Cache lookup (hit) | async task | < 1 ms | p95 ≤ 30 ms end to end (NFR1.1.3) |
| Wait for a slot | async task | 0 at Stage 1 load | deadline (NFR1.1.1) |
| Read ≤ 12 cells by offset | blocking pool | 1–10 ms warm, tens cold | — |
| Decode ≤ 12 cells, filter, de-dup | blocking pool | 20–80 ms | deadline checked between cells |
| Encode one clip | blocking pool | 5–20 ms | — |
| Store and respond | async task | < 1 ms | p95 ≤ 300 ms end to end (NFR1.1.2) |

## Rejected alternatives

- **Memory-mapping the store.** Fewer copies, but `unsafe`, a second view
  of the file's bytes, and no measurable gain at twelve reads per miss.
- **Precomputing each way's extent at build time.** Skips the node pass at
  request time; kept as the first optimisation if B-3's measured miss
  latency needs it (`nfr-design-questions.md`, open items) — not taken now
  because it adds a field the encoder must carry and nothing has been
  measured.
- **Cutting on a dedicated CPU pool (rayon).** A second pool to size and
  test; `spawn_blocking` behind a semaphore gives the same isolation with
  one fewer moving part.
- **Serving hits from a shared `Bytes` with the response body streamed
  from disk.** The cache would then be an index, not bytes, and a hit would
  read the store; hits are the common case and must not touch disk.

## Assumptions & Open Questions

- Decoding a 0.01° dense cell with `osmpbf` takes single-digit milliseconds
  on a shared vCPU; unmeasured until B-3. [assumption]
- The platform's disk keeps a 500 MB store warm enough that a miss touches
  the page cache more often than the disk; if not, the cold-read figure in
  the table above grows and the p95 target is re-examined with a measured
  number. [assumption]
- The manifest as JSON parses inside the start-up budget; if it does not,
  a binary framing is a code-generation change with no design consequence.
  [assumption]
