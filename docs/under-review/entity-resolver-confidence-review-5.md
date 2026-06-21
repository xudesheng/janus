# Entity Resolver Confidence Review 5

- Baseline SHA: `de5d239b67535cf73b66a6059ab6d5d7646ab0f8`
- Current milestone: Milestone 5A final integration slice - derived entity and relationship records usable through the hot-store reference boundary
- Critical path: yes - this is the remaining Definition-of-Done item after Phase 4 comparison was approved
- Milestone progress: added derived entity/relationship store insertion, proved inserted records resolve as `entity` and `relationship` source refs, and bounded current-corpus extra relationships between gold entity ids
- Deferred milestone work: none for Milestone 5A; optional CLI/reporting remains unimplemented because it is explicitly non-mandatory and the tests now cover the inspection contract

This round responds to `entity-resolver-confidence-review-4.md`. Review 4
approved proceeding to the final Milestone 5A integration slice, with one minor
finding to bound actual corpus extras rather than only proving the comparison
mechanism can report extras.

## Response To Review 4

Finding 1: fixed.

- `comparison_accepts_current_fixture_gold_contract` now checks
  `extra_relationships` whose `src` and `dst` are both gold entity ids.
- Unreviewed extra relationships between gold entity ids fail the corpus test.
- The only current reviewed extra is
  `service:orders fans-out-to shard:orders-shard-1` in
  `traffic-shift-hotspot`. That fixture includes a healthy shard-1 trace
  exemplar as counter-evidence; the derived relationship is source-backed
  topology context, while gold relationship output only names the anomalous
  shard-3 edge.
- `fans-out-to` relationships derived from span `orders.shard` attributes now
  preserve trace evidence, including the reviewed shard-1 extra.

## Implementation Summary

Added:

```rust
insert_derived_entity_context(
    store: &mut HotContextStore,
    entities: &[ResolvedEntity],
    relationships: &[ResolvedRelationship],
) -> Result<(), HotStoreError>
```

The helper inserts:

- each `ResolvedEntity` as `StoredRecordKind::Entity` with key `entity.id`,
  `entities: [entity.id]`, no time window, and the serialized entity payload;
- each `ResolvedRelationship` as `StoredRecordKind::Relationship` with
  deterministic key `relationship:{src}|{type}|{dst}`,
  `entities: [src, dst]`, no time window, and the serialized relationship
  payload.

This uses the existing `HotContextStore::insert_record` and source-ref resolver.
No new resolver path or fixture-file shortcut was added.

## Test Coverage Added

New `tests/entity_context.rs` coverage:

- `derived_entity_context_records_resolve_through_hot_store_reference_boundary`
  builds a source-only store by replaying fixture simulator events, derives
  entities and relationships, inserts them, and verifies `SourceSignal::Entity`
  and `SourceSignal::Relationship` refs resolve to concrete hot-store records.
- `inserted_derived_records_are_not_raw_resolver_inputs` verifies inserted
  derived entity/relationship records do not widen `raw_source_records()`.
- The existing full-corpus comparison test now rejects any unreviewed extra
  relationship between gold entity ids.

## Review Focus

Reviewers should focus on:

1. Whether `insert_derived_entity_context` is the right narrow store integration
   for Milestone 5A.
2. Whether the deterministic relationship store key and endpoint `entities`
   metadata satisfy the hot-store reference-boundary requirement.
3. Whether the source-only replay tests prove the resolver is still not copying
   fixture gold records.
4. Whether the reviewed traffic-shift hotspot extra relationship is acceptable,
   or whether the builder should be narrowed further instead.
5. Whether Milestone 5A `entity-resolver-confidence` is now complete without the
   optional CLI/reporting slice.

## Verification

Commands run successfully:

```bash
cargo fmt --check
cargo test --test entity_context
cargo clippy --all-targets --all-features
cargo test
cargo run --bin validate_fixtures
```

`cargo test --test entity_context` now runs 18 tests. Full `cargo test` passed.
Fixture validation reported 12 fixtures, 0 errors, and 0 warnings.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
