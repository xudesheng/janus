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

## Review (by Claude (Opus 4.8))

### Direction Verdict

On critical path: **yes**. This is the approved Phase 1 slice (data model + store
read boundary), and review 0 cleared exactly this.

Milestone progress: **real and positive — judged before local defects.** I
verified the work independently rather than trusting the summary:

- All three blocking review-0 tightenings are now correctly pinned in
  `docs/core/entity-resolver-confidence.md`: the `@canary/@stable/@unresolved`
  variant-token rule (including the two-stable-resources merge), the
  required-fixture set with no pre-approved exemptions, and bands-as-defaults
  with values that must hit within tolerance of the concrete gold confidence.
  The three recommended ones (structural raw-source boundary, set-wise
  discriminator comparison, opaque relationship store key) are in too.
- The Phase 1 model (`src/entity_context.rs`) is well-built: `UnitInterval`
  enforces the `0.0–1.0` confidence bound structurally, `relationship_type`
  renames to JSON `type`, empties are skipped, and `deny_unknown_fields` will
  catch schema drift. The store boundary (`raw_source_records()` +
  `is_raw_source()`) is exactly the structural protection review 0 asked for,
  and `raw_source_records_exclude_expected_derived_records` genuinely proves it
  (loads the full fixture case, confirms gold `Entity`/`AnomalyWindow` records
  are present, then confirms the raw boundary excludes them).
- Scope discipline holds: no resolver, relationship builder, comparison helper,
  CLI, or out-of-scope subsystem was added.
- Verification reproduced locally: `cargo test` (all suites pass, including the 5
  new `tests/entity_context.rs` tests), `cargo clippy --all-targets
  --all-features` clean, `cargo fmt --check` clean, `validate_fixtures` reports
  12 fixtures with no errors.

Process/baseline compliance is correct: baseline `ac77fec` is pushed, is an
ancestor of HEAD, and is the pre-review-document tree (HEAD adds only this review
doc); covered code and design-doc edits were pushed before the review document as
their own commit.

Verdict: **continue.** Phase 1 is substantially delivered and the direction is
right. But there is one concrete defect that defeats the round's own "right
stable shape for current fixtures" claim (review focus #2), and the passing serde
tests gave false confidence because they exercise only the `service` kind. I
approve proceeding to **Phase 2 (entity resolver)** on the condition that the
model/vocabulary correction in finding 1 (and the design correction in finding 2)
lands first or together with it — both are squarely Phase 1 "data model" scope and
cheap. Phase 3 (relationship builder) remains cleared to follow once Phase 2
lands, as review 0 set up.

### Finding 1 — `EntityKind` cannot represent 7/12 fixtures' gold kinds (must fix before Phase 2)

The gold `expected.json` `entities[].kind` vocabulary across the corpus is:
`service, route, pod, host, tenant, cache, external-api, database, partition`.
The Phase 1 `EntityKind` enum (`src/entity_context.rs:51`) instead has `Db`
(serializes `"db"`) and `Shard` (`"shard"`) and has **no `Database` or
`Partition` variant**. With `#[serde(rename_all = "kebab-case")]` and no alias/
`other`, deserializing a gold entity with `"kind": "database"` or `"partition"`
**fails** ("unknown variant"). This hits:

- `database`: `coincidental-deploy-trap`, `dependency-db-degradation`,
  `deploy-bad-rollout`, `missing-data-gap`, `recurring-incident-memory`,
  `schema-migration-errors` (6 fixtures);
- `partition`: `traffic-shift-hotspot`.

Equally, the Phase 4 "exact kind match" rule fails the other direction: a
resolver that emits `EntityKind::Db` serializes `"db"`, which never equals the
gold `"database"`. The existing tests passed only because
`ambiguous-entity-resolution` uses kind `service` exclusively.

Fix: align the `EntityKind` serialization vocabulary with the gold **kind** words
(`database`, `partition`, ...), not the id-prefix words. Then add a serde
round-trip test that loads **every** fixture's gold `entities` and
`relationships` into the model — this is also exactly the coverage the now-pinned
Phase 4 required-fixture set will need, and it would have caught this.

### Finding 2 — design's `{kind}:{name}` id rule is contradicted by the gold (design correction)

`docs/core/entity-resolver-confidence.md` states entity ids follow
`{kind}:{name}`, implying the id prefix equals the kind. The gold decouples them:

- `kind: "database"` -> id `db:orders-pg` (prefix `db`, not `database`);
- `kind: "partition"` -> id `shard:orders-shard-3` (prefix `shard`);
- `kind: "cache"` -> id prefix `db:` (`resource-exhaustion-memory`) **and**
  `infra:` (`coincidental-deploy-trap`).

So id-prefix is a separate, shorter namespace token from kind, and one kind can
take multiple prefixes. Because Phase 4 requires exact id **and** exact kind
match, Phase 2 must emit both correctly. Document the id-prefix↔kind mapping
explicitly in the identity-rules section (the current `db:<name>` / `shard:` /
`cache:`-or-`infra:` prose already hints at it but never states that the id
prefix is not the kind). This unblocks Phase 2 from guessing.

### Finding 3 — `Owns` relationship variant is off-vocabulary (minor, non-blocking)

`RelationshipType::Owns` (`src/entity_context.rs:76`) is not in the design's
minimum relationship list, and the design explicitly defers ownership ("Do not
infer ownership... Ownership and root-cause ranking belong later"). No current
fixture uses `owns` (or `emits`). For a Phase 1 "stable shape," reconcile: either
add `owns` to the design's relationship vocabulary or drop the variant. `emits`
is fine — it is in the design's minimum list even though unused today.

### Answers to the round's review-focus questions

1. Review-0 tightenings captured in the design: **yes**, all six, correctly.
2. Right stable shape for current fixtures: **not yet** — finding 1 (kind
   vocabulary) and finding 2 (id convention) must land first.
3. `raw_source_records()` / `is_raw_source()` enough structural protection:
   **yes** — this is the strongest part of the round.
4. Relationship store key acceptable as opaque key: **yes**.
5. Proceed to Phase 2, Phase 3 after: **yes**, conditioned on findings 1–2
   landing with Phase 2; Phase 3 stays cleared to follow.
