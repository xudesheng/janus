# Evidence Compiler Ranking Review 4

- Baseline SHA: `e55562ba39efd5433db8ef427c4709155df51da8`
- Current milestone: completed Evidence Compiler Slice 3 scoring and suspected-cause ranking implementation ready for review
- Critical path: yes - scoring and suspected-cause ranking are required before token-budget selection can choose final evidence items around a causal hypothesis
- Milestone progress: added compiler-owned evidence-strength scoring dimensions, suspected-cause ranking from generated candidates, false-causality penalties, missing-data uncertainty ranking, tests for the review-3 F1 obligation, and formal design updates for the Slice 3 boundary
- Deferred milestone work: token-budget selection, selected `ev-N` assignment, next-check generation, store insertion, and `get_evidence_bundle` integration remain deferred because review 3 approved Slice 3 only

This round implements only the approved Slice 3 scope. It does not implement
token-budget selection, final selected output ordering, selected `ev-N`
assignment, next-check generation, store insertion of compiled records, or
`get_evidence_bundle` routing.

## Response To Review 3

F1, `strength` must become a real dimension:

- Fixed by adding `score_evidence_candidates` and making
  `generate_evidence_candidates` return scored candidates.
- Metric anomaly strength now combines `detector`, `magnitude`, `coverage`, and
  `source_ref_quality`. `confidence.detector` remains a separate dimension.
- Other candidate families now add source-specific strength dimensions such as
  severity, volume, exemplar quality, span/path specificity, time alignment,
  relationship confidence, signature similarity, gap materiality, validation
  impact, and contradiction quality.
- Added `scored_metric_strength_is_not_detector_confidence_copy`, which asserts
  a metric anomaly's `strength` is not just `confidence.detector`.

F2, counter-evidence claims echo derived notes:

- Not fully changed in this round. The generated counter-evidence candidates
  still may use derived window-comparison notes as candidate text, which review
  3 classified as allowed derived input rather than a gold leak.
- This remains tied to D3 text anchoring and should be fixed before Slice 6,
  when generated prose becomes part of final `get_evidence_bundle` acceptance.

F3, source-ref inference and delta classification heuristics:

- No direct action this round because review 3 marked this as low/robustness
  with no action required.
- Existing candidate-source resolution coverage remains intact.

## What Changed

Added Slice 3 scoring and ranking helpers:

- `score_evidence_candidates(input, candidates)`
- `rank_suspected_causes_from_candidates(input, candidates)`

Candidate generation now returns scored `cand-*` candidates. The ids remain
internal and are not public selected bundle ids.

The suspected-cause ranker:

- aggregates supporting and counter candidates by entity;
- computes causal suspicion separately from evidence item strength;
- links `supporting` and `counter` to generated `cand-*` ids;
- emits deterministic reason category tokens;
- rolls runtime-child evidence such as `pod:` evidence up to owning `service:`
  suspects;
- downgrades suspects whose counter score dominates support and emits a
  `trap_note`;
- emits `under-determined` when material telemetry gaps make diagnosis unsafe.

Updated tests to verify:

- scored candidates remain source-backed and estimator-owned;
- metric anomaly strength is not a detector-confidence copy;
- the coincidental deploy trap ranks `infra:redis-cache` above
  `service:search-ui`, while keeping the false deploy inspectable with explicit
  counter-evidence and a `trap_note`;
- missing-data ranking surfaces `under-determined` with telemetry-gap reasons
  and supporting evidence.

Updated `docs/core/evidence-compiler-ranking.md` to record:

- candidate generation now returns scored candidates after Slice 3;
- `score_evidence_candidates` and `rank_suspected_causes_from_candidates` are
  internal helpers;
- Slice 3 keeps `cand-*` ids and does not perform token-budget selection,
  selected `ev-N` assignment, next-check generation, store insertion, or public
  query integration.

## Reviewer Focus

Please focus this round on:

1. Whether F1 is fully discharged: is `EvidenceItem.strength` now sufficiently
   distinct from `confidence.detector` and from causal suspicion?
2. Whether `rank_suspected_causes_from_candidates` is the right Slice 3
   boundary, given that links still use `cand-*` ids until Slice 4 selection.
3. Whether the false-causality penalties are strong enough for the
   coincidental-deploy and missing-data traps without overfitting to exact gold.
4. Whether the reason category tokens are acceptable as a first deterministic
   suspected-cause surface before final fixture comparison.
5. Whether D3 text anchoring can remain deferred until before Slice 6, or must
   be pulled earlier now that suspected-cause prose exists.

If approved, the next implementation scope should be Slice 4 only: token-budget
selection, whole-item dropping/reporting, selected `ev-N` assignment, and
counter-evidence requirement enforcement. Next-check generation, store
insertion, and `get_evidence_bundle` integration should remain deferred.

## Verification

All checks passed on the baseline commit:

- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets --all-features`
- `cargo run --bin validate_fixtures`
- `git diff --check`

Fixture validation reported `0 error(s), 0 warning(s)`.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
