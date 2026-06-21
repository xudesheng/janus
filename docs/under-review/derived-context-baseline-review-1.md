# Derived Context Baseline Review 1

- Baseline SHA: `21db98abe9c622fec81f5eceb39fe2bc133e2e52`
- Current milestone: committed Milestone 5B design contract in `docs/core/derived-context-baseline.md`, ready for reviewer approval to unblock the first implementation slice
- Critical path: yes - review 0 left actionable design feedback that had to be folded into formal docs before Rust implementation can start
- Milestone progress: resolved review 0 design feedback in `docs/core/derived-context-baseline.md` and corrected the stale near-term review order in `docs/core/roadmap.md`
- Deferred milestone work: Rust implementation remains deferred because this round is a design-feedback response and reviewers must confirm the updated design direction before coding starts

This is a design-only response to `derived-context-baseline-review-0.md`. No
Rust implementation has started.

## Response To Review 0

Review 0's Direction Verdict approved the topic direction and whole-topic
implementation model, but left actionable design feedback. This round addresses
that feedback in formal docs before any implementation work.

Finding 1: pin `non-causal-change` before coding.

Implemented in `docs/core/derived-context-baseline.md`:

- added the named `timeline_non_causal_after_onset_rule`;
- constrained the marker to changes that occur strictly after the earliest
  derived symptom or anomaly onset for the active incident;
- required the changed entity to be absent from the derived symptom or
  propagation path at that time;
- required fallback to ordinary `change` when onset or path cannot be
  established from source-backed derived context;
- added required positive and negative tests for the rule.

This keeps `non-causal-change` as a narrow temporal timeline marker, not a
suspected-cause rank or final evidence-compiler classification.

Finding 2: land the comparison shell first.

Implemented in `docs/core/derived-context-baseline.md`: Slice 1 is now called
out as the prerequisite before generator slices create anomaly, pattern,
timeline, related-anomaly, or comparison artifacts.

Finding 3: preserve richer timeline provenance than one scalar `source_ref`.

Implemented in `docs/core/derived-context-baseline.md`: the scalar timeline
`source_ref` is now explicitly the fixture-compatible projection, not the full
runtime provenance set. The fixture comparison contract now says to validate
runtime provenance first and then project into the fixture-compatible shape.

Finding 4: normalize timeline text comparison.

Implemented in `docs/core/derived-context-baseline.md`: timeline text
comparison should normalize insignificant whitespace and treat text as
secondary to marker, entity, time, and source ref.

Finding 5: reconcile the roadmap.

Implemented in `docs/core/roadmap.md`: `derived-context-baseline` now appears
between `entity-resolver-confidence` and `evidence-compiler-ranking` in the
Near-Term Review Order.

## Reviewer Focus

Please focus on these points:

1. Does the named `timeline_non_causal_after_onset_rule` resolve review 0's
   must-fix design concern without moving final nearby-change classification
   into Milestone 5B?
2. Is the phrase "derived symptom or propagation path" precise enough for the
   first implementation, given that the rule falls back to ordinary `change`
   when the path cannot be established?
3. Are the positive and negative test requirements enough to prevent
   over-labeling nearby changes as `non-causal-change`?
4. Do the provenance and timeline text comparison clarifications preserve the
   fixture contract while giving runtime objects enough auditability?
5. If the updated design is acceptable, should the next round start coding
   Slice 1, the data model and comparison shell?

## Verification

No code verification this round. This was a design-document and review-document
submission only.

Commands run:

- `git diff --check` - passed
- `git status --short --branch` - clean on `derived-context-baseline`, tracking
  `origin/derived-context-baseline`
- `git rev-parse HEAD` and `git rev-parse '@{u}'` - both resolved to
  `21db98abe9c622fec81f5eceb39fe2bc133e2e52` before this review document was
  created

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
