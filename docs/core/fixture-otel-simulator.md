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

This topic should not add `opentelemetry-proto`, a network listener, or Collector
configuration. Those belong to real ingest. The simulator is allowed to be
"OTel-shaped" rather than byte-exact OTLP, matching the fixture contract.

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
2. order timed records by comparable fixture timestamp strings;
3. use fixture file order as the stable tie-breaker;
4. use event `sequence` as the final tie-breaker.

The current fixtures use UTC RFC3339-like strings that compare lexicographically.
This topic can keep that convention rather than adding a time library. If a
future fixture needs offsets, fractional formats, or one-sided windows, that
should be a separate timestamp-normalization topic.

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

This is the main reason the topic needs an ingest sink instead of only calling
`HotContextStore::load_fixture_case`.

## Hot-Store Ingest Boundary

Add a small API around `HotContextStore` rather than making the simulator edit
store internals directly. Exact names are flexible, but the shape should be
close to:

```rust
pub enum HotIngestEvent {
    Resource(serde_json::Value),
    Trace(serde_json::Value),
    Span { trace_id: String, payload: serde_json::Value },
    MetricPoint { series: MetricSeriesKey, point: serde_json::Value },
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
- trace and span events preserve trace and span aliases;
- duplicate primary keys with different payloads remain errors;
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

Optional, useful if small:

- `--until <timestamp>` replays only through a simulated time;
- `--ref <source-ref>` resolves one ref after replay;
- `--query` runs the fixture gold query-context validation after replay.

Do not add wall-clock sleeps in the minimum implementation. A deterministic
logical clock is easier to test and more useful for review.

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
