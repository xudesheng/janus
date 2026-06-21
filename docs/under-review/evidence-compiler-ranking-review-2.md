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

## Review (by Claude (Opus 4.8))

### Direction Verdict

This is the first **implementation** round (Slice 1).

**On critical path: yes. Milestone progress: real and independently verified.
Next action: continue — I approve starting Slice 2 (candidate generation only),
with two comparison-shell fixes (D1, D2) required before the oracle is trusted
to gate suspected causes / next checks.**

Slice 1 delivered what review 1 approved and the pieces hold up:

- `EvidenceCompilerInput` carries only `query` / `store` / `derived`; the
  expected-artifact loader is a separate `load_expected_compilation` used by
  tests. Gold genuinely cannot reach runtime compilation
  (`runtime_input_excludes_expected_artifacts` plus the type itself confirm it).
- The token estimator matches the design: canonical payload struct with
  `token_cost` omitted, `serde_json::to_vec`, `div_ceil(4)`, and a u32 overflow
  guard. The migration is internally consistent — for `deploy-bad-rollout`,
  `122+117+111+118+118 = 586 = tokens_used`, and the pinned test
  (`486 bytes → 122 tokens`) locks it against drift. The old `235 vs 250`
  contradiction is gone.
- Comparison modes are implemented and tested: exact, exact-set
  (`reasons`/`supporting`/`counter`/`entities`/`missing_data`), numeric
  tolerance (0.05 for `strength`/confidence/`score`), estimator-owned token
  fields, and text-structural prose.
- N1/N2/N3 are recorded in the design doc (ev-N ids after selection, `reasons`
  as exact category set, `serde_json::to_vec` + pinned byte count).

I independently ran the full gate on the baseline (`9486360`): `cargo fmt
--check` clean, `cargo test` all green (the 8 `evidence_compiler` tests
included), `cargo clippy --all-targets --all-features` clean, and
`cargo run --bin validate_fixtures` completed. The implementor's verification
claims check out.

The findings below do not retract approval of Slice 1, but **D1 and D2 must be
fixed before slices 3/5 rely on this shell to accept generated suspected causes
and next checks** — the shell is the acceptance oracle for every later slice, so
a hole in it silently weakens all of them. They are small and could fold into
Slice 2.

### D1 — `has_expected_mismatches()` ignores extras, so over-generation reads as clean (medium)

`has_expected_mismatches` (`src/evidence_compiler.rs:173-181`) checks
`missing_suspected_causes` and `missing_next_checks` but **not**
`extra_suspected_causes` (populated at `:552`) or `extra_next_checks`
(populated at `:672`). A compiler that emits a spurious extra suspected cause
(a rank not in gold) or an extra next check would produce a comparison the gate
reports as clean. Over-generation is a likely failure mode for slices 3 and 5,
and this is exactly what the oracle must catch. Either include both extra
vectors in `has_expected_mismatches`, or, if extras are intentionally tolerated,
document why in the function and a test. I recommend failing on extras.

### D2 — `expected_signal` is compared as text-structural, so category errors are invisible (medium)

next-check `expected_signal` values are category tokens (`code_change`,
`metric_anomaly`, `log_cluster`), the same shape as `reasons`, which N2
correctly moved to exact-category-set comparison. But `expected_signal` is still
compared with `compare_text_structural` (`:662-669`), and text-structural
differences are routed to `text_differences`, which `has_expected_mismatches`
excludes entirely. Net effect: a compiler emitting the wrong
`expected_signal` category passes the gate. Treat `expected_signal` as an exact
category token (ideally a closed vocabulary), like `reasons`. The same question
applies to anything else token-shaped that is currently text-structural — audit
the list once.

### D3 — text-structural "anchoring" from the design is not implemented (low, track it)

The design says text-structural fields must be "deterministic, non-empty where
required, and **anchored to the compared entities, source refs, evidence ids, or
reason/check categories**." The shell only enforces non-empty
(`compare_text_structural`, `:774-801`); anchoring is not checked, so today any
non-empty deterministic string passes for `claim` / `hypothesis` / `note` /
`action` / `rationale`. That is defensible for Slice 1 (there is no generated
text to anchor yet), but it means prose is effectively unchecked. Track it as an
explicit Slice 2+ obligation rather than letting it lapse, otherwise the
text-structural mode is "non-empty" in practice.

### Forward note — re-tune `max_tokens` for the new token scale (Slice 4, not a Slice 1 defect)

The migration scaled `token_cost` up ~2.5× (e.g. 45 → 122) but left the
hand-authored `max_tokens` budgets untouched. All twelve fixtures still fit, but
several are now tight (`681/700`, `586/600`) where they previously had wide
headroom for ~50-token items. The budgets' original intent ("comfortably fits N
items") no longer holds at the new scale. Slice 4 should deliberately re-tune
`max_tokens` (and/or design fixtures that force drops) so budget-selection tests
exercise real pressure rather than near-trivial fits. Flagging now so the
re-tune is a conscious decision, not a mid-slice surprise.

### Answers to the implementor's reviewer-focus questions

1. Slice 1 is complete enough to unblock Slice 2. Yes.
2. The modes are implemented correctly except the two oracle holes above (D1
   extras, D2 `expected_signal`).
3. The token migration is acceptable and the pinned serializer test is
   sufficient against drift. Yes.
4. `EvidenceCompilerInput` and the loader keep gold out of runtime. Yes.
5. The `ev-N` selected-id convention and exact selected-order comparison are the
   right contract. Yes — that exact coupling is where slices 2-5 will be judged,
   as flagged in review 1.

**Approved next scope: Slice 2 only** (source-backed candidate generation, no
full ranking/selection policy yet). Fold D1 and D2 in along the way so the oracle
is sound before slices 3/5 depend on it.

### Verification

Independently ran on baseline `9486360`: `cargo fmt --check` (clean),
`cargo test` (all suites green, incl. 8 `evidence_compiler` tests),
`cargo clippy --all-targets --all-features` (clean), `cargo run --bin
validate_fixtures` (completed). Also diffed the design doc and all twelve
`expected.json` token migrations, and checked every fixture's
`tokens_used <= max_tokens`. Branch confirmed `evidence-compiler-ranking`.
