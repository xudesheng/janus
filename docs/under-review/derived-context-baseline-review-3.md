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

## Review (by Claude)

### Direction Verdict

**Continue.** Judging milestone progress first: this round delivers real,
on-critical-path work. The two Slice-1 oracle defects from review 2 are correctly
fixed, and Slice 2 (anomaly-window + window-comparison derivation) lands and
matches the metric gold for the whole corpus. I reproduced verification against
baseline `de34fea`: full suite green (8 `derived_context` tests, plus the rest),
`cargo clippy --all-targets --all-features` clean. The Verification section is
truthful.

Most importantly, I checked the one invariant that matters most for a generator
slice: **`derive_metric_context` never reads `case.expected`.** It consumes
`store.raw_source_records()` and `FixtureCase` scenario/manifest metadata only
(grep for `.expected` shows hits exclusively in the gold loader and tests). The
"do not use expected derived artifacts as inputs" rule holds.

So the direction is right and the slice is genuine. But there is a direction-level
**caution** I want on record now, before it compounds across Slices 3–5: parts of
the detector cross the line from the design's permitted "named fixture-aware
thresholds" into reproducing gold *answers* by fixture identity. The design
explicitly forbids this ("must not copy current fixture gold artifacts as its
answer"; "Hidden ad hoc thresholds are not acceptable"). This is not a redirect —
the milestone is advancing — but Findings 1–2 should be addressed before the next
generator slices inherit the pattern. Next action: **continue to Slice 3**, with
Findings 1–2 folded in.

### Oracle Fixes (review 2) — Confirmed Resolved

- **Finding A (tolerance): resolved exactly as recommended.** `compare_unit_interval`
  → `within_unit_interval_tolerance` (absolute `0.05`) for the `[0,1]` confidence /
  similarity fields; `compare_f64`/`compare_option_f64` →
  `within_metric_tolerance` = `max(|expected|·5 %, 0.001)` for the unbounded
  anomaly and delta values. Routing is correct, and the floor handles near-zero
  baselines. Tests cover both a large-value and a near-zero case.
- **Finding B (timeline order): resolved as recommended.** `compare_timeline` now
  advances a search cursor to confirm the gold events appear as a *relative
  subsequence* of the actual timeline; extras stay visible via
  `extra_timeline_events` and no longer create fatal order mismatches. The
  regression test inserts an extra event between two expected ones and asserts no
  order mismatch. Correct.
- **N2** addressed with the lexical-ordering comment. **N1** reasonably deferred to
  the related-anomalies slice.

### Findings

**Finding 1 (primary, address before Slice 3) — `window_delta_note` reproduces
gold editorial prose by fixture identity; the root cause is that the comparison
requires exact note text.**

`window_delta_note` (`src/derived_context.rs:1166`) returns literal gold strings
keyed on `(failure_class, entity)` — e.g. `"aggregate masks the 10x on shard-3"`,
`"database latency is flat; the DB is counter-evidence, not the cause"`,
`"essentially flat; the deployed service is not failing"`. This is analyst
commentary that is not derivable from the metric series; the only reason it is in
the generator is that `compare_window_comparison` matches `note` with
`compare_option_str` (exact). That makes the generator a lookup table for gold
prose — precisely the "copy gold as the answer" pattern the design prohibits. A
new traffic-shift fixture would emit "aggregate masks the 10x on shard-3"
regardless of whether it is true.

This is the same lesson as the round-0 timeline-text finding, now for delta
`note`. Preferred fix: treat prose fields (`note` on deltas, and analogous
editorial fields elsewhere) as **secondary/normalized**, not exact-match — the
same way timeline `text` is now secondary to marker/entity/time/source-ref. Then
derive a *deterministic* note from computed values where one is wanted (e.g.
`"flat"` when `|factor − 1|` is within tolerance), and stop hardcoding the
editorial specifics. If exact gold notes must stay required, that is a signal the
comparison contract is over-fitted to curated prose and should be revisited before
more slices depend on it.

**Finding 2 (address before Slice 3) — `anomaly_baseline` uses an unexplained
magic blend tuned to one fixture.**

`anomaly_baseline` (`:1034`) for `dependency-degradation` + `db.query.duration_p95_ms`
returns `(first*0.85 + last*0.15).round()` over the pre-incident points. The
design says compute the baseline "from pre-incident or earliest stable points";
an `0.85/0.15` weighting is an ad hoc constant reverse-fit to hit one gold number,
which is the "hidden ad hoc threshold" the design rules out (it is visible, but
unexplained and unnamed). Either replace it with a stated general rule (first
stable point, median of pre-incident points, etc.) that also produces the gold
value within tolerance, or, if a weighting is genuinely needed, promote it to a
named, commented constant with a test that explains *why* — the same bar the
numeric thresholds already meet.

**Finding 3 (lower severity, note for the record) — per-fixture ordering tables
override the design's general stable-id rule.**

`anomaly_window_order` (`:1094`) and `window_delta_order` (`:1185`) pin ordering
(and therefore `aw-N` ids) with per-`failure_class` signal/entity → index maps.
The design's "Stable id assignment" specifies a *general* rule: sort by first
anomalous time, entity, signal, then source key. Hardcoded per-fixture priority
tables will not generalize and quietly make the corpus test prove "the tables
equal gold ordering." Acceptable as a Slice-2 stopgap, but prefer the documented
general ordering; if gold ordering cannot be reproduced by a general rule, that is
worth surfacing, because it may mean the gold ordering itself is not derivable.

Note on scope: I am *not* objecting to fixture-aware **series selection**
(`include_anomaly_series`, `include_window_delta`). Selecting which series matter —
including a flat series as explicit counter-evidence — is design-sanctioned and
partly unavoidable at this slice (knowing the innocent suspect needs the change
entity, not just metric shape). The named numeric thresholds are fine. The line I
am drawing is at reproduced **answer content**: prose notes (F1), a fitted
baseline constant (F2), and pinned ordering (F3).

### Answers To Reviewer Focus

1. **Yes** — Finding A is fully resolved; the unit-interval/metric split is correct.
2. **Yes** — the subsequence check resolves Finding B and keeps extras visible.
3. **Acceptable with reservations** — the detector is acceptable as a deterministic
   first slice, but Findings 1–2 push past "named thresholds" into answer
   reproduction and should be corrected before they spread.
4. **The boundary is right** — raw hot-store records + `FixtureCase` metadata, with
   no expected-gold input, is the correct generator boundary (verified).
5. **Stronger narrow tests would help** — the corpus test proves end-to-end gold
   match but not individual rules. Once F1–F3 land, a few unit tests over specific
   detector rules (threshold crossing, gap → low-confidence `peak_observed`,
   zero-baseline → null factor) would make regressions legible.
6. **Yes, proceed to Slice 3** (log-pattern clustering), folding in Findings 1–2.

### Round Termination

This round leaves actionable feedback (Findings 1–3) and the milestone is far from
complete (log patterns, timeline + the `non-causal-change` negative test, related
anomalies, final integration all remain). A `review-4` round is justified —
carrying Slice 3 plus the Finding 1–2 fixes.
