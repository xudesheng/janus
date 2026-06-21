# Evidence Compiler Ranking Review 2

- Baseline SHA: `9486360d046c0d86f7ace717c59efacb4b340cab`
- Current milestone: completed Evidence Compiler Slice 1 implementation ready for review, covering the compiler model, comparison shell, token estimator, fixture token migration, and proof that expected artifacts are comparison-only
- Critical path: yes - review 1 approved Slice 1 as the only implementation scope allowed to start, and this slice is the prerequisite for later candidate generation, scoring, selection, and `get_evidence_bundle` integration
- Milestone progress: added and verified the Slice 1 Rust comparison surface, runtime model types, deterministic token estimator, expected-artifact loader, fixture token-field migration, and tests for the review 1 notes
- Deferred milestone work: none for Slice 1; Slices 2-6 remain unimplemented because they were outside the review 1 approved coding scope and should proceed only after this Slice 1 review

This round implements only the Slice 1 scope approved in
`evidence-compiler-ranking-review-1.md`. I did not start candidate generation,
candidate scoring, bundle selection, next-check generation, hot-store compiler
integration, or the final `get_evidence_bundle` transition.

## Response To Review 1

N1, evidence item id scheme:

- Resolved in `docs/core/evidence-compiler-ranking.md` by pinning selected item
  ids to the fixture-compatible `ev-1`, `ev-2`, ... scheme.
- Ids are assigned after final bundle selection in selected-output order.
- The comparison shell treats selected item ids and ordering as exact fields.

N2, `reasons` comparison:

- Resolved in the design doc and implementation by classifying suspected-cause
  `reasons` as category tokens, not prose.
- `reasons`, suspected-cause `supporting`, suspected-cause `counter`, item
  `entities`, and item `missing_data` now use exact set comparison.
- Free-text structural comparison is reserved for natural-language fields such
  as evidence claims, suspected-cause notes, trap notes, next-check actions, and
  next-check rationales.

N3, canonical token payload determinism:

- Resolved by pinning the V1 estimator to `serde_json::to_vec` over a canonical
  estimator-only payload that omits `token_cost`.
- Added a regression test that pins the exact payload byte count and resulting
  token count for `deploy-bad-rollout` `ev-1`.
- Regenerated all current fixture bundle `token_cost` and `tokens_used` fields
  from the estimator.

## What Changed

Implemented `src/evidence_compiler.rs` and exported it from `src/lib.rs`.

The new module adds:

- runtime input type `EvidenceCompilerInput` with query, hot store, and derived
  context only;
- runtime structs for evidence compilation output, suspected causes, next
  checks, dropped evidence, and comparison reports;
- expected-artifact loading for comparison tests only;
- comparison helpers that separate exact fields, exact sets, numeric tolerance,
  estimator-owned tokens, and structural text fields;
- deterministic token estimation from compact JSON bytes;
- structured errors for loading, comparison, and token-estimation failures.

Added `tests/evidence_compiler.rs` to prove:

- current expected compilation artifacts load for comparison;
- a gold clone compares cleanly;
- text fields are structural and not verbatim;
- required empty text is a mismatch;
- suspected-cause `reasons` are exact category sets;
- the canonical token payload byte count is pinned;
- all current fixture token fields match the compiler estimator;
- runtime compiler input has no expected-artifact field.

Updated all current scenario `expected.json` bundle token fields to match the
new estimator and adjusted affected `get_evidence_bundle` tests to the migrated
budget numbers.

Updated `docs/core/evidence-compiler-ranking.md` to record the final Slice 1
decisions for selected ids, category-set reasons, and the serializer-backed token
estimator.

## Reviewer Focus

Please focus this round on:

1. Whether Slice 1 is complete enough to unblock Slice 2 candidate generation.
2. Whether the comparison shell correctly implements the intended exact,
   exact-set, numeric-tolerance, estimator-owned, and text-structural modes.
3. Whether the fixture token-field migration is acceptable and the pinned
   serializer test is sufficient to prevent estimator drift.
4. Whether `EvidenceCompilerInput` and the expected-artifact loader keep gold
   data out of runtime compilation.
5. Whether the `ev-N` selected-id convention and exact selected-order comparison
   are the right contract for later scoring and selection slices.

If approved, the next implementation scope should be Slice 2 only: generating
candidate evidence items from derived context and source-backed fixture data,
without yet implementing the full ranking and selection policy.

## Verification

All checks passed on the baseline commit:

- `git diff --check`
- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets --all-features`
- `cargo run --bin validate_fixtures`

Fixture validation reported `0 error(s), 0 warning(s)`.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
