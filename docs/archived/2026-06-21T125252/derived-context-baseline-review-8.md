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

## Review (by Claude)

### Direction Verdict

**Complete. Milestone 5B is done — this is the terminating round; no `review-9`
should be opened.** Review 7 left exactly one blocker, and this round closes it
correctly, minimally, and with a guard against recurrence. There are no new
findings. Per the framework's Round Termination rule (no open defects/requirements
AND milestone work complete), the loop stops here and completion is reported to the
User.

I reproduced verification against baseline `759f7b0`: 20 `derived_context` tests
pass (including the targeted full-corpus and regression tests below), full suite
green, `cargo clippy --all-targets --all-features` clean, `validate_fixtures` 0
errors / 0 warnings across all 12 fixtures.

### Finding 1 (review 7) — Fully Resolved

Both parts of the fix I asked for are present, and I verified each:

- **Substantive fix:** `traffic-shift-hotspot` now declares `log-pattern-clustering`
  in *both* `registry.json` and `scenario.json` (consistent), so
  `derive_full_context` now derives the `lp-1` queue-depth pattern and the
  capability-projected comparison checks it. The full-corpus test
  (`derive_full_context_matches_current_gold_by_capability`) passes with the
  artifact included, confirming the pattern is genuinely derived and matches gold
  — not just declared.
- **Recurrence guard:** `compare_derived_context_for_case` is now a library
  function that records `{fixture}:{artifact}` in the nonfatal
  `undeclared_gold_artifacts` channel whenever gold is present without its
  capability tag, and the full-corpus test asserts that list is empty for every
  fixture. So a future capability-tag omission fails the suite loudly instead of
  silently zeroing out coverage. The regression test
  (`capability_comparison_surfaces_present_but_undeclared_gold`) removes the tag
  from a cloned case and confirms the channel reports
  `traffic-shift-hotspot:log_patterns`.

I independently re-audited all 12 fixtures (declared capabilities vs. artifacts
present in `expected.json`): **every fixture is now consistent — zero
present-but-undeclared gold remains.** The design doc's Fixture Comparison Contract
was also updated to require surfacing undeclared gold. The change is scoped to the
fix with no unrelated cleanup, as requested.

### Definition Of Done — Final Walk (all met)

- metric anomaly windows derived with bounded windows, source refs, detector
  confidence — **met**;
- log patterns preserve templates, counts, first/last seen, stability, exemplars —
  **met** (and traffic-shift `lp-1` is now in coverage);
- timeline deterministic, source-backed, ordered, with the source-derived
  `non-causal-change` / amplification / propagation rules and recorded scope
  boundary — **met**;
- `related_anomalies` and `window_comparison` have concrete derived output and
  comparison tests — **met**;
- derived anomaly, log-pattern, and relationship refs resolve through the hot-store
  reference boundary; timeline / related / window-comparison records inspectable
  via the store + comparison path — **met**;
- no evidence ranking, suspected-cause scoring, MCP, persistence, dashboard, or new
  ingest introduced — **met**;
- gold compared according to capability tags, with undeclared gold surfaced —
  **met**;
- `cargo fmt` / `cargo test` / `cargo clippy --all-targets --all-features` /
  `cargo run --bin validate_fixtures` pass — **met** (reproduced).

### Answers To Reviewer Focus

1. **Yes** — Finding 1 is closed; `traffic-shift-hotspot`'s log pattern is derived
   and compared.
2. **Yes** — `undeclared_gold_artifacts` is the right nonfatal channel, and gating
   the full-corpus test on it empty is what makes the fix durable.
3. **Yes — Milestone 5B is complete** per the Definition Of Done.

### Tracked, Non-Blocking Follow-Ups (out of 5B scope, recorded for later)

These were each consciously scoped out during the topic and are *not* defects in
5B; listing them so they are not lost:

- log-clustering heuristics (notability allowlist, normalization, stability) are
  corpus-coupled and should generalize as the corpus grows (review 4);
- `new-since-incident` catch-all stability label (review 4);
- broad timeline *event selection* remains corpus-bounded scaffolding pending the
  agent/evidence-API milestone, where caller-provided questions/seeds exist
  (reviews 5–7, recorded in the design doc);
- `find_related_anomalies` seed selection becomes caller-driven at the agent
  surface milestone (reviews 6–7, recorded in the design doc).

### Termination

No actionable feedback remains and the milestone work is complete, so I am **not**
requesting a `review-9`. Recommend reporting Milestone 5B complete to the User.
Archiving the `derived-context-baseline-review-*` documents should happen only on
explicit User request, per the review framework.
