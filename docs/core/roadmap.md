# Janus Roadmap

Status: living roadmap, not a release schedule.

This document turns the Janus thesis in
[`what_and_why.md`](what_and_why.md) into ordered implementation milestones. If
this roadmap conflicts with `what_and_why.md`, the canonical design doc wins and
this roadmap should be corrected.

## Roadmap Goal

Janus should make one bet measurable:

> Given the same agent, the same incident, and the same time and token budget,
> Janus should put fewer, more accurate, and more auditable evidence into the
> agent's context than raw-backend access does.

The roadmap is therefore organized around evidence quality, provenance,
counter-evidence, token budget, and agent-facing investigation primitives. It is
not organized around building a complete APM product.

## Current Baseline

The repository already has:

- a canonical design doc: `docs/core/what_and_why.md`;
- a near-term implementation plan: `docs/core/evidence-spine.md`;
- a fixture scheme: `docs/process/fixtures.md`;
- a synthetic incident corpus under `fixtures/`;
- a review workflow in `docs/review-framework.md`.

The repository does not yet have:

- Rust Evidence IR types;
- JSON Schema for agent-facing contracts;
- fixture validation wired into tests;
- a `get_evidence_bundle` implementation;
- a hot context store;
- derivation pipelines;
- MCP tools;
- a comparative eval harness.

The code is still scaffold-level. That is acceptable. The next work should make
the evidence contract executable before building storage or ingestion machinery.

## Sequencing Principles

1. **Contract before storage.** Evidence shape, query shape, and validation come
   before storage-engine choices.
2. **Fixtures before general algorithms.** Gold incident artifacts are the first
   target. Generalization follows only after the contract survives concrete
   scenarios.
3. **Provenance is mandatory.** Every summary, ranking, and evidence item must
   preserve source references or explicitly report why they are unavailable.
4. **False causality is a first-class failure mode.** Counter-evidence,
   dependency direction, time alignment, missing data, and entity confidence are
   acceptance criteria, not polish.
5. **Token budget is part of query semantics.** Evidence selection must optimize
   diagnostic value under a budget, not return `LIMIT N`.
6. **Small local implementation before distributed architecture.** A local store,
   simple jobs, and fixture-backed tests are enough until the contract is proven.
7. **Agent surface before dashboard surface.** APIs and MCP tools that return
   structured evidence take priority over UI parity with observability products.
8. **Evaluation before scale.** Janus must first show better agent outcomes on a
   small corpus, then optimize throughput, latency, and retention.

## Milestone 0: Design And Fixture Baseline

Goal: make the problem space, contract direction, and test corpus explicit.

Status: mostly complete.

Deliverables:

- canonical vision and boundary in `what_and_why.md`;
- near-term implementation spine in `evidence-spine.md`;
- fixture scheme in `docs/process/fixtures.md`;
- synthetic scenarios with `input.json`, `expected.json`, and registry entries;
- review workflow for design and implementation rounds.

Done when:

- all fixture entries in `fixtures/registry.json` exist on disk;
- every fixture is self-contained and uses Janus-owned synthetic data;
- the first implementation milestone can be reviewed against concrete artifacts.

Known gap:

- fixture validation is still manual. Automated validation belongs in Milestone 3.

## Milestone 1: Evidence IR Contract

Goal: make the core evidence contract executable in Rust.

Deliverables:

- `serde`-backed Rust types for `EvidenceItem`, `EvidenceBundle`, and supporting
  objects such as time windows, source refs, missing data, token budget,
  direction, freshness, and privacy scope;
- a thin read-only fixture loader for `expected.json` evidence bundles, scoped
  only to what this milestone needs, including single-fixture resolution by
  scenario id or path such as `fixtures/scenarios/<id>/expected.json`;
- JSON Schema for the evidence contracts;
- at least one serialized example evidence item and bundle;
- tests that validate the Evidence IR shape against existing fixture
  `expected.json` files.

Acceptance criteria:

- `source_refs` is represented as a first-class required field;
- `direction`, `missing_data`, and confidence-related fields can represent both
  supporting evidence and counter-evidence;
- evidence strength is not conflated with causal confidence;
- fixture evidence bundles can be deserialized without ad hoc parsing;
- the fixture loader is intentionally narrow: single-fixture addressing is in
  scope, but source-ref validation, registry coverage, and full fixture metadata
  validation are not;
- schema generation and tests run with `cargo test`.

Suggested review topic:

- `evidence-ir-schema`

## Milestone 2: `get_evidence_bundle` Walking Skeleton

Goal: expose the first investigation primitive end to end, using fixture-backed
gold output before real retrieval exists.

Deliverables:

- `EvidenceQuery` request type with question or hypothesis, time window,
  entities, max items, max tokens, counter-evidence requirement, raw refs
  requirement, freshness requirement, and privacy scope;
- `get_evidence_bundle(EvidenceQuery) -> EvidenceBundle` as a stable Rust
  boundary;
- a fixture-backed stub implementation that returns gold bundles by scenario,
  using the narrow loader introduced in Milestone 1;
- a CLI or test helper that emits Evidence IR JSON for a selected fixture.

Boundary:

- this milestone returns hand-authored gold bundles. The real compiled and
  ranked `get_evidence_bundle` path lands in Milestone 6.

Acceptance criteria:

- `get_evidence_bundle` is not modeled as `LIMIT N`;
- responses include budget accounting and preserve item ordering;
- at least one baseline fixture and one false-causality trap fixture are covered;
- counter-evidence and missing data survive round-trip serialization;
- the skeleton compiles and runs without a storage engine.

Suggested review topic:

- `get-evidence-bundle-contract`

## Milestone 3: Fixture Validation Harness

Goal: make the whole incident corpus executable test data rather than passive
JSON.

Deliverables:

- registry loader for `fixtures/registry.json`, extending the narrow
  Milestone 1 fixture loader into a general corpus loader;
- scenario loader for `scenario.json`, `input.json`, and `expected.json`;
- validation that every declared input and expected artifact exists;
- source-reference validation from `expected.json` back into `input.json` or
  same-fixture derived artifacts;
- capability and failure-class coverage report.

Acceptance criteria:

- one command validates all fixtures;
- dangling `source_refs` fail tests;
- missing counter-evidence or missing-data channels are detected for scenarios
  that declare false-causality or missing-data behavior;
- the harness can select fixtures by capability, failure class, and difficulty.

Suggested review topic:

- `fixture-validation-harness`

## Milestone 4: Local Hot Context Store

Goal: replace pure fixture stubs with a minimal recent-window substrate that can
load OTel-shaped inputs and resolve source references.

Deliverables:

- in-memory or simple local store for resources, traces, metrics, logs, and
  change events;
- time-window and entity selectors;
- source-reference resolver for trace spans, metric series, log records, change
  events, and derived artifacts;
- a clear boundary between raw source data and derived context.

Acceptance criteria:

- fixture `input.json` files can be loaded into the store;
- source refs in evidence items can be dereferenced back to concrete input
  records;
- `get_evidence_bundle` can use the store for at least source lookup and basic
  filtering;
- no distributed storage, live OTLP ingest, or storage engine optimization is
  required yet.

Suggested review topic:

- `hot-context-store`

## Milestone 5: Derived Context V1

Goal: derive the first operational objects that agents need before evidence
selection can become meaningful.

This milestone should land in two reviewable slices rather than one large round.

### Milestone 5A: Entity And Relationship Context

Deliverables:

- entity resolver with confidence, alternatives, unresolved states, and missing
  attributes;
- relationship builder for dependency and runtime relationships;
- comparison against fixture gold `entities` and `relationships`.

Acceptance criteria:

- derived entities and relationships can be compared with fixture gold output;
- entity ambiguity is visible instead of silently collapsed;
- relationship confidence and source evidence are represented explicitly.

Suggested review topic:

- `entity-resolver-confidence`

### Milestone 5B: Anomalies, Patterns, And Timelines

Deliverables:

- anomaly window importer or simple detector over fixture metrics;
- log and error pattern clustering for representative fixture logs;
- timeline builder for symptoms, changes, propagation effects, recovery markers,
  and candidate nearby-change markers;
- derived support for `find_related_anomalies`;
- derived support for `compare_windows`.

Acceptance criteria:

- anomaly windows include bounded time intervals and source refs;
- log clusters preserve exemplars;
- timeline output preserves event ordering and candidate nearby changes, without
  final causal or non-causal classification;
- `related_anomalies` and `window_comparison` fixture artifacts have a concrete
  derived-context home instead of drifting as unused gold output.

Suggested review topic:

- `derived-context-baseline`

## Milestone 6: Evidence Compiler V1

Goal: generate and rank Evidence IR from source and derived context under a token
budget.

Deliverables:

- evidence item generation from changes, anomalies, trace exemplars, log
  clusters, dependency edges, previous incidents, counter-evidence, and missing
  data;
- scoring that separates evidence strength from causal confidence;
- token-cost accounting and dropped-item reporting;
- support for counter-evidence requirements;
- `rank_suspected_causes` output that links candidate causes to supporting
  evidence, counter-evidence, and trap notes where relevant;
- causal and non-causal classification for nearby changes using time alignment,
  dependency direction, blast radius, and counter-evidence;
- initial `suggest_next_checks` logic based on gaps and weak hypotheses.

Boundary:

- `previous_incident` evidence in this milestone is generated only from
  fixture-provided `prior_incidents` input. It becomes real warm/cold memory only
  after Milestone 10.
- The `recurring-incident-memory` fixture is useful for contract pressure before
  Milestone 10, but it is not end-to-end satisfied by a real memory pipeline
  until Milestone 10 lands.

Acceptance criteria:

- false-causality trap fixtures rank the obvious innocent suspect low with
  explicit counter-evidence;
- `suspected_causes` fixture artifacts have a concrete generation path;
- nearby changes are classified only after Evidence Compiler reasoning, not by
  the Milestone 5B timeline builder alone;
- evidence bundles include hypothesis-discriminating evidence rather than only
  the largest or noisiest signals;
- missing input data is returned as evidence about uncertainty, not hidden;
- every generated evidence item is auditable through source refs.

Suggested review topics:

- `evidence-compiler-ranking`
- `false-causality-guard`

## Milestone 7: Agent Surface V1

Goal: expose Janus investigation primitives to external agents.

Deliverables:

- stable API surface for `get_evidence_bundle`;
- first MCP tool for `get_evidence_bundle`;
- strict input and output schemas suitable for tool-use validators;
- initial surfaces for `build_timeline`, `expand_entity_context`,
  `find_related_anomalies`, `compare_windows`, `rank_suspected_causes`, or
  `suggest_next_checks`, depending on which fixture-backed capability is most
  mature.

Boundary:

- `explain_symptom` is treated as a question-driven `get_evidence_bundle`
  workflow plus timeline context in this version. It should become a separate
  primitive only if the evidence contract needs a distinct surface.

Acceptance criteria:

- an external agent can call Janus and receive structured Evidence IR JSON;
- schemas are accepted by strict validators, including array item definitions;
- privacy scope and redaction fields are present even if enforcement is still
  minimal;
- the API returns inspectable evidence, not root-cause prose as the contract.

Suggested review topic:

- `mcp-agent-surface`

## Milestone 8: Comparative Eval V1

Goal: test the central Janus bet against the fixture corpus.

Deliverables:

- raw-access baseline that gives an agent or evaluator realistic raw query
  access under the same token budget, using recency, label, and entity slices
  rather than a naive full dump or arbitrary `LIMIT N`;
- Janus-access path that gives the same agent or evaluator Evidence IR bundles;
- scoring for suspicious-entity accuracy, useful timeline quality,
  false-causality rate, token cost, missing-data awareness, and auditability;
- scoring against `scenario.json` `ground_truth`, including
  `primary_cause_entity`, `blast_radius`, and `not_the_cause`;
- repeatable eval report over the fixture corpus.

Acceptance criteria:

- the eval can be run from a single command;
- results are tied to fixture versions;
- the raw-access baseline is reviewed as an adversarial baseline, not a strawman
  designed to make Janus win;
- token cost is measured from serialized raw-access and Janus-access material,
  not copied from hand-authored Evidence IR `token_cost` fields;
- Janus improves at least one target metric without hiding regressions in others;
- false-causality trap scenarios are reported separately from baseline scenarios.

Suggested review topic:

- `comparative-eval-v1`

## Milestone 9: Real Ingest And Persistence

Goal: move from fixture loading to a small live-ingest prototype while preserving
the evidence contract.

Deliverables:

- OTLP or Collector-exported telemetry ingest path;
- change-event ingest API for deploys, config changes, feature flags, scaling
  events, CI/CD events, and infrastructure events;
- practical local persistence choice for the hot layer;
- replay or backfill path for fixture-like incident windows;
- retention boundary for recent raw telemetry.

Acceptance criteria:

- a local demo can ingest a small telemetry stream and answer evidence queries;
- persisted records retain stable source refs;
- storage choices remain replaceable behind the contract;
- the live path is validated against the same Evidence IR and query schemas.

Suggested review topics:

- `otel-ingest-prototype`
- `change-event-ingest`

## Milestone 10: Warm Memory And Compaction

Goal: preserve useful operational memory after the hot window expires, including
the cold-layer idea of durable understanding plus backlinks.

Deliverables:

- incident summary objects with source backlinks;
- entity history and relationship evolution;
- anomaly and log pattern histories;
- previous-incident evidence items;
- compaction job from hot source data to warm memory;
- cold-layer durable records for long-lived summaries, evidence metadata, entity
  history, and source backlinks without requiring full raw telemetry retention.

Acceptance criteria:

- recurring-incident fixtures can be answered using warm memory;
- summaries remain linked to source refs or durable backlinks;
- old raw telemetry can be reduced without destroying auditability;
- warm and cold memory improve investigation without pretending to be full raw
  retention;
- the roadmap's cold layer means durable understanding plus backlinks, not a
  promise to preserve every raw byte.

Suggested review topic:

- `warm-memory-compaction`

## Milestone 11: Hardening And Production Shape

Goal: make the prototype credible as a deployable evidence substrate.

Deliverables:

- tenant and privacy-scope enforcement;
- performance and storage-cost measurements;
- backfill and replay behavior;
- failure handling for partial ingest and missing sources;
- deployment documentation;
- compatibility notes for coexistence with existing observability backends.

Acceptance criteria:

- throughput, latency, storage, and compaction costs are measured;
- missing or delayed telemetry produces explicit missing-data evidence;
- Janus can coexist with an existing backend through Collector fan-out or a
  similar low-risk integration path;
- operational limits are documented honestly.

Suggested review topic:

- `production-shape`

## Deliberately Later

These are not near-term roadmap goals:

- dashboard parity with full APM products;
- a custom distributed storage engine;
- automatic RCA as the Janus API contract;
- long-term full raw telemetry retention as the main value proposition;
- mitigation execution or agent action orchestration;
- broad UI work beyond what is needed to inspect evidence contracts.

They may become useful later, but they should not displace the evidence
substrate milestone chain.

## Near-Term Review Order

The next review topics should follow the contract-first path:

1. `evidence-ir-schema`
2. `get-evidence-bundle-contract`
3. `fixture-validation-harness`
4. `hot-context-store`
5. `entity-resolver-confidence`
6. `derived-context-baseline`
7. `evidence-compiler-ranking`
8. `mcp-agent-surface`
9. `comparative-eval-v1`

Each topic should name a concrete artifact as its milestone, follow
`docs/review-framework.md`, and stop once the milestone is complete with no open
review findings.
