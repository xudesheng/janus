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

## Review (by Claude)

### Direction Verdict

On critical path: **yes** — this round settles the last open design feedback that
review-0 required before coding. Still a design-only round, so its milestone progress
is the agreed contract, not code.

Moves the milestone: **yes**. All three round-0 findings are resolved, and I verified
each fix against the actual corpus rather than taking the response at its word:

- **F1 (resolved):** `data-gap` is now in the `fixtures.md` marker enum, and a new
  Stage 4 "Derived Artifact Shape And Vocabulary Validation" validates
  `relationships[*].type`, `timeline[*].marker`, and `anomaly_windows[*].signal`. I
  ran the new rules over all 12 fixtures: zero relationship-type violations, zero empty
  anomaly signals, and zero anomaly-signal/input-metric mismatches. The corpus is
  already clean against the rule, so it adds a guard without forcing churn. Treating
  anomaly signal as fixture data cross-checked against present input metrics (rather
  than a closed enum) is the right call — metric names are scenario data.
- **F2 (resolved):** witness is now "present and non-empty," with array/object/budget
  specifics and an explicit `expected.evidence_bundle.budget` rule for
  `token-budget-retrieval`.
- **F3 (resolved):** the warning is declared temporary, mismatch warnings are counted
  by category in the issue report, and the conversion-to-hard-error trigger ("zero
  mismatch warnings in the committed corpus") is concrete and observable.

**I agree with the design. From my side the design gate is cleared: implementation may
begin, proceeding through the four phases already listed in the design doc.** I have no
remaining design-level objections. (Per the design's own gate, coding starts once every
active reviewer agrees; if the User is also reviewing, that agreement still applies.)

Next action: **continue into implementation.** This is not a stop-and-report case — the
milestone artifact (the executable harness) does not exist yet, so implementation rounds
continue. The one concrete finding below is an implementation-time heads-up, not a
design blocker.

### Answers to this round's review-focus questions

1. **F1 resolution** — yes, resolved (verified above).
2. **`anomaly_windows[*].signal` strictness** — yes, correctly strict without a global
   enum. Cross-checking only against *present* input metrics for the same entity avoids
   false positives in gap/partial-signal fixtures (e.g. `missing-data-gap` passes).
3. **Non-empty witness clarity** — yes, clear enough to implement. One edge to nail down
   during coding: define "object witnesses contain the fields that make the artifact
   usable" concretely per capability (`entity_context`, `window_comparison`,
   `related_anomalies`) so it is a deterministic check, not a judgment call.
4. **Mismatch warning trigger** — yes, concrete enough to prevent permanent warnings.
5. **Is design agreement sufficient to begin implementation** — yes; proceed through the
   four phases.

### Finding (implementation heads-up, not a design defect)

**The corpus already fails the new non-empty witness rule in one place.**
`deploy-bad-rollout` declares `compare_windows` in both `registry.json` and
`scenario.json.capabilities`, but its `expected.json` has no `window_comparison` key
(and `scenario.json.expected` omits it too). Under the F2 rule (missing/empty witness =
hard error), the harness will fail `deploy-bad-rollout` on its first run.

This is the harness doing its job — surfacing a real declared-but-unexercised capability
— so it is good news for the design's value. But it has a direct consequence for the
Definition Of Done and the first test ("current registry and all current fixtures
validate successfully"): that test cannot pass until this is reconciled. When the witness
phase lands, the implementor should either:
- add a non-empty `window_comparison` artifact to
  `deploy-bad-rollout/expected.json` (and add `window_comparison` to
  `scenario.json.expected`), or
- drop the `compare_windows` capability claim from that fixture's registry and manifest
  entries.

Fixing a validation-revealed concrete defect is explicitly in scope per the design's
Scope section, so no scope change is needed — just flagging it so the first
implementation round expects it rather than being surprised by a red corpus. (I did not
re-scan every fixture exhaustively for other capability gaps; the witness phase will
enumerate them — worth a `log()`/report line listing all such gaps in one pass rather
than failing on the first.)
