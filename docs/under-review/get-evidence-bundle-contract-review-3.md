# Get Evidence Bundle Contract Review 3

- Baseline SHA: `47f05f94878493863c55421c006d1cb8a74f5a66`
- Current milestone: complete Milestone 2 `get_evidence_bundle` walking
  skeleton, including the missing counter-evidence rejection-path test.
- Critical path: yes - round 2 found exactly one remaining test gap before topic
  completion.
- Milestone progress: this round adds the missing negative test for unsatisfied
  counter-evidence requirements.
- Deferred milestone work: none for Milestone 2.

Round 2 found one actionable item: the new
`GetEvidenceBundleError::UnsatisfiedRequirement` path for counter-evidence was
not covered by a negative test. No design changes were requested.

## Response To Round 2

L1 is addressed in `tests/get_evidence_bundle.rs` with
`rejects_unsatisfied_counter_evidence_requirement`.

The test uses the `coincidental-deploy-trap` fixture, which has two
counter-evidence items, then requests three by setting:

- `require_counter_evidence = true`;
- `budget.min_counter_evidence_items = Some(3)`.

It asserts that `get_evidence_bundle` returns
`GetEvidenceBundleError::UnsatisfiedRequirement` with
`requirement: "counter_evidence"`.

No production logic changed in this round.

## Reviewer Focus

Please verify that L1 is resolved and that the topic should now stop.

1. Does the new negative test cover the counter-evidence rejection path that was
   missing in round 2?
2. Does the Milestone 2 implementation now satisfy the Definition of Done with no
   remaining defects or new requirements?

If yes, the expected next action is stop and report completion, not submit
another review round.

## Verification

Passed:

- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `cargo clippy --all-targets --all-features`
- `git diff --check`
- `cargo build`

`cargo test` now reports 15 integration tests passing: 7 in
`tests/evidence_ir.rs` and 8 in `tests/get_evidence_bundle.rs`.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude)

### Direction Verdict

On the critical path: yes. This round does exactly what round 2 asked and nothing
more — it adds the one missing negative test and touches no production logic
(`git diff 1e2eb0f..HEAD` is a single 16-line addition to
`tests/get_evidence_bundle.rs`). **L1 is resolved and Milestone 2 is complete.
Next action: stop and report topic completion.** No further round.

I verified independently rather than trusting the summary:

- `cargo test`: 15 integration tests pass (7 in `tests/evidence_ir.rs`, 8 in
  `tests/get_evidence_bundle.rs`);
- `cargo clippy --all-targets --all-features`: exit 0, zero warnings;
- `cargo fmt --check`: clean;
- `git status`: clean tree.

The new test, `rejects_unsatisfied_counter_evidence_requirement`, is meaningful,
not a placeholder. It uses `coincidental-deploy-trap` (two counter-evidence items:
ev-3 `weakens`, ev-4 `contradicts`), sets `require_counter_evidence = true` with
`min_counter_evidence_items = Some(3)`, and asserts
`UnsatisfiedRequirement { requirement: "counter_evidence", .. }`. I checked it
genuinely reaches the requirement check: the query budget (5 items / 380 tokens)
fits the bundle so `ensure_budget_fits` passes first, and `Some(3)` is non-zero so
`validate_budget` does not short-circuit it at query validation. It closes the
exact mutation hole I flagged — gutting `ensure_required_counter_evidence` to
always return `Ok` would now fail this test. The false-causality-relevant
enforcement path (roadmap Sequencing Principle 4) is now covered in both
directions.

### Topic completion

Definition of Done — all 9 items satisfied and verified across rounds 2–3. The
counter-evidence rejection path was the only open item from round 2; it is closed.
No defects, no open questions, no new requirements remain.

Per the framework's Round Termination rule, round 3's review leaves no actionable
feedback and the milestone work is complete, so **no `review-4.md` should be
submitted.** I am marking `get-evidence-bundle-contract` complete. The natural
next topic per `roadmap.md`'s near-term order is `fixture-validation-harness`
(Milestone 3), but that is a new topic, not a continuation of this loop.
