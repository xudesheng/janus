# Evidence Compiler Ranking Review 3

- Baseline SHA: `6d09a43556c404f92d631c6d2e1407fabdd4baa4`
- Current milestone: completed Evidence Compiler Slice 2 candidate-generation implementation ready for review, plus the Slice 1 comparison-oracle fixes required by review 2
- Critical path: yes - review 2 approved Slice 2 only, and source-backed candidates are the required input to later scoring, suspected-cause ranking, token selection, store integration, and `get_evidence_bundle` routing
- Milestone progress: added internal evidence candidate generation across the approved source families, fixed the comparison oracle holes for extra suspected causes / next checks and next-check `expected_signal`, and verified source-backed candidates across the current corpus
- Deferred milestone work: none for Slice 2; Slices 3-6 remain unimplemented because they were outside the approved scope and should proceed only after this review

This round implements only the approved Slice 2 scope. It does not implement
full ranking, token-budget selection, selected `ev-N` assignment,
suspected-cause generation, next-check generation, store insertion of compiled
evidence, or `get_evidence_bundle` integration.

## Response To Review 2

D1, extras ignored by `has_expected_mismatches()`:

- Fixed by including `extra_suspected_causes` and `extra_next_checks` in
  `EvidenceCompilationComparison::has_expected_mismatches()`.
- Added a test that appends an extra suspected cause and an extra next check and
  asserts the comparison is no longer clean.

D2, `expected_signal` compared as structural text:

- Fixed by comparing next-check `expected_signal` exactly instead of routing it
  through text-structural comparison.
- Updated `docs/core/evidence-compiler-ranking.md` to classify
  `expected_signal` as an exact category token alongside suspected-cause
  `reasons`.
- Added a test that changes `expected_signal` and asserts an expected mismatch.

D3, text-structural anchoring:

- Not fully implemented in this round because generated prose is still
  candidate-level and is not yet the acceptance gate for final selected output.
- Tracked in the formal design doc: text-structural fields are currently
  non-empty plus tracked differences, and later slices must add field-specific
  anchoring before relying on generated prose as final evidence acceptance.

Forward note, budget retuning:

- Not changed in this round. Slice 2 does not select final bundles or retune
  fixture budgets. The formal design now states that internal `cand-*` token
  costs are preliminary and selected `ev-N` outputs must recompute token fields
  in later selection slices.

## What Changed

Added an internal candidate-generation surface:

- `EvidenceCandidate`
- `EvidenceCandidateSource`
- `generate_evidence_candidates(EvidenceCompilerInput)`

The generator produces internal `cand-001`, `cand-002`, ... evidence items from:

- metric anomaly windows;
- log patterns;
- change records, including non-causal changes as counter-evidence candidates;
- trace exemplars;
- dependency relationship records;
- prior incident records;
- telemetry gaps;
- flat or counter-indicated window-comparison deltas.

Candidate generation is intentionally not final bundle selection:

- candidate ids are internal `cand-*`, not selected public `ev-N` ids;
- selected ordering and budget selection are not implemented;
- suspected causes and next checks are not generated;
- `get_evidence_bundle` still uses the existing fixture-backed path.

Each generated candidate must have at least one resolvable source ref before it
is emitted. Candidate token costs are computed with the existing compiler-owned
estimator, but later slices must recompute token fields after selected `ev-N`
assignment.

Updated `tests/evidence_compiler.rs` to verify:

- D1 extras fail comparison;
- D2 `expected_signal` category drift fails comparison;
- current fixture candidates validate as Evidence IR;
- every candidate source ref resolves through the hot store;
- candidate ids are internal and unique per compilation;
- candidate token costs come from the compiler estimator;
- the current corpus exercises all approved candidate source families.

Updated `docs/core/evidence-compiler-ranking.md` to record:

- internal Slice 2 candidate ids and the lack of final selected output;
- exact-category comparison for next-check `expected_signal`;
- the current limitation of text-structural anchoring;
- the explicit boundary that Slice 2 does not perform ranking, selection,
  suspected-cause ranking, next-check generation, store insertion, or public
  query integration.

## Reviewer Focus

Please focus this round on:

1. Whether D1 and D2 are fully resolved and the comparison oracle is sound
   enough for Slice 3.
2. Whether `generate_evidence_candidates` is the right internal boundary for
   Slice 2, or whether a different candidate API shape is needed before scoring.
3. Whether the source families and provenance rules are broad enough for the
   current corpus without leaking fixture gold.
4. Whether provisional candidate strengths, directions, and claims are
   acceptable as candidate metadata, given that ranking and final text anchoring
   are deferred.
5. Whether D3 must be upgraded from tracked limitation to blocking work before
   Slice 3.

If approved, the next implementation scope should be Slice 3 only: evidence
strength dimensions, causal suspicion scoring, false-causality penalties, and
suspected-cause ranking from generated candidates. Token-budget selection,
next-check generation, store insertion, and `get_evidence_bundle` integration
should remain deferred.

## Verification

All checks passed on the baseline commit:

- `git diff --check`
- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets --all-features`
- `cargo run --bin validate_fixtures`

Fixture validation reported `0 error(s), 0 warning(s)`.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
