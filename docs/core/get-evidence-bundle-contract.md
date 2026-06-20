# Get Evidence Bundle Contract Design

Status: design for the `get-evidence-bundle-contract` implementation topic.

This document defines the Milestone 2 walking skeleton for
`get_evidence_bundle`. It builds on [`evidence-ir-schema.md`](evidence-ir-schema.md),
[`roadmap.md`](roadmap.md), [`evidence-spine.md`](evidence-spine.md), and the
fixture scheme in [`../process/fixtures.md`](../process/fixtures.md). If this
document conflicts with [`what_and_why.md`](what_and_why.md), the canonical
design doc wins and this document should be corrected.

## Purpose

Milestone 2 should expose the first investigation primitive end to end:

```rust
get_evidence_bundle(EvidenceQuery) -> EvidenceBundle
```

This milestone is still a walking skeleton. It should return hand-authored gold
bundles from fixtures, not compile evidence from raw telemetry. The value is a
stable request/response contract and executable JSON flow through the already
implemented Evidence IR types.

The implementation topic should deliver:

- an `EvidenceQuery` request type;
- generated JSON Schema for the request type;
- a fixture-backed `get_evidence_bundle` stub;
- tests covering baseline and false-causality fixtures;
- a small CLI or test helper that emits Evidence IR JSON for a selected fixture.

## Scope

In scope for this topic:

- `EvidenceQuery`;
- request-side types for query intent, budget, required refs, freshness, privacy,
  and fixture selection;
- `get_evidence_bundle` as a stable Rust boundary;
- fixture-backed implementation that loads current gold `EvidenceBundle` data;
- schema generation for the request type;
- tests proving round-trip request to response behavior;
- a small command or helper for emitting bundle JSON.

Out of scope for this topic:

- real retrieval;
- evidence ranking or scoring;
- source-reference resolution into `input.json`;
- registry-wide fixture validation;
- entity resolution;
- anomaly detection;
- log clustering;
- storage engines;
- live OpenTelemetry ingest;
- MCP tool schemas.

The stub should be honest about being a stub. Real compiled and ranked bundle
generation lands in Milestone 6.

## Contract Principles

1. **Request shape before retrieval.** The important artifact is the stable
   request contract that later retrieval and compiler work must honor.
2. **Token budget is part of semantics.** The request must carry budget fields
   even though the stub returns already-authored gold bundles.
3. **Do not model this as `LIMIT N`.** `max_items` and `max_tokens` constrain
   diagnostic selection, not raw row count.
4. **Counter-evidence is explicit.** A request can require counter-evidence, and
   tests should show the response preserves it.
5. **Raw refs are explicit.** A request can require source-backed evidence; the
   stub should reject or fail validation if a returned item lacks source refs.
6. **Fixture selection is a temporary adapter.** Milestone 2 may use a
   `scenario_id` selector so the stub can choose a gold bundle. That selector is
   not the long-term production query mechanism.
7. **No causal claims from this layer.** The function returns structured
   evidence bundles. It does not generate root-cause prose.

## Rust Module Shape

The existing Milestone 1 library surface should be extended rather than
rewritten:

```text
src/
  evidence.rs        # existing Evidence IR response contract
  fixtures.rs        # existing narrow fixture loader
  query.rs           # EvidenceQuery and get_evidence_bundle boundary
  lib.rs             # exports evidence, fixtures, query
  bin/
    generate_schemas.rs
    emit_bundle.rs   # optional small CLI
schemas/
  evidence-ir/
    evidence-item.schema.json
    evidence-bundle.schema.json
    evidence-query.schema.json
```

The exact CLI name is flexible. If adding a binary feels too much for this
milestone, a test helper that emits JSON is acceptable, but there must be an
executable path that proves the contract can produce response JSON.

## EvidenceQuery

`EvidenceQuery` is the request object for the first investigation primitive.

Required fields:

| Field | Type | Notes |
|---|---|---|
| `intent` | object | The question or hypothesis being investigated. |
| `time_window` | `TimeWindow` | Investigation window. |
| `budget` | object | Query-side max item and token budget. |
| `scenario_id` | string | Temporary fixture selector for Milestone 2 only. |

Optional fields:

| Field | Type | Notes |
|---|---|---|
| `entities` | string array | Candidate or known relevant entity ids. |
| `require_counter_evidence` | bool | Default should be `false` if omitted. |
| `require_raw_refs` | bool | Default should be `true` if omitted. |
| `freshness` | enum | Request preference for settled/changing evidence. |
| `privacy_scope` | string | Request-side privacy or tenant scope. |

The `scenario_id` field is deliberately temporary. It lets the stub return
fixture gold output without introducing a registry loader or real retrieval.
Later milestones should remove or isolate it from production query surfaces.

## Query Intent

The request must contain at least one of:

- `question`;
- `hypothesis`.

Shape:

```json
{
  "question": "Why did checkout start returning 5xx around 14:05?",
  "hypothesis": null
}
```

Rules:

- `question` and `hypothesis` are both optional fields at the serde layer;
- validation must require at least one non-empty value;
- if both are present, both should be preserved;
- the stub should not rewrite either field.

## Query Budget

`EvidenceQueryBudget` is request-side budget. It is intentionally smaller than
response-side `EvidenceBudget`.

Required fields:

- `max_items`
- `max_tokens`

Optional fields:

- `min_counter_evidence_items`
- `reserve_tokens_for_raw_refs`

The optional fields are useful contract pressure for later retrieval but do not
need real behavior in the stub. The stub should still validate that numeric
budget values are non-zero where appropriate.

## Freshness Requirement

The request can express freshness preference without forcing all returned items
to match it.

Suggested enum values:

- `any`
- `settled`
- `changing`

`any` should be the default. This keeps the request contract explicit while
avoiding false precision in the fixture-backed stub.

## Privacy Scope

`privacy_scope` should start as a string matching the Evidence IR response field:

- `none`
- `tenant:<id>`
- future redaction scopes

Milestone 2 does not enforce privacy. If a query has a privacy scope and the
fixture bundle contains different item scopes, the stub should not silently
redact or filter. Privacy enforcement belongs to later agent-surface and product
hardening work.

## Stub Behavior

The fixture-backed implementation should do this:

1. validate `EvidenceQuery`;
2. load `fixtures/scenarios/<scenario_id>/expected.json` using the existing
   narrow fixture loader;
3. validate the loaded `EvidenceBundle`;
4. return the loaded bundle unchanged, except for optional metadata that is
   explicitly part of the response contract.

The stub must not:

- inspect `input.json`;
- derive entities, anomalies, timelines, or log patterns;
- rank or re-rank evidence;
- truncate items to fit `max_items` or `max_tokens`;
- synthesize missing counter-evidence;
- rewrite source refs.

If the query budget is lower than the fixture bundle budget, the stub should
return a clear error rather than pretending it optimized selection. Budget-aware
selection belongs to Milestone 6.

## Error Model

Milestone 2 needs a small error type so callers can distinguish contract
problems from fixture loading problems.

Suggested variants:

- invalid query;
- fixture load error;
- invalid fixture bundle;
- unsupported budget for fixture stub.

The exact Rust names can follow local style. The error type should implement
`std::error::Error` and preserve source errors where useful.

## JSON Schema

Schema generation should extend the Milestone 1 path.

Committed schema artifact:

- `schemas/evidence-ir/evidence-query.schema.json`

Schema requirements:

- object fields use `snake_case`;
- enum values are explicit;
- arrays define `items`;
- request budget fields expose integer minimums where practical;
- `scenario_id`, `intent`, `time_window`, and `budget` are required;
- unknown fields are rejected for request-side structs;
- the schema is generated from Rust types, not handwritten as the source of
  truth.

JSON Schema may not express all validation rules cleanly, especially "question
or hypothesis must be present." Rust validation helpers should cover those.

## Tests

Milestone 2 should include tests that:

1. construct an `EvidenceQuery` for `deploy-bad-rollout` and return its fixture
   gold bundle;
2. construct an `EvidenceQuery` for `coincidental-deploy-trap` and verify
   counter-evidence survives unchanged;
3. reject a query with neither question nor hypothesis;
4. reject path traversal in `scenario_id`;
5. reject or clearly fail a budget smaller than the fixture bundle budget;
6. serialize a returned `EvidenceBundle` to JSON;
7. generate and compare the committed `evidence-query.schema.json`;
8. verify generated array schemas declare `items`.

These tests should run with:

```bash
cargo test
```

No live services, storage engine, network access, or OpenTelemetry collector are
required.

## CLI Or Helper

The topic should provide one small way to emit bundle JSON for a selected
fixture. Example shape:

```bash
cargo run --bin emit_bundle -- deploy-bad-rollout
```

Acceptable output:

- pretty JSON `EvidenceBundle`;
- no prose wrapper;
- process exits non-zero on invalid scenario id or invalid bundle.

If the implementation chooses a test helper instead of a binary, the review
should explain why and show how to produce equivalent JSON during development.

## Definition Of Done

The `get-evidence-bundle-contract` topic is complete when:

1. `EvidenceQuery` and supporting request types exist and are exported from the
   crate library surface.
2. `get_evidence_bundle(EvidenceQuery) -> Result<EvidenceBundle, _>` exists.
3. The implementation returns fixture-backed gold bundles by scenario id.
4. Baseline and false-causality trap fixtures are covered by tests.
5. Query validation rejects missing intent and unsafe scenario ids.
6. Stub budget limitations are explicit and tested.
7. `evidence-query.schema.json` is generated and committed.
8. A CLI or equivalent helper can emit response JSON.
9. `cargo fmt`, `cargo test`, and `cargo clippy --all-targets --all-features`
   pass.

## Review Focus

The review for this topic should focus on:

- whether the query contract matches the roadmap and `what_and_why.md`;
- whether the stub is honest about not doing retrieval or ranking;
- whether `scenario_id` is clearly contained as a fixture-only Milestone 2
  adapter;
- whether budget fields are treated as semantic constraints, not `LIMIT N`;
- whether counter-evidence and source refs survive round-trip unchanged;
- whether the schema is strict enough for later agent-facing use without pulling
  MCP tool details into this milestone.

