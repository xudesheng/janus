# Derived Context Baseline Design

Status: design for the `derived-context-baseline` topic.

This document defines the Milestone 5B derived context slice: anomaly windows,
log patterns, timelines, related anomalies, and window comparisons. It is
grounded in [`what_and_why.md`](what_and_why.md),
[`roadmap.md`](roadmap.md), [`hot-context-store.md`](hot-context-store.md),
[`entity-resolver-confidence.md`](entity-resolver-confidence.md),
[`fixture-otel-simulator.md`](fixture-otel-simulator.md), and
[`otel-ingest-prototype.md`](otel-ingest-prototype.md). If this document
conflicts with `what_and_why.md`, the canonical design doc wins and this
document should be corrected.

## Why This Topic Is Next

The simulator and OTel JSON ingest topics are complete. Janus can now accept
fixture-shaped replay streams and local OTLP JSON through the same hot-store
ingest boundary.

Milestone 5A is also complete. Janus can derive source-backed entities and
relationships with confidence, alternatives, unresolved states, and relationship
evidence.

The next missing layer is Milestone 5B: derived operational context over those
entities. Jumping directly to `evidence-compiler-ranking` would force the
compiler to work from raw metrics, logs, and changes without stable anomaly,
pattern, timeline, and comparison objects. That would make the compiler too
large and blur the boundary between "derive facts" and "rank evidence".

`derived-context-baseline` should therefore come next. It turns source records
and entity context into the first reusable facts that later evidence selection
can rank.

## Purpose

This topic should produce a small, fixture-backed derived context pipeline:

```text
HotContextStore raw records
      -> entity and relationship context
      -> anomaly windows
      -> log patterns
      -> timeline events
      -> related anomalies
      -> window comparisons
      -> hot-store derived records and fixture comparison
```

The output should answer these questions without producing final root-cause
rankings:

- which metric series changed enough to become bounded anomaly windows;
- which logs collapse into useful patterns with representative exemplars;
- what happened in what order across changes, symptoms, propagation, recovery,
  triggers, amplification, and data gaps;
- which anomaly windows are related by topology, timing, or shared entities;
- how a healthy window differs from an anomalous window.

This topic creates context objects. It does not decide the final explanation.

## Design Review Gate

No Rust implementation should start for this topic until every active reviewer
has agreed on the design direction in their `Direction Verdict`.

Reviewers should explicitly decide whether this topic is the right boundary for
Milestone 5B and whether the comparison contract below is strict enough. If a
reviewer wants implementation in phases, their verdict must name the approved
phase.

The first review round should resolve these design decisions before any coding:

- whether Milestone 5B should be approved as one whole implementation topic or
  only phase by phase;
- whether `non-causal-change` belongs in the Milestone 5B timeline output, or
  should be held until the evidence compiler can classify nearby changes;
- whether the derived provenance contract below is strong enough even where
  current fixture gold shapes do not expose explicit `source_refs`;
- whether stable natural-language timeline text is acceptable for the current
  corpus, or whether the timeline payload should become more structured before
  implementation.

## Scope

In scope:

- anomaly window derivation or import from fixture-shaped metric series;
- log and error pattern clustering with source exemplars;
- timeline generation for fixture-supported markers;
- related anomaly derivation for `find_related_anomalies`;
- window comparison derivation for `compare_windows`;
- derived record insertion or exposure through the hot-store boundary;
- comparison against fixture gold artifacts;
- tests over the current fixture corpus.

Out of scope:

- final evidence item generation;
- suspected-cause ranking;
- causal or non-causal classification beyond preserving source-backed timeline
  markers already represented in fixture gold;
- MCP or external API surfaces;
- durable persistence;
- new OTel ingest protocols;
- production anomaly detection algorithms;
- machine-learned log clustering;
- dashboard or UI features.

## Inputs

The pipeline should consume the same source substrate as Milestone 5A:

- raw records from `HotContextStore`;
- derived entities and relationships from `entity-resolver-confidence`;
- telemetry gaps already represented in fixture inputs;
- fixture scenario time windows when tests need a bounded incident window.

The derived context pipeline must not copy current fixture gold artifacts as its
answer. Gold `anomaly_windows`, `log_patterns`, `timeline`,
`related_anomalies`, and `window_comparison` records are comparison targets only.

If implementation uses `HotContextStore::load_fixture_case` in tests, it must
filter resolver input to raw source records plus explicitly inserted derived
entity context. It must not use expected derived artifacts as inputs.

## Output Model

The output model should serialize close to the fixture shapes:

```rust
struct DerivedAnomalyWindow {
    id: String,
    entity: String,
    signal: String,
    start: Option<String>,
    end: Option<String>,
    baseline: Option<f64>,
    peak: Option<f64>,
    trough: Option<f64>,
    peak_observed: Option<f64>,
    detector_confidence: f64,
    note: Option<String>,
}

struct DerivedLogPattern {
    id: String,
    template: String,
    entity: String,
    severity: String,
    first_seen: String,
    last_seen: String,
    count: usize,
    exemplars: Vec<String>,
    stability: String,
}

struct DerivedTimelineEvent {
    t: String,
    marker: TimelineMarker,
    entity: String,
    text: String,
    source_ref: String,
}
```

Exact Rust names are flexible. The required contract is:

- every derived object is source-backed;
- ids are deterministic for the same fixture input;
- time ordering is stable;
- source refs resolve through existing reference rules;
- confidence fields describe derivation quality, not causal confidence;
- missing data reduces confidence or appears as a data-gap timeline marker.

Suggested additional models:

```rust
struct DerivedRelatedAnomalies {
    seed: String,
    related: Vec<RelatedAnomaly>,
}

struct WindowComparison {
    healthy: TimeWindow,
    anomalous: TimeWindow,
    deltas: Vec<WindowDelta>,
}
```

### Provenance Contract

Fixture gold shapes are the comparison target, but they are not always the full
runtime shape Janus needs. The implementation may therefore carry additional
provenance fields in derived outputs or store envelopes, as long as serialized
comparison can still match the fixture artifacts.

Minimum provenance expectations:

- anomaly windows point back to the metric series and any telemetry gap records
  that affected the window or confidence;
- log patterns point back to representative log exemplar ids, and those
  exemplar ids resolve through the hot store;
- timeline events carry one scalar `source_ref` that resolves to the source or
  derived artifact represented by the event;
- the scalar timeline `source_ref` is the fixture-compatible projection, not
  necessarily the full runtime provenance set;
- related anomalies point back to the seed anomaly, related anomaly windows,
  and relationship or prior-incident refs when those inputs explain the
  relation label;
- window comparisons point back to the compared metric series and selected
  healthy/anomalous windows.

If a fixture gold artifact lacks a provenance field, comparison should still
verify provenance on the derived runtime object before projecting it into the
fixture-compatible shape. A derived object without inspectable provenance is not
acceptable just because the current gold JSON can be matched without it.

## Anomaly Windows

The first anomaly implementation should be deterministic and explainable. It
does not need production-grade statistics.

Input:

- metric-series records from the hot store;
- entity context and relationships;
- optional scenario time window from the fixture manifest;
- telemetry gaps.

Minimum behavior:

- compute a baseline from pre-incident or earliest stable points when present;
- identify contiguous points that exceed a simple relative or absolute change
  threshold;
- produce a bounded `start` and `end` from observed points;
- preserve `baseline`, `peak`, `trough`, or `peak_observed` depending on the
  fixture shape;
- assign a deterministic confidence score;
- lower confidence and add a note when telemetry gaps overlap the likely peak.

The implementation may use fixture-aware thresholds for the first slice, but
those thresholds must be named in code and tests. Hidden ad hoc thresholds are
not acceptable.

Stable id assignment:

- sort by first anomalous time, entity, signal, then source key;
- assign `aw-1`, `aw-2`, and so on;
- keep ordering deterministic across platforms.

Comparison should require expected anomaly windows by entity, signal, time
window, and detector confidence tolerance. Exact ids should match once ordering
is pinned.

## Log Patterns

The log patterner should reduce noisy raw logs into source-backed pattern
objects without using an LLM.

Input:

- log records from the hot store;
- entity context;
- optional trace ids or span ids from log attributes;
- scenario time window.

Minimum behavior:

- group logs by entity, severity, and normalized template;
- normalize obvious variable values such as integers, long ids, durations, and
  quoted request ids;
- keep first seen, last seen, count, and exemplar ids;
- classify simple stability values such as `new-since-incident`,
  `transient-trigger`, and `overload-symptom` only when deterministic signals
  support them;
- preserve representative exemplars instead of replacing raw logs.

The patterner should not summarize log meaning into root-cause claims. It
should produce compact, inspectable pattern objects for later evidence
generation.

Stable id assignment:

- sort by first seen, entity, severity, template;
- assign `lp-1`, `lp-2`, and so on.

Comparison should check template, entity, severity, first and last seen, count,
stability, and exemplar refs.

## Timeline Builder

The timeline builder should order already-derived and source-backed events. It
should not rank causes.

Supported marker values should match the fixture validator:

- `change`;
- `symptom`;
- `propagation`;
- `recovery`;
- `trigger`;
- `amplification`;
- `non-causal-change`;
- `data-gap`.

Input event candidates:

- change records;
- anomaly windows;
- log patterns and notable log exemplars;
- trace exemplars;
- telemetry gaps;
- recovery points when metrics return to baseline;
- related anomaly propagation markers.

Timeline rules:

- sort by timestamp, marker priority, entity id, and source ref;
- use the earliest relevant time for each artifact;
- avoid duplicate events that say the same thing about the same source ref;
- keep text deterministic and compact;
- preserve `source_ref` as a scalar ref.

Boundary:

- `non-causal-change` may be emitted only by a named
  `timeline_non_causal_after_onset_rule`;
- that rule may mark a change as `non-causal-change` only when the change
  timestamp is strictly after the earliest derived symptom or anomaly onset for
  the active incident, and the changed entity is not already on the derived
  symptom or propagation path at that time;
- if the active incident onset or path cannot be established from source-backed
  derived context, the builder must emit an ordinary `change` marker instead of
  guessing `non-causal-change`;
- the timeline must not produce suspected-cause ranks or final causal labels;
- final classification of nearby changes belongs to the evidence compiler.

Comparison should check marker, entity, time, source ref, and stable text for
the current corpus. Timeline text comparison should normalize insignificant
whitespace and treat text as secondary to marker, entity, time, and source ref.
If stable natural text proves too brittle, reviewers should approve a
structured timeline payload before implementation broadens.

## Related Anomalies

`find_related_anomalies` needs a concrete derived-context home before it becomes
an API surface.

Input:

- anomaly windows;
- entity relationships;
- time windows;
- prior incidents when fixtures provide them;
- telemetry gaps as negative or uncertainty signals.

Minimum behavior:

- select a seed anomaly by id;
- find related windows on downstream or upstream entities using relationships;
- find windows on the same entity or sibling metric series with overlapping
  time;
- compute lag seconds from seed start to related start;
- preserve a simple relation label such as `downstream-dependency`,
  `amplifier`, `load-amplification`, or `same-signature`;
- preserve optional prior incident references when fixture data supports it.

This is relationship-aware retrieval, not causality ranking. A related anomaly
can support, weaken, or remain neutral later; this topic only makes it
available.

Comparison should check seed, related window ids, relation labels, lag seconds
where present, and referenced prior incidents.

## Window Comparison

`compare_windows` should compare a healthy window and an anomalous window over
the same entities and signals.

Input:

- metric-series records;
- selected healthy and anomalous windows;
- entity context;
- anomaly windows.

Minimum behavior:

- choose or accept a healthy window and anomalous window from the fixture
  scenario;
- compute `from`, `to`, and `factor` per entity and signal;
- represent flat counter-evidence explicitly with small factors and notes;
- allow `factor: null` when the baseline is zero;
- include aggregate or sibling-series comparisons when fixture input provides
  those series.

The comparison output is a diagnostic contrast. It should not state which entity
is the root cause.

Comparison should check healthy/anomalous bounds and the expected set of deltas
by entity and signal, with numeric tolerance for values and factors.

## Store Integration

Derived artifacts should be inserted into or exposed through the hot-store
derived-record boundary:

- `StoredRecordKind::AnomalyWindow`;
- `StoredRecordKind::LogPattern`;
- `StoredRecordKind::TimelineEvent`;
- `StoredRecordKind::RelatedAnomaly`;
- `StoredRecordKind::WindowComparison`.

Anomaly windows and log patterns already have source-ref categories. They must
resolve through `SourceSignal::AnomalyWindow` and `SourceSignal::LogPattern`.

Timeline events, related anomalies, and window comparisons currently do not map
to first-class `SourceSignal` variants. They still should be inspectable through
store records or comparison helpers so future API surfaces can reuse the same
payloads.

Derived records should not become raw resolver inputs. The existing
`raw_source_records()` separation from Milestone 5A must remain true.

## Fixture Comparison Contract

Add a comparison helper, for example:

```rust
compare_derived_context(case, derived) -> DerivedContextComparison
```

It should report:

- missing anomaly windows;
- anomaly time, baseline, peak, trough, confidence, and note mismatches;
- missing log patterns;
- log template, count, stability, and exemplar mismatches;
- missing timeline events;
- timeline marker, order, entity, source ref, and text mismatches;
- missing related anomalies;
- related anomaly seed, relation, lag, and prior-incident mismatches;
- missing window comparisons;
- window bounds and delta mismatches;
- extra derived objects that are source-backed but not in gold.

Gold fixture artifacts should be treated as the required subset. Extra derived
objects are allowed only when they are deterministic, source-backed, and do not
contradict current gold. The comparison report must make extras visible so they
can be reviewed.

When a runtime object carries richer provenance than the fixture shape, the
comparison should validate the runtime provenance first and then project the
object into the fixture-compatible shape. For example, a timeline event may
carry multiple provenance refs internally while matching one scalar
`source_ref` in gold.

The current corpus has mixed coverage:

- all fixtures declare `anomaly-windows`;
- only some fixtures declare `log-pattern-clustering`;
- most fixtures declare `build_timeline`;
- a smaller set declares `find_related_anomalies`;
- a smaller set declares `compare_windows`.

The comparison tests should follow those capability tags instead of requiring
every artifact type from every fixture.

## Suggested Implementation Slices After Design Approval

No slice should start until reviewers agree on the design direction, or
explicitly approve that slice in their `Direction Verdict`.

Recommended slices:

1. Data model and comparison shell: define derived context output types,
   comparison structs, store insertion helpers, and deserialization of gold
   artifacts.
2. Anomaly windows and window comparison: derive metric windows, compare deltas,
   handle missing data and flat counter-evidence.
3. Log patterns: add deterministic template grouping, exemplar preservation,
   stability labels, and fixture comparison.
4. Timeline builder: produce source-backed ordered timeline events from changes,
   anomalies, logs, traces, recovery, amplification, and data gaps.
5. Related anomalies: connect anomaly windows through relationships and time,
   compare relation labels and lag.
6. Final integration: insert derived records into the hot store, prove source
   refs resolve where applicable, and run full-corpus comparison.

Slice 1 should land and pass before the generator slices produce anomaly,
pattern, timeline, related-anomaly, or comparison artifacts. The comparison
shell is the guardrail against drifting fixture gold output.

These are implementation slices only. The topic is complete only when the
Definition Of Done below is met or reviewers explicitly narrow the milestone.

## Tests

Add tests that prove:

- every fixture with `anomaly-windows` derives required anomaly windows;
- every fixture with `log-pattern-clustering` derives required log patterns;
- every fixture with `build_timeline` derives required timeline events;
- every fixture with `find_related_anomalies` derives required related anomaly
  output;
- every fixture with `compare_windows` derives required window comparison
  output;
- anomaly and log-pattern source refs resolve through the hot store;
- derived records do not become raw source records;
- missing-data fixtures lower confidence or produce data-gap markers;
- false-causality trap fixtures preserve counter-evidence context without
  producing final root-cause ranks;
- the `timeline_non_causal_after_onset_rule` has both a positive test for the
  current coincidental-change fixture and a negative test proving the timeline
  builder does not over-label nearby changes as non-causal;
- `entity-resolver-confidence` tests continue to pass.

Existing verification should continue to pass:

```bash
cargo fmt
cargo test
cargo clippy --all-targets --all-features
cargo run --bin validate_fixtures
```

## Definition Of Done

This topic is complete when:

- metric anomaly windows are derived or imported from source records with
  bounded time windows, source refs, and detector confidence;
- log patterns preserve templates, counts, first/last seen times, stability, and
  exemplars;
- timeline output is deterministic, source-backed, and ordered;
- `related_anomalies` and `window_comparison` fixture artifacts have concrete
  derived output and comparison tests;
- derived anomaly and log-pattern records resolve through the hot-store
  reference boundary;
- derived timeline, related-anomaly, and window-comparison records are
  inspectable through a reviewed store or comparison path;
- current fixture gold artifacts are compared according to capability tags;
- the topic does not introduce evidence ranking, suspected-cause scoring, MCP,
  persistence, dashboard features, or new ingest protocols;
- `cargo fmt`, `cargo test`, `cargo clippy --all-targets --all-features`, and
  `cargo run --bin validate_fixtures` pass.

## Review Focus

Reviewers should focus on:

1. Whether `derived-context-baseline` is the right topic after Milestone 5A,
   given that simulator and OTel JSON ingest are already complete.
2. Whether anomaly, log-pattern, timeline, related-anomaly, and window-comparison
   responsibilities are split cleanly from evidence ranking.
3. Whether the simple deterministic algorithms are acceptable for the fixture
   corpus without pretending to be production-grade detection.
4. Whether timeline markers avoid premature causality while preserving useful
   operational order.
5. Whether source refs and hot-store derived records are strong enough for the
   future evidence compiler.
6. Whether the fixture comparison contract is strict enough to prevent drifting
   gold artifacts.
