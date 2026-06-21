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

## Review (by Claude (Opus 4.8))

### Direction Verdict

Implementation round (Slice 2: candidate generation, plus the review-2 oracle
fixes).

**On critical path: yes. Milestone progress: real and independently verified.
Next action: continue — I approve starting Slice 3 (scoring + suspected-cause
ranking only).**

Slice 2 delivers the approved scope and the review-2 fixes hold up under code
inspection, not just the prose:

- **D1 fixed (verified):** `has_expected_mismatches`
  (`src/evidence_compiler.rs:196-204`) now includes `extra_suspected_causes` and
  `extra_next_checks`, and `extra_suspected_causes_and_next_checks_are_mismatches`
  asserts an appended extra cause (rank 99) and extra check (index 3) fail the
  gate. The over-generation hole is closed.
- **D2 fixed (verified):** next-check `expected_signal` is now `compare_exact`
  (`:1424-1431`), the design reclassifies it as an exact category token, and
  `next_check_expected_signal_is_exact_category_token` covers it.
- **D3:** correctly left as a tracked limitation; the design now states
  text-structural = non-empty-required + tracked non-blocking differences until
  anchoring lands. See my answer to focus-question 5 for the deadline.

The candidate surface itself is sound. `generate_evidence_candidates` returns
`Vec<EvidenceCandidate>` with internal `cand-NNN` ids, separate from public
`ev-N` selection, covering all eight approved families. Generation is
source-backed (`push_resolvable_source_ref` only keeps refs that resolve
`Found` in the store) and deterministic (Vec iteration + push-order ids). Most
importantly, the anti-leak boundary holds: the generator reads only derived
context and raw store records and never touches `expected.evidence_bundle`,
`expected.suspected_causes`, or `expected.next_checks`. The corpus test builds
the store from raw replay (`plan_fixture_replay` / `replay_plan_into_store`),
not from gold, and asserts every candidate validates as Evidence IR, resolves
its source refs, carries estimator token costs, and that all eight families are
exercised. That is a genuinely strong Slice 2 test.

I independently ran the gate on baseline `6d09a43`: `cargo fmt --check` clean,
`cargo clippy --all-targets --all-features` clean, `cargo test` all green
(`evidence_compiler` now 11 tests, the 3 new ones included).

None of the notes below retracts approval. F1 is the one that matters: it is the
central obligation Slice 3 must discharge.

### F1 — Slice 3 must make `strength` a real, distinct dimension (forward obligation, not a Slice 2 defect)

Right now provisional candidate `strength` either mirrors another signal or is a
hardcoded constant:

- `push_metric_anomaly_candidates` sets `strength = window.detector_confidence`
  **and** `confidence["detector"] = window.detector_confidence` — the two are
  literally the same number, which is exactly the strength/detector conflation
  the design's Scoring Model warns against ("keep `confidence.detector` distinct
  from strength").
- other families use fixed constants (`0.72` / `0.78` / `0.70` / `0.75`).

This is acceptable as Slice 2 candidate metadata, and the implementor flagged it
as provisional. But Slice 3 must replace these with genuine evidence-strength
dimensions (source-ref quality, magnitude, exemplar specificity, coverage,
entity-resolution confidence, recency) that are demonstrably distinct from both
`confidence.detector` and causal suspicion. Please add a Slice 3 test that
asserts an item's `strength` is not just a copy of its `confidence.detector`, so
the placeholder cannot silently survive.

### F2 — Counter-evidence claims currently echo hand-authored derived notes (low)

`push_counter_evidence_candidates` uses `delta.note` verbatim as the claim
(e.g. "database latency is flat; the DB is counter-evidence, not the cause"
from the `window_comparison` delta). Window comparison is an allowed derived
input, not one of the three forbidden artifacts, so this is **not** a gold leak,
and claim text is non-blocking. Noting it only so that when D3 anchoring lands,
these claims are compiler-anchored rather than pass-throughs of hand-authored
derived prose.

### F3 — Source-ref inference and delta classification are heuristics (low/robustness)

`infer_source_signal` is a prefix/substring heuristic and
`push_resolvable_source_ref` silently drops any ref whose inferred signal does
not resolve. For the current corpus this is covered by the resolve test, but a
misinference drops a legitimate ref rather than surfacing it.
`is_counter_evidence_delta` likewise leans on substring `flat`/`counter` and a
`factor <= 1.20` threshold. Both are fine for Slice 2 (over-generation is
filtered later in selection), but they are corpus-shaped; if ref naming or
delta semantics broaden, revisit them. No action required this round.

### Answers to the implementor's reviewer-focus questions

1. D1 and D2 are fully resolved and the oracle is sound enough for Slice 3.
   Yes — verified in code and tests.
2. `generate_evidence_candidates -> Vec<EvidenceCandidate>` is the right
   internal boundary. Yes; internal `cand-*` ids kept distinct from selected
   `ev-N` is exactly what Slice 3 needs to consume.
3. Source families and provenance rules are broad enough without leaking gold.
   Yes — all eight families exercised, every candidate source-backed, forbidden
   artifacts untouched.
4. Provisional candidate strengths/directions/claims are acceptable as candidate
   metadata. Yes, as placeholders — subject to F1 for Slice 3.
5. D3 does **not** need to be upgraded to blocking before Slice 3.
   Suspected-cause acceptance in Slice 3 rests on `entity`, `reasons` (exact
   category set), `supporting`/`counter` (exact id sets), `rank`, and `score`
   (numeric tolerance), all of which the oracle checks; unanchored prose is
   low-risk there. D3 must land before Slice 6, when generated prose becomes the
   final `get_evidence_bundle` acceptance gate. Slice 3 should still keep
   suspected-cause prose deterministic and referencing the cause entity.

**Approved next scope: Slice 3 only** — evidence-strength dimensions, causal
suspicion scoring, false-causality penalties, and suspected-cause ranking from
the generated candidates. Token-budget selection, next-check generation, store
insertion, and `get_evidence_bundle` integration remain deferred.

### Verification

Independently ran on baseline `6d09a43`: `cargo fmt --check` (clean),
`cargo clippy --all-targets --all-features` (clean), `cargo test` (all suites
green, `evidence_compiler` 11 tests incl. the 3 new). Read the full
`src/evidence_compiler.rs` candidate-generation diff, the design-doc diff, and
the test diff; confirmed the D1/D2 fixes are present in code and that candidate
generation never reads the three forbidden expected artifacts. Branch confirmed
`evidence-compiler-ranking`.
