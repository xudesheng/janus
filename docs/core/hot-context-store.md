# Hot Context Store Design

Status: design for the `hot-context-store` topic.

This document defines the Milestone 4 local hot context store. It is grounded in
[`what_and_why.md`](what_and_why.md), [`roadmap.md`](roadmap.md),
[`evidence-ir-schema.md`](evidence-ir-schema.md),
[`get-evidence-bundle-contract.md`](get-evidence-bundle-contract.md), and the
fixture validation design in
[`../process/fixture-validation-harness.md`](../process/fixture-validation-harness.md).
If this document conflicts with `what_and_why.md`, the canonical design doc wins
and this document should be corrected.

## Why This Topic Is Next

The next visible demo goal is to ingest telemetry-like data and answer evidence
queries from Janus. The tempting topic is therefore OTLP ingest or an OTel
simulator. That would be premature without an internal hot-store boundary.

`hot-context-store` should land first because it gives Janus a place to put
recent OTel-shaped records, assign stable source references, resolve those
references back to concrete records, and run basic time/entity filtering. Once
that exists, an OTLP receiver or simulator is only another input adapter into the
same store instead of a one-off demo path.

## Purpose

Milestone 4 should replace pure fixture stubs with a small local recent-window
substrate:

```text
fixture input.json
      -> LocalHotContextStore
      -> source-ref resolver and time/entity selectors
      -> get_evidence_bundle source lookup checks
      -> later simulator / OTLP ingest adapters
```

The goal is not real retrieval or evidence generation yet. The goal is a
concrete, testable store boundary that preserves Janus's most important
invariant: every evidence claim can be traced back to source data or an explicit
missing-data record.

## Design Review Gate

No Rust implementation should start for this topic until every active reviewer
has agreed on the design direction in their `Direction Verdict`.

This topic should finalize the Milestone 4 store contract before coding by
default. Reviewers may explicitly approve phase-by-phase implementation, but
each phase must preserve the same Definition Of Done below.

If reviewers approve phase-by-phase implementation, the recommended slices are:

1. Store envelope, fixture loading, deterministic source keys, aliases, and
   source-reference resolution.
2. Time-window and entity selectors plus store-aware source lookup checks in
   the fixture-backed `get_evidence_bundle` path.
3. Optional inspection CLI and error-report polish.

These are implementation slices only. They do not split the Milestone 4
acceptance criteria, and none may start until reviewers agree on the design
direction for this topic or explicitly approve the corresponding phase.

## Scope

In scope:

- an in-memory local hot store for fixture-shaped resources, traces, metrics,
  logs, change events, prior incidents, and telemetry gaps;
- a load path from validated fixture `input.json` files into the store;
- optional loading of same-fixture expected derived artifacts as reference
  targets for `anomaly_window`, `log_pattern`, entity, relationship, and other
  derived refs;
- stable source-reference keys for raw and derived records;
- source-reference resolution for Evidence IR `SourceRef` values and scalar
  refs used by fixtures;
- time-window and entity selectors over stored records;
- tests proving fixture inputs can be loaded and evidence source refs can be
  dereferenced to concrete records;
- minimal integration with the fixture-backed `get_evidence_bundle` path so the
  query boundary can use the store for source lookup checks.

Out of scope:

- live OTLP ingest over HTTP or gRPC;
- byte-exact OTLP protobuf decoding;
- a simulator that emits telemetry over the network;
- durable persistence beyond process memory;
- entity resolution algorithms;
- anomaly detection, log clustering, timeline generation, or evidence ranking;
- replacing fixture gold bundles with generated evidence;
- MCP or external API surfaces.

## Relationship To Simulator And OTLP Ingest

Simulator and real ingest should not be forgotten; they should depend on this
topic.

After this topic, a simulator can be a narrow adapter:

```text
fixture scenario or scripted incident
      -> simulated OTel-shaped events
      -> HotContextStore insert APIs
      -> Janus source-ref resolver and query checks
```

Later, real OTLP ingest can use the same internal write model:

```text
OTLP / Collector exporter
      -> protocol decoder
      -> normalized hot-store records
      -> same source-ref resolver and query checks
```

This topic should therefore name the ingest boundary clearly, but it should not
implement network ingest. A good follow-up topic after the hot store is either
the roadmap's `entity-resolver-confidence` or, if a demo is the priority, a
narrow `fixture-otel-simulator` topic that replays fixture inputs into the store
without pretending to be production OTLP ingest.

## Store Model

The store should keep raw source records and derived reference targets under one
common envelope:

```rust
struct StoredRecord {
    key: SourceKey,
    kind: StoredRecordKind,
    time_window: Option<TimeWindow>,
    entities: Vec<String>,
    payload: serde_json::Value,
}
```

The exact Rust names are flexible, but the responsibilities are not:

- `SourceKey` is the stable lookup key used by the resolver;
- `StoredRecordKind` identifies the category of record;
- `time_window` enables window overlap filters;
- `entities` enables entity filters;
- `payload` keeps the original fixture-shaped record available for inspection.

The primary store identity is the pair `(StoredRecordKind, SourceKey)`, not the
raw source-key string alone. This lets the store represent same-string refs in
different categories without corrupting either record. Signal-aware resolution
uses the requested `SourceRef.signal` to choose the matching category; scalar
resolution can report ambiguity when a raw ref string maps to more than one
category.

The store should not discard original JSON fields. Keeping `payload` as
`serde_json::Value` is acceptable for this milestone because fixture inputs are
logical OTel-shaped records, not byte-exact OTLP.

Suggested record kinds:

- resource;
- trace;
- span;
- metric_series;
- log;
- change;
- prior_incident;
- telemetry_gap;
- entity;
- relationship;
- anomaly_window;
- log_pattern;
- evidence_item;
- timeline_event;
- suspected_cause;
- next_check;
- entity_context;
- related_anomaly;
- window_comparison.

The raw input kinds are required. The derived kinds are loaded only from
same-fixture `expected.json` artifacts in this milestone, so current gold
evidence can be dereferenced before real derivation exists.

## Source Keys

Source keys should reuse the existing Milestone 3 reference-index conventions
rather than re-deriving keys in a parallel implementation. The current validator
already has the key and alias scheme in `fixture_validation::ReferenceIndex`;
Milestone 4 should promote or extend that logic into a shared helper/module so
the validator and hot store use one source of truth.

The distinction is value richness, not key semantics:

- Milestone 3 maps refs to reference categories to prove closure.
- Milestone 4 should map the same refs to concrete stored-record handles while
  still allowing the validator to project the category-only view it needs.

The descriptive key scheme is:

| Input | Key |
|---|---|
| `resources[*].id` | resource id |
| `traces[*].trace_id` | trace id |
| `traces[*].spans[*]` | `{trace_id}/{span_id}` |
| `metrics[*]` | `{name}@{entity}` |
| `logs[*].id` | log id |
| `changes[*].id` | change id |
| `prior_incidents[*].id` | prior incident id |
| `telemetry_gaps[*].id` | telemetry gap id |
| `expected.anomaly_windows[*].id` | anomaly window id |
| `expected.log_patterns[*].id` | log pattern id |
| `expected.evidence_bundle.items[*].id` | evidence item id |

The store should also support resolver aliases used by fixtures:

- `trace:<trace_id>` resolves to a trace record;
- `trace:<trace_id>/<span_id>` resolves to a span record;
- scalar refs such as `aw-1`, `log-1`, and `change:deploy-checkout-v2` resolve
  through the same index;
- Evidence IR `SourceRef { signal, ref }` resolves through signal-aware category
  matching.

If a ref exists but its signal category does not match, the resolver must make
that visible as a mismatch outcome. For the committed corpus, this is a hard
failure in store-aware validation and `get_evidence_bundle` source lookup. Do
not keep a warning-only compatibility path unless a future fixture explicitly
needs it and provides a test-covered witness or fixture flag.

Primary keys and aliases should remain distinct in the implementation model.
Primary keys identify stored records. Aliases such as `trace:t-0001` or
`trace:t-0001/s-3` are additional lookup routes to the same record, not extra
records. Alias conflicts should be reported unless both aliases resolve to the
same concrete target and category.

Duplicate-key behavior is defined per kind:

- same kind + same primary key + different record is a loader error;
- same raw key across different kinds is allowed in the store index;
- signal-aware resolution should use `SourceRef.signal` to disambiguate;
- scalar resolution should report ambiguity when the raw key has multiple
  category matches and no signal can choose one.

Signal-aware resolution should report both the requested signal and the matched
record kind when they differ. This is especially important for the known
fixture compatibility pressure where an Evidence IR source ref may use
`signal: "log"` while the ref string points at a derived log pattern id. The hot
store should preserve that distinction instead of silently accepting the closest
record.

## Store API Shape

The implementation should expose a small library surface, not only a CLI:

```rust
pub struct HotContextStore;
pub struct SourceKey;
pub struct StoredRecord;
pub struct SourceQuery;
pub struct SourceResolution;

impl HotContextStore {
    pub fn new() -> Self;
    pub fn load_fixture_case(case: &FixtureCase) -> Result<Self, HotStoreError>;
    pub fn insert_record(&mut self, record: StoredRecord) -> Result<(), HotStoreError>;
    pub fn resolve_source_ref(&self, source_ref: &SourceRef) -> SourceResolution;
    pub fn resolve_scalar_ref(&self, scalar_ref: &str) -> SourceResolution;
    pub fn select(&self, query: SourceQuery) -> Vec<&StoredRecord>;
}
```

`SourceQuery` should support:

- optional time window;
- optional entity list;
- optional record kind list;
- stable ordering.

Stable ordering should be deterministic and should prefer fixture order when
records come from fixture input. This keeps tests and later eval output
repeatable.

`SourceResolution` should distinguish:

- found one concrete target;
- found multiple possible targets;
- found the ref string but with a signal/category mismatch;
- missing target.

Do not collapse these cases into `Option<&StoredRecord>`. Ambiguity and mismatch
are important investigation signals.

## Error And Resolution Semantics

The implementation should keep loader errors separate from lookup outcomes.
Fixture loading errors describe invalid store construction. Source resolution
outcomes describe what an investigation or validation lookup found inside a
successfully constructed store.

Loader errors should cover at least:

- duplicate primary source keys within the same record kind;
- alias conflicts;
- missing required fixture fields for a recognized record kind;
- malformed time windows where the fixture field is present but unusable;
- unsupported or ambiguous derived artifact shape when expected artifacts are
  loaded as reference targets.

Resolution outcomes should cover at least:

- unsupported `external` refs;
- unsupported `profile` refs until a profile source record kind exists;
- missing refs;
- ambiguous refs;
- signal/category mismatches;
- found refs with the concrete stored record payload.

The store-aware `get_evidence_bundle` integration should fail on missing,
ambiguous, unsupported, or mismatched source refs for the committed corpus. Do
not let the integration silently drop unverifiable refs.

## Fixture Loading

The first loader should build from the already validated fixture corpus rather
than duplicate parsing logic. It can use `FixtureCorpus` and `FixtureCase` from
the `fixture_validation` module, and it should share the promoted
reference-index helpers described in "Source Keys" for source-key and alias
construction.

Required behavior:

- load all raw top-level input keys recognized by `fixtures.md`;
- preserve `_` helper annotations in payload but do not index them as records;
- extract entity ids from obvious fields such as `entity`, `resource`, span
  resource ids, metric entity ids, log entity ids, change entity ids, and
  telemetry gap affected entities;
- extract time windows from `t`, `start`, `end`, trace span start/end, metric
  point timestamps, and telemetry gap start/end;
- load same-fixture expected artifacts as derived reference targets when an
  expected file is provided;
- detect duplicate primary keys within a record kind unless the duplicate is an
  intentional alias to the same concrete record.

The loader should report structured errors with fixture id, file path, JSON
path, and message. Reusing the validation issue style from
`fixture_validation.rs` is preferred if it keeps the code simple.

## Source-Reference Resolution

The resolver must be able to dereference every `source_refs[*]` item in every
current fixture evidence bundle after loading that fixture's input and expected
artifacts into the store.

Resolution should return a concrete `StoredRecord` payload, not just "the string
exists." This is the difference between the Milestone 3 validation harness and
the Milestone 4 store:

- Milestone 3 proves references close over the fixture corpus.
- Milestone 4 proves Janus can retrieve the referenced source material.

The resolver should cover at least:

- trace ids and span refs;
- metric series refs;
- log ids and log pattern refs;
- change ids;
- prior incident ids;
- telemetry gap ids;
- anomaly window ids;
- entity ids and relationship refs when present.

`external` refs should still fail unless a future design adds a self-contained
external-record target. `profile` refs should return the same deterministic
unsupported outcome until Janus has a profile source record kind. Janus must not
produce unverifiable source pointers.

## Query Integration

The existing `get_evidence_bundle` path can remain fixture-backed in this
milestone. It should, however, be able to use the hot store for source lookup:

1. validate `EvidenceQuery`;
2. load the fixture-backed gold bundle as today;
3. load that fixture's input and expected artifacts into `HotContextStore`,
   which means using the registry-backed `FixtureCorpus`/`FixtureCase` path
   rather than only the narrow `expected.json` bundle loader;
4. validate or resolve every returned evidence source ref through the store;
5. apply basic store selection tests for the query time window and query
   entities;
6. return the bundle unchanged.

Returning the bundle unchanged is intentional. Evidence compilation and ranking
belong to Milestone 6. The value of this milestone is that the returned bundle
is no longer detached from a concrete recent-window context substrate.

If this integration makes the existing public function too heavy, add a
store-aware helper first and keep the old fixture-backed function as a thin
wrapper. The tests should prove the source lookup path is exercised.

The store-aware path should not filter or rewrite the returned gold bundle in
this milestone. Time-window and entity selector checks should prove that the
store can retrieve relevant records for the query, while budgeted selection,
evidence pruning, and query-dependent bundle generation remain Milestone 6 work.

## CLI

A small inspection command is useful for development and demos, but it is not
the main contract. Suggested shape:

```bash
cargo run --bin inspect_hot_context -- --fixture deploy-bad-rollout --ref t-0001/s-3
cargo run --bin inspect_hot_context -- --fixture deploy-bad-rollout --entity service:checkout
```

Minimum useful behavior:

- load one fixture into the store;
- resolve a scalar ref or Evidence IR source ref;
- print the resolved record kind, key, time window, entities, and compact JSON
  payload.

This should not become the simulator. It is only a local inspection aid.

## Tests

Add focused tests that prove behavior, not only compile-time shape:

- every current fixture input loads into `HotContextStore`;
- duplicate primary keys within the same record kind fail with a useful error;
- same-string refs across different record kinds are disambiguated by
  `SourceRef.signal` or reported as ambiguous for scalar resolution;
- span refs such as `t-0001/s-3` resolve to a concrete span payload;
- metric series refs resolve to concrete metric payloads;
- log, change, prior-incident, and telemetry-gap refs resolve where present;
- derived anomaly-window and log-pattern refs resolve when expected artifacts
  are loaded;
- every Evidence IR source ref in every current fixture evidence bundle
  resolves through the store;
- missing refs, unsupported refs, ambiguous refs, and signal/category mismatches
  fail distinctly;
- time-window selectors return records overlapping the requested window;
- entity selectors return records tied to the requested entity;
- selector output order is deterministic;
- the store-aware `get_evidence_bundle` path still returns the same bundle while
  exercising source lookup.

## Definition Of Done

This topic is complete when:

- all registered fixture `input.json` files can be loaded into the local hot
  store;
- all current fixture evidence source refs can be resolved to concrete stored
  records or same-fixture derived artifact records;
- the store exposes stable time-window and entity selectors;
- `get_evidence_bundle` or a store-aware helper uses the store for source lookup
  checks without generating new evidence;
- source lookup failures are structured and test-covered;
- the design keeps live OTLP ingest and simulator work as follow-up adapters,
  not hidden requirements;
- `cargo fmt`, `cargo test`, `cargo clippy --all-targets --all-features`, and
  `cargo run --bin validate_fixtures` pass.

## Review Focus

Reviewers should focus on these points:

1. Whether the store boundary is strong enough for later simulator and OTLP
   ingest topics.
2. Whether source-ref resolution retrieves concrete records rather than merely
   repeating Milestone 3 closure checks.
3. Whether this topic stays small enough by excluding derivation, ranking, live
   ingest, and durable persistence.
4. Whether the hard-failure policy for signal/category mismatches is correct
   for the now-clean committed corpus.
5. Whether the recommended implementation slices are acceptable after design
   approval, or whether reviewers want the whole Milestone 4 implementation
   reviewed as one coding round.
6. Whether the selector semantics and error model are explicit enough to test
   without inventing retrieval behavior.
