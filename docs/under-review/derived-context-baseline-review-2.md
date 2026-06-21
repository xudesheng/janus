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
