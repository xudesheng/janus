# Evidence Compiler Ranking Design

Status: design for the `evidence-compiler-ranking` topic.

This document defines the Milestone 6 Evidence Compiler V1 slice. It is grounded
in [`what_and_why.md`](what_and_why.md), [`roadmap.md`](roadmap.md),
[`evidence-ir-schema.md`](evidence-ir-schema.md),
[`get-evidence-bundle-contract.md`](get-evidence-bundle-contract.md),
[`hot-context-store.md`](hot-context-store.md),
[`entity-resolver-confidence.md`](entity-resolver-confidence.md),
[`derived-context-baseline.md`](derived-context-baseline.md),
[`fixture-otel-simulator.md`](fixture-otel-simulator.md), and
[`otel-ingest-prototype.md`](otel-ingest-prototype.md). If this document
conflicts with `what_and_why.md`, the canonical design doc wins and this
document should be corrected.

## Why This Topic Is Next

The simulator and OTel JSON ingest topics are complete. Janus can replay
fixture-owned source telemetry and ingest a local OTLP JSON payload into the hot
store boundary.

The hot context store, entity resolver, relationship builder, and derived
context baseline are also complete. Janus now has source-backed raw records,
entities, relationships, anomaly windows, log patterns, timelines, related
anomalies, and window comparisons.

The remaining gap before an agent-facing surface is evidence compilation:

```text
source records + derived context + query intent
      -> generated EvidenceItem candidates
      -> causal-suspicion ranking with counter-evidence
      -> token-budget selection
      -> EvidenceBundle + suspected causes + next checks
```

Jumping directly to MCP tools would expose the old fixture-backed
`get_evidence_bundle` stub. Expanding OTel ingest or persistence now would make
the demo wider without making the agent evidence better. The next topic should
therefore be `evidence-compiler-ranking`.

## Purpose

This topic should replace the hand-authored bundle path with the first
source-backed compiler that can generate and rank Evidence IR from the current
fixture corpus.

The compiler should answer:

- which source and derived artifacts should become evidence items;
- which items support, weaken, contradict, or stay neutral toward a hypothesis;
- which entity hypotheses are plausible suspected causes;
- which high-prior suspects should be rejected by counter-evidence;
- which items fit into the requested token budget;
- which items were dropped because of budget;
- which missing data should reduce confidence or become a next check;
- which next checks would most improve or confirm the investigation.

This topic is not an RCA agent. It should output structured, auditable evidence
and ranked suspicion records, not narrative root-cause prose.

## Design Review Gate

No Rust implementation should start for this topic until every active reviewer
has agreed on the design direction in their `Direction Verdict`.

Reviewers should explicitly decide whether this document draws the right
boundary between Milestone 6 and the later agent surface. If reviewers want the
implementation split into phases, their verdict should name the approved phase.

The first review round should resolve these decisions before coding:

- whether `get_evidence_bundle` should switch from fixture-gold output to the
  compiler in this topic, while keeping the existing public request and response
  contract stable;
- whether `suspected_causes` and `next_checks` should be compiled and compared
  now as internal/store outputs, before MCP exposes them later;
- whether the first token-cost estimator should be a deterministic local
  approximation rather than an LLM tokenizer;
- whether expected fixture evidence is a comparison oracle only, never a
  compiler input;
- whether causal classification for nearby changes belongs here rather than in
  the Milestone 5B timeline builder.

## Scope

In scope:

- evidence item generation from change events, anomaly windows, log patterns,
  trace exemplars, dependency relationships, previous incidents, counter-
  evidence, and missing data;
- a compiler entry point that consumes a query, a hot store, and derived context;
- replacement of the fixture-backed `get_evidence_bundle` return path with
  compiled output, or an equivalent reviewed transition path;
- deterministic scoring that separates evidence strength from causal
  suspicion;
- token-cost accounting and whole-item budget selection;
- dropped-item reporting through bundle budget fields and an internal compiler
  report;
- `suspected_causes` generation and ranking;
- initial `suggest_next_checks` generation;
- insertion or inspectable exposure of evidence items, suspected causes, and
  next checks through the hot-store derived-record boundary;
- comparison against fixture gold `evidence_bundle`, `suspected_causes`, and
  `next_checks`;
- tests over the current fixture corpus.

Out of scope:

- MCP tools or external agent APIs;
- new OTel protocols, OTLP/HTTP, OTLP/gRPC, or Collector receiver behavior;
- durable persistence or warm/cold compaction;
- production-grade anomaly detection beyond the current derived context inputs;
- LLM-generated explanations;
- automatic mitigation or action execution;
- dashboard/UI work;
- real warm memory beyond fixture-provided `prior_incidents`;
- broad privacy enforcement beyond preserving existing `privacy_scope` fields.

## Inputs

The compiler should consume:

- `EvidenceQuery`, including the nested `EvidenceQueryIntent` question and/or
  hypothesis, time window, entities, budget, counter-evidence requirement,
  raw-ref requirement, freshness preference, and privacy scope;
- raw source records from `HotContextStore`;
- derived entities and relationships from Milestone 5A;
- derived context from Milestone 5B, including anomaly windows, log patterns,
  timeline events, related anomalies, and window comparisons;
- fixture-provided `prior_incidents` records when available;
- fixture scenario metadata and ground truth only for tests and comparison, not
  for runtime compilation.

`EvidenceQuery.scenario_id` is a current fixture-selection adapter, not a
production query primitive. It may still be used by fixture helpers during this
topic, but the compiler boundary should be defined around query intent, source
records, and derived context rather than around scenario id.

The compiler must not use these expected artifacts as inputs:

- `expected.evidence_bundle`;
- `expected.suspected_causes`;
- `expected.next_checks`.

Those artifacts are comparison targets only. Loading them into the compiler
would make the topic a copier rather than an evidence compiler.

## Outputs

The primary output remains the stable Evidence IR response:

```rust
EvidenceBundle {
    question,
    hypothesis,
    time_window,
    budget,
    items,
}
```

The compiler should also produce an internal report, for example:

```rust
struct EvidenceCompilation {
    bundle: EvidenceBundle,
    suspected_causes: Vec<SuspectedCause>,
    next_checks: Vec<NextCheck>,
    report: EvidenceCompilationReport,
}

struct EvidenceCompilationReport {
    generated_items: usize,
    selected_items: usize,
    dropped_items: Vec<DroppedEvidenceItem>,
    requirement_failures: Vec<String>,
}
```

Exact Rust names are flexible. The required contract is:

- every selected evidence item is a valid `EvidenceItem`;
- every selected evidence item has non-empty, resolvable `source_refs`;
- evidence item ids are deterministic;
- item ordering is deterministic;
- `EvidenceBudget.tokens_used` is computed by the compiler, not copied from
  fixture gold;
- `EvidenceBudget.items_dropped` reflects generated but unselected candidates;
- `suspected_causes` link to selected or generated evidence item ids;
- `next_checks` are generated from weak hypotheses, gaps, or confirmation
  opportunities;
- outputs can be compared against fixture gold without making gold a runtime
  input.

## Compiler Entry Point

Suggested shape:

```rust
pub fn compile_evidence(
    query: &EvidenceQuery,
    store: &HotContextStore,
    derived: &DerivedContext,
) -> Result<EvidenceCompilation, EvidenceCompileError>
```

The implementation may add a fixture helper:

```rust
pub fn compile_fixture_evidence(
    query: EvidenceQuery,
) -> Result<EvidenceCompilation, EvidenceCompileError>
```

The helper can load the selected fixture, replay source telemetry into a fresh
store, derive entity and derived context, and then call `compile_evidence`.

The public `get_evidence_bundle(EvidenceQuery)` boundary should keep its
request and response types. The current path is not only a gold-bundle loader:
it already validates the query, checks budget fit, enforces raw-ref and
counter-evidence requirements, resolves every returned source ref through
`HotContextStore`, and checks that the query selects hot-context records. Slice
6 should preserve those acceptance checks and swap the bundle source from
fixture gold to compiled evidence.

This topic should either:

1. make `get_evidence_bundle` call the compiler and return
   `EvidenceCompilation.bundle`; or
2. add a clearly named compiler-backed path and keep the fixture-gold stub only
   as a temporary compatibility path approved by review.

The preferred outcome is option 1. Milestone 6 should be the point where
`get_evidence_bundle` becomes compiled evidence rather than fixture-gold lookup.
If a temporary gold path coexists during Slice 6, it should be explicitly named
as a compatibility path and removed or quarantined by the end of that slice.

## Candidate Evidence Generation

The compiler should generate a broad candidate set before ranking and budget
selection. Candidate generation should be source-backed and deterministic.

For V1, selected evidence item ids should follow the current fixture convention
`ev-1`, `ev-2`, and so on, assigned after final selection in selected-output
order. This keeps exact selected-id comparison meaningful without a fixture id
migration. Internal candidate ids may use a separate deterministic scheme, but
only selected `ev-N` ids are part of the public `EvidenceBundle` comparison.

Slice 2 may expose candidate generation directly as an internal helper such as
`generate_evidence_candidates(input) -> Vec<EvidenceCandidate>`. These candidates
use internal ids such as `cand-001` and are not public selected bundle items.
Later selection slices must assign the final selected `ev-N` ids and recompute
estimator-owned token fields for that final selected output.

After Slice 3, `generate_evidence_candidates` returns generated candidates with
compiler-owned evidence-strength scores applied. `score_evidence_candidates` is
also exposed as an internal helper so tests and later slices can rescore a
candidate set explicitly. The ids remain internal `cand-*` ids; this is still
not final bundle selection.

### Metric Anomaly Evidence

Inputs:

- `DerivedAnomalyWindow`;
- metric series source refs;
- window comparison deltas.

Minimum behavior:

- create `EvidenceKind::MetricAnomaly` items for strong anomaly windows;
- include both `metric` and `anomaly_window` source refs when available;
- use anomaly magnitude, detector confidence, coverage, and query relevance to
  set evidence strength;
- keep `confidence.detector` or similar dimensions distinct from strength;
- use `direction: supports` when the anomaly supports a plausible hypothesis;
- use `direction: weakens` or `contradicts` when a flat or healthy metric rules
  out a suspect.

### Log Pattern Evidence

Inputs:

- `DerivedLogPattern`;
- exemplar logs;
- related traces when available.

Minimum behavior:

- create `EvidenceKind::LogCluster` items for new, recurring, or high-severity
  patterns;
- source refs should include `log_pattern` and representative `log` refs when
  useful;
- strength should consider severity, count, first-seen alignment, and exemplar
  quality;
- log patterns should not become root-cause prose. They remain operational
  facts supporting or weakening hypotheses.

### Change Event Evidence

Inputs:

- change records;
- timeline events;
- resolved entities and relationships;
- anomaly onset times.

Minimum behavior:

- create `EvidenceKind::ChangeEvent` items for changes near the incident window;
- score time alignment from change timestamp to symptom/anomaly onset;
- penalize changes that occur after onset;
- penalize changes on entities outside the active symptom or dependency path;
- preserve change source refs;
- do not treat change proximity alone as a causal conclusion.

### Trace Exemplar Evidence

Inputs:

- trace and span records;
- span status/error attributes;
- relationships derived from spans;
- log or anomaly context when linked.

Minimum behavior:

- select representative traces that show the failure path or a counterexample;
- include span-level refs when possible, not only trace-level refs;
- use trace exemplars to connect symptoms to dependency direction;
- prefer compact claims over dumping trace payloads into the evidence item.

### Dependency Edge Evidence

Inputs:

- resolved relationships;
- relationship evidence refs;
- related anomalies.

Minimum behavior:

- create dependency evidence when direction matters to a hypothesis;
- distinguish upstream symptom propagation from downstream dependency cause;
- include relationship refs and supporting trace refs;
- do not inflate a relationship into causality without time alignment and
  supporting symptoms.

### Previous Incident Evidence

Inputs:

- fixture-provided `prior_incidents`;
- related anomaly similarity records.

Minimum behavior:

- create `EvidenceKind::PreviousIncident` items only from available
  `prior_incidents`;
- include `prior_incident` refs;
- keep similarity as evidence strength, not proof of recurrence;
- do not require real warm memory in this topic.

### Missing Data Evidence

Inputs:

- telemetry gaps;
- missing or low-coverage derived context;
- query requirements.

Minimum behavior:

- create `EvidenceKind::MissingData` items when a gap overlaps an incident peak
  or candidate-cause validation window;
- include telemetry-gap, change, and log source refs where available;
- populate `missing_data` with concrete unavailable signals;
- lower confidence for related evidence instead of hiding the gap.

### Counter-Evidence

Counter-evidence is not a separate late filter. It should be generated at the
same time as supporting evidence.

Examples:

- an innocent deployed service has flat error rate;
- incident onset precedes a suspected deploy;
- downstream DB latency is flat while service errors rise;
- dependency direction contradicts a candidate cause;
- key telemetry is missing, so a confident root-cause claim is weakened.

Counter-evidence can be represented as:

- `kind: counter_evidence`; or
- another kind with `direction: weakens` or `direction: contradicts`.

The compiler should prefer explicit `counter_evidence` for items whose main
purpose is to reject a hypothesis.

## Scoring Model

The scoring model must separate two concepts:

- evidence strength: how strong this item is as an operational fact;
- causal suspicion: how plausible an entity or change is as a cause.

Evidence strength may consider:

- source-ref quality;
- detector confidence;
- magnitude of change;
- exemplar specificity;
- entity-resolution confidence;
- recency and freshness;
- coverage and missing data.

Causal suspicion may consider:

- time alignment;
- dependency direction;
- blast radius;
- change proximity;
- error signature specificity;
- related anomaly lag;
- previous incident similarity;
- counter-evidence;
- missing-data uncertainty.

The compiler should store causal dimensions in `confidence` maps or suspected
cause records, but it must not overwrite `EvidenceItem.strength` with a root-
cause probability.

Slice 3 implements evidence-strength scoring before suspected-cause ranking.
Candidate `strength` is computed from source-specific dimensions such as
`source_ref_quality`, `magnitude`, `coverage`, `severity`, `volume`,
`exemplar_quality`, `span_specificity`, `time_alignment`,
`relationship_confidence`, `signature_similarity`, `gap_materiality`, and
`contradiction_quality`. For metric anomalies, `confidence.detector` is retained
as a separate detector dimension and is not copied into `strength`.

Slice 3 also introduces causal suspicion scoring for suspected causes. This
score is computed from linked supporting evidence, counter-evidence penalties,
source-family causal weights, runtime-child rollups such as `pod:` to owning
`service:`, and material missing-data uncertainty. It is deliberately separate
from any individual evidence item's `strength`.

Direction decision after Slice 3: suspected-cause ranking should be judged by
structural outcomes, not by reproducing every hand-authored reason token. Before
suspected causes become a gold-gated final comparison in Slice 6, ranking
heuristics must move away from fixture-specific entity-name multipliers and
toward structural signals such as the suspect's own anomaly state, dependency
direction, onset ordering, blast radius, change proximity, counter-evidence,
and missing-data uncertainty. Reason tokens should be derived from structured
source content such as signal names, log templates, change kinds, relationships,
and detected gaps.

## Suspected Causes

`expected.suspected_causes` already exists in the fixture corpus. This topic
should give that artifact a concrete generation path.

Only the hot-store `StoredRecordKind::SuspectedCause` category exists today.
The runtime `SuspectedCause` struct, gold parser, comparison helper, and store
payload projection are all Slice 1 work.

Suggested runtime shape:

```rust
struct SuspectedCause {
    rank: u32,
    entity: String,
    hypothesis: String,
    score: UnitInterval,
    reasons: Vec<String>,
    supporting: Vec<String>,
    counter: Vec<String>,
    note: Option<String>,
    trap_note: Option<String>,
}
```

Minimum behavior:

- create candidates from changed entities, anomalous entities, dependency
  entities, prior incidents, and an `under-determined` candidate when evidence
  is insufficient;
- link candidates to supporting and counter evidence item ids;
- rank by causal suspicion, not by evidence item order;
- rank obvious false-causality traps low when counter-evidence is strong;
- allow an uncertainty candidate to outrank a weak concrete cause when telemetry
  gaps make diagnosis under-determined;
- insert or expose suspected causes as inspectable store records, even before
  MCP exposes `rank_suspected_causes`.

Slice 3 exposes `rank_suspected_causes_from_candidates(input, candidates)` as an
internal helper. It ranks suspected causes from generated candidate ids, links
supporting and counter evidence with `cand-*` ids, emits deterministic reason
category tokens, lowers suspects whose counter score dominates support, and
emits an `under-determined` suspect when missing-data candidates materially
weaken diagnosis. It does not yet rewrite links to selected `ev-*` ids because
token-budget selection is Slice 4.

## Next Checks

`expected.next_checks` should also get a concrete generation path in this topic.

Only the hot-store `StoredRecordKind::NextCheck` category exists today. The
runtime `NextCheck` struct, gold parser, comparison helper, and store payload
projection are all Slice 1 work.

Suggested runtime shape:

```rust
struct NextCheck {
    action: String,
    rationale: String,
    expected_signal: String,
}
```

Minimum behavior:

- generate checks from missing data, weak top candidates, strong confirmation
  opportunities, and dangerous false-causality traps;
- prefer checks that discriminate between hypotheses;
- include checks that avoid bad mitigations when counter-evidence is strong;
- keep action text deterministic for the fixture corpus;
- insert or expose next checks as inspectable store records, even before MCP
  exposes `suggest_next_checks`.

Examples:

- backfill missing metrics across a telemetry gap;
- inspect a dependency log source outside the missing pipeline;
- confirm rollback effect only when deploy evidence is strong;
- avoid rolling back an innocent service when flat metrics and timing contradict
  the deploy hypothesis.

Slice 5 exposes `suggest_next_checks(input, bundle, suspected_causes)` through
`compile_evidence`. The generator is deterministic and derives checks only from
selected evidence, suspected-cause links, counter-evidence, and missing-data
state. It emits at most three checks in priority order:

1. recover or inspect selected missing-data evidence;
2. validate selected counter-evidence for false-causality risks;
3. confirm the top suspected cause, or gather another independent signal when
   the top score is weak.

`expected_signal` remains an exact category token. The V1 vocabulary includes
tokens such as `metric_anomaly`, `log_cluster`, `change_event`,
`compare_windows`, `relationship`, `find_related_anomalies`, `profile_hotspot`,
and `trace`.

## Token Budget Selection

Token budget is a query constraint, not a presentation detail.

Design decision: token costs are estimator-owned fixture fields, not exact
comparison against the currently hand-authored token numbers.

Slice 1 should adopt a deterministic local estimator and then regenerate
fixture `token_cost`, `tokens_used`, and other token-budget expected fields from
that estimator as formal fixture data. That fixture migration is not "gold as
runtime input"; it makes the fixture oracle reflect the reviewed estimator.
Until that migration exists, token fields are not an exact comparison target
for generated compiler output.

The first estimator should use this shape:

```text
estimated_tokens = ceil(canonical_evidence_item_payload_json_bytes / 4)
```

Where `canonical_evidence_item_payload_json_bytes` means compact JSON bytes for
a deterministic token payload derived from the selected `EvidenceItem`:

- no incidental whitespace;
- stable struct field order;
- map keys sorted, using `BTreeMap` or an equivalent canonical representation;
- array order exactly as selected by the compiler;
- `token_cost` omitted from the estimator payload to avoid self-reference;
- deterministic number formatting from the chosen JSON serializer.

The V1 serializer is `serde_json::to_vec` over the canonical estimator payload.
Slice 1 tests must pin at least one payload byte count so token costs cannot
drift silently under dependency or payload-shape changes.

The exact estimator can change in a later reviewed round, but the V1 estimator
must be:

- deterministic;
- tested;
- independent of fixture gold `token_cost`;
- applied before final selection;
- reflected in each selected item's `token_cost`.

For V1, `EvidenceBudget.tokens_used` is the sum of selected item `token_cost`
values. Bundle envelope fields such as `question`, `time_window`, and `budget`
are not counted in the selection budget. The later comparative eval may still
measure full serialized response size separately.

Selection should operate on whole evidence items. It should not truncate claims,
entities, source refs, or missing-data lists to squeeze an item into budget.

Selection constraints:

- respect `max_items`;
- respect `max_tokens`;
- satisfy `require_raw_refs` by only selecting source-backed items;
- satisfy `require_counter_evidence` or return a clear unsatisfied-requirement
  error when impossible;
- preserve at least one high-value support item for the top plausible
  hypothesis when budget allows;
- preserve at least one high-value counter item for the top false-causality risk
  when budget allows;
- preserve missing-data evidence when it is material to confidence;
- prefer hypothesis-discriminating items over redundant noisy evidence.

Ordering should be deterministic after selection:

1. strongest support for the top candidate;
2. key symptom or anomaly evidence;
3. trace or log exemplars that explain mechanism;
4. counter-evidence for plausible false leads;
5. missing-data evidence;
6. previous-incident evidence.

If this ordering conflicts with a stronger reviewed local rule, document the
rule in code and tests.

Slice 4 exposes `compile_evidence(query, store, derived)` and
`select_evidence_compilation(input, candidates, suspected_causes)` as internal
compiler paths. The selector:

- starts from scored `cand-*` candidates;
- enforces `max_items`, `max_tokens`, and `reserve_tokens_for_raw_refs`;
- selects whole evidence items only;
- forces the requested number of counter-evidence items or returns a compiler
  requirement error;
- assigns final selected `ev-N` ids in deterministic selected-output order;
- recomputes token costs after final id assignment;
- remaps selected suspected-cause evidence links from `cand-*` to `ev-*`;
- reports unselected candidates as dropped with a stable reason.

This remains internal compiler output. Store insertion, next-check generation,
and `get_evidence_bundle` routing are still later slices.

## False-Causality Guard

False causality is the core failure mode for this milestone.

The compiler should explicitly reject or downgrade causal narratives when:

- symptom onset precedes the suspected change;
- the changed entity is not on the active symptom or propagation path;
- the suspected entity's own metrics stay flat;
- dependency direction makes the proposed cause unlikely;
- the evidence comes from a single weak exemplar;
- telemetry gaps hide the key validation window;
- a previous incident is similar but not aligned with current evidence.

False-causality trap fixtures must produce:

- low-ranked innocent suspects;
- explicit counter-evidence linked to source refs;
- a `trap_note` or equivalent note where the fixture expects one;
- no confident root-cause wording when the right answer is uncertainty.

## Fixture Comparison Contract

Add comparison helpers, for example:

```rust
compare_compiled_evidence(case, compilation) -> EvidenceCompilationComparison
```

It should compare:

- `EvidenceBundle.question` and `hypothesis`;
- bundle time window;
- selected item ids and ordering;
- item claim, kind, direction, strength, entities, time window, source refs,
  freshness, missing data, privacy scope, confidence dimensions, and notes;
- token budget fields, with compiler-owned token estimates;
- generated suspected causes and their ranks, scores, reasons, supporting ids,
  counter ids, notes, and trap notes;
- generated next checks.

Gold fixture artifacts are the semantic comparison target for the current
corpus. The compiler may generate extra candidates internally, but selected
output should be budgeted and deterministic. Extra unselected candidates should
appear only in the internal report.

Comparison mode must be explicit per field family:

- Exact: bundle `question` and `hypothesis` echo, bundle time window, selected
  item ids and ordering, item kind, direction, freshness, privacy scope,
  source refs, missing-data entries, suspected-cause rank/entity/supporting ids
  and counter ids, and next-check ordering.
- Set or ordered-structural: item `entities`, using documented ordering when
  the compiler owns ordering and set comparison when order is not semantically
  meaningful.
- Exact category token: next-check `expected_signal`, which is a category token
  such as `metric_anomaly` or `log_cluster`, not free-form prose.
- Structural category subset: suspected-cause `reasons` must be non-empty when
  gold declares reasons and must be drawn from a derivable category vocabulary.
  Full exact-set equality is not a stable acceptance target until the compiler
  owns the reason vocabulary and the fixtures are migrated to it.
- Numeric tolerance: item `strength`, confidence dimensions, suspected-cause
  score, and other derived numeric confidence or score fields.
- Estimator-owned exact after fixture migration: item `token_cost`,
  `EvidenceBudget.tokens_used`, and `EvidenceBudget.items_dropped`.
- Text structural by default: `claim`, suspected-cause `hypothesis`, `note`,
  `trap_note`, next-check `action`, and `rationale` must be deterministic,
  non-empty where required, and anchored to the compared entities, source refs,
  evidence ids, or reason/check categories. They should not require verbatim
  equality with hand-authored gold prose unless a later reviewed slice
  introduces compiler-owned templates and migrates the fixtures to those
  templates. Until field-specific anchoring checks are implemented, the
  comparison shell treats text-structural equality as non-empty required text
  plus tracked non-blocking text differences.

Before Slice 6 makes selected evidence items gold-gated, the project must
confirm whether selected item ids and ordering remain exact comparison targets
or move to a structural presence/order check. This decision should be explicit
so final selection does not become fixture-tuned by accident.

The comparison must fail if:

- selected evidence uses missing or unresolved source refs;
- generated output copies gold-only fields without source-backed inputs;
- counter-evidence expected by a false-causality fixture is absent;
- missing-data fixtures produce confident cause claims without reporting the
  gap;
- token budget fields are copied from fixture gold instead of recomputed;
- `suspected_causes` or `next_checks` are missing for fixtures that declare the
  corresponding capability.

## Store Integration

The hot store already has record kinds for:

- `EvidenceItem`;
- `SuspectedCause`;
- `NextCheck`.

This topic should insert or expose compiled records through that boundary.

Minimum expectations:

- evidence item records use the evidence item id as key;
- suspected cause records use a stable key such as `suspected-cause:<rank>`;
- next check records use a stable key such as `next-check:<rank>`;
- compiled records do not become raw source records;
- source refs inside evidence items continue to resolve through the store;
- store insertion is optional only if a reviewer approves an equivalent
  inspectable compiler result path.

Slice 5 exposes `insert_evidence_compilation(store, compilation)`. It inserts
selected evidence items, suspected causes, and next checks through
`HotContextStore::insert_record` using the store's derived-record kinds. These
records are selectable and inspectable, but `StoredRecordKind::is_raw_source`
continues to exclude them, so compiled output does not pollute raw source
records.

## Implementation Slices After Design Approval

No slice should start until reviewers agree on the design direction or approve
that slice explicitly.

Recommended slices:

1. Compiler model and comparison shell: define `EvidenceCompilation`,
   suspected-cause and next-check runtime types, comparison structs, comparison
   modes, token estimator, errors, and tests that prove gold is only a
   comparison target. This slice also owns the fixture token-field migration
   needed to make estimator-owned token comparison exact.
2. Candidate generation: generate source-backed EvidenceItem candidates from
   changes, anomaly windows, log patterns, traces, dependency edges, prior
   incidents, missing data, and counter-evidence. This slice exposes internal
   candidates for later scoring and selection, but does not yet perform final
   ranking, token-budget selection, selected `ev-N` assignment, suspected-cause
   ranking, next-check generation, store insertion, or `get_evidence_bundle`
   integration.
3. Scoring and suspected causes: add evidence-strength dimensions, causal
   suspicion scoring, false-causality penalties, and suspected cause ranking.
   This slice keeps `cand-*` ids and does not perform final token-budget
   selection, selected `ev-N` assignment, next-check generation, store
   insertion, or `get_evidence_bundle` integration.
4. Token budget selection: compute deterministic token costs, select whole
   items under `max_items` and `max_tokens`, report dropped candidates, and
   enforce counter-evidence requirements. This slice also assigns selected
   `ev-N` ids and remaps selected suspected-cause links, but does not generate
   next checks, insert compiled records, or route `get_evidence_bundle`.
5. Next checks and store integration: generate deterministic next checks and
   insert evidence, suspected-cause, and next-check records without polluting
   raw source records. This slice does not route `get_evidence_bundle` through
   the compiler.
6. `get_evidence_bundle` integration and full-corpus verification: route the
   public query path through compiled evidence while preserving the existing
   query validation, budget, raw-ref, counter-evidence, source-ref, and
   query-context acceptance checks; compare against fixture gold; and remove or
   quarantine the old fixture-gold bundle source.

The topic is complete only when the Definition Of Done below is met or
reviewers explicitly narrow the milestone.

## Tests

Add tests that prove:

- the compiler does not read expected `evidence_bundle`, `suspected_causes`, or
  `next_checks` as inputs;
- generated evidence items validate as Evidence IR;
- generated evidence source refs resolve through the hot store;
- metric anomaly, log pattern, change event, trace exemplar, dependency,
  previous incident, missing-data, and counter-evidence generation each have
  focused coverage;
- evidence strength and causal suspicion are not conflated;
- token costs are computed locally and not copied from fixture gold;
- budget selection drops whole items and reports dropped candidates;
- `require_counter_evidence` is enforced;
- false-causality trap fixtures rank innocent high-prior suspects low with
  explicit counter-evidence;
- missing-data fixtures surface uncertainty and can rank `under-determined`
  above a weak concrete cause;
- suspected causes and next checks compare against current fixture gold;
- compiled records do not become raw source records;
- the existing derived-context, entity, hot-store, simulator, OTLP ingest,
  fixture-validation, and query tests continue to pass.

Existing verification should continue to pass:

```bash
cargo fmt
cargo test
cargo clippy --all-targets --all-features
cargo run --bin validate_fixtures
```

## Definition Of Done

This topic is complete when:

- `compile_evidence` or an equivalent reviewed compiler boundary exists;
- `get_evidence_bundle` returns compiler-generated bundles, or reviewers have
  approved a clearly named temporary compiler-backed path;
- evidence items are generated from source and derived context rather than
  copied from fixture gold;
- evidence strength is distinct from causal suspicion;
- token cost and budget selection are compiler-owned and tested;
- dropped candidate reporting exists;
- false-causality trap fixtures produce explicit counter-evidence and low ranks
  for innocent suspects;
- missing-data fixtures preserve uncertainty and avoid confident unsupported
  causes;
- `suspected_causes` and `next_checks` have concrete generation paths;
- compiled evidence, suspected causes, and next checks are inspectable through
  store records or an approved equivalent result path;
- source refs for selected evidence resolve through the hot store;
- no MCP, dashboard, new ingest protocol, persistence layer, warm memory, or
  mitigation execution is introduced;
- `cargo fmt`, `cargo test`, `cargo clippy --all-targets --all-features`, and
  `cargo run --bin validate_fixtures` pass.

## Review Focus

Reviewers should focus on:

1. Whether `evidence-compiler-ranking` is the right next topic after
   `derived-context-baseline`, given that simulator and OTel JSON ingest are
   already complete.
2. Whether the compiler boundary is narrow enough to avoid MCP, persistence,
   production ingest, and dashboard work.
3. Whether expected fixture evidence is clearly comparison-only and never a
   compiler input.
4. Whether the design separates evidence strength from causal suspicion strongly
   enough.
5. Whether the false-causality guard is concrete enough for trap fixtures.
6. Whether token budget behavior is semantic selection rather than `LIMIT N`.
7. Whether `suspected_causes` and `next_checks` should be internal/store outputs
   in this milestone before the later agent surface exposes them.
