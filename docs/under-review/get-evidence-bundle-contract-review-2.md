# Get Evidence Bundle Contract Review 2

- Baseline SHA: `1e2eb0f9ba2d3c0fd6726346570c36b0ec381b6b`
- Current milestone: implemented Milestone 2 `get_evidence_bundle` walking
  skeleton with request contract, schema, fixture-backed stub, tests, and emit
  helper.
- Critical path: yes - this is the approved post-design implementation for the
  first Janus investigation primitive.
- Milestone progress: `EvidenceQuery` is now executable end to end, returns
  fixture gold `EvidenceBundle` data through the Rust boundary, emits a committed
  request schema, and is covered by focused fixture and schema tests.
- Deferred milestone work: none for Milestone 2. Real retrieval, ranking,
  storage, registry-wide fixture validation, and MCP schemas remain later
  roadmap work and were intentionally not pulled into this round.

Round 1 cleared implementation to begin as one walking skeleton. This round
implements that skeleton and responds to the two non-blocking clarifications from
the latest review.

## Response To Round 1

The requirement-flag behavior is now implemented and reflected in the formal
design doc.

- `require_raw_refs: true` is satisfied by any bundle that passes Milestone 1
  `EvidenceBundle` validation, because every item must already have non-empty
  `source_refs`. Setting it to `false` does not weaken fixture-backed Evidence
  IR validation.
- `require_counter_evidence: true` requires at least one returned item whose
  `kind` is `counter_evidence` or whose `direction` is `weakens` or
  `contradicts`.
- `budget.min_counter_evidence_items`, when present, requires at least that many
  counter-evidence items whether or not `require_counter_evidence` is set.

The post-load budget-fit check is now explicit in both the formal design and the
implementation. The function validates the query, loads the fixture bundle,
validates the bundle, checks requested budget against actual returned
`tokens_used` and `items.len()`, checks fixture-stub evidence requirements, and
then returns the loaded bundle unchanged.

## Implementation Summary

Added `src/query.rs` with:

- `EvidenceQuery`, `EvidenceQueryIntent`, `EvidenceQueryBudget`, and
  `FreshnessPreference`;
- query validation for intent, time window, non-zero budgets, missing or unsafe
  `scenario_id`, empty entities, and empty privacy scope;
- `get_evidence_bundle(EvidenceQuery) -> Result<EvidenceBundle, _>`;
- explicit error variants for invalid query, fixture load failure, invalid
  fixture bundle, unsupported fixture budget, and unsatisfied fixture
  requirements;
- generated JSON Schema support for `EvidenceQuery`.

Also added:

- `schemas/evidence-ir/evidence-query.schema.json`;
- `src/bin/emit_bundle.rs` for `cargo run --bin emit_bundle -- <scenario-id>`;
- `query` export from `src/lib.rs`;
- schema generation for the query schema in `src/bin/generate_schemas.rs`;
- integration tests covering baseline fixture round trip, false-causality
  counter-evidence preservation, invalid intent, missing/unsafe scenario ids,
  budget-fit errors, JSON serialization, committed schema matching, and array
  `items` declarations.

## Reviewer Focus

Please review whether this implementation should complete the
`get-evidence-bundle-contract` topic.

1. Does the implementation match the approved design without adding retrieval,
   ranking, storage, registry validation, or MCP scope?
2. Is the fixture-stub error model clear enough for callers and later
   milestones?
3. Are the `require_raw_refs` and `require_counter_evidence` behaviors acceptable
   for Milestone 2?
4. Is the query schema strict enough for this milestone, including optional
   `scenario_id`, required core fields, integer minimums, unknown-field
   rejection, and array `items` declarations?
5. Are the tests and emit helper sufficient to consider Milestone 2 complete?

If there are no defects or new requirements, the expected next action is stop
and report topic completion rather than submit another empty round.

## Verification

Passed:

- `cargo run --bin generate_schemas`
- `git diff --check`
- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `cargo clippy --all-targets --all-features`
- `cargo build`
- `cargo run --quiet --bin emit_bundle -- deploy-bad-rollout | ConvertFrom-Json | Select-Object -ExpandProperty question`

The emit helper smoke test printed:

```text
Why did checkout start returning 5xx around 14:05 on 2026-06-01?
```

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
