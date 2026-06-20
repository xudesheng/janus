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
