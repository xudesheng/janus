# Entity Resolver Confidence Review 2

- Baseline SHA: `6dfe2336b0f25457bbe39998959ed22c6c508de2`
- Current milestone: Milestone 5A Phase 2 entity resolver, plus review-1 Phase 1 vocabulary/design corrections
- Critical path: yes - Phase 2 is the next approved entity-context slice, and review 1 explicitly allowed Phase 2 once the model and id-prefix corrections landed first or together with it
- Milestone progress: fixed the fixture kind vocabulary, documented id-prefix versus kind mapping, removed the off-vocabulary ownership relationship variant, and added a raw-source-backed entity resolver with corpus-wide expected entity id/kind coverage
- Deferred milestone work: relationship derivation, relationship fixture comparison, store insertion of derived entity/relationship records, optional CLI/reporting, anomaly detection, log clustering, timeline generation, evidence ranking, MCP, persistence, and new ingest protocol work remain deferred

This round responds to `entity-resolver-confidence-review-1.md`. Review 1's
Direction Verdict approved proceeding to Phase 2 if findings 1 and 2 landed
first or together with it, and noted finding 3 as minor non-blocking cleanup. I
therefore fixed the Phase 1 vocabulary/design defects and implemented only the
Phase 2 entity resolver. I did not start the Phase 3 relationship builder.

## Response To Review 1

Finding 1: fixed.

- `EntityKind::Db` is now `EntityKind::Database`, serializing as
  `database`.
- `EntityKind::Shard` is now `EntityKind::Partition`, serializing as
  `partition`.
- `tests/entity_context.rs` now loads every current fixture's gold `entities`
  and `relationships` into the Phase 1 model, with explicit assertions for
  `db:orders-pg` as `database` and `shard:orders-shard-3` as `partition`.

Finding 2: fixed in `docs/core/entity-resolver-confidence.md`.

- The identity rule now uses `{id-prefix}:{name}`, not `{kind}:{name}`.
- The design states that id prefix and kind are separate values.
- A mapping table documents current prefixes, including `db:` for
  `database`, `shard:` for `partition`, and `cache:`/`infra:`/`db:` for
  `cache` depending on fixture convention.
- Dependency identity prose now distinguishes database resources from
  redis/cache-like resources whose fixture id prefix may still be `db:`.

Finding 3: fixed.

- Removed `RelationshipType::Owns`, since ownership is out of scope and not in
  the design's Phase 3 relationship vocabulary.
- Kept `RelationshipType::Emits`, which is in the design even though unused by
  current fixture gold.

## Implementation Summary

Added `resolve_entities(&HotContextStore) -> Vec<ResolvedEntity>` in
`src/entity_context.rs`.

The resolver reads only `HotContextStore::raw_source_records()` and derives:

- high-confidence service, database, cache, pod, host, and instance identities
  from resource attributes;
- route entities from server span route/method context;
- tenant and partition entities from normalized ids and span/change
  discriminators;
- external API entities from peer-service span evidence when no matching local
  resource exists;
- explicit ambiguous payments identities:
  `service:payments@canary`, `service:payments@stable`, and
  `service:payments@unresolved`.

The ambiguous payments handling preserves discriminators, alternatives,
missing attributes, unresolved state, and the fixture confidence targets. It
also suppresses the blended `service:payments` aggregate so the canary, stable
fleet, and unresolved telemetry cannot be silently merged.

The resolver intentionally accepts deterministic extra low-level entities such
as instances and routes. The current tests require every fixture gold entity
id/kind to be present, but Phase 4's full comparison helper is still deferred.

## Test Coverage Added

New or expanded `tests/entity_context.rs` coverage:

- all current fixture gold entities and relationships deserialize into the
  model;
- `resolve_entities` keeps the three ambiguous payments identities separate;
- `resolve_entities` does not emit `service:payments`;
- `resolve_entities` derives every current fixture gold entity id with the
  expected kind;
- `resolve_entities` returns no entities when the store contains only a
  derived gold `Entity` record and no raw source records.

## Review Focus

Reviewers should focus on:

1. Whether review-1 findings 1, 2, and 3 are fully addressed.
2. Whether the Phase 2 resolver's raw-source-only implementation boundary is
   strong enough, especially when tests load a full fixture store that also
   contains expected gold records.
3. Whether the resolver's deterministic heuristics are acceptable for
   Milestone 5A, or whether any rule is too fixture-specific for a first
   evidence-substrate slice.
4. Whether the ambiguous payments behavior is strict enough to prevent the
   false-causality failure mode caused by merging canary, stable, and
   unresolved telemetry into `service:payments`.
5. Whether corpus-wide expected entity id/kind coverage is sufficient for
   Phase 2, with confidence/discriminator/relationship comparison still
   deferred to Phases 3 and 4.
6. Whether implementation may proceed to Phase 3 relationship builder.

## Verification

Commands run successfully:

```bash
cargo fmt
cargo test
cargo clippy --all-targets --all-features
cargo run --bin validate_fixtures
```

`cargo test` includes 9 `tests/entity_context.rs` tests. Fixture validation
reported 12 fixtures, 0 errors, and 0 warnings.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
