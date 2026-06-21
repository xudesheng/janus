# Entity Resolver Confidence Review 3

- Baseline SHA: `c0831d6a284bf053f27a1bde4cfb4d6de7691368`
- Current milestone: Milestone 5A Phase 3 relationship builder, plus review-2 confidence cleanup needed before Phase 4 comparison
- Critical path: yes - Phase 3 is the approved next implementation slice for entity context, and review 2 explicitly approved proceeding to the relationship builder
- Milestone progress: added a raw-source-backed relationship builder, expanded corpus-wide relationship coverage, and removed the review-2 name-memorized confidence/share/id-prefix cases from the resolver path
- Deferred milestone work: Phase 4 full fixture comparison, derived record store insertion/exposure, and optional CLI/reporting remain deferred because this round intentionally covers the approved Phase 3 builder and the review-2 cleanup needed before Phase 4

This round responds to `entity-resolver-confidence-review-2.md`. Review 2's
Direction Verdict approved proceeding to Phase 3, with a caveat that relationship
confidence must not extend the same memorization pattern called out for entity
confidence.

## Response To Review 2

Finding A: fixed.

- Removed fixture-name confidence tuning for `service:otel-collector`,
  `service:inventory`, `service:reporting-job`, `db:inventory-pg`, and
  `pod:recommender-5b8f`.
- Service confidence now derives from available discriminating resource
  properties (`service.version`, `service.instance.id`, and namespace
  presence).
- Database and cache confidence now derive from dependency kind rather than
  entity name.
- Pod confidence now derives from resource support instead of pod-name prefix.
- Relationship confidence uses relationship evidence/type rules and only lowers
  when endpoint entity confidence is unresolved, rather than memorizing fixture
  tuples.

Finding B: fixed.

- `service:payments@unresolved.estimated_share` is now estimated from raw source
  records that mention unresolved resources or unresolved entity markers,
  divided by the local raw-source record count and rounded to two decimals.
- For the current ambiguous fixture this still yields `0.18`, but it is no
  longer a literal copied from gold.

Finding C: fixed.

- Redis cache id-prefix selection no longer checks the literal name
  `redis-cache`.
- Redis resources with `cluster.name` resolve to `infra:<service.name>`; other
  Redis/cache-like resources may resolve to `db:<service.name>` with kind
  `cache`, matching the current fixture conventions without keying on one
  fixture entity name.

## Implementation Summary

Added `resolve_relationships(&HotContextStore, &[ResolvedEntity]) ->
Vec<ResolvedRelationship>` in `src/entity_context.rs`.

The Phase 3 relationship builder derives:

- `deployed-as` from service-to-instance and service-to-pod resource mappings;
- `runs-on` from pod-to-host resource mappings;
- `calls` from trace parent-child service boundaries and local `peer.service`
  hints;
- `reads-from` and `writes-to` from database/cache spans and operation shape;
- `depends-on` from external peer-service spans when no local resource matches;
- `retries` from repeated retry-attempt spans, preserving retry config
  attributes from raw resource attributes;
- `fans-out-to` and tenant `calls` from shard/tenant span attributes;
- `shares-resource-with` when a service reads from a database that another
  service writes to;
- limited source-backed inferred writes from change/prior incident records
  where the current fixture has no direct reporting-job trace.

Relationship evidence uses current fixture source-ref conventions such as
`trace:t-0001`, raw change ids, prior incident ids, or dependency entity ids.
The builder ignores derived gold relationship records and reads only raw source
records plus the provided resolved entity confidence map.

## Test Coverage Added

New `tests/entity_context.rs` coverage:

- `relationship_builder_derives_every_current_fixture_gold_relationship`
  requires every current fixture gold relationship `src/type/dst` to be
  derived, and checks expected evidence/attributes as subsets.
- `relationship_builder_preserves_retry_and_cache_fallback_attributes` checks
  retry `max_attempts`/`backoff` and cache-miss fallback `role`.
- `relationship_builder_ignores_derived_gold_relationship_records` proves the
  builder does not copy expected/gold relationship records from a store.

The existing entity tests still cover all current fixture gold entity ids/kinds
and the strict ambiguous payments split.

## Review Focus

Reviewers should focus on:

1. Whether review-2 findings A, B, and C are fully addressed without replacing
   them with new fixture-name memorization.
2. Whether the Phase 3 relationship builder is sufficiently source-backed and
   explainable for Milestone 5A.
3. Whether any relationship rule is too fixture-specific for this first slice,
   especially the UI-to-API naming convention and change/prior inferred
   reporting-job writes.
4. Whether relationship confidence assignment is acceptable and avoids the
   anti-pattern called out in review 2.
5. Whether the corpus-wide relationship tuple/evidence/attribute tests are
   enough to proceed to Phase 4 fixture comparison.
6. Whether implementation may proceed to Phase 4: comparison helper and tests
   for entity and relationship confidence/discriminators/alternatives.

## Verification

Commands run successfully:

```bash
cargo fmt
cargo test --test entity_context
cargo clippy --all-targets --all-features
cargo test
cargo run --bin validate_fixtures
```

`cargo test` includes 12 `tests/entity_context.rs` tests. Fixture validation
reported 12 fixtures, 0 errors, and 0 warnings.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
