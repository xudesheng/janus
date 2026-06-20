# Fixture Validation Harness Review 0

- Baseline SHA: `437b47873994142eada9e84cbbe201ab3df2b8fe`
- Current milestone: A committed Milestone 3 fixture validation harness that validates the registered corpus with one command, fails structural fixture defects, exposes stable selectors, and reports coverage.
- Critical path: yes - the harness is the executable acceptance gate for fixture-backed Janus work before hot-store and derivation implementation.
- Milestone progress: Design-only round submitted; `docs/process/fixture-validation-harness.md` now includes an explicit design review gate, implementation phases, and sharper review-focus questions.
- Deferred milestone work: Rust implementation, CLI, tests, and fixture defect fixes are deferred because the User explicitly required reviewer agreement on the design before coding.

This is the first review round for the `fixture-validation-harness` topic, so
there are no prior review findings to answer.

The covered design is `docs/process/fixture-validation-harness.md`. It keeps the
Milestone 3 boundary from `docs/core/roadmap.md`: make the incident corpus
executable acceptance data before building storage, derivation, or real
retrieval.

The proposed harness contract is:

- load `fixtures/registry.json` and each registered fixture's `scenario.json`,
  `input.json`, and `expected.json`;
- validate registry vocabulary, paths, manifest/file agreement, declared
  capabilities, failure classes, difficulty, and id consistency;
- reuse the existing `EvidenceBundle` Rust validation for
  `expected.evidence_bundle`;
- build a same-fixture reference index from input artifacts and expected derived
  artifacts;
- fail dangling source references and reject external refs unless a future
  fixture model gives them an in-fixture target;
- report initial `signal`/ref mismatches as warnings when the ref resolves,
  especially for current `signal: "log"` references to derived log patterns;
- enforce structural witnesses for declared capabilities;
- enforce structural false-causality and missing-data guards without claiming to
  calculate true causality;
- print deterministic coverage by failure class, capability, difficulty, and
  false-causality-trap status;
- expose selectors by capability, failure class, and difficulty for later tests
  and eval runs;
- provide a no-argument validation command, currently proposed as
  `cargo run --bin validate_fixtures`.

Reviewers should focus on these decisions:

1. Direction: should this whole Milestone 3 design be approved before coding, or
   should implementation be approved phase by phase?
2. Reference closure: are the supported reference forms complete enough, and is
   failing `external` refs the right default?
3. Compatibility: should `signal`/ref mismatches start as warnings, or should
   the design require immediate fixture cleanup and hard errors?
4. Capability declarations: should missing structural witnesses be hard errors
   for every declared capability?
5. False causality and missing data: are the proposed checks structural enough
   to be implementable now, while still preventing confident unsupported
   conclusions later?
6. Implementation shape: is it acceptable to keep nested OTel-shaped input as
   `serde_json::Value` with targeted extractors for this milestone?

## Verification

- `git diff --check`: passed for the formal design-doc update before this review
  document was created.
- No code verification this round; this is a design-only review submission and
  no Rust implementation was started.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
