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

## Review (by Claude (Opus 4.8))

### Direction Verdict

On critical path: **yes** — this is the last Definition-of-Done item for
Milestone 5A.

Milestone progress (judged before local defects): **complete and verified.** This
round closes the final DoD gap and the round-4 finding, with no regressions:

- `insert_derived_entity_context` is the right narrow integration — it matches the
  design's "Store And Query Integration" proposal almost verbatim (entities as
  `StoredRecordKind::Entity` keyed by id; relationships keyed by the deterministic
  `relationship:{src}|{type}|{dst}` with both endpoints in `entities`), reusing
  the existing `insert_record`/source-ref resolver with no new resolver path.
- The reference-boundary proof is genuine and leak-free: 
  `derived_entity_context_records_resolve_through_hot_store_reference_boundary`
  builds a store by **replaying simulator events into a fresh
  `HotContextStore::new()`** (not `load_fixture_case`), so the store holds only
  raw source records — no gold. It then inserts derived records and resolves
  `SourceSignal::Entity` / `SourceSignal::Relationship` refs to concrete records.
  This is the DoD item ("derived records usable by the hot-store reference
  boundary") proven end-to-end, and it doubly answers review focus #3.
- `inserted_derived_records_are_not_raw_resolver_inputs` proves the resolver is
  idempotent w.r.t. its own output: inserting derived records does not widen
  `raw_source_records()`, so a second resolve cannot consume derived context as
  raw input. Correct and important.
- The round-4 extras finding is resolved and enforced:
  `comparison_accepts_current_fixture_gold_contract` now fails on any unreviewed
  extra relationship whose `src` and `dst` are both gold entity ids, against a
  one-entry reviewed allowlist.

I verified the single reviewed exception against the fixture rather than taking it
on trust. `traffic-shift-hotspot` gold lists shard-1/2/3 as entities but only
`service:orders fans-out-to shard:orders-shard-3` as a relationship. The input has
fan-out spans for shard 3 (anomalous, t-6001) and shard 1 (healthy exemplar,
t-6002), and **no shard-2 span** — so exactly one extra edge
(`fans-out-to shard:orders-shard-1`) is derived, it is source-backed with
`trace:t-6002` evidence, and it does not contradict gold. Narrowing the builder to
suppress it would require knowing which shard is anomalous — the causal/anomaly
inference the design explicitly defers. So **accept the reviewed extra; do not
narrow** (review focus #4).

Verification reproduced locally: `cargo test` all suites pass, `cargo clippy
--all-targets --all-features` clean, `cargo fmt --check` clean,
`validate_fixtures` 12/0/0. Process correct: baseline `de5d239` is pushed, an
ancestor, and the pre-review-document tree; covered code was pushed before the
review document.

Verdict: **stop — Milestone 5A is complete.** Every Definition-of-Done item is now
met and test-backed:

- raw `HotContextStore` records resolve into source-backed entities — done;
- entity output carries confidence, discriminators, alternatives, unresolved
  state, missing attributes — done;
- relationship output carries endpoints, type, confidence, evidence — done;
- all current fixture gold entities and relationships are compared by an automated
  test, with extras bounded — done (rounds 4–5);
- `ambiguous-entity-resolution` keeps canary/stable/unresolved separate — done;
- derived records are usable through the hot-store reference boundary — done (this
  round);
- no anomaly / log-pattern / timeline / evidence-ranking / MCP / persistence / new
  ingest work was introduced — confirmed;
- `cargo fmt`, `cargo test`, `cargo clippy --all-targets --all-features`,
  `cargo run --bin validate_fixtures` pass — confirmed.

I am leaving **no defect, open question, or new requirement**, so per the
framework's Round Termination rule the Implementor should **not** open a round 6.
This verdict is one reviewer's; completion is subject to any other active
reviewers and User sign-off.

### Notes (non-blocking, no new round required)

- The optional CLI (`derive_entities`) is correctly left unimplemented — the
  design marks it non-mandatory when tests cover the behavior, and they do.
- `insert_derived_entity_context` uses `.expect(...)` on `serde_json::to_value`;
  serialization of these structs is infallible in practice, so this is fine. If a
  future entity/relationship field could serialize fallibly, propagate instead.
  Not in scope for this milestone.
- The accepted bridge rules and the "prefer structured attributes for future
  fixtures" guidance are already captured in the formal design doc, so no
  follow-up review round is needed to track them.

### Answers to the round's review-focus questions

1. `insert_derived_entity_context` the right narrow integration: **yes**.
2. Deterministic relationship key + endpoint `entities` metadata satisfy the
   reference-boundary requirement: **yes** — both endpoints are in `entities` and
   the key resolves as a `Relationship` ref.
3. Source-only replay proves no gold copying: **yes** — the store is built from
   simulator events into a fresh store, never `load_fixture_case`.
4. Reviewed traffic-shift extra acceptable vs. narrowing: **accept it** —
   source-backed, non-contradicting, and narrowing would demand forbidden causal
   inference.
5. Milestone 5A complete without the optional CLI: **yes**.
