# Fixture Validation Harness Review 2

- Baseline SHA: `7dbad289086adbe19b8ed7fed8b911d648ca0b3c`
- Current milestone: A committed Milestone 3 fixture validation harness that validates the registered corpus with one command, fails structural fixture defects, exposes stable selectors, and reports coverage.
- Critical path: yes - this round implements the executable fixture acceptance gate required before hot-store, derivation, and eval work can safely build on the corpus.
- Milestone progress: Implemented the corpus loader, staged validator, source-reference closure checks, capability witnesses, uncertainty guards, selectors, validation CLI, fixture cleanup, and focused tests. `cargo run --bin validate_fixtures` now validates all 12 registered fixtures with 0 errors and 0 warnings.
- Deferred milestone work: none.

Review 1 cleared the design gate and allowed implementation. It also flagged one
implementation-time heads-up: `deploy-bad-rollout` claimed `compare_windows`
without a `window_comparison` witness. This round fixed that by adding a
non-empty `window_comparison` artifact and declaring it in the scenario
manifest.

## Implementation Summary

Added `src/fixture_validation.rs` with:

- typed registry and scenario-manifest models;
- `FixtureCorpus`, `FixtureCase`, `FixtureSelector`, `ValidationIssue`, and
  `CoverageReport`;
- deterministic validation stages for registry, manifest/file agreement,
  Evidence IR reuse, derived-artifact vocabulary, reference-index construction,
  source-reference closure, capability witnesses, false-causality checks,
  missing-data checks, and coverage;
- selector support by fixture id, capability, failure class, and difficulty;
- warning support for resolved signal/ref mismatches, while the committed corpus
  is now warning-clean.

Added `src/bin/validate_fixtures.rs`:

- no-argument validation path for the whole corpus;
- optional `--fixture`, `--capability`, `--failure-class`, `--difficulty`, and
  `--json` flags;
- non-zero exit when validation has errors.

Added `tests/fixture_validation.rs` with coverage for:

- current corpus validation;
- stable selector ordering;
- duplicate registry ids;
- unknown capabilities;
- manifest input mismatch;
- dangling Evidence IR source refs;
- dangling timeline refs;
- resolved signal/ref mismatch warnings;
- missing false-causality counter evidence.

Fixture cleanup performed by the harness:

- added `window_comparison` to `deploy-bad-rollout`;
- updated stale `scenario.expected` lists where `expected.json` already had
  `related_anomalies` or `window_comparison`;
- changed current derived log-pattern source refs from `signal: "log"` to
  `signal: "log_pattern"`, leaving the mismatch warning path covered by a
  synthetic negative test instead of committed corpus warnings.

## Review Focus

Reviewers should focus on:

1. Whether the staged validator actually covers the Milestone 3 Definition Of
   Done from `docs/process/fixture-validation-harness.md`.
2. Whether source-reference closure is strict enough without overfitting to the
   current JSON shapes.
3. Whether allowing `related_anomalies.related[*].prior_incident` as a
   prior-incident ref is acceptable for the recurring-incident fixture.
4. Whether selector behavior and CLI output are sufficient for later tests and
   eval runs.
5. Whether the fixture cleanup is the right reconciliation for the
   validation-revealed defects.

## Verification

- `git pull --ff-only`: already up to date before implementation.
- `cargo run --bin validate_fixtures`: passed; 12 fixtures, 0 proposed, 2
  false-causality traps, 0 errors, 0 warnings.
- `cargo fmt --check`: passed.
- `cargo test`: passed; 24 integration tests plus binary/lib harnesses.
- `cargo clippy --all-targets --all-features`: passed.
- `git diff --check`: passed.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
