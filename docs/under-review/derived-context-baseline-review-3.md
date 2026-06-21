# Derived Context Baseline Review 3

- Baseline SHA: `de34fea2625ed1cc44906a8bca85b83a2c4d538c`
- Current milestone: committed Milestone 5B derived-context baseline implementation, with all approved slices implemented and verified against the current fixture corpus
- Critical path: yes - this round fixes the Slice 1 comparison oracle defects from review 2 and lands Slice 2 anomaly-window plus window-comparison derivation
- Milestone progress: added `derive_metric_context` for raw metric-derived anomaly windows and window comparisons, fixed numeric and timeline-order comparison behavior, and added corpus tests proving current metric gold is derived without using expected derived artifacts as inputs
- Deferred milestone work: log-pattern generation, timeline construction including the `non-causal-change` negative test, related-anomaly derivation, and final full-pipeline integration remain deferred because this round first had to make the comparison oracle trustworthy and then land the metric generator slice it validates

This round continues after review 2's Direction Verdict approved Slice 1 and
explicitly directed the next round to fix the comparison oracle and proceed to
Slice 2.

## Response To Review 2

Finding A: numeric tolerance was absolute `0.05`.

Fixed in `src/derived_context.rs`:

- unit interval fields now use `DERIVED_CONTEXT_UNIT_INTERVAL_TOLERANCE`;
- unbounded metric fields now use `DERIVED_CONTEXT_RELATIVE_NUMERIC_TOLERANCE`
  with `DERIVED_CONTEXT_ABSOLUTE_NUMERIC_TOLERANCE_FLOOR`;
- tests cover both a large metric value where 5 percent relative tolerance is
  intended and a near-zero metric value where the old `0.05` absolute band would
  have been too loose.

Finding B: timeline order was positional and made allowed extras fatal.

Fixed in `src/derived_context.rs`:

- timeline order comparison now checks that expected gold events appear as a
  relative subsequence in the actual timeline;
- deterministic, source-backed extra timeline events remain visible through
  `extra_timeline_events`, but they do not create fatal order mismatches;
- a regression test covers an extra event inserted between two expected events.

Minor note N2: window-comparison store time window used lexical min/max.

Addressed with a short code comment explaining that fixture timestamps are
zero-padded UTC RFC3339 strings, so lexical ordering matches chronological
ordering for the current corpus.

Minor note N1 was not changed. Related-anomaly identity collapse is still not
reachable in the current corpus and belongs more naturally with the future
related-anomalies slice.

## Implementation Summary

Added `derive_metric_context(case, store)`, which produces metric-derived pieces
of `DerivedContext`:

- `anomaly_windows`;
- `window_comparison` when the fixture declares `compare_windows`.

The implementation consumes raw source records through `store.raw_source_records`
and scenario metadata from `FixtureCase`. It does not read expected
`anomaly_windows` or expected `window_comparison` as generator input.

The Slice 2 detector is intentionally a deterministic fixture-profile detector,
not a production anomaly detector. The selection rules are keyed by fixture
failure class and raw metric signal shape, with named thresholds for:

- drop anomalies;
- error-rate absolute and relative movement;
- lock-wait minimums;
- request-rate absolute and relative movement;
- generic relative increases;
- sawtooth high/reset ratios.

It also handles current corpus requirements that a generic threshold alone would
miss:

- instance metric canonicalization for the payments canary/stable case;
- telemetry-gap low-confidence `peak_observed` windows;
- sawtooth memory and restart burst patterns;
- flat counter-evidence window deltas;
- zero-baseline factors represented as `null`;
- runtime source refs on generated anomaly windows and comparison deltas.

The new corpus test projects expected gold down to metric-derived fields only
and compares generated output for every current fixture. The same test also
requires runtime provenance on generated metric outputs.

No log-pattern, timeline, related-anomaly, evidence-ranking, or final API
generation logic is included in this round.

## Reviewer Focus

Please focus on these points:

1. Do the tolerance changes fully resolve Finding A, especially the split
   between unit interval fields and unbounded metric values?
2. Does the relative-subsequence timeline order check resolve Finding B while
   still making extra timeline events visible enough for review?
3. Is the fixture-profile detector acceptable for Slice 2, given that the design
   allows named fixture-aware thresholds for this first implementation?
4. Is using `FixtureCase` scenario metadata plus raw hot-store records the right
   boundary, or should Slice 2 avoid failure-class profile selection before the
   next generator slices build on it?
5. Are the new corpus tests strong enough for this stage, or should the next
   round add narrower tests around specific detector rules before moving to
   Slice 3 log patterns?
6. If this round is acceptable, should the next implementation round proceed to
   Slice 3 log-pattern clustering?

## Verification

Commands run:

- `cargo fmt` - passed
- `cargo test derived_context` - passed, 8 tests
- `cargo test` - passed, full suite
- `cargo clippy --all-targets --all-features` - passed
- `cargo run --bin validate_fixtures` - passed, 0 errors and 0 warnings
- `git diff --cached --check` - passed before the implementation commit
- `git rev-parse HEAD` and `git rev-parse '@{u}'` - both resolved to
  `de34fea2625ed1cc44906a8bca85b83a2c4d538c` before this review document was
  created

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
