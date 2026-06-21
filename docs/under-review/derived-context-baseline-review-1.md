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

## Review (by Claude)

### Direction Verdict

**AGREE — design approved; implementation may begin.** From my side the design
gate is satisfied. (The topic's Design Review Gate requires *every* active
reviewer to agree, so if the User has other active reviewers they still need to
sign off before Rust starts.)

- On critical path: **yes**. This round folded review 0's must-fix design
  feedback into the formal docs, which the Design Review Gate required before any
  code.
- Moves the milestone: **yes**. The design contract is now implementation-ready;
  the only thing that was blocking Slice 1 was reviewer agreement on the
  `non-causal-change` rule, and that is now resolved.
- Next action: **continue**. Round 2 should start **Slice 1 (data model +
  comparison shell)** — that directly answers Reviewer Focus question 5.

I verified the formal-doc edits against the diff (`063c0b2..21db98a`) rather than
trusting the response prose; every claim in the response matches the actual
changes to `docs/core/derived-context-baseline.md` and `docs/core/roadmap.md`.

### Round 0 Findings — All Resolved

- **Finding 1 (must-fix): resolved, and slightly stronger than I asked.** The new
  `timeline_non_causal_after_onset_rule` carries both conditions I required
  (change strictly after earliest derived symptom/anomaly onset **and** the
  changed entity not on the derived symptom/propagation path), plus a safe
  fallback to an ordinary `change` marker when onset or path cannot be
  established — which closes the "builder guesses" gap. Both a positive
  (`coincidental-deploy-trap`) and a negative (no over-labeling) test are now
  required. This keeps the marker a deterministic temporal annotation, not a
  causal rank.
- **Finding 2: resolved.** Slice 1 is now a hard prerequisite before any
  generator slice produces artifacts.
- **Finding 3: resolved.** The scalar timeline `source_ref` is now explicitly the
  fixture-compatible *projection*; the comparison contract validates richer
  runtime provenance first, then projects to the gold shape.
- **Finding 4: resolved.** Timeline text comparison normalizes whitespace and
  treats text as secondary to marker/entity/time/source-ref.
- **Finding 5: resolved.** `roadmap.md` Near-Term Review Order now lists
  `derived-context-baseline` at position 6, renumbered cleanly.

### Answers To Reviewer Focus

1. **Yes** — the rule resolves the must-fix concern without moving final
   classification into 5B. It is purely temporal + path-membership and
   deterministic; the actual causal ranking still lives in `suspected_causes`
   (Milestone 6, out of scope here).
2. **Precise enough for the current corpus.** "Derived symptom or propagation
   path" is operationally pinned by the anomaly-window entity set the builder
   already has: in `coincidental-deploy-trap`, `service:search-ui` has no anomaly
   window, so "not on the symptom path" is derivable at Slice 4 from Slice 2
   output alone — the positive test does not depend on Slice 5. And because the
   rule falls back to plain `change` whenever onset/path is unestablished, any
   residual imprecision can only *under-*label, never invent a false
   `non-causal-change`. That is the safe failure direction.
3. **Yes, sufficient** — the negative test is the real guard. One optional
   strengthening for the implementation round (guidance, not a gate): make the
   negative test exercise *both* failure conditions, not just one — i.e. include
   a change at/before onset **and** a change whose entity *is* on the failing
   path — so a future regression that drops either clause is caught.
4. **Yes** — the provenance and text clarifications keep the fixture contract
   matchable while requiring inspectable runtime provenance. Good balance.
5. **Yes** — start Slice 1 next (see Direction Verdict).

### Round Termination Note

This round left **no new actionable design defects**; the design is approved.
The single item under Focus answer 3 is optional implementation guidance, not a
required fix. A `review-2` round is nonetheless justified — not because design
feedback is open, but because the topic's milestone work (the 5B implementation)
is not yet started. Round 2 should therefore be an **implementation round
carrying Slice 1**, after which I will judge milestone progress before local
defects per the framework.
