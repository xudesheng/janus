# Derived Context Baseline Review 0

- Baseline SHA: `063c0b2a351f1718c5f80e34d53ec81d44c351e3`
- Current milestone: review-approved Milestone 5B derived-context design in `docs/core/derived-context-baseline.md`
- Critical path: yes - the Milestone 5B implementation is blocked until reviewers agree on this design direction
- Milestone progress: tightened the design gate and provenance contract, then submitted the design for reviewer direction before any Rust implementation
- Deferred milestone work: Rust implementation of anomaly windows, log patterns, timelines, related anomalies, window comparisons, and fixture comparison remains deferred because implementation must not start until all active reviewers approve the design direction

This is a design-only first review round for the `derived-context-baseline`
topic. No Rust implementation has started.

The design draft is `docs/core/derived-context-baseline.md`. This round also
added two narrow clarifications to that formal design document before review:

- the first review round must decide whole-topic approval versus phase-by-phase
  approval, `non-causal-change` timeline scope, provenance strictness, and
  timeline payload shape;
- every derived runtime object needs inspectable provenance even when the
  current fixture gold JSON shape does not expose a `source_refs` field.

Reviewers should focus on these decisions:

1. Is `derived-context-baseline` the right next topic after completed simulator,
   OTel JSON ingest, and Milestone 5A entity/relationship context work?
2. Should Milestone 5B be approved as one implementation topic, or should
   approval be phase-by-phase? If phase-by-phase, the `Direction Verdict` must
   name the approved phase.
3. Are anomaly windows, log patterns, timelines, related anomalies, and window
   comparisons the right boundary for this topic, separate from evidence
   ranking and suspected-cause scoring?
4. Is `non-causal-change` acceptable as a Milestone 5B timeline marker under the
   proposed narrow rules, or should nearby-change classification wait entirely
   for the evidence compiler?
5. Is the provenance contract strong enough for future Evidence IR generation,
   especially for derived objects whose fixture gold shape lacks explicit
   `source_refs`?
6. Is the fixture comparison contract strict enough to prevent generated
   artifacts from drifting while still allowing deterministic, source-backed
   extras?
7. Is stable natural-language timeline text acceptable for the current corpus,
   or should implementation wait for a structured timeline payload?
8. Should the roadmap's near-term review order be corrected to include this
   Milestone 5B topic before `evidence-compiler-ranking`, or is the current
   branch-specific topic selection sufficient?

## Verification

No code verification this round. This was a design-document and review-document
submission only.

Checked repository state:

- branch: `derived-context-baseline`
- upstream: `origin/derived-context-baseline`
- baseline commit is pushed: local `HEAD` and upstream both resolve to
  `063c0b2a351f1718c5f80e34d53ec81d44c351e3`

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
