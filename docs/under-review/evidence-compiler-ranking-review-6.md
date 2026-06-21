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

## Review (by Claude (Opus 4.8))

### Direction Verdict

Implementation round (Slice 5: next-check generation + store insertion).

**On critical path: yes. Milestone progress: real and verified. Next action:
continue to Slice 6 — but Slice 6 is now a hard gate: the D-OVERFIT structural
ranking refactor and the selected-output comparison decision MUST land in or
before it. D-OVERFIT has now been deferred across two consecutive rounds; a
third deferral cannot be accepted under the review framework.**

Slice 5 delivers the approved scope cleanly:

- **`suggest_next_checks` is deterministic and source-derived.** It reads only
  selected bundle items + suspected causes (never gold), emits ≤3 checks in a
  sensible priority order (missing-data recovery → counter-evidence validation →
  top-cause confirmation / weak-score discrimination), and — unlike the
  suspected-cause reason tokens — derives `expected_signal` structurally from
  `EvidenceKind` rather than from hardcoded fixture names. That is the right
  pattern.
- **Store insertion is correct and non-polluting.** `insert_evidence_compilation`
  writes `ev-N` / `suspected-cause:<rank>` / `next-check:<n>` records via
  `insert_record`, the test confirms `raw_source_records().count()` is unchanged,
  evidence payloads round-trip back to `EvidenceItem`, and duplicate keys are
  rejected with `DuplicatePrimaryKey`. Inspectability through the existing store
  boundary is achieved as the design requires.
- **The review-5 note is addressed:** `sorted_candidates_for_cause` now filters
  to `Supports`, so the top-cause preservation step can no longer be satisfied by
  an already-selected counter item.

I ran the gate on baseline `040fff3`: `cargo fmt --check` clean, `cargo clippy
--all-targets --all-features` clean, `cargo test` all green (`evidence_compiler`
now 22 tests, +3), `cargo run --bin validate_fixtures` reports `0 error(s), 0
warning(s)`.

### Deferral tracking — D-OVERFIT is at its limit (this is the binding constraint)

Per the framework, the same milestone-critical item cannot be deferred more than
twice consecutively. The D-OVERFIT structural ranking refactor (replace
`entity_causal_multiplier`'s entity-name multipliers with structural signals;
derive reason tokens from structured source content) was deferred in round 5
(#1) and remains undone in round 6 (#2). Both deferrals were reviewer-approved as
orthogonal scope with a stated deadline of "before/with Slice 6." That deadline
is now binding: **Slice 6 must perform the refactor — it cannot be pushed to a
Slice 7.** I will not approve a round that routes `get_evidence_bundle` through
the compiler while the causal layer is still entity-name-keyed.

### The Slice 6 comparison decision now covers THREE artifacts, not one

Review 5 flagged that selected `items` (ids + ordering) face the same
exact-vs-structural tension as `reasons`. Slice 5 makes clear the decision spans
all three gold-gated artifacts, and I now have data on each:

- **Selected items:** Exact ids + ordering vs structural presence/order
  (undecided; the selector's `candidate_selection_group` order is unverified
  against gold).
- **Suspected causes:** `reasons` already relaxed to subset, but `rank`,
  `entity`, `supporting`, `counter`, and `score` are still exact/tolerance — and
  the entity-name ranking that produces them is the D-OVERFIT overfit.
- **Next checks:** compared positionally with **exact** `expected_signal`. The
  compiler's `expected_signal` vocabulary does not cover gold: gold uses
  `code_change` (1×) and `entity-resolution` (1×), which the compiler never
  emits (it emits `change_event`), and the compiler's own 3 checks in its own
  priority order will not positionally align with hand-authored gold checks.

So Slice 6 must make one explicit comparison decision covering all three. My
recommendation, consistent with the D-OVERFIT resolution: judge by **structural
outcomes** — for items, the key support and counter items are present and ordered
by the documented group rule; for suspected causes, the true cause ranks #1 with
innocents low and trap-noted; for next checks, the right *category* of check is
present (subset / structural `expected_signal`), not positional verbatim
equality. Exact verbatim positional equality across hand-authored prose and
vocabularies will force fixture tuning, which is the trap we already chose to
avoid for `reasons`.

### Answers to the implementor's reviewer-focus questions

1. `suggest_next_checks` is the right V1 boundary; a single deterministic
   generator over selected output is fine. No need to split into per-family
   helpers now.
2. The next-check priority order (missing-data → counter → top-cause
   confirm/weak-score) is acceptable and matches the design's next-check
   examples.
3. Keep `insert_evidence_compilation` as a separate explicit step. Slice 6 may
   add a thin compile-and-insert convenience for the `get_evidence_bundle` path,
   but do not fuse compile+insert by default — callers may want compiled output
   without store mutation.
4. The derived-store keys and payloads are sufficient for inspectability:
   per-kind selectable, full-payload round-trip, stable keys.
5. **Slice 6 sequencing:** do the D-OVERFIT structural refactor and the
   comparison decision **first** (a focused pre-routing step, or a diagnosis-only
   sub-round), prove the corpus passes the chosen comparison, and only then flip
   `get_evidence_bundle` to compiled output. Bundling them into one round is
   acceptable only if the refactor and decision land before the routing flip.
   Either way, no third deferral.

**Approved next scope: Slice 6** — `get_evidence_bundle` integration and
full-corpus verification, carrying (a) the D-OVERFIT structural ranking refactor
and (b) the selected-items / suspected-causes / next-checks comparison decision.
This is the final slice; after its review, if the Definition Of Done is met with
no open feedback, the topic terminates rather than spawning an empty round.

### Smaller note (low)

- `insert_evidence_compilation` uses `.expect(...)` on `serde_json::to_value` for
  the compiled structs. Safe in practice (plain `Serialize` structs, string
  keys), but propagating a `HotStoreError`/serialization error would be tidier if
  these types ever gain non-string-keyed maps. Non-blocking.

### Verification

Independently ran on baseline `040fff3`: `cargo fmt --check` (clean),
`cargo clippy --all-targets --all-features` (clean), `cargo test` (all green,
`evidence_compiler` 22 tests incl. the 3 new), `cargo run --bin validate_fixtures`
(`0 error(s), 0 warning(s)`). Read the full Slice 5 diff and cross-checked the
generated next-check `expected_signal` vocabulary against gold
`next_checks[].expected_signal` across the corpus. Branch confirmed
`evidence-compiler-ranking`.
