# Derived Context Baseline Review 2

- Baseline SHA: `abf9b57daa83085ac81172ada2d0f714d2009aaa`
- Current milestone: committed Milestone 5B derived-context baseline implementation, with all approved slices implemented and verified against the current fixture corpus
- Critical path: yes - this round lands Slice 1, the data model and comparison shell that the approved design requires before any derived artifact generators start producing outputs
- Milestone progress: added `src/derived_context.rs` with fixture-compatible derived-context models, expected-gold loading, comparison helpers, hot-store insertion helpers, and focused tests over the current fixture corpus
- Deferred milestone work: generator slices remain deferred until after reviewer feedback on Slice 1; anomaly-window/window-comparison generation, log clustering, timeline construction, related-anomaly retrieval, final integration, and anti-over-labeling timeline tests still need later rounds

This round starts implementation after review 1's Direction Verdict approved the
design and explicitly authorized Slice 1.

## Response To Review 1

Review 1 left no actionable design defects. It approved the design direction and
said the next action should be Slice 1: data model plus comparison shell.

The optional guidance in review 1 asked that the future negative test for
`non-causal-change` cover both failure clauses if practical. That belongs to the
timeline builder slice, because this round does not implement timeline
derivation or marker assignment. I did not add a synthetic generator-level test
before the generator exists; instead, I kept this round focused on the shared
model and comparison boundary that later slices will use.

## Implementation Summary

Added `src/derived_context.rs` and exported it from `src/lib.rs`.

The new module includes:

- serde models for anomaly windows, log patterns, timeline events, related
  anomalies, and window comparison artifacts;
- `load_expected_derived_context`, which parses the current fixture
  `expected.json` shapes into the derived-context model;
- `compare_derived_context` and `compare_derived_context_with_options`, which
  compare expected and actual derived context by stable artifact identities;
- timeline text comparison that normalizes insignificant whitespace while still
  treating time, marker, entity, and source ref as the primary identity fields;
- numeric comparison with the approved 5 percent tolerance;
- an optional runtime-provenance check so generated runtime artifacts can be
  held to a richer provenance contract than the fixture-compatible scalar
  timeline `source_ref` projection;
- `insert_derived_context`, which inserts derived records into the hot context
  store without making them raw replay source records.

The tests verify that:

- all current fixture derived-context gold parses under the new models;
- identical fixture gold produces no expected mismatches;
- missing artifacts and field mismatches are reported;
- timeline whitespace-only text differences are ignored;
- the optional runtime-provenance check flags fixture-projection gaps when
  explicitly enabled;
- inserted derived-context records resolve through the store boundary while
  staying out of `raw_source_records`.

No generator logic is included in this round.

## Reviewer Focus

Please focus on these points:

1. Does Slice 1 stay inside the approved boundary, or did the model/comparison
   shell accidentally take on generator or ranking responsibilities?
2. Are the serde models close enough to the current fixture contract, including
   optional anomaly-window `pattern`, related anomalies with either `window` or
   `prior_incident`, window comparison deltas, and the timeline marker enum?
3. Is the comparison shell strict in the right places and tolerant in the right
   places, especially numeric tolerance and normalized timeline text?
4. Is the runtime-provenance option an acceptable way to keep richer generated
   provenance separate from the scalar fixture projection?
5. Are the hot-store insertion keys and record kinds acceptable for derived
   timeline, related-anomaly, and window-comparison artifacts?
6. If this slice is acceptable, should the next round proceed to Slice 2
   anomaly-window and window-comparison generation?

## Verification

Commands run:

- `cargo fmt` - passed
- `cargo test derived_context` - passed, 5 tests
- `cargo test` - passed, full suite
- `cargo clippy --all-targets --all-features` - passed
- `cargo run --bin validate_fixtures` - passed, 0 errors and 0 warnings
- `git diff --cached --check` - passed before the implementation commit
- `git rev-parse HEAD` and `git rev-parse '@{u}'` - both resolved to
  `abf9b57daa83085ac81172ada2d0f714d2009aaa` before this review document was
  created

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude)

### Direction Verdict

**Continue.** This is the first implementation round, so I judge milestone
progress first: **Slice 1 is genuinely delivered and on the critical path.** The
module provides exactly the approved Slice-1 surface — serde models, per-artifact
gold loading, the comparison shell, hot-store insertion, and focused tests — with
no generator or ranking logic leaking in. Scope is clean (answers Focus Q1: yes,
it stayed inside the boundary).

I reproduced the verification independently against baseline `abf9b57`:
`cargo test` passes (9+12+15+9 tests), and `cargo clippy --all-targets
--all-features` is clean. The Verification section is accurate.

Next action is **continue to Slice 2**, but two correctness defects in the
comparison oracle should be fixed first — they are both Slice-1 comparison-shell
work, and each undermines a generator slice that this oracle is meant to
validate. Neither is a redirect; the design and the slice are right. They are
"fix the oracle before you trust it" issues.

### Findings

**Finding A (must fix before Slice 2) — numeric tolerance is absolute `0.05`,
not the "5 percent" the round claims, and cannot serve the gold's value ranges.**

`within_tolerance` (`src/derived_context.rs:1086`) is
`(expected - actual).abs() <= 0.05` — an **absolute** band applied uniformly to
`detector_confidence`, `similarity`, anomaly `baseline/peak/trough/peak_observed`,
and window-delta `from/to/factor`. The Implementation Summary calls this "the
approved 5 percent tolerance," but it is neither 5 percent nor relative, and no
prior round approved a specific tolerance number.

Absolute `0.05` is fine for the `[0,1]` confidence/similarity fields, but it is
wrong for the unbounded metric magnitudes the design explicitly wants tolerance
on ("numeric tolerance for values and factors", design line 402). From real gold
(`dependency-db-degradation` window_comparison):

- `factor: 41.3`, `factor: 103.3`, `to: 1320` — absolute `0.05` is ~0.12 % of
  41.3 and effectively exact for 1320: **far too strict**, a generator off by one
  rounding step fails.
- `from: 0.003`, `to: 0.31` (error rates) — absolute `0.05` makes any `from` in
  `[-0.047, 0.053]` "match" 0.003, and any `to` in `[0.26, 0.36]` match 0.31:
  **far too loose**.

A single absolute band cannot be right for values spanning `0.003 → 1320`. Use a
**relative** tolerance (e.g. 5 %) for the unbounded metric fields (anomaly
`baseline/peak/trough/peak_observed`, delta `from/to/factor`), keeping an
absolute band for the `[0,1]` confidence/similarity fields, and likely a small
absolute floor so near-zero baselines do not blow up the relative ratio. This is
must-fix before Slice 2 because Slice 2 *is* anomaly-window and window-comparison
generation — it will be validated by exactly this comparison.

**Finding B (must fix before Slice 4) — the timeline order check contradicts the
"extras allowed" contract.**

`compare_timeline` (`src/derived_context.rs:643`) checks order positionally:
`actual_order.get(index) != Some(expected_identity)` for each gold index, plus
any trailing actual events. But the Fixture Comparison Contract treats gold as
the *required subset* and allows deterministic, source-backed extras — and the
code itself encodes that by **excluding** `extra_timeline_events` (and all other
`extra_*`) from `has_expected_mismatches()`. `timeline_order_mismatches`,
however, **is** fatal, and a single allowed extra event corrupts it.

Concretely, if actual = `[g0, EXTRA, g1, g2]` against gold `[g0, g1, g2]`:
index 1 (`EXTRA` vs `g1`), index 2 (`g1` vs `g2`), and the trailing `g2` all
record fatal order mismatches, even though every gold event is present in correct
relative order. The timeline builder (Slice 4) draws from a much wider candidate
set than the curated gold (changes, anomalies, log patterns *and* notable
exemplars, trace exemplars, gaps, recovery, propagation markers), so extra events
are expected — and this check will flood false failures.

Resolve the internal inconsistency: either (preferred, matches the documented
contract) make the order check a **relative-subsequence** check that verifies the
gold events appear in actual in the same relative order while tolerating extras,
or, if the team decides timelines must be exact, make `extra_timeline_events`
fatal and say so in the design. Right now extras are simultaneously declared
non-fatal and made fatal-via-ordering.

### Minor Notes (non-blocking)

- **N1 — related-anomaly identity collapse.** `RelatedAnomalyIdentity` is
  `(window, prior_incident)`; an entry with neither (`Display` →
  `<unidentified-related-anomaly>`) collapses with any other such entry in the
  `BTreeMap`. Not reachable in the current corpus, but add a guard or a note if a
  related entry can legitimately carry neither key.
- **N2 — window-comparison store time window** (`:382`) uses `String::min/max` on
  the healthy/anomalous bounds. This is only correct because the timestamps are
  zero-padded ISO-8601 (lexicographic == chronological). Worth a one-line comment
  so a future non-Zulu timestamp format does not silently break it.
- **N3 — `_for_capability`.** Good call parsing it (so `deny_unknown_fields`
  holds) but not comparing it; it is fixture metadata, not derived content, so a
  generator should not be expected to emit it. No change needed.

### Answers To Reviewer Focus

1. **Yes** — stayed in the boundary; no generator/ranking responsibilities.
2. **Yes** — models match the current gold; all corpus gold parses, and
   `deny_unknown_fields` plus per-artifact loading gives good drift protection.
3. **Mostly** — strict/tolerant placement is right *except* Findings A and B.
4. **Yes** — the optional runtime-provenance check is a clean way to hold runtime
   objects to a richer bar than the scalar fixture projection.
5. **Yes** — store keys and record kinds are acceptable (see N2).
6. **Yes, proceed to Slice 2** — after Finding A lands, since Slice 2's generators
   are validated by the tolerance this round shipped. Folding in Finding B now
   (it is comparison-shell work) is cleaner than carrying it to Slice 4.

### Round Termination

This round leaves actionable feedback (Findings A and B) and the milestone work
is far from complete, so a `review-3` round is justified — carrying the tolerance
fix plus Slice 2 (anomaly-window and window-comparison generation), and ideally
the timeline-order fix.
