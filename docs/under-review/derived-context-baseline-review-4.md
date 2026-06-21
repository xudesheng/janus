# Derived Context Baseline Review 4

- Baseline SHA: `276d520a53d8a901322ac2e899744f3f097ec93f`
- Current milestone: Milestone 5B derived-context baseline, with review-3 cleanup and Slice 3 log-pattern derivation now implemented
- Critical path: yes - this round removes two review-3 blockers before extending the next generator slice, then adds source-backed log-pattern derivation for fixtures that declare `log-pattern-clustering`
- Milestone progress: replaced fixture-specific delta-note prose with secondary note comparison and deterministic flat notes, named and tested the dependency baseline warm-up blend, and added `derive_log_context` with corpus coverage over current log-pattern gold
- Deferred milestone work: Slice 4 timeline construction, Slice 5 related-anomaly derivation, final full-pipeline/store integration, and the lower-severity metric ordering cleanup remain; this round focused on the review-3 blockers plus the next approved generator slice

This round follows review 3's Direction Verdict: continue to Slice 3 after
folding in Findings 1-2.

## Response To Review 3

Finding 1: `window_delta_note` reproduced gold editorial prose by fixture
identity.

Fixed in `src/derived_context.rs`:

- removed the fixture-class/entity lookup table that emitted exact gold prose;
- derived only a compact deterministic `flat` note when the computed delta is
  flat by metric tolerance or by a small factor tolerance;
- moved window-delta `note` comparison into
  `window_delta_note_differences`, which is visible but not part of
  `has_expected_mismatches`;
- added the same secondary-note channel for related-anomaly notes before that
  generator slice depends on exact prose;
- added a regression test proving an editorial expected note and a derived
  `flat` note do not create a fatal mismatch.

Finding 2: `anomaly_baseline` used an unexplained `0.85/0.15` blend.

Addressed in `src/derived_context.rs`:

- promoted the latest-point weight to
  `PRE_ONSET_WARMUP_BLEND_LATEST_WEIGHT`;
- added a code comment explaining the rule: one pre-onset warm-up point should
  be acknowledged without replacing the earliest stable baseline;
- added a focused test for the dependency fixture baseline.

Finding 3: per-fixture anomaly/window ordering tables remain lower-severity
debt.

Not fully changed in this round. The existing metric ordering tables were not
expanded. For the new log-pattern slice, the generator uses the design's general
stable ordering by first seen, entity, severity, template, and input order. That
exposes one existing fixture-gold wrinkle: the retry-storm expected log pattern
ids are not chronological by first seen. To avoid adding a new per-fixture id
table, log-pattern comparison now matches by natural identity
`entity|severity|template`, reports id drift through
`log_pattern_id_differences`, and keeps the mismatch nonfatal.

## Implementation Summary

Added `derive_log_context(case, store)`:

- reads raw `StoredRecordKind::Log` records from `HotContextStore`;
- only runs for fixtures declaring `log-pattern-clustering`;
- groups by entity, severity, and normalized template;
- normalizes selected variable values that the current corpus exercises:
  leading lock-wait counts, retry attempt numerators, and parenthesized numeric
  queue depths;
- excludes low-signal INFO logs and non-notable WARN logs;
- preserves first/last seen, count, exemplar ids, and runtime source refs;
- assigns deterministic stability labels for `new-since-incident`,
  `transient-trigger`, `overload-symptom`, and `recurring-each-cycle`;
- assigns stable ids from the general ordering rule rather than from fixture
  identity.

The new corpus test compares every fixture that declares
`log-pattern-clustering` and requires runtime provenance. It also checks that
every emitted exemplar id resolves as a raw log source ref through the hot store.

No timeline builder, related-anomaly generator, evidence-ranking logic, MCP
surface, or final full-context API was added in this round.

## Reviewer Focus

Please focus on these points:

1. Does the secondary comparison treatment for window-delta notes resolve the
   review-3 prose overfitting problem without hiding important differences?
2. Is the named pre-onset warm-up blend now acceptable for Slice 2, or should it
   still be replaced by a different baseline rule before more metric work lands?
3. Is matching log patterns by natural identity, while reporting id drift
   separately, the right response to retry-storm's non-chronological gold ids?
4. Are the log template normalization rules general enough for this first slice,
   or are they still too close to the current fixture wording?
5. Is `derive_log_context` correctly bounded to raw hot-store logs plus fixture
   capability metadata, with expected gold used only by tests?
6. If this round is acceptable, should the next round proceed to Slice 4
   timeline construction, or should the lower-severity metric ordering cleanup
   happen first?

## Verification

Commands run:

- `cargo fmt` - passed
- `cargo test derived_context` - passed, 13 tests
- `cargo test` - passed, full suite
- `cargo clippy --all-targets --all-features` - passed
- `cargo run --bin validate_fixtures` - passed, 0 errors and 0 warnings
- `git diff --check` - passed before staging
- `git diff --cached --check` - passed before the implementation commit
- `git rev-parse HEAD` and `git rev-parse '@{u}'` - both resolved to
  `276d520a53d8a901322ac2e899744f3f097ec93f` before this review document was
  created

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

