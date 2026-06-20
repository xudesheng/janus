# Fixture Validation Harness Design

Status: design for the `fixture-validation-harness` topic.

This document designs the Milestone 3 harness from
[`docs/core/roadmap.md`](../core/roadmap.md). It is grounded in
[`docs/core/what_and_why.md`](../core/what_and_why.md),
[`docs/process/fixtures.md`](fixtures.md),
[`docs/core/evidence-ir-schema.md`](../core/evidence-ir-schema.md), and
[`docs/core/get-evidence-bundle-contract.md`](../core/get-evidence-bundle-contract.md).
If this document conflicts with `what_and_why.md` or `fixtures.md`, those
documents win and this document should be corrected.

## Purpose

The fixture corpus should become executable acceptance data, not passive JSON.
The harness should answer three questions automatically:

1. Can every registered fixture be loaded as a coherent scenario?
2. Do all declared expected artifacts and evidence references close over that
   fixture's own input or same-fixture derived artifacts?
3. Does the corpus cover the capabilities and failure classes that future Janus
   work claims to support?

This should happen before the hot store and derivation pipeline, so later
implementation work has a concrete target and cannot silently drift away from
the evidence contract.

## Design Review Gate

No Rust implementation should start for this topic until every active reviewer
has agreed on the design direction in their `Direction Verdict`.

This topic should finalize the Milestone 3 harness design before coding by
default. Registry validation, source-reference closure, capability witnesses,
coverage reporting, and uncertainty guards share one corpus model and one issue
model, so agreeing on the full contract first reduces churn in the implementation
rounds.

Reviewers may explicitly approve phase-by-phase implementation instead. If they
do, each phase still remains part of the same Milestone 3 contract and should
not weaken the Definition Of Done below. Until reviewer agreement exists, review
rounds for this topic are design-only or diagnosis-only rounds.

## Scope

In scope:

- load and validate `fixtures/registry.json`;
- load each scenario's `scenario.json`, `input.json`, and `expected.json`;
- validate manifest fields, declared input keys, declared expected keys,
  canonical capability tags, failure classes, difficulty, and path/id
  consistency;
- validate `expected.evidence_bundle` with the existing Evidence IR Rust types;
- validate source references from expected artifacts back to `input.json` or to
  derived artifacts in the same `expected.json`;
- report coverage by capability, failure class, difficulty, and
  false-causality-trap status;
- select fixtures by capability, failure class, and difficulty for later tests
  and eval runs;
- expose one command that validates the whole corpus and fails on errors.

Out of scope:

- deriving `expected.json` from `input.json`;
- proving the semantic correctness of every gold conclusion;
- byte-exact OTLP parsing or live OTLP ingest;
- storage engine behavior;
- scoring agent answers;
- changing fixture contents except where validation reveals a concrete defect.

## Relationship To Existing Code

Milestones 1 and 2 introduced a narrow fixture loader that returns
`expected.evidence_bundle` for one scenario id. That loader should remain useful
for `get_evidence_bundle` walking-skeleton tests.

The new harness should extend the fixture layer without overloading the narrow
API. A practical shape is:

- keep `load_bundle_by_scenario_id` and `load_bundle_from_expected_path` as the
  small Evidence IR loader;
- add a corpus-level loader and validator beside it, either in `src/fixtures.rs`
  if it stays readable or in a new `src/fixture_validation.rs` module if the
  file grows;
- share scenario-id path safety logic so user-supplied ids cannot escape
  `fixtures/scenarios`.

The implementation should continue to use `serde` and `serde_json`. It does not
need a full Rust model for every nested OTel-shaped input record in this
milestone. Typed structs are valuable for registry and manifest fields; nested
input and expected artifacts can stay as `serde_json::Value` with targeted
extractors for ids and references.

## Data Model

The harness should model these top-level objects:

- `FixtureRegistry`
  - `schema_version`
  - `description`
  - `capabilities`
  - `failure_classes`
  - `fixtures`
  - `proposed`
- `FixtureRegistryEntry`
  - `id`
  - `path`
  - `failure_class`
  - `difficulty`
  - `false_causality_trap`
  - `capabilities`
  - `title`
- `ScenarioManifest`
  - fields from the `scenario.json` contract in `fixtures.md`
- `FixtureCase`
  - registry entry
  - resolved directory
  - manifest
  - input JSON
  - expected JSON
- `ValidationIssue`
  - severity
  - fixture id when applicable
  - file path
  - JSON path
  - message

The validator should allow `_`-prefixed top-level helper keys such as `_note`.
Unknown non-helper top-level keys should be reported once the known key set is
modeled. This catches accidental drift while preserving the annotation pattern
already used by the fixtures.

## Validation Stages

Validation should run in deterministic stages and collect all issues before
returning, so a contributor gets one complete report instead of one error per
run.

### 1. Registry Validation

Required checks:

- `schema_version` is `fixtures/v1`;
- fixture ids are unique;
- fixture paths are unique and stay under `fixtures/scenarios`;
- every registered fixture directory exists;
- every registered fixture path basename equals the registry `id`;
- `failure_class` is in the registry's canonical `failure_classes`;
- every capability is in the registry's canonical `capabilities`;
- `difficulty` is one of `baseline` or `hard`;
- entries under `proposed` are reported separately and are not required to
  exist on disk.

The registry's `capabilities` and `failure_classes` arrays are the machine
source for canonical vocabulary. They should stay aligned with `fixtures.md`.

### 2. Scenario Manifest Validation

Required checks for each registered fixture:

- `scenario.json`, `input.json`, and `expected.json` all exist;
- `scenario.id` equals the directory basename and registry id;
- `scenario.schema_version` is `fixtures/v1`;
- `scenario.version` is a positive integer;
- manifest `failure_class`, `difficulty`, `false_causality_trap`, `title`, and
  `capabilities` match the registry entry;
- manifest capabilities are canonical;
- manifest `inputs` equals the non-helper top-level keys present in
  `input.json`;
- manifest `expected` equals the non-helper top-level keys present in
  `expected.json`;
- required narrative fields such as `summary`, `question`, `time_window`, and
  `ground_truth` are present.

The harness should not try to decide whether the narrative is correct. It should
only ensure the manifest and files agree structurally.

### 3. Evidence IR Validation

When `scenario.expected` includes `evidence_bundle`, `expected.json` must include
`evidence_bundle`. The harness should deserialize it as the existing
`EvidenceBundle` type and call `EvidenceBundle::validate()`.

This reuses Milestone 1 instead of duplicating Evidence IR validation. Any
Evidence IR error should be reported with fixture id and JSON path context.

### 4. Reference Index Construction

For each fixture, build a reference index from input and expected artifacts.
Reference validation should use actual ids from JSON, not naming guesses.

Input refs:

- resources: each `resources[*].id`;
- traces: each `traces[*].trace_id`, plus each span as
  `{trace_id}/{span_id}` using the actual `span_id`;
- metrics: each series as `{name}@{entity}`;
- logs: each `logs[*].id`;
- changes: each `changes[*].id`;
- prior incidents: each `prior_incidents[*].id`;
- telemetry gaps: each `telemetry_gaps[*].id`, plus metric `_gap.ref` values
  when present.

Expected refs:

- entities: each `entities[*].id`;
- relationships: a stable relationship ref if the fixture defines one later;
- anomaly windows: each `anomaly_windows[*].id`;
- log patterns: each `log_patterns[*].id`;
- evidence items: each `evidence_bundle.items[*].id`;
- any other derived artifact with an explicit `id` under a known expected key.

The index should record both the raw ref string and its source category. This
lets the validator produce useful messages such as "found ref `lp-1` as a log
pattern, but the source signal says `log`."

### 5. Source Reference Validation

The harness should validate known reference-bearing fields first:

- `evidence_bundle.items[*].source_refs[*]`;
- `timeline[*].source_ref`;
- `relationships[*].evidence[*]`;
- `log_patterns[*].exemplars[*]`;
- `suspected_causes[*].supporting[*]`;
- `suspected_causes[*].counter[*]`;
- metric `_gap.ref` and telemetry gap `cause` values in `input.json`;
- source-like fields in `related_anomalies`, `window_comparison`, and
  `entity_context` when their shapes contain explicit refs.

Supported reference forms:

- Evidence IR source refs: `{ "signal": "metric", "ref": "..." }`;
- scalar refs such as `aw-1`, `log-1`, `change:deploy-checkout-v2`, or
  `db.query.duration_p95_ms@db:orders-pg`;
- string shorthands used by relationship evidence, such as `trace:t-0001`.

Rules:

- every source ref must resolve to input or same-fixture expected artifacts;
- `external` source refs should fail unless a future fixture model gives them a
  self-contained in-fixture target;
- scalar `trace:<trace_id>` resolves to an input trace id;
- scalar `trace:<trace_id>/<span_id>` resolves to an input span ref;
- a signal/ref mismatch should be a warning at first if the ref resolves to a
  known same-fixture artifact, then tightened after current fixtures are clean.

The initial compatibility warning matters because some existing gold bundles use
`signal: "log"` with `ref: "lp-1"` to point at a derived log pattern. The
Evidence IR enum has `log_pattern`, so the long-term clean form should be
`signal: "log_pattern"` for derived pattern refs and `signal: "log"` for raw
log records.

### 6. Capability Exercise Checks

Each declared capability should have a minimal structural witness. The goal is
not deep semantic proof; it is preventing empty capability declarations.

Suggested witness mapping:

- `entity-resolution`: `expected.entities`
- `relationship-building`: `expected.relationships`
- `change-ingestion`: `input.changes`
- `anomaly-windows`: `expected.anomaly_windows`
- `log-pattern-clustering`: `expected.log_patterns`
- `evidence-ir`: `expected.evidence_bundle`
- `get_evidence_bundle`: `expected.evidence_bundle`
- `build_timeline`: `expected.timeline`
- `find_related_anomalies`: `expected.related_anomalies`
- `compare_windows`: `expected.window_comparison`
- `rank_suspected_causes`: `expected.suspected_causes`
- `expand_entity_context`: `expected.entity_context`
- `suggest_next_checks`: `expected.next_checks`
- `false-causality-guard`: counter-evidence or a ranked suspected cause with a
  non-empty `counter` list;
- `token-budget-retrieval`: `expected.evidence_bundle.budget`.

If a capability lacks its witness, validation should fail because downstream
tests will otherwise believe a scenario exercises behavior that is not present.

### 7. Uncertainty And False-Causality Checks

Janus treats false causality and missing data as core failure modes. The harness
should enforce this structurally:

- if `scenario.false_causality_trap` is true, the expected bundle must include
  at least one `counter_evidence` item or at least one suspected cause with
  counter evidence;
- if `scenario.ground_truth.not_the_cause` is present and non-empty, at least
  one counter-evidence path should exist;
- if `scenario.failure_class` is `missing-data`, or `input.telemetry_gaps`
  exists, the expected bundle must include a `missing_data` evidence item or
  non-empty item-level `missing_data`;
- suspected causes should not rank an explicitly innocent suspect first when
  ground truth includes an obvious `not_the_cause` entity.

These checks should stay structural. They should not try to calculate true
causality from telemetry.

### 8. Coverage Report

The harness should print a coverage report after validation:

- fixture count;
- counts by failure class;
- counts by capability;
- counts by difficulty;
- count of false-causality trap scenarios;
- proposed fixture count.

Coverage gaps should be visible in the report. They do not need to fail the
command unless the registry itself declares an invalid capability/failure class
or the corpus becomes empty.

## Selection API

The corpus loader should expose selectors for later tests and eval code:

- select by capability;
- select by failure class;
- select by difficulty;
- combine selectors with AND semantics;
- return stable ordering by registry order.

The command-line tool can use the same selectors for focused runs, but the Rust
API is the important contract.

## Implementation Phases

After design approval, implementation can land in focused reviewable phases:

1. Corpus model, registry loading, manifest loading, deterministic issue
   collection, and fixture selectors.
2. Evidence IR reuse, reference-index construction, and source-reference closure
   validation.
3. Capability witness checks, false-causality checks, missing-data checks,
   coverage reporting, and the validation CLI.
4. Focused negative tests and cleanup if the earlier phases need to stay small.

These phases are sequencing guidance, not separate product milestones. Phase 1
is not a complete Milestone 3 outcome if source-reference closure and
uncertainty checks are still missing.

## CLI

Add one command, for example:

```bash
cargo run --bin validate_fixtures
```

Minimum behavior:

- validates all registered fixtures;
- prints grouped errors and warnings;
- prints the coverage report;
- exits non-zero if any error exists.

Useful optional flags:

- `--fixture <id>`;
- `--capability <tag>`;
- `--failure-class <class>`;
- `--difficulty <baseline|hard>`;
- `--json` for machine-readable issue and coverage output.

The no-argument command is the acceptance path. Optional filters should not be
required for Milestone 3 to be useful.

## Tests

Add focused tests around the harness rather than only snapshotting CLI output.
Suggested tests:

- current registry and all current fixtures validate successfully;
- duplicate registry ids fail;
- unknown capability or failure class fails;
- manifest `inputs` mismatch fails;
- manifest `expected` mismatch fails;
- dangling `source_refs` fail;
- dangling `timeline.source_ref` fails;
- dangling `suspected_causes.supporting` or `counter` evidence ids fail;
- missing false-causality counter evidence fails for a trap scenario;
- missing-data scenarios without a missing-data channel fail;
- selectors return stable registry-order results.

Negative tests can use temporary fixture directories. A small dev dependency
such as `tempfile` is acceptable if the standard library setup becomes noisy.

## Definition Of Done

This topic is complete when:

- `cargo run --bin validate_fixtures` validates the whole registered corpus;
- dangling source refs fail validation;
- declared input and expected artifacts are checked against actual JSON keys;
- fixture capabilities and failure classes are validated against canonical
  vocabulary;
- false-causality and missing-data scenarios are structurally guarded;
- fixture selection by capability, failure class, and difficulty exists;
- coverage is reported in deterministic output;
- `cargo fmt`, `cargo test`, and `cargo clippy --all-targets --all-features`
  pass.

## Review Focus

Reviewers should pay closest attention to:

- whether source-reference closure is strict enough without forcing unnecessary
  fixture churn;
- whether `signal`/ref mismatches should start as warnings or immediate errors;
- whether capability witness checks should be hard validation failures;
- whether the false-causality and missing-data checks are structural enough to
  avoid pretending to solve causality;
- whether implementation should proceed only after whole-design approval or be
  approved phase by phase.

The first and fourth points protect Janus from its highest-risk failure modes:
unverifiable summaries and confident false causality.
