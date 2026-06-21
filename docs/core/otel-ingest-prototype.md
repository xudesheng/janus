# OTel Ingest Prototype Design

Status: design for the `otel-ingest-prototype` topic.

This document defines a small OTLP/Collector-shaped ingest prototype for Janus.
It is grounded in [`what_and_why.md`](what_and_why.md),
[`roadmap.md`](roadmap.md), [`hot-context-store.md`](hot-context-store.md),
and [`fixture-otel-simulator.md`](fixture-otel-simulator.md). If this document
conflicts with `what_and_why.md`, the canonical design doc wins and this
document should be corrected.

External protocol reference:

- OpenTelemetry OTLP specification:
  <https://opentelemetry.io/docs/specs/otlp/>

## Why This Topic Is Next

The strict near-term roadmap still has important derived-context work:
`entity-resolver-confidence` and then the evidence compiler path. But the
project also has a clear demo pressure: Janus should soon be able to run locally
and accept OpenTelemetry-shaped input.

`fixture-otel-simulator` already created the reusable ingest boundary. It
proved that fixture-owned telemetry-like events can be replayed into
`HotContextStore` through `HotIngestEvent` instead of bypassing the store. The
next smallest demo step is therefore not a broad production receiver. It is an
OTLP/Collector-shaped adapter that maps real-looking OTel payloads into that
same hot-store ingest model while preserving source references.

This topic is a narrow Milestone 9 preview. It should enable a local demo and
validate the ingest boundary, without turning Janus into a production APM
backend yet.

## Purpose

The prototype should let Janus ingest a small local telemetry stream:

```text
OTLP JSON or Collector-exported JSON
      -> OTel ingest adapter
      -> normalized HotIngestEvent values
      -> HotContextStore
      -> source-ref resolver and query-context checks
      -> demo output
```

The main proof is simple: Janus can accept OTel-shaped traces, metrics, logs,
and resources, assign stable source refs, and resolve those refs through the
same store boundary used by fixture simulation.

## Design Review Gate

No Rust implementation should start for this topic until every active reviewer
has agreed on the design direction in their `Direction Verdict`.

Reviewers should explicitly decide whether this JSON-first ingest slice is the
right demo bridge now, or whether the project should return immediately to
derived context. If the answer is "return to derived context", this topic can
stop as a design-only branch.

## Scope

In scope:

- local ingest of OTLP JSON or Collector-exported JSON for traces, metrics, and
  logs;
- resource attribute handling sufficient to identify the emitting service and
  preserve the original resource payload;
- normalization into existing `HotIngestEvent` variants;
- deterministic source-key and source-ref generation for traces, spans, metric
  series, metric points, logs, and resources;
- a small CLI that ingests one or more JSON files into a fresh `HotContextStore`
  and prints a compact summary;
- optional OTLP/HTTP JSON receiver only if it stays small and reuses the same
  adapter;
- demo fixture material under a Janus-owned fixture or test-data directory;
- tests proving source refs are stable and resolvable after ingest.

Out of scope:

- production-grade OTLP/gRPC;
- binary protobuf decoding as the first slice;
- profiles;
- durable persistence;
- high-throughput batching, backpressure, retry, or queueing behavior;
- full OpenTelemetry semantic convention coverage;
- dashboard views;
- entity resolution confidence, relationship derivation, anomaly detection, log
  clustering, timeline generation, evidence ranking, or MCP tools;
- deploy/config/feature-flag change-event ingest, except for preserving OTel log
  records that happen to mention changes.

## Input Boundary

The first implementation should prefer a file or stdin boundary:

```bash
cargo run --bin ingest_otlp -- --input fixtures/otel/deploy-bad-rollout.otlp.json
cargo run --bin ingest_otlp -- --input fixtures/otel/*.json --summary
```

The input format should be OTLP JSON-compatible enough to exercise the same
envelopes used by the official OTLP/HTTP JSON encoding:

- trace payloads shaped like `ExportTraceServiceRequest`;
- metric payloads shaped like `ExportMetricsServiceRequest`;
- log payloads shaped like `ExportLogsServiceRequest`;
- lowerCamelCase OTLP field names;
- hex `traceId` and `spanId` strings for trace context.

For this topic, Janus does not need to become a complete OTLP validator. Unknown
fields should be preserved where practical and ignored for normalization.
Malformed records that cannot produce a stable source key should fail with
structured errors and record counts.

Optional HTTP mode, if reviewers approve it and implementation cost stays small:

```bash
cargo run --bin ingest_otlp -- --listen 127.0.0.1:4318
```

The optional receiver should accept JSON requests on these OTLP/HTTP paths:

- `/v1/traces`;
- `/v1/metrics`;
- `/v1/logs`.

It should not implement binary protobuf, gRPC, TLS, auth, gzip, retry semantics,
or production flow control in this topic.

## Relationship To The Fixture Simulator

The simulator and this topic should share the same store boundary:

```text
FixtureReplaySource  -> HotIngestEvent -> HotContextStore
OtlpJsonIngestSource -> HotIngestEvent -> HotContextStore
```

`FixtureReplaySource` owns fixture source-key conventions such as `t-0001`,
`t-0001/s-3`, and `name@entity`. The OTel adapter owns OTLP source-key
derivation from trace ids, span ids, resource attributes, metric identity, and
log record fields.

The store should not know whether an event came from a fixture or OTLP JSON
except through provenance metadata on payloads or source refs. That keeps future
OTLP/HTTP, OTLP/gRPC, and Collector integration paths replaceable.

## Normalization Model

The adapter should produce the smallest normalized events that the current hot
store can accept:

```rust
HotIngestEvent::Resource(resource_payload)
HotIngestEvent::Trace(trace_payload)
HotIngestEvent::Span { trace_id, payload: span_payload }
HotIngestEvent::MetricPoint { series, payload: point_payload }
HotIngestEvent::Log(log_payload)
```

If the current `HotIngestEvent` shape needs a small extension for provenance,
prefer adding optional fields to the stored payload over widening every enum
variant. The event boundary should remain simple enough that the simulator can
continue using it.

The normalized payloads should preserve:

- source signal kind;
- original OTLP envelope path where the record was found;
- resource attributes relevant to entity hints;
- instrumentation scope name and version when present;
- timestamps used by the hot store;
- original attributes that may later matter for evidence or entity resolution.

## Source Keys And Source Refs

Stable source refs are the central acceptance criterion for this topic.

Suggested source-key rules:

- resource: `resource:<service.name>@<service.instance.id>` when both exist;
- resource fallback: `resource:<service.name>` when only `service.name` exists;
- trace: lowercase hex `traceId`;
- span: `<traceId>/<spanId>` using lowercase hex ids;
- metric series: `<metric.name>@<entity>`;
- log: prefer an explicit `janus.log.id` attribute, otherwise generate a
  deterministic key from `traceId`, `spanId`, timestamp, and record sequence.

For resource fallback, if `service.name` is missing, use a deterministic
resource key derived from the resource's stable attributes and report that
entity quality is low. The adapter must not silently collapse unrelated
resources into one anonymous service.

For logs without explicit ids, generated ids should be deterministic for the
same input file and envelope order. They do not need to be globally stable
across arbitrary Collector transformations yet, but the limitation must be
reported in the ingest summary.

Metric points should update a metric-series record in the same way the fixture
simulator does. The source ref should identify the series, while the stored
payload preserves individual points and timestamps.

## Minimal Entity Hints

This topic should not implement `entity-resolver-confidence`. It should only
attach conservative entity hints so the hot store can answer basic selectors.

Minimum rules:

- `service.name` maps to `service:<name>`;
- `service.instance.id` may be preserved as an attribute but should not replace
  the service entity;
- span names, routes, hosts, pods, containers, databases, queues, and peer
  services should be preserved as attributes for later entity resolution;
- when the adapter cannot identify a service entity, it should leave the record
  unresolved and count the missing hint.

Do not invent high-confidence relationships in this topic. Relationship
building belongs to `entity-resolver-confidence` or a later derived-context
topic.

## Error And Partial-Ingest Behavior

The adapter should distinguish these outcomes:

- accepted record count by signal;
- rejected record count by signal;
- missing source-key fields;
- unsupported signal or shape;
- records accepted with low-quality entity hints;
- duplicate source keys;
- malformed timestamp or id values.

For file-mode ingest, any rejected records should make the command exit nonzero
unless a reviewer-approved `--allow-partial` option exists. For optional HTTP
mode, the receiver may return a simple failure response for malformed payloads.
It does not need to implement the full OTLP partial-success response contract in
this topic.

## CLI

Add a small binary, for example:

```bash
cargo run --bin ingest_otlp -- --input fixtures/otel/deploy-bad-rollout.otlp.json
cargo run --bin ingest_otlp -- --input fixtures/otel/deploy-bad-rollout.otlp.json --json-summary
cargo run --bin ingest_otlp -- --input fixtures/otel/deploy-bad-rollout.otlp.json --ref <source-ref>
```

Minimum useful behavior:

- `--input <path>` can be repeated or accept globs if implementation stays
  small;
- default mode ingests into a fresh hot store and prints counts by signal;
- summary includes records inserted, records updated, rejected records, entity
  hint quality, and source refs resolved;
- `--json-summary` prints a stable machine-readable report for tests;
- `--ref <source-ref>` resolves one source ref after ingest.

Optional behavior:

- `--stdin` for pipe-based demos;
- `--listen 127.0.0.1:4318` for OTLP/HTTP JSON demos;
- `--emit-fixture` to write a Janus fixture-shaped `input.json` for debugging.

Do not add a long-running service lifecycle unless the optional HTTP slice is
explicitly approved.

## Test Data

Add a small Janus-owned OTLP JSON sample. It should be synthetic and derived
from an existing fixture scenario such as `deploy-bad-rollout`, not copied from
external systems.

The sample should include:

- one resource with `service.name`;
- one trace with at least two spans;
- one error span or status-like attribute;
- one metric with multiple data points;
- one log record correlated by trace id or span id;
- at least one record with missing or weak entity hints if that is useful for
  testing error reporting.

Keep the sample small. The goal is ingest correctness and source refs, not a
large telemetry corpus.

## Suggested Implementation Slices After Design Approval

No slice should start until reviewers agree on the design direction, or
explicitly approve that slice in their `Direction Verdict`.

Recommended slices:

1. OTLP JSON file parser and normalization: parse trace, metric, log, and
   resource envelopes into internal normalized records without mutating the
   store.
2. Hot-store integration: convert normalized records into `HotIngestEvent`,
   insert them into `HotContextStore`, and prove source refs resolve.
3. CLI and fixture material: add the demo command, stable summary output, and
   synthetic OTLP JSON sample.
4. Optional HTTP JSON receiver: only if the file-mode adapter is complete and
   reviewers agree that the extra surface is still small.

## Tests

Add tests for:

- parsing an OTLP JSON trace payload with resource and scope spans;
- parsing an OTLP JSON metrics payload into metric-point ingest events;
- parsing an OTLP JSON logs payload into log ingest events;
- deterministic lower-case trace and span source keys;
- resource fallback when `service.name` or `service.instance.id` is missing;
- generated log ids are deterministic for the same input;
- metric points accumulate under the expected metric-series source ref;
- every emitted source ref resolves against `HotContextStore` after ingest;
- malformed ids, missing timestamps, and unsupported shapes produce structured
  errors;
- CLI smoke test for one synthetic OTLP JSON sample.

Existing validation should continue to pass:

```bash
cargo fmt
cargo test
cargo clippy --all-targets --all-features
cargo run --bin validate_fixtures
```

## Definition Of Done

This topic is complete when:

- Janus can ingest a small OTLP JSON or Collector-exported JSON sample locally;
- traces, spans, metrics, logs, and resources are normalized into the existing
  hot-store ingest path;
- stable source refs are generated and resolvable after ingest;
- metric points update metric-series records instead of becoming disconnected
  rows;
- entity hints are conservative and unresolved cases are reported;
- a local CLI can run the demo and print a deterministic summary;
- the implementation remains JSON/file-first unless reviewers explicitly approve
  optional HTTP JSON mode;
- no production gRPC/protobuf receiver, persistence layer, derived context,
  ranking, MCP surface, or dashboard feature is introduced;
- `cargo fmt`, `cargo test`, `cargo clippy --all-targets --all-features`, and
  `cargo run --bin validate_fixtures` pass.

## Review Focus

Reviewers should focus on:

1. Whether `otel-ingest-prototype` is the right next topic now that the fixture
   simulator exists, despite the strict derived-context roadmap.
2. Whether JSON/file-first is the right first ingest boundary, or whether the
   topic must include OTLP/HTTP JSON immediately for demo credibility.
3. Whether source-key generation is stable enough for auditability without
   pretending to solve full entity resolution.
4. Whether the adapter properly reuses `HotIngestEvent` and `HotContextStore`
   instead of creating a parallel ingest path.
5. Whether the exclusions are strong enough to keep this topic from absorbing
   production ingest, persistence, derivation, ranking, MCP, or dashboard work.
