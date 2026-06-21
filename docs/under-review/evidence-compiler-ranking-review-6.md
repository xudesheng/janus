# Evidence Compiler Ranking Review 6

- Baseline SHA: `040fff37ecc8c7b8d2344eaff5c3e72bd325d14c`
- Current milestone: completed Evidence Compiler Slice 5 next-check generation and store insertion implementation ready for review
- Critical path: yes - next checks and inspectable compiled records are required before Slice 6 can route `get_evidence_bundle` through compiler output
- Milestone progress: added deterministic next-check generation to `compile_evidence`, added `insert_evidence_compilation` for selected evidence, suspected-cause, and next-check records, kept compiled records out of raw source inputs, addressed the top-cause support-selection note, and recorded Slice 5 plus Slice 6 comparison decisions in the formal design
- Deferred milestone work: public `get_evidence_bundle` integration remains deferred because review 5 approved Slice 5 only; the D-OVERFIT structural ranking refactor and selected-item comparison decision remain deferred to before/with Slice 6 gold-gating

This round implements only the approved Slice 5 scope. It does not route the
public query path through compiled evidence, does not gold-gate selected bundle
items against fixture expected output, and does not perform the D-OVERFIT
structural causal-ranking refactor.

## Response To Review 5

Review 5 approved Slice 5: deterministic next-check generation and compiled
record store insertion without polluting raw source records. This round follows
that approved scope.

D-OVERFIT structural ranking refactor:

- Not implemented in this round because review 5 explicitly allowed it to defer
  once and approved Slice 5 as orthogonal.
- The formal design still records that suspected-cause ranking must move away
  from fixture-specific entity-name multipliers before Slice 6 gold-gated
  suspected-cause comparison.
- Review 6 asks reviewers to confirm whether Slice 6 should bundle this refactor
  with public query integration, or whether it should be split into a
  diagnosis/refactor round before public query integration begins.

Forward concern, exact selected-item comparison:

- Added a formal design note that before Slice 6 makes selected evidence items
  gold-gated, the project must explicitly decide whether selected item ids and
  ordering remain exact targets or move to a structural presence/order check.
- No code comparison change was made in this round because Slice 5 does not yet
  run generated selected bundles against fixture gold.

Smaller review notes:

- Adjusted the top-cause preservation step so it filters to supporting
  candidates. Counter-evidence can still be forced by the counter requirement
  and final deterministic fill, but the top-cause preservation intent is now
  explicit.
- Did not add exact assertions for dropped-candidate reasons. The existing
  dropped reason remains diagnostic and post-hoc, as review 5 recommended.

## What Changed

`compile_evidence` now attaches deterministic next checks after candidate
generation, scoring, suspected-cause ranking, and token-budget selection.

Added `suggest_next_checks(input, bundle, suspected_causes)`. The V1 generator
derives checks from selected output only:

- selected missing-data evidence;
- selected counter-evidence for false-causality risks;
- top suspected-cause support links;
- weak top-cause scores that need another independent signal.

It emits at most three checks with exact `expected_signal` category tokens such
as `metric_anomaly`, `log_cluster`, `change_event`, `compare_windows`,
`relationship`, `find_related_anomalies`, `profile_hotspot`, and `trace`.

Added `insert_evidence_compilation(store, compilation)`, which inserts:

- selected evidence items keyed by their `ev-N` ids as
  `StoredRecordKind::EvidenceItem`;
- suspected causes keyed as `suspected-cause:<rank>`;
- next checks keyed as `next-check:<rank>`.

The insertion path uses `HotContextStore::insert_record`, making compiled
records selectable and inspectable through the existing store boundary while
leaving `raw_source_records()` unchanged.

Updated tests to verify:

- `compile_evidence` generates deterministic, non-empty next checks;
- missing-data evidence produces a recovery/inspection next check;
- compiled evidence, suspected-cause, and next-check records are inserted under
  the expected derived record kinds;
- insertion does not change the raw source record count;
- inserted evidence record payloads round-trip to the selected `EvidenceItem`;
- duplicate compiled record keys are rejected by the store.

Updated `docs/core/evidence-compiler-ranking.md` to record:

- the Slice 5 next-check generation contract;
- the Slice 5 store insertion contract;
- the selected-item comparison decision that must be made before Slice 6.

## Reviewer Focus

Please focus this round on:

1. Whether `suggest_next_checks` is the right deterministic V1 boundary, or
   whether next-check generation should be split into smaller source-family
   helpers before Slice 6.
2. Whether the next-check priority order is acceptable: missing data first,
   false-causality counter checks second, top-cause confirmation or weak-score
   discrimination third.
3. Whether `insert_evidence_compilation` should remain a separate explicit
   store-insertion step, or whether Slice 6 should introduce a combined
   compile-and-insert helper before routing `get_evidence_bundle`.
4. Whether the derived-store record keys and payloads are sufficient for
   inspectability at this stage.
5. Whether Slice 6 should begin with the D-OVERFIT structural ranking refactor
   and selected-item comparison decision before public query routing, or can
   include them inside the same Slice 6 implementation round.

If approved, the next implementation scope should be Slice 6. That round must
not defer D-OVERFIT again: it should either perform the structural ranking
refactor and selected-item comparison decision first, or explicitly split them
into a pre-routing round before `get_evidence_bundle` integration.

## Verification

All checks passed on the baseline commit:

- `cargo fmt --check`
- `cargo test --test evidence_compiler -- --nocapture`
- `cargo test`
- `cargo clippy --all-targets --all-features`
- `cargo run --bin validate_fixtures`
- `git diff --check`

Fixture validation reported `0 error(s), 0 warning(s)`.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
