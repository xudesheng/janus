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

## Review (by Claude (Opus 4.8))

### Direction Verdict

On critical path: **yes** — Phase 3 (relationship builder) is the approved next
slice, and the round also lands the review-2 confidence cleanup required before
Phase 4.

Milestone progress (judged before local defects): **strong and verified.** I
read the code, not just the summary:

- **Review-2 Finding A is genuinely fixed**, not relocated. `service_confidence`
  now derives from a `(service.version, service.instance.id, namespace)` presence
  tuple (`0.99/0.98/0.95/0.90`); `pod_confidence` from host presence; dependency
  confidence from `db.system` kind. No fixture entity names remain in the
  confidence path. Relationship confidence (`relationship_confidence`) is a base
  per relationship *type*, lowered by `0.20` (floor `0.30`) only when an endpoint
  entity is unresolved — a real rule, not memorized tuples. This directly honors
  the review-2 caveat about not memorizing relationship confidence.
- **Finding B is fixed**: `estimated_share` is computed
  (`unresolved-mentioning records / total raw records`, rounded to 2 dp); `0.18`
  now falls out of the data instead of being copied.
- **Finding C is fixed**: the redis id-prefix choice keys on `cluster.name`
  presence, not the literal `redis-cache` name.
- **Phase 3 builds the priority relationship types structurally**: `calls` from
  trace parent-child via `nearest_service_ancestor`, `reads-from`/`writes-to`
  from dependency spans + SQL-verb classification, `deployed-as`/`runs-on` from
  runtime resource mappings, `depends-on` from unmatched `peer.service`. These
  are source-backed.
- **Coverage is real**: `relationship_builder_derives_every_current_fixture_gold_relationship`
  passes corpus-wide and asserts gold `evidence`/`attributes` are subsets of
  derived; the retry/cache test pins concrete evidence + attributes; an isolation
  test proves the builder ignores gold relationship records. I reproduced
  `cargo test` (all pass), `cargo clippy --all-targets --all-features` clean,
  `cargo fmt --check` clean, `validate_fixtures` 12/0/0.
- Process correct: baseline `c0831d6` is pushed, an ancestor, and the
  pre-review-document tree; covered code (`d6914ee`, `c0831d6`) was pushed before
  the review document.

Verdict: **continue, and I approve proceeding to Phase 4 (comparison helper +
tests).** There is no memorization regression. The one open direction issue is
the relationship builder's fixture-content heuristics (Finding 1), which Phase 4
is the right place to make transparent and to guard. Two requirements attach to
Phase 4 (see Findings 1–2). Next action: continue.

### Finding 1 — relationship builder encodes fixture-content/naming heuristics (document + bound before/at Phase 4)

To derive every gold relationship corpus-wide, the builder reaches past direct
telemetry into fixture-content inference:

- `insert_resource_name_relationships`: a `calls` edge inferred purely from the
  `*-ui` -> `*-api` service-name convention (coincidental-deploy-trap), confidence
  `0.95`, empty evidence.
- `insert_change_inferred_relationships`: matches the free-text token `"vacuum"`
  in a change `summary` to emit `writes-to` a database.
- `insert_peer_service_relationship`: span name containing `"charge"` sets
  `role: payment-provider`.
- `insert_prior_inferred_relationships`: infers a *current* `writes-to` from a
  `PriorIncident` signature.
- `resource_attribute`: hard-codes the literal attribute namespace
  `checkout.retry.max_attempts` / `checkout.retry.backoff`.

These are deterministic and, importantly, **contract-honest on evidence**: the
gold edges they target (`search-ui->search-api`, `reporting-job->db:orders-pg`,
the runtime edges) are authored in gold with `evidence: None`, so emitting them
without evidence is correct, not a shortcut. So this is *not* the review-2
name->number memorization. But it is fixture-domain knowledge baked into code,
and the design's own Review Focus #5 ("relationship evidence and confidence are
source-backed instead of inferred from weak correlation") is in tension with
naming/free-text inference.

The design explicitly allows "deterministic rules for the current fixtures" for
this first slice, so I am not blocking on these. The requirement is
**transparency**: document these accepted Milestone-5A inference rules in
`docs/core/entity-resolver-confidence.md` (the `-ui`/`-api` convention, the
change-summary and prior-incident inferences, and the `checkout.retry.*`
attribute namespace) so Phase 4's comparison is blessing a *reviewed* decision,
not hidden heuristics. Prefer generalizing the most brittle free-text matches
(`"vacuum"`, `"charge"`) to a structured attribute or a documented narrow scope.

### Finding 2 — corpus test only proves gold ⊆ derived; Phase 4 must report extra/spurious relationships and entities

Every coverage test checks that gold entities/relationships are *present* in the
derived output; nothing checks the other direction. The keyword/naming heuristics
in Finding 1 are exactly the kind of rule that can over-fire on a future fixture
(another `*-ui`/`*-api` pair that should not call; a `"charge"` span that is not a
payment provider). The design's Fixture Comparison Contract already requires
reporting "unresolved extra entities and extra relationships" — Phase 4's
`compare_entity_context` must implement that direction and the round should add a
test that bounds spurious edges (e.g. derived relationships among gold entity ids
that are not in gold are surfaced). Without it, the heuristics' false-positive
rate is invisible.

### Finding 3 — Phase 4 must finally exercise confidence/discriminators/alternatives end-to-end (minor, scope reminder)

Through Phase 3 the only confidence/discriminator/alternative assertions are on
`ambiguous-entity-resolution`. Now that entity confidence is property-derived, the
Phase 4 comparison helper should compare confidence (within the pinned `±0.05`),
discriminators (set-wise), and alternatives across the full required-fixture set —
this is the deferral that Phase 4 is meant to close, and it is the real test of
the de-memorized confidence rule from this round.

### Answers to the round's review-focus questions

1. Review-2 A/B/C addressed without new name memorization: **yes** — verified in
   code; confidence and share are now property-derived.
2. Relationship builder sufficiently source-backed/explainable: **mostly** — the
   priority types are; the inference rules in Finding 1 are deterministic but
   fixture-specific and need documentation.
3. Any rule too fixture-specific (UI/API, change/prior reporting-job): **yes,
   those named ones** — acceptable for 5A but document and bound them (Findings
   1–2).
4. Relationship confidence avoids the review-2 anti-pattern: **yes** —
   type-based with an endpoint-unresolved penalty, not memorized.
5. Corpus tuple/evidence/attribute tests enough to proceed to Phase 4: **yes for
   presence**, but add the spurious-edge direction (Finding 2).
6. Proceed to Phase 4: **yes**, with Findings 1–3 folded into the comparison
   round.
