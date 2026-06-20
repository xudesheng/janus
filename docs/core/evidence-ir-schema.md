# Evidence IR Schema Design

Status: design for the `evidence-ir-schema` implementation topic.

This document defines the first executable Evidence IR contract for Janus. It is
grounded in [`what_and_why.md`](what_and_why.md),
[`evidence-spine.md`](evidence-spine.md), [`roadmap.md`](roadmap.md), and the
fixture scheme in [`../process/fixtures.md`](../process/fixtures.md). If this
document conflicts with `what_and_why.md`, the canonical design doc wins and this
document should be corrected.

## Purpose

Milestone 1 should make the Evidence IR real enough that Rust code, JSON Schema,
and existing fixture gold bundles all agree on one contract.

The goal is not to build retrieval, ranking, entity resolution, or storage yet.
The goal is to pin the data shape that those later systems must produce and
consume.

The implementation topic should deliver:

- `serde`-backed Rust types for Evidence IR response objects;
- generated JSON Schema for agent-facing Evidence IR contracts;
- a narrow read-only fixture loader for `expected.json` evidence bundles;
- tests that deserialize current fixture evidence bundles;
- at least one serialized example bundle or evidence item.

## Design Review Gate

No Rust implementation should start for this topic until every active reviewer
has agreed on the design direction in their `Direction Verdict`. Until then,
review rounds for `evidence-ir-schema` are design-only or diagnosis-only rounds.

After reviewer agreement, the implementation can land as one small milestone or
as phase-by-phase review rounds:

1. Rust Evidence IR types, enum vocabulary, and validation helpers.
2. Narrow fixture loader plus tests against current `expected.json` bundles.
3. Generated JSON Schema artifacts and repeatability tests.

The phases are sequencing guidance, not separate product milestones. All three
remain part of Milestone 1, and none should pull `EvidenceQuery`,
`get_evidence_bundle`, MCP tools, or full fixture validation into this topic.

## Scope

In scope for this topic:

- `EvidenceItem`;
- `EvidenceBundle`;
- supporting types such as `TimeWindow`, `SourceRef`, `EvidenceBudget`,
  `EvidenceKind`, `EvidenceDirection`, and `EvidenceFreshness`;
- optional confidence dimensions as structured numeric data;
- strict enum values for current Evidence IR vocabulary;
- a narrow fixture loader that resolves one fixture by scenario id or path;
- JSON Schema generation from Rust types.

Out of scope for this topic:

- `EvidenceQuery` and `get_evidence_bundle` behavior;
- source-reference resolution back into `input.json`;
- registry coverage checks;
- full fixture validation;
- evidence scoring or ranking;
- MCP tool schemas;
- live OpenTelemetry ingest;
- storage engine choices.

`EvidenceQuery` begins in the next topic,
`get-evidence-bundle-contract`. Source-reference validation and registry-wide
fixture checks belong to `fixture-validation-harness`.

## Contract Principles

1. **Match the fixture shape first.** Current `fixtures/scenarios/*/expected.json`
   files are the first consumer of the contract. If the Rust type cannot
   deserialize existing `evidence_bundle` objects, the contract is wrong or the
   fixture needs an explicit migration.
2. **Keep source references mandatory.** `source_refs` is required on every
   `EvidenceItem` and should be non-empty. A summary that drops provenance is not
   an evidence item.
3. **Do not conflate evidence strength with causal confidence.** `strength`
   measures how strong this evidence item is. Causal reasoning remains later
   compiler logic.
4. **Make uncertainty structural.** `direction`, `missing_data`, `confidence`,
   and `counter_evidence` items are part of the contract, not optional prose.
5. **Use strict names at the boundary.** JSON field names and enum values use
   `snake_case`, matching current fixtures and future tool schemas.
6. **Allow named confidence dimensions without schema churn.** The contract knows
   that confidence dimensions are numeric `0..1` values, but the dimension names
   are open-ended.
7. **Prefer simple types until behavior needs more.** Timestamps can start as
   validated ISO-8601 strings in the contract. Temporal arithmetic belongs to
   later derivation and compiler work.
8. **Keep fixture annotations outside Evidence IR.** `expected.json` may contain
   `_`-prefixed helper keys outside `evidence_bundle`, but `EvidenceBundle`,
   `EvidenceItem`, `SourceRef`, and budget objects should reject unknown fields,
   including `_`-prefixed annotation keys. If a future fixture needs annotation
   inside Evidence IR, the formal contract must add an explicit field first.

## Rust Module Shape

The first implementation should introduce a library surface so tests and later
tools can use the contract without depending on the binary entry point:

```text
src/
  lib.rs
  evidence.rs
  fixtures.rs
  main.rs
schemas/
  evidence-ir/
    evidence-item.schema.json
    evidence-bundle.schema.json
```

Expected responsibilities:

- `src/evidence.rs`: Evidence IR structs, enums, validation helpers, and schema
  generation hooks.
- `src/fixtures.rs`: narrow fixture loading for Milestone 1.
- `src/lib.rs`: exports the modules.
- `src/main.rs`: remains minimal unless a schema generation command is added.
- `schemas/evidence-ir/*.schema.json`: generated artifacts, not hand-written
  source of truth.

The exact schema generation command can be small. A dedicated binary is fine if
it keeps `cargo test` straightforward.

## Evidence Item

`EvidenceItem` is the smallest source-backed unit Janus gives to an agent.

Required fields:

| Field | Type | Notes |
|---|---|---|
| `id` | string | Fixture-local or response-local evidence id, e.g. `ev-1`. |
| `claim` | string | The statement this evidence supports, weakens, contradicts, or contextualizes. |
| `kind` | enum | Evidence category. |
| `direction` | enum | Relationship between this item and the relevant claim or hypothesis. |
| `strength` | number | `0..1`; evidence strength, not root-cause confidence. |
| `time_window` | object | Inclusive evidence window with `start` and `end`. |
| `entities` | string array | Operational entity ids such as `service:checkout`. |
| `source_refs` | object array | Required and non-empty provenance references. |
| `freshness` | enum | Whether the evidence is still changing. |
| `missing_data` | string array | Data this item depends on but lacks. Empty is allowed. |
| `token_cost` | unsigned integer | Approximate cost to place this item in agent context. |
| `privacy_scope` | string | `none`, `tenant:<id>`, or later redaction scope strings. |

Optional fields:

| Field | Type | Notes |
|---|---|---|
| `confidence` | map string -> number | Named confidence dimensions, each `0..1`. |
| `note` | string | Human-authored note for traps, gaps, or fixture explanation. |

The first implementation should reject unknown fields on `EvidenceItem` unless a
deliberate extension field is added in the formal contract.

## Evidence Bundle

`EvidenceBundle` is the Milestone 1 response object loaded from fixture
`expected.json` files.

Required fields:

| Field | Type | Notes |
|---|---|---|
| `time_window` | object | Bundle-wide investigation window. |
| `budget` | object | Token and item budget accounting. |
| `items` | `EvidenceItem` array | Ordered evidence items. |

Optional fields:

| Field | Type | Notes |
|---|---|---|
| `question` | string | Present in current fixtures. |
| `hypothesis` | string | Reserved for future response shapes. |

At least one of `question` or `hypothesis` should be present. Current fixtures
use `question`. JSON Schema may not express this invariant cleanly in the first
pass, so a small Rust validation helper is acceptable.

`EvidenceBundle` is not `EvidenceQuery`. The request contract is a separate
Milestone 2 object.

## Supporting Types

### `TimeWindow`

Shape:

```json
{ "start": "2026-06-01T14:00:00Z", "end": "2026-06-01T14:15:00Z" }
```

For Milestone 1, `start` and `end` should deserialize as strings and the JSON
Schema should mark them as date-time formatted strings. Later milestones can add
typed timestamp parsing where temporal reasoning is needed.

### `SourceRef`

Shape:

```json
{ "signal": "trace", "ref": "t-0001/s-3" }
```

Fields:

- `signal`: source signal or derived artifact kind;
- `ref`: fixture-local or backend-local pointer.

Milestone 1 should support at least these `signal` values:

- `trace`
- `metric`
- `log`
- `change`
- `profile`
- `anomaly_window`
- `log_pattern`
- `prior_incident`
- `telemetry_gap`
- `entity`
- `relationship`
- `external`

Current fixtures use `trace`, `metric`, `log`, `change`, `anomaly_window`,
`prior_incident`, and `telemetry_gap`.

External backend pointers can use `external` later, but source resolution is not
part of this milestone.

Fixture compatibility note: current fixtures sometimes use `signal: "log"` with
refs to log-pattern ids such as `lp-1`. Milestone 1 should accept that current
shape unless reviewers explicitly choose a fixture migration. The `log_pattern`
signal value is reserved for a more precise future split, not a reason to churn
fixtures during the first executable contract.

### `EvidenceBudget`

Shape:

```json
{
  "max_items": 6,
  "max_tokens": 600,
  "tokens_used": 250,
  "items_dropped": 0,
  "note": "optional explanation"
}
```

Required fields:

- `max_items`
- `max_tokens`
- `tokens_used`
- `items_dropped`

Optional field:

- `note`

`token_cost` and `tokens_used` are budget accounting fields. They are not the
eval measurement source for token cost; the comparative eval should measure
serialized material directly.

## Enum Vocabulary

### EvidenceKind

The first strict enum should include:

- `metric_anomaly`
- `trace_exemplar`
- `log_cluster`
- `change_event`
- `dependency_edge`
- `profile_hotspot`
- `previous_incident`
- `counter_evidence`
- `missing_data`

This includes both values already used by fixtures and canonical values named in
`what_and_why.md`.

### EvidenceDirection

Values:

- `supports`
- `weakens`
- `contradicts`
- `neutral`

`weakens` means the item lowers confidence in a claim. `contradicts` means the
item directly conflicts with it. Both are important for false-causality guard
behavior.

### EvidenceFreshness

Values:

- `settled`
- `changing`

Freshness describes whether the evidence is still evolving, not whether it is
old or new.

## Confidence Dimensions

`confidence` should be modeled as an optional map:

```json
{
  "time_alignment": 0.93,
  "entity_mapping": 0.99
}
```

Known dimensions from current fixtures include:

- `amplification`
- `blast_radius_direction`
- `change_proximity`
- `coverage`
- `dependency_direction`
- `entity_mapping`
- `signature_similarity`
- `time_alignment`

The schema should require each confidence value to be a number between `0` and
`1`, but it should not freeze the set of confidence dimension names.

This open-ended map is acceptable for Milestone 1 because confidence is
response-side data and MCP tool schemas are out of scope. Some strict tool-use
validators may reject open `additionalProperties` objects; Milestone 7
(`mcp-agent-surface`) should revisit whether agent-facing tool schemas need a
closed confidence representation.

## Fixture Loader

Milestone 1 needs only a narrow loader. It should not become the full fixture
validation harness.

Required behavior:

- load one bundle by scenario id:
  `fixtures/scenarios/<scenario-id>/expected.json`;
- load one bundle by explicit `expected.json` path;
- deserialize only the `evidence_bundle` field from `expected.json`;
- ignore the rest of the expected artifacts for this milestone;
- reject scenario ids that contain path separators or traversal segments.

Not required yet:

- reading `fixtures/registry.json`;
- checking that every registry fixture exists;
- validating `scenario.json`;
- validating source refs back into `input.json`;
- checking capability coverage;
- enforcing fixture failure-class rules.

The loader exists only because Milestone 1 tests need to prove that current gold
bundles match the Evidence IR type.

## JSON Schema

JSON Schema should be generated from the Rust types, not handwritten as the
source of truth.

The committed schema artifacts should include:

- `schemas/evidence-ir/evidence-item.schema.json`
- `schemas/evidence-ir/evidence-bundle.schema.json`

Schema requirements:

- object fields use `snake_case`;
- required fields match the tables above;
- enum values are explicit;
- arrays define `items`;
- numeric `0..1` fields expose `minimum` and `maximum` where practical;
- `source_refs` is required and should have `minItems: 1`;
- `items` should define `items` and should preserve `EvidenceItem` shape;
- optional fields are nullable or absent according to the chosen generator's
  normal serde behavior.

Schema compatibility with strict tool validators matters, even though MCP tool
schemas are not part of this milestone.

Tests should fail if any generated schema object with `type: array` lacks an
`items` declaration.

## Validation Helpers

Rust validation helpers should stay small. They should check contract invariants
that are easy to miss in plain deserialization:

- `source_refs` is non-empty;
- `strength` is between `0` and `1`;
- confidence values are between `0` and `1`;
- `token_cost`, `tokens_used`, `max_tokens`, and item counts are non-negative by
  type;
- at least one of `EvidenceBundle.question` or `EvidenceBundle.hypothesis` is
  present;
- `budget.tokens_used <= budget.max_tokens` unless a fixture explicitly marks an
  over-budget case later.

This is not a replacement for the full fixture validation harness. It only
guards Evidence IR invariants.

## Dependency Choices

The implementation should keep dependencies boring and contract-focused:

- `serde` for serialization and deserialization;
- `serde_json` for fixture and schema artifact tests;
- `schemars` or an equivalent crate for JSON Schema generation.

Timestamp parsing crates are optional in Milestone 1. If adding one complicates
fixture compatibility, prefer a `Timestamp` newtype around `String` with schema
format metadata and defer parsed time behavior.

## Tests

Milestone 1 should include tests that:

1. deserialize every current `fixtures/scenarios/*/expected.json`
   `evidence_bundle`;
2. run the small Evidence IR validation helper on those bundles;
3. serialize at least one bundle back to JSON;
4. generate schemas for `EvidenceItem` and `EvidenceBundle`;
5. verify committed schema files match generated schemas, or otherwise make
   schema regeneration explicit and repeatable.

These tests should run with:

```bash
cargo test
```

No live services, storage engine, network access, or OpenTelemetry collector are
required.

## Definition Of Done

The `evidence-ir-schema` topic is complete when:

1. Evidence IR Rust types exist and are exported from the crate library surface.
2. Current fixture evidence bundles deserialize without ad hoc parsing.
3. Basic Evidence IR invariants are validated in tests.
4. JSON Schema artifacts for `EvidenceItem` and `EvidenceBundle` are generated
   and committed.
5. The schema generation path is repeatable.
6. `cargo fmt` and `cargo test` pass.

## Review Focus

The review for this topic should focus on:

- whether the Rust types preserve the Janus evidence-substrate boundary;
- whether `source_refs`, `direction`, `missing_data`, and `confidence` are
  strong enough to support false-causality guard work later;
- whether the schema is strict enough for agent-facing contracts but not so rigid
  that current fixtures need unnecessary churn;
- whether the fixture loader stays narrow and does not pull Milestone 3 forward;
- whether schema generation is repeatable and testable.
