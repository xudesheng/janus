# Fixtures: Janus Incident Corpus Scheme

This document defines how Janus incident **fixtures** are structured, named,
stored, and registered, so any contributor (human or agent) can add new ones
consistently. The fixture data itself lives under the top-level
[`fixtures/`](../../fixtures) directory; this doc is the spec for it.

It is grounded in [`docs/core/what_and_why.md`](../core/what_and_why.md) (the
canonical design) and [`docs/core/evidence-spine.md`](../core/evidence-spine.md)
(the first implementation plan). Where they disagree with this doc, they win and
this doc should be corrected.

## Why fixtures exist

Janus is greenfield: there is no storage engine, no derivation pipeline, and no
query API yet. What there *is* is a contract — the Evidence IR and a set of
investigation primitives. Fixtures pin that contract down with concrete examples
*before* the code exists, so that:

1. **Feature work has a target.** Each new capability (entity resolution,
   anomaly windows, `get_evidence_bundle`, the false-causality guard, …) can be
   built against gold input/output pairs instead of a vague description.
2. **The Evidence IR stays honest.** If a realistic incident's gold evidence
   bundle can't be expressed in the IR, the IR is wrong. Fixtures surface that
   early (the forcing-function role described in `evidence-spine.md`).
3. **The MVP bet becomes measurable.** `what_and_why.md` ("初始赌注", "评估标准")
   stakes Janus on a comparison: same agent, same incident — does Janus put
   fewer, more accurate, more auditable evidence into context than raw access?
   These fixtures are the seed corpus for that eval.

A fixture is therefore an **(input, gold-output) pair**: synthetic OTel-shaped
telemetry plus the derived artifacts a correct Janus pipeline should produce.

## Hard rules

1. **Self-contained.** A fixture must not reference, quote, or copy anything from
   the git-ignored `references/` directory, any external dataset, paper, or
   product. All telemetry is original synthetic data. (Consistent with
   `AGENTS.md`: do not cite local-only notes from committed files.)
2. **OTel-shaped input.** Inputs use OpenTelemetry concepts (resources, spans,
   metric points, log records) plus change events. They are a *logical*
   representation, not byte-exact OTLP (see "Representation conventions").
3. **Provenance closes inside the fixture.** Every `source_refs` / `source_ref`
   in `expected.json` must point at something that exists in that fixture's
   `input.json` (a trace/span id, a metric series, a log id, a change id) or at a
   derived artifact in the same `expected.json` (e.g. an anomaly window id). No
   dangling references.
4. **Uncertainty is representable, not optional.** Counter-evidence, missing
   data, and confidence must be expressible and used where the scenario calls for
   it. A scenario with no counter-evidence path is usually under-modeled.

## Directory layout

```text
fixtures/
├── README.md                       # short pointer to this doc
├── registry.json                   # machine-readable index (see below)
└── scenarios/
    └── <scenario-id>/
        ├── scenario.json           # manifest + ground truth
        ├── input.json              # synthetic OTel-shaped telemetry
        └── expected.json           # gold derived artifacts
```

**Why this location.** The fixture *data* is a first-class eval asset — the
`what_and_why.md` MVP bet treats the incident corpus as a deliverable comparable
to source code, not documentation — so it lives at the repo top level in
`fixtures/`, as language-neutral JSON consumable by future Rust tests, an eval
harness, or external tooling. The *spec* (this file) is a supporting process doc,
so it lives in `docs/process/` (the top-level process guide is
[`docs/review-framework.md`](../review-framework.md)). When Janus grows Rust
tests, they can load these fixtures in place (e.g. from `../fixtures`) or a thin
`tests/` layer can wrap them; the data does not need to move.

A fixture may split `input.json` / `expected.json` into a sub-directory of
smaller files if a single file becomes unwieldy (e.g. `input/traces.json`); keep
the same top-level keys. Prefer single files until size forces a split.

## Naming conventions

- **Scenario id**: ASCII kebab-case, `{failure-class-ish}-{short-name}`, e.g.
  `deploy-bad-rollout`, `dependency-db-degradation`, `coincidental-deploy-trap`.
  The directory name equals the id equals `scenario.json`'s `id`.
- **Entity ids**: `{kind}:{name}` with optional `@{variant}`, e.g.
  `service:checkout`, `db:orders-pg`, `route:checkout/POST /checkout`,
  `infra:redis-cache`, `pod:checkout-7c9d-abcde`, `service:payments@canary`.
  Kinds align with `what_and_why.md` ("数据模型方向"): service, route, operation,
  host, container, pod, deployment, database, queue, cache, external-api, tenant,
  region, feature-flag, build, model.
- **Reference ids** inside a fixture:
  - trace: `t-XXXX`; span ref: `t-XXXX/s-Y`
  - metric series ref: `{metric.name}@{entity-id}`
  - log: `log-N`; log pattern: `lp-N`
  - change: `change:{slug}`
  - anomaly window: `aw-N`; evidence item: `ev-N`

## `scenario.json` (manifest + ground truth)

```jsonc
{
  "id": "deploy-bad-rollout",          // == directory name
  "title": "...",                       // one line
  "version": 1,                          // bump on breaking edits to this fixture
  "schema_version": "fixtures/v1",      // this scheme's version
  "failure_class": "deploy",            // from the taxonomy below
  "difficulty": "baseline",             // baseline | hard
  "false_causality_trap": false,         // true if the obvious suspect is wrong
  "summary": "...",                      // a few sentences of narrative
  "question": "...",                     // the agent-facing investigation question
  "time_window": { "start": "...Z", "end": "...Z" },
  "ground_truth": {                      // what a correct investigation concludes
    "primary_cause_entity": "service:checkout",
    "cause_kind": "change_event",
    "cause_ref": "change:deploy-checkout-v2",
    "blast_radius": ["service:checkout", "service:api-gateway"],
    "not_the_cause": ["db:orders-pg"],
    "notes": "why this is the answer and what disambiguates it"
  },
  "capabilities": ["get_evidence_bundle", "..."],  // tags exercised, see below
  "inputs":   ["resources","traces","metrics","logs","changes"],  // keys present in input.json
  "expected": ["entities","relationships","anomaly_windows","..."] // keys present in expected.json
}
```

`ground_truth` is the human-authored answer key. It is separate from
`expected.json` (the machine artifacts) so an eval can score a pipeline's output
against derived artifacts *and* check the final conclusion against ground truth.

## `input.json` (synthetic OTel-shaped telemetry)

Top-level keys, each optional but declared in `scenario.inputs`:

- `resources`: `{ id, attributes }` — `attributes` use OTel semantic-convention
  keys (`service.name`, `service.version`, `service.instance.id`,
  `k8s.namespace.name`, `host.name`, `db.system`, `deployment.environment`, …).
- `traces`: `[{ trace_id, exemplar_of?, spans: [{ span_id, parent_id, resource,
  name, kind, start, end, status, attributes }] }]`. `kind` is OTel
  (SERVER/CLIENT/INTERNAL/…); `status` is OK/ERROR; `resource` references a
  resource `id`.
- `metrics`: `[{ name, entity, unit, points: [{ t, v }] }]` — one logical series
  per object; `entity` is an entity id (or `instance:…`).
- `logs`: `[{ id, t, entity, severity, body, attributes }]`.
- `changes`: `[{ id, t, kind, entity, summary, attributes }]` — `kind` ∈
  deploy, rollback, config_change, feature_flag, traffic_shift, scaling_event,
  schema_migration, dependency_version, infrastructure_event, job_start,
  external_event, … Change events are first-class per `what_and_why.md`; include
  them even when the cause is *not* a change (so the guard can rule changes out).

Optional extension keys (declare in `scenario.inputs` when used):

- `prior_incidents`: `[{ id: "prior:…", first_seen, title, signature, mitigation,
  resolution_minutes?, summary }]` — a warm-layer memory of past incidents the
  resolver can match against, for recurring-incident scenarios. Keep prior
  incidents described inline so the fixture stays self-contained (do not reference
  another fixture). Matched via a `previous_incident` evidence item.
- `telemetry_gaps`: `[{ id: "telemetry_gap:…", start, end, affected_entities,
  affected_signals, cause, note }]` — declares missing-data windows for
  missing-data scenarios. Metric series may also mark a hole inline with a `_gap`
  field referencing the gap id. Surfaced via a `missing_data` evidence item.

## `expected.json` (gold derived artifacts)

Top-level keys, each optional but declared in `scenario.expected`. Produce the
ones the scenario meaningfully exercises:

- `entities`: `[{ id, kind, from: [resource-ids], confidence, discriminators?,
  alternatives?, unresolved?, missing_attributes?, estimated_share? }]`.
- `relationships`: `[{ src, type, dst, confidence, evidence?, attributes? }]`.
  `type` ∈ calls, depends-on, runs-on, owns, deployed-as, emits, retries,
  fans-out-to, reads-from, writes-to, shares-resource-with.
- `anomaly_windows`: `[{ id, entity, signal, start, end, baseline,
  peak|trough, detector_confidence, note? }]`.
- `log_patterns`: `[{ id, template, entity, severity, first_seen, last_seen,
  count, exemplars: [log-ids], stability }]`.
- `evidence_bundle`: the gold output of `get_evidence_bundle` — see next section.
- `timeline`: `[{ t, marker, entity, text, source_ref }]` (`build_timeline`).
  `marker` ∈ change, symptom, propagation, recovery, trigger, amplification,
  non-causal-change.
- `suspected_causes`: `[{ rank, entity, hypothesis, score, reasons, supporting:
  [ev-ids], counter: [ev-ids], trap_note? }]` (`rank_suspected_causes`).
- `next_checks`: `[{ action, rationale, expected_signal }]` (`suggest_next_checks`).
- `entity_context`: object for `expand_entity_context`.
- `related_anomalies`, `window_comparison`: objects for `find_related_anomalies`
  and `compare_windows` (see the dependency-degradation fixture for shape).

Optional helper keys may carry a leading `_` (e.g. `_note`, `_for_capability`);
treat `_`-prefixed keys as non-normative annotations.

### Evidence item shape (the Evidence IR)

`evidence_bundle.items[*]` use the exact Evidence IR field set from
`what_and_why.md` ("Evidence IR") and `evidence-spine.md`:

```jsonc
{
  "id": "ev-1",
  "claim": "...",                 // statement this item supports/weakens
  "kind": "change_event",          // metric_anomaly | trace_exemplar | log_cluster |
                                    // change_event | dependency_edge | profile_hotspot |
                                    // previous_incident | counter_evidence | missing_data
  "direction": "supports",         // supports | weakens | contradicts | neutral
  "strength": 0.9,                 // 0..1 evidence strength (NOT causal confidence)
  "time_window": { "start": "...Z", "end": "...Z" },
  "entities": ["service:checkout"],
  "source_refs": [{ "signal": "change", "ref": "change:deploy-checkout-v2" }],
  "freshness": "settled",          // settled | changing
  "missing_data": [],              // data this item depends on but lacks
  "token_cost": 45,                // approx tokens to place in agent context
  "privacy_scope": "none",         // permission / tenant / redaction scope
  "confidence": { "time_alignment": 0.93 }  // OPTIONAL extension: named confidence dims
}
```

`evidence_bundle` also carries the query and a `budget`:

```jsonc
"budget": { "max_items": 6, "max_tokens": 600, "tokens_used": 250, "items_dropped": 0 }
```

This encodes the design rule that `get_evidence_bundle` is a *budgeted selection*,
not `LIMIT N` (`what_and_why.md`, "Token Budget 是查询约束"): the gold bundle should
prefer hypothesis-discriminating evidence, include counter-evidence, and report
what it dropped. The `confidence` sub-object is an optional extension (per the
"store confidence, don't just speak it" lesson); use it where a scenario turns on
time alignment, dependency direction, entity mapping, or change proximity.

## Capability tags

A fixture declares which Janus capabilities its gold artifacts exercise. The
canonical list (also in `registry.json`):

`entity-resolution`, `relationship-building`, `change-ingestion`,
`anomaly-windows`, `log-pattern-clustering`, `evidence-ir`,
`get_evidence_bundle`, `build_timeline`, `find_related_anomalies`,
`compare_windows`, `rank_suspected_causes`, `expand_entity_context`,
`suggest_next_checks`, `false-causality-guard`, `token-budget-retrieval`.

These map directly to `what_and_why.md`'s data-model objects ("数据模型方向"),
investigation primitives ("Agent-Oriented Query Surface"), and the
false-causality priority ("设计优先级" #7).

## Failure-class taxonomy

`failure_class` is grounded in the failure framing of `what_and_why.md`
("核心转变"), extended with evidence-quality classes that the design doc treats as
first-class concerns:

| Class | What it stresses |
|---|---|
| `deploy` | change as cause; change-proximity reasoning |
| `dependency-degradation` | dependency direction; blast radius |
| `retry-storm` | symptom ≠ cause; amplification |
| `config-change` | non-deploy changes |
| `traffic-shift` | load redistribution; tenant/partition entities |
| `resource-exhaustion` | saturation; host/pod entities; restarts |
| `schema-change` | migrations breaking a query path |
| `downstream-outage` | external dependency; cascade |
| `coincidental-correlation` | **false-causality trap**: obvious suspect is wrong |
| `entity-ambiguity` | entity-resolution uncertainty; alternatives |
| `missing-data` | gaps; report, don't invent |
| `recurring-incident` | warm-layer previous-incident memory |

Set `false_causality_trap: true` whenever the most obvious suspect (by
co-occurrence or change-proximity) is *not* the cause, and make the gold
`suspected_causes` rank it low with explicit `counter` evidence and a `trap_note`.
These are the highest-value fixtures for the design's central safety goal.

## `registry.json`

The registry is the index an eval harness or contributor reads to select
fixtures. Keep it in sync by hand when adding a fixture. It holds:

- `schema_version`, `description`
- `capabilities`, `failure_classes`: the canonical vocabularies
- `fixtures[]`: one entry per *built* fixture (`id`, `path`, `failure_class`,
  `difficulty`, `false_causality_trap`, `capabilities`, `title`)
- `proposed[]`: a backlog of not-yet-built scenarios (`id`, `failure_class`,
  `note`)

Only list fixtures in `fixtures[]` that actually exist on disk; everything else
goes in `proposed[]`.

## Representation conventions

- **Timestamps** are ISO-8601 UTC strings (e.g. `2026-06-01T14:05:11.000Z`) for
  readability. Real OTLP uses unix-nanos; fixtures are a logical representation.
  State this in each `input.json` `_note`.
- **Numbers**: metric values are plain numbers in the declared `unit`; `strength`,
  `score`, and `confidence` are `0..1`.
- **Keep telemetry compact but realistic**: a handful of spans, a few metric
  points spanning baseline → anomaly → (optional) recovery, a few representative
  logs, and the relevant change events. Enough to derive the gold artifacts, not
  a full capture.

## How to add a fixture

1. Pick a `failure_class` and a kebab-case `scenario-id`. If it's in
   `registry.json`'s `proposed[]`, reuse that id.
2. Create `fixtures/scenarios/<id>/` with `scenario.json`, `input.json`,
   `expected.json`.
3. Author `input.json` first (the telemetry), then hand-derive `expected.json`
   (what a correct Janus should produce). Authoring the gold output by hand is the
   point — it is the contract.
4. Ensure every `source_ref` in `expected.json` resolves inside the fixture.
5. Add an entry to `registry.json` `fixtures[]` (and remove it from `proposed[]`
   if present).
6. Run the checklist below.

### Checklist

- [ ] No reference to `references/`, external datasets, papers, or products.
- [ ] `scenario.json.id` == directory name; `schema_version` set.
- [ ] `inputs` / `expected` lists match the keys actually present.
- [ ] Entity and reference ids follow the naming conventions.
- [ ] Every `source_refs` / `source_ref` resolves to something in this fixture.
- [ ] Evidence items use the full Evidence IR field set; `budget` present on the
      bundle.
- [ ] At least one counter-evidence or missing-data item where the scenario
      warrants it; for traps, the obvious suspect is ranked low with `counter`.
- [ ] `capabilities` tags are from the canonical list and are actually exercised.
- [ ] `registry.json` updated.

## Versioning

- `schema_version` (`fixtures/v1`) versions *this scheme*. Bump it only for a
  breaking change to the file layout or field semantics, and migrate fixtures.
- Per-fixture `version` bumps when a fixture's data changes in a
  backward-incompatible way (so a stored eval result can be tied to a version).

## Backlog

`registry.json` `proposed[]` lists scenarios worth adding next, covering the
failure classes not yet represented (`config-change`, `traffic-shift`,
`resource-exhaustion`, `schema-change`, `downstream-outage`,
`recurring-incident`, `missing-data`). Contributors should pull from there before
inventing new classes.
