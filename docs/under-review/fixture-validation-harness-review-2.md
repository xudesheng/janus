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

## Review (by Claude)

### Direction Verdict

This is the first **implementation** round, so I judge milestone progress before
local defects.

**Milestone progress: the Milestone 3 artifact is achieved.** This round delivers the
executable acceptance gate the topic exists to produce, and I verified it end-to-end
rather than trusting the summary. From a clean checkout of the baseline tree:

- `cargo fmt --check` — passes.
- `cargo clippy --all-targets --all-features` — clean.
- `cargo test` — passes (fixture-validation suite + `get_evidence_bundle` suite +
  lib/doc harnesses, all green).
- `cargo run --bin validate_fixtures` — 12 fixtures, 0 proposed, 2 false-causality
  traps, **0 errors, 0 warnings**, with a deterministic (BTree-ordered) coverage report.

Every Definition-Of-Done item in `docs/process/fixture-validation-harness.md` is met:
one-command whole-corpus validation; dangling source refs fail; declared
`inputs`/`expected` checked against actual non-helper keys; capabilities and failure
classes validated against canonical vocabulary; false-causality and missing-data
structurally guarded; selection by capability/failure-class/difficulty; deterministic
coverage; fmt/test/clippy green.

The review-1 heads-up is correctly resolved: `deploy-bad-rollout` now carries a
non-empty `window_comparison` witness, the stale `scenario.expected` lists were
reconciled, and the `signal: "log"` → `signal: "log_pattern"` cleanup landed with the
warning path preserved by a synthetic negative test rather than committed corpus
warnings — exactly the F3 outcome.

Code quality is high: staged, all-issues-collected validation; typed registry/manifest
with `Value`+extractors for nested input (the agreed boundary); path-safety reused;
warnings vs errors used precisely (signal/ref compatibility = warning, structural
category violations = error). The negative tests are real — e.g. the mismatch test
asserts `error_count == 0 && warning_count == 1` with the specific message.

**Next action: stop — milestone complete.** I found no blocking defects. The items below
are non-blocking polish; none changes the verdict. Per Round Termination, no further
round is required. I'd recommend one short optional hardening pass (F1) before later
milestones lean on this gate, but that is the User's call, not a milestone blocker.

### Answers to this round's review-focus questions

1. **DoD coverage** — yes, fully covered and verified (above).
2. **Source-reference closure strictness** — appropriately strict without overfitting:
   refs resolve against actual JSON ids (not naming guesses), the `trace:` prefix and
   `trace_id/span_id` span form are handled, `external` is a hard error, and the
   signal/category mismatch stays a warning. Good.
3. **`related_anomalies.related[*].prior_incident` as a prior-incident ref** —
   acceptable. It keeps closure inside the fixture (resolves against `input.prior_incidents`
   ids) and lets the recurring-incident fixture model "this anomaly relates to a past
   incident." Slight semantic stretch (a *related anomaly* pointing at an incident rather
   than a window), but structurally sound and worth keeping documented in `fixtures.md`
   if it becomes a common shape.
4. **Selector / CLI sufficiency** — yes. Stable registry-order selection is tested,
   AND-combined filters work, and `--json` plus non-zero exit on error make it usable
   from later tests and eval. Note coverage under a selector reflects the *filtered*
   subset (read_cases filters before build_coverage); the no-arg acceptance path is
   whole-corpus, so this is fine — just be aware filtered runs report filtered coverage.
5. **Fixture cleanup correctness** — yes, the right reconciliation: add the missing
   witness / fix stale manifest key lists / migrate to the clean signal form, rather
   than weakening the checks.

### Non-blocking findings

**F1 (recommended before this gate is depended on) — negative-test coverage gaps on the
highest-value safety checks.** The corpus positive test passes, but several
design-suggested negative tests are absent, so some failure paths I verified *by reading*
are not pinned by tests:
- missing-data scenario with no missing-data channel (Stage 8) — untested;
- dangling `suspected_causes.supporting` / `counter` evidence ids — untested;
- manifest `expected` mismatch (only `inputs` is tested) — untested;
- unknown failure class (only unknown capability is tested) — untested.
The first two guard Janus's stated top-risk failure modes (false causality, missing
data); they deserve negative tests so a future refactor can't silently neuter them.
This is test hardening, not a correctness defect.

**F2 (latent, zero impact today) — two signals have no resolvable target.**
`categories_for_signal` returns `&[]` for `Profile`, and no `Relationship` ref is ever
inserted into the index. So a future `signal: "profile"` source ref would warn forever
(never matches a category) and a `signal: "relationship"` ref would be a hard error
(nothing to resolve against). No current fixture uses either, so the corpus is clean —
but a one-line code comment or a `fixtures.md` note would save a future fixture author
from a confusing failure when `profile_hotspot` evidence is first added.

**F3 (accepted, recording only) — object witnesses are lenient.** `entity_context`,
`related_anomalies`, and `window_comparison` witnesses pass on "has any non-`_` field,"
not on capability-specific required fields — the lenient reading of review-1's Q3. Fine
for a "minimal structural witness," just noting it won't catch a structurally-present
but meaningless object.

**F4 (nit) — `evidence_bundle` is deserialized twice** (once in
`validate_evidence_bundle`, again in `validate_source_references`). The already-parsed
`bundle` could be threaded through. Pure efficiency; no behavior impact.
