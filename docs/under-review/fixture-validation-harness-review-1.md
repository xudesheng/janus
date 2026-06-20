# Fixture Validation Harness Review 1

- Baseline SHA: `733be4f6345801327c9932b95e1f9515ec8b19ab`
- Current milestone: A committed Milestone 3 fixture validation harness that validates the registered corpus with one command, fails structural fixture defects, exposes stable selectors, and reports coverage.
- Critical path: yes - this round reconciles review feedback on the design gate that must be settled before the executable harness can be implemented.
- Milestone progress: Formal docs now specify derived-artifact validation, `data-gap` timeline markers, non-empty capability witnesses, and a hard-error conversion trigger for temporary signal/ref mismatch warnings.
- Deferred milestone work: Rust implementation, CLI, tests, and fixture cleanup are still deferred because review-0 left actionable design feedback and the User required reviewer agreement before coding.

Round 0's direction verdict agreed with the overall design and all six proposed
defaults, but required one design-doc reconciliation plus two clarifications
before coding. This round updates the formal docs only; no Rust implementation
was started.

## Response To Review 0

**F1 - derived-artifact vocabularies.** Addressed in two places:

- `docs/process/fixtures.md` now includes `data-gap` as a valid
  `timeline[*].marker` value.
- `docs/process/fixture-validation-harness.md` now adds a
  "Derived Artifact Shape And Vocabulary Validation" stage before
  source-reference closure.

That stage requires:

- `relationships[*].type` to match the relationship types in `fixtures.md`;
- `timeline[*].marker` to match the marker values in `fixtures.md`, including
  `data-gap`;
- `anomaly_windows[*].signal` to be a non-empty signal name;
- anomaly-window signals to match an input metric `name` for the same entity
  when matching input metric series exist.

I treated `anomaly_windows[*].signal` as fixture data rather than a closed
global enum because the actual corpus uses metric-like names such as
`http.server.error_rate`, `db.query.duration_p95_ms`, and
`upstream.timeout.count`. The validator should still catch mismatch against
present input metrics because otherwise a gold anomaly may not trace back to the
fixture telemetry.

**F2 - witness semantics.** Addressed. The design now states that a capability
witness must be present and non-empty. Arrays must contain at least one item,
objects must contain fields that make the artifact usable, and
`token-budget-retrieval` specifically requires `expected.evidence_bundle.budget`
with required budget fields.

**F3 - signal/ref mismatch conversion trigger.** Addressed. The design now says
the warning state is temporary, the issue report must count mismatch warnings by
category, and once the committed corpus has zero mismatch warnings, any future
mismatch becomes a hard validation error in the same implementation round or the
next review round. It also says Milestone 3 should either clean current corpus
mismatches or keep the warning path covered only by negative test fixtures.

## Review Focus

Reviewers should focus on:

1. Whether F1 is resolved by adding `data-gap` to the fixture scheme and by
   validating derived-artifact shape/vocabulary in the harness design.
2. Whether the `anomaly_windows[*].signal` rule is strict enough without turning
   metric names into a global closed enum.
3. Whether the non-empty witness rule is clear enough for implementation.
4. Whether the signal/ref mismatch warning trigger is concrete enough to prevent
   permanent compatibility warnings.
5. Whether design agreement is now sufficient to begin implementation, and if so
   whether to proceed through the four phases already listed in the design doc.

## Verification

- `git pull --ff-only`: branch was already up to date before this round.
- Corpus inspection: confirmed the current expected-artifact values include
  `marker: "data-gap"` and relationship types within the existing fixture
  vocabulary.
- `git diff --check`: passed for the formal doc updates before this review
  document was created.
- No code verification this round; this is still a design-only review
  submission and no Rust implementation was started.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
