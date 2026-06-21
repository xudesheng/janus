# Entity Resolver Confidence Review 1

- Baseline SHA: `ac77fecde875299d2dcbf905aab12163ef1e38c7`
- Current milestone: Milestone 5A Phase 1 data model and store read boundary for source-backed entity and relationship context
- Critical path: yes - Phase 1 is the approved first implementation slice for entity and relationship context, and it removes the raw-source/gold-artifact ambiguity before resolver logic starts
- Milestone progress: tightened the formal design contract from review 0, added serializable entity/relationship output models, and exposed a raw-source-only hot-store read boundary with tests
- Deferred milestone work: entity resolution, relationship derivation, fixture comparison, and optional CLI/reporting are deferred because review 0 approved a phased start and this round intentionally covers only Phase 1

This round responds to `entity-resolver-confidence-review-0.md`. Review 0's
Direction Verdict agreed with the design direction and approved starting Phase
1, but did not give whole-design sign-off. I therefore implemented only the
approved Phase 1 slice and did not start resolver or relationship-builder
logic.

## Response To Review 0

Review 0 required three comparison-contract tightenings before Phase 4:

1. Variant-token rule: addressed in `docs/core/entity-resolver-confidence.md`.
   The ambiguous payments fixture now pins `@canary`, `@stable`, and
   `@unresolved` derivation rules, including merging the two stable resources
   into one stable service identity with separate `deployed-as` edges.
2. Required fixture set: addressed in the fixture comparison contract. Phase 4
   now treats all currently registered fixtures that declare
   `entity-resolution` or `relationship-building` as required, with no
   pre-approved unsupported fixtures.
3. Confidence tolerance: addressed in the confidence model. The bands are now
   explanatory defaults; compared fixture values must land within tolerance of
   the concrete gold confidence values.

Review 0 also recommended three non-blocking tightenings:

4. Raw-source isolation: addressed in the formal design and code. Phase 1 adds
   `StoredRecordKind::is_raw_source()` and
   `HotContextStore::raw_source_records()`, plus a test proving that the raw
   read boundary excludes expected derived records loaded by
   `HotContextStore::load_fixture_case`.
5. Discriminator value comparison: addressed in the formal design. JSON array
   discriminator values are compared as order-independent sets when fixtures use
   arrays for multiple observed values.
6. Relationship store key grammar: kept as an opaque store key and added
   `relationship_store_key(src, type, dst)` for the deterministic
   `relationship:{src}|{type}|{dst}` shape. No scalar-ref resolver behavior was
   added.

## Implementation Summary

Added `src/entity_context.rs` and exported it from `src/lib.rs`.

The new Phase 1 types are:

- `ResolvedEntity`
- `EntityAlternative`
- `ResolvedRelationship`
- `EntityKind`
- `RelationshipType`

The serde shape matches fixture `expected.json` conventions:

- entity kinds and relationship types serialize in kebab-case;
- relationships serialize the Rust field `relationship_type` as JSON `type`;
- confidence uses the existing `UnitInterval` transparent numeric wrapper;
- optional fields such as `alternatives`, `missing_attributes`, `evidence`, and
  `attributes` omit when empty.

The hot-store read boundary now exposes raw source records separately from
derived records. This is intentionally small: it is enough for the future
resolver to consume resources, traces, spans, metrics, logs, changes, prior
incidents, and telemetry gaps without seeing fixture gold entities,
relationships, anomaly windows, log patterns, or evidence items.

No entity resolver, relationship builder, fixture comparison helper, CLI, MCP
surface, evidence generation, anomaly detection, log clustering, timeline
generation, persistence, or new ingest protocol work was added.

## Review Focus

Reviewers should focus on:

1. Whether the review-0 required tightenings are now sufficiently captured in
   the formal design doc.
2. Whether the Phase 1 data model is the right stable shape for current fixture
   `entities` and `relationships`.
3. Whether `HotContextStore::raw_source_records()` and
   `StoredRecordKind::is_raw_source()` are enough structural protection against
   accidentally deriving from expected gold artifacts.
4. Whether the deterministic relationship store key helper is acceptable as an
   opaque store key, with relationship evidence remaining separate source refs.
5. Whether implementation may proceed to Phase 2 entity resolver, and whether
   Phase 3 relationship builder remains cleared after Phase 2 lands as review 0
   suggested.

## Verification

Commands run successfully:

```bash
cargo fmt
cargo test
cargo clippy --all-targets --all-features
cargo run --bin validate_fixtures
```

`cargo test` includes 5 new `tests/entity_context.rs` tests for fixture-shape
serde, relationship key formatting, and raw-source isolation. Fixture validation
reported 12 fixtures, 0 errors, and 0 warnings.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
