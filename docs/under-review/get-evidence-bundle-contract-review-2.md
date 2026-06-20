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

## Review (by Claude)

### Direction Verdict

On the critical path: yes. This is the approved post-design implementation of the
first Janus investigation primitive, and **milestone progress is real and
substantial**: `EvidenceQuery` → `get_evidence_bundle` → gold `EvidenceBundle`
now runs end to end through the Rust boundary, with a committed request schema, an
emit helper, and focused fixture/schema tests. The work stays inside the approved
scope — no retrieval, ranking, storage, registry validation, or MCP schema crept
in. It matches the design I cleared in rounds 0–1, including both round-1
clarifications, which were folded into both the code and the formal doc.

I did not take the round-2 Verification block on trust — I re-ran it. All checks
reproduce locally:

- `cargo test`: 14 tests pass (7 in `tests/get_evidence_bundle.rs`, 7 in
  `tests/evidence_ir.rs`);
- `cargo clippy --all-targets --all-features`: clean, exit 0, no warnings;
- `cargo fmt --check`: clean;
- `cargo run --bin generate_schemas` then `git status`: empty — the committed
  `evidence-query.schema.json` is byte-reproducible.

Definition of Done: all 9 items satisfied. `EvidenceQuery` and supporting types
are exported from `src/lib.rs` (#1); `get_evidence_bundle(EvidenceQuery) ->
Result<EvidenceBundle, _>` exists (#2); returns gold bundles by scenario id (#3);
baseline `deploy-bad-rollout` and false-causality `coincidental-deploy-trap` are
both covered (#4); validation rejects missing intent and missing/unsafe scenario
ids (#5); the budget-fit limitation is explicit and tested at the 249-vs-250 and
4-vs-5 boundaries (#6); the query schema is generated and committed (#7); the
`emit_bundle` binary emits response JSON (#8); fmt/test/clippy pass (#9).

Contract fidelity I specifically checked: the bundle is returned unchanged
(`Ok(bundle)`, no added fields — M2 honored); `FreshnessPreference`
(`any|settled|changing`) is a distinct request enum and `any` does not leak into
response `EvidenceFreshness`; `scenario_id` is optional in schema
(`required: ["budget","intent","time_window"]`) but required by stub validation;
the loader's `InvalidScenarioId` is remapped to `InvalidQuery` on the public path
(`map_fixture_load_error`), matching the round-1 S3 decision; budget compatibility
compares against actual `tokens_used` / `items.len()`, not the fixture's declared
ceilings.

**Next action: continue — one small round.** There is exactly one actionable
gap (L1 below). It is a missing test, not a logic defect, and it is the only thing
between this implementation and topic completion. I am not asking for a redirect
or any design change.

### Actionable

**L1 — The counter-evidence requirement's rejection path has no test.** The new
`GetEvidenceBundleError::UnsatisfiedRequirement` variant never fires in the suite.
`preserves_counter_evidence_for_coincidental_deploy_trap` sets
`require_counter_evidence = true` with `min_counter_evidence_items = Some(2)` on
the trap fixture, which has exactly two counter-evidence items, so the check
passes; no test sets a requirement the bundle cannot meet. The logic in
`ensure_required_counter_evidence` is correct (I traced the
`require`/`min`/`max(…,1)` interaction), but right now if that function were
changed to always return `Ok`, every test would still pass. This matters more
than a typical coverage nit because counter-evidence enforcement is exactly the
false-causality-guard behavior `roadmap.md` calls a first-class failure mode
(Sequencing Principle 4). Add one negative test — e.g. `require_counter_evidence`
or `min_counter_evidence_items` exceeding the available counter-evidence count,
asserting `UnsatisfiedRequirement { requirement: "counter_evidence", .. }`. With
that test in, the topic is complete.

### Non-blocking observations (no action required)

- `require_raw_refs` is structurally always satisfied because Milestone 1 already
  guarantees non-empty `source_refs` on every item, so its rejection path is
  genuinely untestable against current fixtures. This is acknowledged in the
  design and is acceptable contract pressure — I am not asking for a test here,
  only noting the asymmetry with L1 (raw-refs cannot be tested; counter-evidence
  can and should be).
- `emit_bundle.rs` loads the fixture twice (once to seed a guaranteed-fitting
  query, once inside `get_evidence_bundle`). Harmless for a dev helper and it does
  exercise the real public path; no change needed.
- `get_evidence_bundle` uses `.expect(...)` on `scenario_id` after `validate()`.
  The invariant holds (validation rejects `None` first), so the panic is
  unreachable; fine as written.

### Answers to Reviewer Focus

1. Yes — no retrieval/ranking/storage/registry/MCP scope leaked in.
2. Yes — the five-variant error model cleanly separates query, load, bundle,
   budget, and requirement failures, and preserves sources where useful.
3. Yes — both flag behaviors are acceptable for M2 (with the L1 test added for
   the counter-evidence path).
4. Yes — optional `scenario_id`, required core fields, integer minimums,
   `additionalProperties: false`, and array `items` are all present and tested.
5. Almost — the emit helper is sufficient; the tests are sufficient except for
   the L1 negative test.
