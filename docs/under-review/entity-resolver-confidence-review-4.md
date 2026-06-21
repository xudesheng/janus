# Entity Resolver Confidence Review 4

- Baseline SHA: `52794da11879fc0ab5ddb5ca10fd8307ffacb30d`
- Current milestone: Milestone 5A Phase 4 fixture comparison helper and tests
- Critical path: yes - fixture comparison is the approved Phase 4 gate for proving derived entity and relationship context against all current gold fixtures
- Milestone progress: added a reusable comparison helper that checks expected entity and relationship fields, reports extra derived entities and relationships, and exercises the full current fixture corpus end to end
- Deferred milestone work: derived entity/relationship store insertion or exposure remains for the next implementation slice; optional CLI/reporting remains deferred because tests now cover the Phase 4 contract directly

This round responds to `entity-resolver-confidence-review-3.md`. Review 3
approved proceeding to Phase 4 and attached three requirements: document and
bound the narrow relationship inference rules, report extra/spurious derived
context, and finally compare confidence/discriminators/alternatives across the
full required fixture set.

## Response To Review 3

Finding 1: fixed.

- Added `Accepted Milestone 5A Inference Rules` to
  `docs/core/entity-resolver-confidence.md`.
- The formal doc now names and bounds the current `*-ui` -> `*-api` convention,
  change-summary `VACUUM` bridge, prior-incident signature bridge,
  `charge`/payment-provider role bridge, and `checkout.retry.*` retry attribute
  namespace.
- The doc states that these are reviewed current-corpus bridge rules, not a
  general text inference or causality mechanism.

Finding 2: fixed.

- Added `compare_entity_context(...) -> EntityContextComparison`.
- The comparison reports `extra_entities` and `extra_relationships` separately
  from expected-gold mismatches, so useful lower-level extras are visible but do
  not automatically fail the required-subset contract.
- Added a targeted test proving an extra `calls` relationship between two gold
  entity ids is surfaced.

Finding 3: fixed.

- The full-corpus comparison test now runs over every registered fixture and
  checks expected entity and relationship gold against derived output.
- Entity comparison checks id presence, kind, source refs, confidence within
  `0.05`, discriminator values with order-independent JSON arrays,
  alternatives, unresolved markers, missing attributes, and estimated share.
- Relationship comparison checks tuple identity, confidence within `0.05`,
  expected evidence refs as a subset, and expected attributes.

## Implementation Summary

Added public comparison report types in `src/entity_context.rs`:

- `EntityContextComparison`;
- `RelationshipIdentity`;
- entity and relationship mismatch structs for kind, confidence, field, evidence,
  attributes, missing expected records, and extras.

The helper treats gold fixture records as the required subset:

- `has_expected_mismatches()` returns true for missing or mismatched expected
  gold fields;
- extra derived entities and relationships are reported but do not make
  `has_expected_mismatches()` true.

While enabling the stricter full-corpus comparison, the test exposed one real
resolver gap: `external-api:stripe` was derived without the expected
`peer.service` and `server.address` discriminators. The resolver now preserves
those discriminators from the source span attributes.

## Test Coverage Added

New `tests/entity_context.rs` coverage:

- `comparison_accepts_current_fixture_gold_contract` derives entities and
  relationships for all 12 current fixtures and compares them against gold.
- `comparison_reports_extra_entities_and_relationships_between_gold_entities`
  proves extra derived context is surfaced, including a spurious relationship
  between gold ids.
- `comparison_checks_confidence_discriminators_and_alternatives` pins confidence
  tolerance, order-independent discriminator arrays, and alternative confidence
  mismatch reporting.
- `comparison_checks_unresolved_markers_and_relationship_details` pins
  unresolved, missing-attribute, estimated-share, relationship-confidence,
  evidence, and attribute mismatch reporting.

## Review Focus

Reviewers should focus on:

1. Whether `EntityContextComparison` reports the right fields for Phase 4 and
   keeps extra derived context visible without making useful extras fatal.
2. Whether the formal doc now sufficiently documents and bounds the current
   fixture-specific relationship inference bridges.
3. Whether the full-corpus comparison test is strict enough on confidence,
   discriminators, alternatives, unresolved state, missing attributes, evidence,
   and attributes.
4. Whether preserving `peer.service` and `server.address` discriminators for
   external APIs is the right source-backed fix.
5. Whether implementation may proceed to the remaining Milestone 5A integration
   slice: derived entity/relationship store insertion or exposure through the
   hot-store reference boundary.

## Verification

Commands run successfully:

```bash
cargo fmt --check
cargo test --test entity_context
cargo clippy --all-targets --all-features
cargo test
cargo run --bin validate_fixtures
```

`cargo test --test entity_context` now runs 16 tests. Full `cargo test` passed.
Fixture validation reported 12 fixtures, 0 errors, and 0 warnings.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
