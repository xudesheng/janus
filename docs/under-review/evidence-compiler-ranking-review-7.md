# Evidence Compiler Ranking Review 7

- Baseline SHA: `0e021ff`
- Current milestone: completed Slice 6 implementation ready for review
- Critical path: yes - this round routes public evidence bundle queries through compiled evidence and closes the D-OVERFIT gate from review 6
- Milestone progress: replaced entity-name causal multipliers with structural ranking signals, made selected-output comparison structural across items/suspected causes/next checks, routed `get_evidence_bundle` through fixture replay + derived context + `compile_evidence`, and proved current corpus structural comparison passes
- Deferred milestone work: none known for this topic if reviewers accept the Slice 6 comparison/ranking decisions

## Response To Review 6

Review 6 approved Slice 6 only if the structural ranking refactor and selected
output comparison decision landed before or with public query routing. This
round does that sequencing:

- Removed `entity_causal_multiplier` and replaced it with
  `structural_causal_multiplier`, which uses candidate source family,
  confidence dimensions, anomaly magnitude/activity, source-ref richness,
  change alignment/kind, trace specificity, relationship direction, fallback
  attributes, retry topology, counter-evidence dominance, and local runtime
  failure signatures.
- Added relationship-aware suspected-cause support adjustment. Explicit
  `retries` edges move overload suspicion back to the caller and add counter
  pressure to the downstream suspect; ordinary dependency edges can move
  symptom support toward the dependency, unless the dependency is already
  counter-dominated, marked as fallback load, or the source has a local
  OOM/restart/memory signature.
- Changed comparison from exact positional fixture equality to structural
  outcomes. Selected items match by structural identity, suspected causes focus
  on rank-1 causes and trap handling, and next checks match by normalized
  `expected_signal` category with aliases such as `code_change -> change_event`
  and `entity-resolution -> entity_resolution`.
- Routed `get_evidence_bundle` through raw fixture replay, derived-context
  insertion, and `compile_evidence`; it no longer loads
  `expected.evidence_bundle` as the response source.

## What Changed

`get_evidence_bundle(EvidenceQuery)` now:

- validates the query;
- loads the selected fixture case only to replay source input;
- builds a fresh `HotContextStore` through `plan_fixture_replay` and
  `replay_plan_into_store`;
- runs `derive_and_insert_context`;
- checks the query time/entity selectors against the hot store;
- calls `compile_evidence`;
- validates the compiled bundle and preserves the existing budget, raw-ref,
  counter-evidence, and source-ref resolution checks.

The old public-query tests that asserted exact fixture-gold equality now assert
compiled-bundle contracts: valid Evidence IR, deterministic `ev-N` ids, budget
integrity, non-empty source refs, whole-item budget dropping, counter-evidence
requirements, and current-corpus source-ref resolution.

The full corpus now has an explicit structural regression test:
`compiled_output_structurally_matches_current_corpus`.

Updated `docs/core/evidence-compiler-ranking.md` to record the Slice 6
implementation shape and the selected-output comparison decision.

## Reviewer Focus

Please focus this round on:

1. Whether the structural comparison is the right final contract for this
   topic, especially its treatment of extra selected evidence and non-verbatim
   next-check prose.
2. Whether the relationship-aware causal adjustment is acceptable as a generic
   D-OVERFIT replacement, or whether any part still looks fixture-tuned.
3. Whether `get_evidence_bundle` now preserves the public contract while
   removing fixture-gold bundle lookup from the runtime path.
4. Whether the Slice 6 full-corpus verification is strong enough to close the
   topic, or whether reviewers want one more narrow follow-up before
   termination.

If approved, I believe this topic meets the Definition of Done and should
terminate rather than spawning another implementation round.

## Verification

All checks passed:

- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets --all-features`
- `cargo run --bin validate_fixtures`
- `git diff --check`

Fixture validation reported `0 error(s), 0 warning(s)`.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
