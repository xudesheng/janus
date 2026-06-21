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

## Review (by Claude (Opus 4.8))

### Direction Verdict

On critical path: **yes** — Phase 4 (the fixture comparison helper) is the gate
that proves the derived entity/relationship context against the full gold corpus.

Milestone progress (judged before local defects): **delivered and verified — the
strongest round so far.** I read the code and reproduced the suite:

- `compare_entity_context` covers the full Fixture Comparison Contract: per-entity
  kind, confidence (±0.05), `from` (gold ⊆ derived), discriminators
  (order-independent via `json_values_equivalent`), alternatives (by id, with
  reason + confidence tolerance), `unresolved`, `missing_attributes` (set
  equality), `estimated_share`; per-relationship confidence (±0.05), evidence
  (gold ⊆ derived), attributes. `has_expected_mismatches()` deliberately excludes
  `extra_entities`/`extra_relationships`, so useful extras are visible but
  non-fatal — exactly the contract the design specifies.
- The decisive test, `comparison_accepts_current_fixture_gold_contract`, runs all
  12 fixtures and asserts **no expected mismatches** on every dimension above. It
  passes. This is the end-to-end proof Phase 4 was meant to produce, and it
  retroactively confirms round 3's claim: the property-derived confidence rule
  lands within `0.05` of gold for every gold entity corpus-wide, by rule, not by
  name lookup. The earlier rounds' coverage only checked id/kind presence; this
  closes that gap.
- The targeted tests pin real behavior: extras (including a spurious `calls` edge
  between two gold ids) are surfaced; confidence/discriminator/alternative,
  unresolved/missing-attr/share, and relationship confidence/evidence/attribute
  mismatches each produce the expected report entries.
- Honest signal of a working harness: enabling the stricter comparison exposed a
  real resolver gap (`external-api:stripe` lacked the gold `peer.service` /
  `server.address` discriminators), which was then fixed at the source span. A
  comparison that finds a real defect on first strict run is doing its job.

All three review-3 findings are addressed:

- **Finding 1 (document/bound bridge rules):** done well. The new "Accepted
  Milestone 5A Inference Rules" section names and scopes each bridge
  (`*-ui`->`*-api`, `VACUUM` change-summary, prior-incident signature,
  `charge`/payment-provider, `checkout.retry.*`), explicitly marks them
  current-corpus-only and not a general inference/causality engine, and points
  forward to structured attributes.
- **Finding 2 (report extras/spurious):** done. Extras are reported separately
  and non-fatal, with a test proving a spurious gold-id `calls` edge is surfaced.
- **Finding 3 (full-corpus confidence/discriminators/alternatives):** done — see
  the decisive test above.

Verification reproduced locally: `cargo test` (all suites pass), `cargo clippy
--all-targets --all-features` clean, `cargo fmt --check` clean,
`validate_fixtures` 12/0/0. Process correct: baseline `52794da` is pushed, an
ancestor, and the pre-review-document tree; covered code was pushed before the
review document.

Verdict: **continue.** Phase 4 is complete and verified. I **approve proceeding
to the final Milestone 5A slice**: derived entity/relationship record insertion
or exposure through the hot-store reference boundary (review focus #5). That is
the one remaining Definition-of-Done item ("derived ... records are usable by the
hot-store reference boundary or have an explicitly reviewed path to become
usable"); the topic is not complete until it lands or is explicitly reviewed.
One minor finding below should ride along with that slice. Next action: continue.

### Finding 1 — bound *actual* corpus extras, not just the mechanism (minor)

`comparison_accepts_current_fixture_gold_contract` asserts no *expected*
mismatches, and the synthetic test proves the *extras* machinery works — but
nothing asserts the *current corpus* produces no unexpected extra
**relationships**. So a bridge rule that already over-fires on a real fixture
today (a `*-ui`/`*-api` pair that should not call, a stray `charge` span) would
appear only in the unchecked `extra_relationships` field and pass silently. The
design's own new text promises these rules "cannot silently over-fire on future
fixtures"; close the loop now by adding a corpus-level assertion that
`extra_relationships` whose `src` and `dst` are both gold entity ids is empty (or
an explicitly reviewed allowlist). Extra lower-level *entities* (instances,
routes) are legitimately allowed and need no such bound. This is cheap and turns
the documented promise into an enforced one.

### Answers to the round's review-focus questions

1. `EntityContextComparison` reports the right fields and keeps extras visible
   without making them fatal: **yes**.
2. Formal doc sufficiently documents/bounds the bridge rules: **yes** — named,
   scoped, and marked current-corpus-only.
3. Full-corpus comparison strict enough on confidence/discriminators/
   alternatives/unresolved/missing-attrs/evidence/attributes: **yes** — and it
   asserts zero expected mismatches across all 12 fixtures.
4. Preserving `peer.service`/`server.address` discriminators for external APIs is
   the right source-backed fix: **yes** — it reads them from the source span, not
   a fixture-name table.
5. Proceed to the store-insertion/exposure slice: **yes** — it is the last DoD
   item; carry Finding 1 with it.
