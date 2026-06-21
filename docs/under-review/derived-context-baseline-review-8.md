# Derived Context Baseline Review 8

- Baseline SHA: `759f7b0c04319fde7e2a8c3b8158f14a83435e19`
- Current milestone: Milestone 5B derived-context baseline, with the review-7 coverage blocker fixed
- Critical path: yes - this round closes the only remaining blocker identified before Milestone 5B completion
- Milestone progress: declared `traffic-shift-hotspot` as a log-pattern fixture, made capability-projected comparison surface present-but-undeclared gold artifacts, and added a regression test for the silent coverage gap
- Deferred milestone work: none for Milestone 5B

This round follows review 7's Direction Verdict: continue for one focused fix to
close the traffic-shift log-pattern coverage hole. No unrelated cleanup was
included.

## Response To Review 7

Finding 1: capability projection silently hid a legitimate
`traffic-shift-hotspot` gold log pattern.

Fixed in the fixture metadata and comparison surface:

- added `log-pattern-clustering` to `traffic-shift-hotspot` in
  `fixtures/registry.json`;
- added the same capability to
  `fixtures/scenarios/traffic-shift-hotspot/scenario.json`, keeping registry and
  scenario metadata consistent;
- replaced the test-only capability projection helper with
  `compare_derived_context_for_case(case, actual)`, which projects expected
  artifacts by declared capability and reports `undeclared_gold_artifacts`;
- added `DerivedContextComparison::undeclared_gold_artifacts` as a nonfatal
  comparison channel;
- updated the full-corpus derived-context comparison test to assert no current
  fixture has undeclared gold artifacts;
- added a regression test that removes `log-pattern-clustering` from a cloned
  `traffic-shift-hotspot` case and verifies the comparison reports
  `traffic-shift-hotspot:log_patterns`;
- updated `docs/core/derived-context-baseline.md` so the fixture comparison
  contract explicitly requires present-but-undeclared gold to be surfaced
  instead of silently projected away.

With the capability tag present, `derive_full_context` now derives and compares
the traffic-shift `lp-1` log pattern as part of the full-corpus path.

## Reviewer Focus

Please focus on these points:

1. Does this close review-7 Finding 1 by deriving and comparing the
   `traffic-shift-hotspot` log pattern?
2. Is `undeclared_gold_artifacts` the right nonfatal channel for future
   capability/gold mismatches?
3. If there are no new findings, is Milestone 5B complete per the Definition of
   Done?

## Verification

Commands run:

- `cargo fmt` - passed
- `cargo test derived_context -- --nocapture` - passed, 20 tests
- `cargo test` - passed, full suite
- `cargo clippy --all-targets --all-features` - passed
- `cargo run --bin validate_fixtures` - passed, 0 errors and 0 warnings
- `git diff --check` - passed before staging
- `git diff --cached --check` - passed before the implementation commit
- `git rev-parse HEAD` and `git rev-parse '@{u}'` - both resolved to
  `759f7b0c04319fde7e2a8c3b8158f14a83435e19` before this review document was
  created

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
