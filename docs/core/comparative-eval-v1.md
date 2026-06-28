# Comparative Eval V1 Design

Status: design for the `comparative-eval-v1` topic.

This document defines the Milestone 8 Comparative Eval V1 slice. It is
grounded in [`what_and_why.md`](what_and_why.md), [`roadmap.md`](roadmap.md),
[`evidence-ir-schema.md`](evidence-ir-schema.md),
[`fixture-validation-harness.md`](../process/fixture-validation-harness.md),
[`hot-context-store.md`](hot-context-store.md),
[`derived-context-baseline.md`](derived-context-baseline.md),
[`evidence-compiler-ranking.md`](evidence-compiler-ranking.md), and
[`mcp-agent-surface.md`](mcp-agent-surface.md). If this document conflicts with
`what_and_why.md`, the canonical design doc wins and this document should be
corrected.

## Why This Topic Is Next

Janus now has the core pieces needed for a measurable evidence pipeline:

- fixture-backed OTel-shaped source data;
- fixture validation and coverage reporting;
- a hot context store with source-reference resolution;
- entity, relationship, anomaly, log pattern, timeline, related-anomaly, and
  window-comparison derivation;
- an evidence compiler that generates, ranks, and budgets Evidence IR;
- a local MCP `get_evidence_bundle` surface for external agents.

The next topic should not widen Janus with another surface or storage feature.
The next topic should test the central Janus claim:

```text
same incident + same budget
      -> raw backend access
      vs
      -> Janus Evidence IR access
      => fewer, better, more auditable evidence for the agent
```

Without this eval, Janus can demonstrate that it runs, but not that the evidence
contract improves investigation quality. `comparative-eval-v1` turns that bet
into a repeatable local command and report.

## Purpose

This topic should build the first comparative eval harness for the fixture
corpus.

The V1 harness should compare two access paths under the same scenario, time
window, and token budget:

1. **Raw-access baseline**: a fair, query-shaped baseline that selects raw
   telemetry material using time, entity, severity, change proximity, direct
   metric deltas, and trace/log selectors.
2. **Janus-access path**: `get_evidence_bundle` or the same compiled Evidence IR
   path behind it, returning structured evidence with source refs,
   counter-evidence, missing-data channels, and budget accounting.

The harness should score both paths against fixture `scenario.json`
`ground_truth` and relevant gold artifacts in `expected.json`. The eval should
not prove automatic root cause analysis. It should prove whether Janus gives a
better investigation substrate than direct raw access.

## Design Review Gate

No Rust implementation should start for this topic until every active reviewer
has agreed on the design direction in their `Direction Verdict`.

Reviewers should explicitly decide:

- whether `comparative-eval-v1` is the right next topic after
  `mcp-agent-surface`;
- whether V1 should use a deterministic local evaluator rather than an LLM judge
  or full agent loop;
- what raw-access capabilities are fair enough to avoid a strawman baseline;
- which metrics are required for topic completion and which are report-only;
- whether the first report schema is sufficient for future agent-in-the-loop
  evaluation;
- whether the pass/fail bar should require Janus to beat raw access, or only
  require the harness to expose the comparison honestly.

Current proposed V1 direction:

- build a deterministic local eval first;
- use the current fixture corpus as the eval set;
- keep raw access competitive but limited to raw telemetry and simple backend
  selectors, not Janus derived context or expected artifacts;
- measure token cost from serialized payload size, not hand-authored
  `token_cost` fields;
- report Janus improvements and regressions without hiding either;
- treat this milestone as an eval harness milestone, not a product-quality claim
  that Janus wins every fixture.

## Approval And Completion Policy

This topic should seek whole-design approval before implementation.
Phase-by-phase implementation is allowed only if every active reviewer
explicitly approves a named slice in their `Direction Verdict` and states that
any unresolved design questions are outside that slice. Without that explicit
slice approval, no Rust implementation starts.

V1 should distinguish harness correctness from Janus product victory:

- harness correctness is mandatory: both access paths must run under the same
  budget, gold artifacts must be scoring-only, and the report must show wins
  and regressions honestly;
- the proposed milestone success bar is that the first completed run shows
  Janus improving at least one roadmap target metric without hiding regressions,
  and without masking false-causality-trap or auditability failures behind an
  aggregate score;
- the default CLI should exit non-zero for harness, schema, runtime, or
  validation failures, not merely because one raw-access scenario beats Janus;
- `--fail-on-regression` may enforce a stricter required-metric policy for CI,
  release checks, or a later reviewer-approved completion gate.

This preserves the Milestone 8 acceptance criterion in `roadmap.md`: Janus must
improve at least one target metric without hiding regressions in others. If
reviewers want this topic to be "harness only" even when Janus improves no
target metric, that is a roadmap change, not just a local review verdict. The
roadmap should be updated in the same covered formal-doc tree before
implementation proceeds under a harness-only policy.

## Scope

In scope:

- a repeatable local eval command over all fixtures or a selected subset;
- an eval data model for scenario results, access-path outputs, scores, and
  summary metrics;
- a Janus-access adapter that calls the compiled `get_evidence_bundle` path or
  its reviewed equivalent;
- a raw-access baseline adapter that selects raw telemetry under the same budget;
- token-cost measurement from serialized material for both paths;
- scoring against `scenario.json` `ground_truth`, including
  `primary_cause_entity`, `blast_radius`, `not_the_cause`, and
  `innocent_suspect` when present;
- separate reporting for false-causality trap fixtures;
- score dimensions for suspicious-entity accuracy, useful timeline quality,
  false-causality risk, missing-data awareness, auditability, and token cost;
- tests that prove expected/gold artifacts are used only as scoring oracles, not
  as inputs to either access path;
- a JSON and human-readable summary report.

Out of scope:

- using a live LLM, external judge, or hosted agent as the required V1 evaluator;
- production benchmarking, load testing, or storage-cost measurement;
- new OTLP ingest protocols, Collector receivers, persistence, warm memory, or
  compaction;
- changing Evidence IR semantics to make the eval easier;
- dashboard or UI work;
- root-cause prose generation;
- mitigation planning or action execution;
- claiming broad external validity beyond the synthetic fixture corpus.

## Evaluation Shape

V1 should be deterministic and local. It should not depend on network access,
model availability, prompt drift, or human scoring.

The harness should normalize both access paths into a common evaluation shape,
for example:

```rust
struct EvalSubmission {
    scenario_id: String,
    access_path: EvalAccessPath,
    budget: EvalBudget,
    serialized_context: serde_json::Value,
    measured_tokens: u32,
    candidate_entities: Vec<EvalCandidateEntity>,
    timeline_events: Vec<EvalTimelineEvent>,
    evidence_refs: Vec<EvalSourceRef>,
    counter_evidence_refs: Vec<EvalSourceRef>,
    missing_data_refs: Vec<EvalSourceRef>,
}
```

Exact names are flexible. The contract is that both raw access and Janus access
produce comparable investigation material:

- what candidate entities were made visible;
- whether the primary cause is present and ranked well;
- whether known innocent suspects are promoted or suppressed;
- whether useful timeline points are visible;
- whether missing data is surfaced;
- whether source references are resolvable;
- how much budget the material consumes.

The normalized submission is an eval adapter artifact, not a new public Janus
API.

## Access Path A: Janus Evidence

The Janus path should use the reviewed evidence pipeline, not fixture gold.

Preferred runtime path:

```text
scenario manifest question + time window + eval budget
      -> EvidenceQuery
      -> get_evidence_bundle
      -> compiled EvidenceBundle
      -> EvalSubmission
```

If the implementation needs richer compiler outputs such as suspected causes or
next checks, it may call an internal reviewed compiler result path. That is
acceptable only if the path uses the same source and derived context that
`get_evidence_bundle` uses, and does not load expected artifacts as runtime
inputs.

The Janus submission should extract:

- candidate entities from suspected causes when available, otherwise from
  evidence item entities;
- source refs from `EvidenceItem.source_refs`;
- counter-evidence from `EvidenceKind::CounterEvidence` or directions
  `weakens` and `contradicts`;
- missing-data awareness from `EvidenceKind::MissingData` and item-level
  `missing_data`;
- timeline hints from selected evidence item time windows and timeline-derived
  evidence.

The V1 Janus eval query should not set a hard
`min_counter_evidence_items` mandate. Counter-evidence should be surfaced and
scored when the reviewed compiler pipeline selects genuine source-backed
weakening evidence under the normal budget, but the eval harness must not force
the compiler to fabricate one counter item per fixture. This keeps the
false-causality scorer focused on the material the agent would actually receive
and avoids changing Evidence IR behavior to satisfy the eval.

Janus path token cost must be recomputed from the serialized eval payload or
serialized Evidence IR payload. Do not use fixture gold `token_cost` as the eval
measurement.

## Access Path B: Raw Baseline

The raw baseline must be fair enough that a Janus win is meaningful.

The baseline may use:

- fixture `scenario.json` question and time window;
- fixture `input.json` raw source records;
- direct time-window filtering;
- direct entity labels and resource attributes present on raw records;
- direct severity/status filters for logs and traces;
- direct change-event proximity to the incident window;
- direct metric deltas computed from raw metric points;
- simple grouping for records that already share the same raw entity, service,
  route, directly present dependency, or trace id;
- source refs for selected raw records.

The baseline must not use:

- `expected.json` artifacts as input;
- `scenario.json.ground_truth` as input;
- Janus Evidence IR items;
- Janus suspected-cause rankings;
- derived context artifacts such as anomaly windows, log patterns, relationship
  graphs, window comparisons, related anomalies, or compiler scores;
- inferred dependency edges or relationship direction not directly present on a
  raw record;
- fixture-specific hard-coded entity names or failure-class special cases.

The baseline should select a compact raw context pack, not dump the whole
fixture. A reasonable V1 raw pack includes:

1. nearby change events around the incident window;
2. error logs and failed spans in the incident window;
3. high-delta metric series from before/after windows;
4. trace exemplars connected by direct trace/span ids;
5. raw telemetry gaps that overlap the incident window;
6. a dropped-record count when the budget excludes candidate raw records.

The pack should be sorted deterministically and trimmed by measured token cost.
It may be strong at retrieving obvious raw symptoms, but it should not silently
perform Janus's cross-signal reasoning.

For entity scoring fairness, the raw baseline may canonicalize direct resource
ids such as `res:<service>` into the fixture's canonical entity namespace using
only attributes present on the raw `resources` records, such as `service.name`
and `db.system`, plus direct disambiguators such as `rollout`,
`service.instance.id`, `service.version`, and `cluster.name`. This is a
deliberate concession to raw access so scoring does not penalize it for a
namespace mismatch; it must be applied symmetrically to normalized Janus and raw
candidate entities and must not infer relationships or fixture-specific entity
names.

For raw-baseline grouping, "dependency" means only a relationship already
visible inside a raw record, such as span parent/child structure, a client span
with `peer.service`, `db.system`, or similar call attributes, or a trace id that
directly links spans. The baseline must not infer a relationship graph from
cross-signal context, derive dependency direction from topology, or reuse Janus
relationship records.

## Budget Model

Every scenario should run under the same eval budget shape:

```rust
struct EvalBudget {
    max_items: u32,
    max_tokens: u32,
}
```

The default budget should be small enough to create real pressure. The exact
default can be reviewed during implementation, but V1 should start from the
fixture query budget when available, or from a repository-level default such as:

```text
max_items = 6
max_tokens = 1200
```

Token measurement must be owned by the eval harness:

```text
measured_tokens = ceil(compact_serialized_payload_bytes / 4)
```

This mirrors the deterministic estimator already used by the evidence compiler
without treating fixture gold token fields as truth. The eval may also report
raw byte size for easier debugging.

The raw and Janus paths should be measured over comparable serialized payloads.
If Janus uses an Evidence IR envelope and raw uses a raw-context envelope, both
envelopes should include only material the downstream agent or evaluator would
actually receive.

Both adapters must call one shared serialization and measurement helper. That
helper should use compact JSON, no pretty-printing, no fixture-gold token
fields, and the same envelope-inclusion rules for both paths. If a path needs
debug metadata such as dropped-candidate diagnostics, that metadata should be
reported outside the measured agent payload unless both paths expose comparable
diagnostics to the evaluator.

## Scoring Model

V1 should produce per-scenario scores and aggregate summaries. Scores should be
simple, inspectable, and resistant to overfitting.

V1 should split metrics into required metrics and report-only metrics.

Required metrics for the first completion gate:

- suspicious-entity accuracy;
- false-causality risk;
- missing-data awareness;
- auditability;
- token efficiency.

Timeline quality remains structural and report-only in V1. It should still
validate and report whether useful timeline points are visible, chronologically
ordered, and source-backed, but it should not be part of the initial "Janus
improves at least one target metric" bar until reviewers approve a less brittle
timeline scorer.

### Suspicious Entity Accuracy

Inputs:

- `scenario.ground_truth.primary_cause_entity`;
- `scenario.ground_truth.not_the_cause`;
- optional `scenario.ground_truth.innocent_suspect`;
- normalized candidate entity list from each access path.

Suggested scoring:

- full credit when the primary cause is ranked first;
- partial credit when the primary cause appears in the top three or in the
  visible candidate set;
- penalty when a `not_the_cause` or `innocent_suspect` entity is ranked first;
- extra trap penalty when the fixture is marked `false_causality_trap`.

### False-Causality Risk

Inputs:

- false-causality fixture flag;
- `not_the_cause` and `innocent_suspect` ground-truth fields;
- counter-evidence refs and candidate ranking.

Suggested scoring:

- high risk when an innocent suspect appears above the primary cause;
- lower risk when an innocent suspect appears but is paired with
  counter-evidence;
- best result when the innocent suspect is either absent or explicitly weakened
  by source-backed counter-evidence.

False-causality trap fixtures should be reported separately from the aggregate
score so overall averages cannot hide regressions.

### Timeline Quality

Inputs:

- `expected.timeline` when present;
- scenario time window;
- normalized timeline events from each access path.

Suggested scoring:

- event ordering is chronological;
- symptom onset, nearby changes, propagation events, recovery markers, and data
  gaps are visible when the fixture declares them;
- non-causal changes are either absent from top causal candidates or marked with
  weakening context;
- source refs remain available for timeline evidence.

The V1 comparison should be structural. It should not require exact prose
equality with hand-authored timeline text.

### Missing-Data Awareness

Inputs:

- `scenario.failure_class == "missing-data"`;
- `input.telemetry_gaps`;
- expected missing-data evidence where present;
- normalized missing-data refs.

Suggested scoring:

- credit for surfacing telemetry gaps that overlap the incident window;
- credit for reducing confidence or producing an under-determined candidate
  when the primary cause cannot be established;
- penalty for confident concrete-cause ranking when the fixture ground truth is
  `under-determined`.

### Auditability

Inputs:

- normalized source refs;
- fixture reference index;
- selected evidence or raw record refs.

Suggested scoring:

- source-ref coverage ratio;
- resolvable-ref ratio;
- refs distributed across relevant signal families when the scenario requires
  cross-signal evidence;
- penalty for claims or candidates without any source-backed support.

### Token Efficiency

Inputs:

- measured tokens for both paths;
- score dimensions above.

Suggested reporting:

- tokens used;
- useful score per 100 tokens;
- items dropped by each path;
- whether a path exceeded, exactly filled, or stayed under budget.

Token efficiency should not reward under-filled responses that fail to expose
useful evidence.

## Report Format

The command should emit a JSON report and a concise human summary.

Suggested JSON shape:

```json
{
  "schema_version": "comparative-eval/v1",
  "repo_sha": "...",
  "fixture_registry": {
    "schema_version": "fixtures/v1"
  },
  "budget": { "max_items": 6, "max_tokens": 1200 },
  "summary": {
    "fixture_count": 12,
    "janus": {},
    "raw": {},
    "delta": {},
    "false_causality_traps": {},
    "missing_data": {},
    "regression_gates": {}
  },
  "scenarios": [
    {
      "id": "coincidental-deploy-trap",
      "scenario_schema_version": "fixtures/v1",
      "scenario_version": 1,
      "failure_class": "coincidental-correlation",
      "difficulty": "hard",
      "false_causality_trap": true,
      "janus": {},
      "raw": {},
      "comparison": {}
    }
  ]
}
```

Each scenario entry must record the fixture scenario `schema_version` and
`version`. The top-level report must also record the registry `schema_version`.
`repo_sha` ties the run to the committed source tree, but per-scenario versions
make fixture changes visible even when multiple fixtures share the same repo
commit history.

The report should be written under `target/` by default, for example:

```text
target/eval/comparative-eval-v1.json
target/eval/comparative-eval-v1.txt
```

Generated eval reports should not be committed unless a later review explicitly
adds a small stable fixture snapshot. The stable contract should be the report
schema and tests, not a timestamped output file.

The report should make regressions visible. A run where raw access beats Janus
on a fixture is useful information, not an output to hide.

## CLI

Suggested binary:

```bash
cargo run --bin compare_evidence_access -- --all
```

Useful flags:

```text
--fixture <id>            run one fixture
--failure-class <name>    filter by failure class
--difficulty <name>       filter by difficulty
--max-items <n>           override default item budget
--max-tokens <n>          override default token budget
--format json|text        select stdout format
--output <path>           write JSON report
--fail-on-regression      non-zero exit when Janus is worse on required metrics
```

Operational examples and report-reading guidance live in
[`../process/comparative-eval-v1.md`](../process/comparative-eval-v1.md).

For V1, `--fail-on-regression` should gate on aggregate and subgroup health
rather than requiring Janus to win every fixture. The command should fail when:

- aggregate required-score delta is negative beyond a small tolerance;
- false-causality trap fixtures regress as a subgroup or any trap fixture is a
  raw win;
- a required metric regresses in aggregate;
- a non-allowlisted fixture becomes a raw win.

Known raw wins may be allowlisted as expected regressions only when they remain
visible in the report. The initial expected-regression allowlist is
`traffic-shift-hotspot` and `missing-data-gap`; both should stay visible in the
regression-gate summary so they remain pressure to improve Janus rather than
silent exemptions.

The first implementation can keep argument parsing simple. It should still avoid
hard-coded fixture ids or hidden defaults that make the report irreproducible.

## Required Fixtures

V1 should run over all current fixtures by default.

At minimum, tests should include:

- `deploy-bad-rollout` for a straightforward deploy cause;
- `coincidental-deploy-trap` for change-proximity false causality;
- `retry-storm-amplification` for dependency direction and amplification;
- `ambiguous-entity-resolution` for entity ambiguity;
- `missing-data-gap` for honest uncertainty;
- one downstream or dependency scenario that exercises blast radius.

The harness should also be able to group by:

- failure class;
- difficulty;
- false-causality trap flag;
- capability tags.

## Ground Truth And Gold Artifact Use

`scenario.json.ground_truth` and selected `expected.json` artifacts are scoring
oracles only.

Allowed scoring oracle inputs:

- `ground_truth.primary_cause_entity`;
- `ground_truth.blast_radius`;
- `ground_truth.not_the_cause`;
- `ground_truth.innocent_suspect`;
- `expected.timeline`;
- expected artifact source refs for structural comparison and audit checks.

Forbidden runtime inputs:

- ground truth for selecting raw records;
- ground truth for choosing Janus query entities;
- expected artifacts for raw baseline selection;
- expected artifacts for Janus evidence compilation;
- fixture-specific scoring shortcuts that only recognize one scenario id.

Tests must make this boundary explicit. If a helper loads expected artifacts for
scoring, the runtime adapters should not be able to access that helper.

The implementation should make that boundary structural where Rust visibility
allows it. Runtime adapter modules for Janus access and raw access should not
import the scoring-oracle loader. The oracle loader should live behind the
scoring/reporting boundary, with module visibility narrow enough that adapters
cannot call it accidentally. Tests remain a second guard, but the preferred
invariant is that adapter code cannot compile if it tries to use gold
artifacts.

## Relationship To MCP

The Janus path may call the internal `get_evidence_bundle` Rust boundary for
speed and determinism. A full MCP client loop is not required for V1 completion.

However, because Milestone 7 exposed a local MCP surface, this topic should keep
the report compatible with future MCP-based eval:

- the Janus adapter should be replaceable by an MCP adapter;
- the serialized Janus payload should look like what an agent would receive;
- at least one smoke test may assert that the MCP tool output can be normalized
  into the same `EvalSubmission` shape, but this is optional unless reviewers
  require it.

Do not let MCP protocol mechanics dominate the eval design. The milestone is the
comparison, not another agent-surface implementation.

## Implementation Slices After Design Approval

No slice should start until reviewers agree on the design direction or approve
that slice explicitly.

Recommended slices:

1. Eval models and report schema: define `EvalSubmission`, score structs, report
   structs, budget model, shared token estimator, required/report-only metric
   classification, fixture-version fields, and a CLI skeleton that loads the
   fixture corpus and emits an empty-but-valid report.
2. Janus adapter: run the compiled `get_evidence_bundle` path for selected
   fixtures and normalize Evidence IR into eval submissions without reading gold
   artifacts.
3. Raw baseline adapter: select raw telemetry packs under the same budget using
   reviewed raw selectors, group only on directly present raw-record
   dependency links, and normalize them into eval submissions.
4. Scoring: compare both submissions against ground truth and expected timeline
   or audit artifacts; report per-scenario and aggregate metrics; keep timeline
   quality structural/report-only in V1.
5. Regression and trap reporting: add false-causality trap grouping, missing-data
   grouping, and optional `--fail-on-regression` behavior.
6. Documentation and examples: add a concise command example and explain how to
   read the report without treating a single score as the whole product claim.

These are implementation slices, not separate milestones.

## Tests

Add tests that prove:

- the eval command can load the fixture corpus;
- default fixture selection includes all current fixtures;
- fixture filters by id, failure class, difficulty, and trap flag work;
- the Janus adapter does not read expected artifacts as runtime input;
- the raw baseline does not read expected artifacts or ground truth as runtime
  input;
- raw baseline selection is deterministic;
- raw baseline dependency grouping uses only links directly present in raw
  records, not inferred or Janus-derived relationships;
- both paths enforce the same item and token budgets;
- measured token cost is computed from serialized eval material through one
  shared helper used by both paths;
- report output records the fixture registry `schema_version` and each
  scenario's `schema_version` and `version`;
- scoring finds the primary cause when it appears at rank 1;
- scoring penalizes `not_the_cause` or `innocent_suspect` at rank 1;
- false-causality traps are summarized separately;
- missing-data scenarios reward explicit missing-data awareness;
- auditability scoring validates source refs through the fixture reference
  index;
- required metrics are separated from report-only metrics, with timeline
  quality report-only in V1;
- runtime adapters cannot access the scoring-oracle loader for `expected.json`
  or `scenario.json.ground_truth`;
- generated reports validate against the V1 report shape;
- the existing fixture validation, evidence compiler, MCP, and query tests
  continue to pass.

Existing verification should continue to pass:

```bash
cargo fmt
cargo test
cargo clippy --all-targets --all-features
cargo run --bin validate_fixtures
```

## Definition Of Done

This topic is complete when:

- a repeatable local comparative eval command exists;
- the command can run over all current fixtures or a selected subset;
- Janus and raw access paths run under the same budget model;
- the raw baseline is documented, deterministic, and not a strawman full dump or
  arbitrary `LIMIT N`;
- Janus access uses compiled Evidence IR rather than fixture gold;
- ground truth and expected artifacts are used only as scoring oracles;
- the report includes per-scenario scores and aggregate summaries;
- the report records the fixture registry `schema_version` and each scenario's
  `schema_version` and `version`;
- required metrics are separated from report-only metrics, with timeline
  quality structural/report-only in V1;
- false-causality trap fixtures are reported separately;
- measured token cost is computed from serialized material for both paths using
  one shared helper;
- source-ref auditability is scored;
- missing-data awareness is scored;
- Janus improves at least one required target metric without hiding regressions
  in the other required metrics;
- the report can show Janus wins and regressions honestly;
- no LLM judge, new ingest protocol, persistence layer, dashboard, warm memory,
  or RCA prose generator is required;
- `cargo fmt`, `cargo test`, `cargo clippy --all-targets --all-features`, and
  `cargo run --bin validate_fixtures` pass.

## Review Focus

Reviewers should focus on:

1. Whether `comparative-eval-v1` is the right next topic after the MCP agent
   surface.
2. Whether the V1 evaluator should stay deterministic, or whether an
   agent-in-the-loop path is necessary now.
3. Whether the raw-access baseline is fair enough to be meaningful without
   becoming another Janus evidence compiler.
4. Whether raw dependency grouping is constrained tightly enough to direct
   raw-record links.
5. Whether token budget is measured on comparable serialized payloads through a
   shared helper.
6. Whether the score dimensions map cleanly to Janus's core claim: accuracy,
   false-causality reduction, auditability, missing-data awareness, and token
   efficiency.
7. Whether timeline quality should remain structural/report-only in V1.
8. Whether the report schema pins fixture versions strongly enough for
   reproducibility.
9. Whether the report can expose regressions while preserving the roadmap's
   "improve at least one target metric" acceptance bar.
