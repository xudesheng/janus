# Fixture OTel Simulator Design

Status: design for the `fixture-otel-simulator` topic.

This document defines a small simulator and replay adapter for fixture-owned
OTel-shaped telemetry. It is grounded in [`what_and_why.md`](what_and_why.md),
[`roadmap.md`](roadmap.md), [`hot-context-store.md`](hot-context-store.md), and
the fixture scheme in [`../process/fixtures.md`](../process/fixtures.md). If
this document conflicts with `what_and_why.md`, the canonical design doc wins
and this document should be corrected.

## Why This Topic Is Next

The strict roadmap topic after the hot context store is
`entity-resolver-confidence`. That is still the right next derived-context topic.
But there is also an explicit demo pressure: Janus should soon be able to run
locally and accept telemetry-like input.

`fixture-otel-simulator` is the smallest safe step toward that demo. It should
not jump to real OTLP ingest yet. Instead, it should replay Janus-owned fixture
inputs as deterministic OTel-shaped events into the hot store. That gives us a
repeatable "Janus accepts telemetry" path without mixing protocol decoding,
network serving, persistence, derivation, and evidence ranking in one topic.

Real OTLP ingest remains a later topic. The simulator should create the adapter
boundary that later OTLP ingest can reuse.

## Purpose

The simulator should turn a fixture scenario into a deterministic event stream:

```text
fixtures/scenarios/<id>/input.json
      -> replay planner
      -> simulated OTel-shaped events
      -> hot-store ingest sink
      -> source-ref resolver / query-context checks
      -> demo output
```

The output should prove that the same hot-store source refs used by Evidence IR
can be produced by an ingest-like path, not only by the current direct fixture
loader.

## Design Review Gate

No Rust implementation should start for this topic until every active reviewer
has agreed on the design direction in their `Direction Verdict`.

This is a demo-enabling adapter topic, not a replacement for Milestone 5 derived
context. If reviewers decide that the project should return immediately to
derived context, they should say so in the direction verdict and this topic can
stop as a design-only branch.

## Scope

In scope:

- deterministic replay planning from fixture `input.json`;
- simulated events for resources, traces, spans, metric points, logs, change
  events, prior incidents, and telemetry gaps;
- a hot-store ingest sink or equivalent API that accepts replay events without
  bypassing `HotContextStore`;
- final-store source-ref resolution checks against current fixture gold
  evidence bundles;
- step or dry-run mode that shows event order without needing wall-clock sleeps;
- a small CLI for local demos;
- tests proving replay is deterministic and source refs become resolvable
  through the simulated ingest path.

Out of scope:

- OTLP protobuf decoding;
- HTTP or gRPC receivers;
- OpenTelemetry SDK or Collector integration;
- durable persistence;
- entity resolution algorithms;
- anomaly detection, log clustering, timeline generation, or evidence ranking;
- MCP or external API surfaces;
- replacing fixture gold bundles with generated evidence.

## Relationship To Real OTLP Ingest

The simulator should define a reusable internal boundary:

```text
FixtureReplaySource -> HotContextIngestSink
OtlpReceiverSource  -> HotContextIngestSink  # later topic
```

The future `otel-ingest-prototype` topic should be able to replace
`FixtureReplaySource` with an OTLP receiver while keeping the same normalized
hot-store write model and source-ref semantics.

The source-specific adapter owns source-key derivation. `FixtureReplaySource`
uses fixture conventions such as `t-0001`, `t-0001/s-3`, and `name@entity`; a
future `OtlpReceiverSource` must normalize real OTLP trace ids, span ids, metric
identity, resource attributes, and entity hints into the same hot-store write
model without assuming fixture ids.

This topic should not add `opentelemetry-proto`, a network listener, or Collector
configuration. Those belong to real ingest. The simulator is allowed to be
"OTel-shaped" rather than byte-exact OTLP, matching the fixture contract.

## Incremental Replay Requirement

The current hot store already has an all-at-once fixture loader. This topic
should not make the simulator a thin wrapper around that loader. The simulator
must build a replay plan and apply events to a fresh `HotContextStore` through an
incremental ingest boundary.

It is acceptable to use `HotContextStore::load_fixture_case` as a test oracle for
full-replay source-ref compatibility, but not as the main replay implementation.
Full replay must preserve source-key and source-ref resolution semantics; it does
not require byte-for-byte `StoredRecord` payload equality with batch fixture
loading except where this document explicitly pins a stored shape. Partial replay
is a required behavior: before an event is ingested, source refs owned by that
event should be missing; after the event is ingested, the same refs should be
resolvable.

This distinction is what makes the topic useful for later real ingest. A future
OTLP receiver should be able to feed the same normalized ingest boundary without
going through fixture-only batch loading.

## Event Model

Introduce a replay event type with enough structure for deterministic tests and
demo output:

```rust
struct SimulationEvent {
    scenario_id: String,
    sequence: u64,
    simulated_time: Option<String>,
    signal: SimulatedSignal,
    source_key: String,
    record_kind: StoredRecordKind,
    payload: serde_json::Value,
}
```

Suggested `SimulatedSignal` values:

- resource;
- trace;
- span;
- metric_point;
- log;
- change;
- prior_incident;
- telemetry_gap.

The event should carry the original fixture-shaped payload or the smallest
self-contained fragment needed to reconstruct the hot-store record. Do not
invent source ids that differ from fixture conventions.

## Replay Ordering

Replay order should be deterministic:

1. preload records without event time, such as resources, before timed records;
2. order timed records by normalized UTC fixture timestamp keys;
3. use fixture file order as the stable tie-breaker;
4. use event `sequence` as the final tie-breaker.

The current fixtures use UTC RFC3339-like strings with a `Z` suffix and optional
fractional seconds. Replay planning should normalize those forms for ordering
while preserving the original timestamp string on the emitted event. If a future
fixture needs offsets or one-sided windows, that should be a separate timestamp
normalization topic.

Tests should assert that produced plans are monotonic when timestamps are parsed
for the current fixture corpus. Before adding time-slicing features such as
`--until`, any broader timestamp formats should be normalized or rejected rather
than relying on accidental lexical order.

Suggested event timing:

- resources: no simulated time, emitted before timed records;
- trace records: trace start time from the earliest span;
- span records: span start time;
- metric points: point timestamp `t`;
- logs: `t`;
- changes: `t`;
- prior incidents: preload, or `first_seen` if the simulator is running a
  warm-context scenario;
- telemetry gaps: gap `start`, carrying the full gap payload.

For this topic, timed signal events are invalid if the required timestamp field
is missing. Prior incidents are the exception in the current corpus because they
may represent warm-context memory instead of a replay-time signal.

## Metric Handling

Fixtures model metrics as series, but a simulator should expose metric points as
events. The hot-store ingest sink should accumulate points into the existing
metric-series source key:

```text
metric point event:
  name=http.server.error_rate
  entity=service:checkout
  t=...
  v=...

stored source key:
  http.server.error_rate@service:checkout
```

The stored metric-series record should contain only observed points up to the
current replay step. After replay completes, it should preserve the same source
key that Evidence IR uses today.

The accumulated metric-series payload should converge to the fixture metric
shape after full replay:

- preserve `name`, `entity`, `unit`, and other non-point fields from the fixture
  metric record;
- append point payloads in replay order, which should match fixture point order
  for the current fixtures;
- keep only observed points before full replay and all points after full replay;
- never drop or synthesize metric points.

This is the main reason the topic needs an ingest sink instead of only calling
`HotContextStore::load_fixture_case`.

## Trace And Span Handling

Trace and span replay should follow the same incremental principle as metrics:
records become resolvable as the relevant simulated event is ingested, not when
the fixture file is first parsed.

The stored source keys remain compatible with the current hot-store convention:

```text
trace record:
  key=t-0001

span record:
  key=t-0001/s-3
```

The design should preserve these rules:

- the `Trace` event makes the trace source key resolvable and should be emitted
  when the trace first has observable data, normally the earliest span time;
- if a trace has no span start time, including an empty `spans` array, the
  trace event uses a trace-level `start` field when present; otherwise it is an
  untimed preload event;
- a span source ref does not resolve before that span's event is ingested;
- full replay preserves the same trace and span lookup semantics as current
  fixture loading;
- trace aggregation can be simple and fixture-shaped for this topic, as long as
  it does not hide partial-replay behavior.

## Hot-Store Ingest Boundary

Add a small API around `HotContextStore` rather than making the simulator edit
store internals directly. Exact names are flexible, but the shape should be
close to:

```rust
pub enum HotIngestEvent {
    Resource(serde_json::Value),
    Trace(serde_json::Value),
    Span { trace_id: String, payload: serde_json::Value },
    MetricPoint { series: MetricSeriesKey, payload: serde_json::Value },
    Log(serde_json::Value),
    Change(serde_json::Value),
    PriorIncident(serde_json::Value),
    TelemetryGap(serde_json::Value),
}

impl HotContextStore {
    pub fn ingest(&mut self, event: HotIngestEvent) -> Result<IngestOutcome, HotStoreError>;
}
```

Required behavior:

- inserted records use the same `SourceKey` and `StoredRecordKind` conventions
  as direct fixture loading;
- metric points update their metric-series record instead of conflicting on
  duplicate primary keys;
- metric series is the only merge-eligible `(StoredRecordKind, SourceKey)` pair;
- trace and span events preserve trace and span aliases;
- duplicate primary keys with different payloads remain errors for all non-metric
  record kinds;
- source-ref resolution outcomes stay unchanged.

It is acceptable for `HotIngestEvent` to be internal to this topic at first, as
long as the future OTLP ingest topic can reuse or promote it.

## Query And Demo Behavior

The current `get_evidence_bundle` path still returns fixture gold bundles. This
topic should not change that into generated evidence.

Instead, add a reusable validation helper if needed:

```rust
validate_bundle_against_store(query, bundle, store)
```

The simulator CLI can then:

1. load a fixture;
2. replay the input into a fresh hot store;
3. load the fixture gold evidence bundle;
4. validate that every returned source ref resolves against the simulated store;
5. run the existing query-context checks;
6. print a compact demo report.

The returned bundle should remain unchanged. The simulator proves source-backed
ingest plumbing, not evidence compilation.

## CLI

Add a small binary, for example:

```bash
cargo run --bin simulate_fixture -- --fixture deploy-bad-rollout
cargo run --bin simulate_fixture -- --fixture deploy-bad-rollout --dry-run
cargo run --bin simulate_fixture -- --fixture deploy-bad-rollout --jsonl
```

Minimum useful behavior:

- `--fixture <id>` selects one registered fixture;
- default mode replays all events into a hot store and prints a summary:
  events emitted, records stored, source refs resolved, errors;
- `--dry-run` prints event order without mutating the store;
- `--jsonl` prints one event per line as JSON for inspection or later scripts.

`--dry-run` and `--jsonl` are mutually exclusive render modes.

Optional, useful if small:

- `--until <timestamp>` replays only through a simulated time;
- `--ref <source-ref>` resolves one ref after replay;
- `--query` runs the fixture gold query-context validation after replay.

Do not add wall-clock sleeps in the minimum implementation. A deterministic
logical clock is easier to test and more useful for review.

## Suggested Implementation Slices After Design Approval

No slice should start until reviewers agree on the design direction, or
explicitly approve that slice in their `Direction Verdict`.

If reviewers accept phase-by-phase implementation, the recommended slices are:

1. Replay planning and dry-run output: deterministic event extraction, ordering,
   sequence assignment, and event rendering without mutating a store.
2. Hot-store ingest boundary: `HotIngestEvent`, incremental record insertion or
   update behavior, metric-point accumulation, trace/span availability, and
   partial-replay source-ref tests.
3. Demo and validation surface: CLI summary, JSONL mode, bundle source-ref
   validation after replay, and a smoke test.

These are implementation slices only. The topic's Definition Of Done remains the
full simulator contract below.

## Tests

Add tests that prove the simulator is a real adapter, not only a wrapper around
the old fixture loader:

- every registered fixture can produce a non-empty replay plan;
- replay plans are deterministic across runs;
- dry-run event order is stable;
- resources are emitted before timed records;
- metric points accumulate into metric-series records with the existing source
  keys;
- after full replay, every current fixture Evidence IR source ref resolves
  against the simulated store;
- a partial replay before a known event leaves that event's source ref missing;
- replaying through that event makes the same source ref resolvable;
- duplicate conflicting fixture ids or source keys fail with structured errors;
- CLI smoke test for one fixture succeeds without requiring network access.

The partial replay tests are important. They prove this is a stream simulator,
not just another all-at-once fixture loader.

Implementation slices may prove raw source refs before the final demo validation
slice. Derived evidence refs, such as anomaly windows and log patterns, still
need the final validation path to prove the full Definition Of Done.

## Definition Of Done

This topic is complete when:

- a registered fixture can be replayed into `HotContextStore` through an
  ingest-like adapter;
- metric points are represented as replay events and end up under the expected
  metric-series source refs;
- full replay resolves all current fixture evidence source refs through the hot
  store;
- partial replay can show a source ref becoming available over simulated time;
- a local CLI can run a fixture simulation and print a deterministic summary;
- no OTLP protobuf, network receiver, persistence, derivation, ranking, or MCP
  surface is introduced;
- `cargo fmt`, `cargo test`, `cargo clippy --all-targets --all-features`, and
  `cargo run --bin validate_fixtures` pass.

## Review Focus

Reviewers should focus on:

1. Whether this topic is the right demo bridge before real OTLP ingest.
2. Whether the hot-store ingest boundary is reusable by a future OTLP receiver.
3. Whether metric-point replay and partial replay semantics are strong enough to
   make this a simulator rather than a second direct fixture loader.
4. Whether the topic stays small by excluding derivation, ranking, persistence,
   MCP, and network ingest.
