# Derived Context Baseline Review 4

- Baseline SHA: `276d520a53d8a901322ac2e899744f3f097ec93f`
- Current milestone: Milestone 5B derived-context baseline, with review-3 cleanup and Slice 3 log-pattern derivation now implemented
- Critical path: yes - this round removes two review-3 blockers before extending the next generator slice, then adds source-backed log-pattern derivation for fixtures that declare `log-pattern-clustering`
- Milestone progress: replaced fixture-specific delta-note prose with secondary note comparison and deterministic flat notes, named and tested the dependency baseline warm-up blend, and added `derive_log_context` with corpus coverage over current log-pattern gold
- Deferred milestone work: Slice 4 timeline construction, Slice 5 related-anomaly derivation, final full-pipeline/store integration, and the lower-severity metric ordering cleanup remain; this round focused on the review-3 blockers plus the next approved generator slice

This round follows review 3's Direction Verdict: continue to Slice 3 after
folding in Findings 1-2.

## Response To Review 3

Finding 1: `window_delta_note` reproduced gold editorial prose by fixture
identity.

Fixed in `src/derived_context.rs`:

- removed the fixture-class/entity lookup table that emitted exact gold prose;
- derived only a compact deterministic `flat` note when the computed delta is
  flat by metric tolerance or by a small factor tolerance;
- moved window-delta `note` comparison into
  `window_delta_note_differences`, which is visible but not part of
  `has_expected_mismatches`;
- added the same secondary-note channel for related-anomaly notes before that
  generator slice depends on exact prose;
- added a regression test proving an editorial expected note and a derived
  `flat` note do not create a fatal mismatch.

Finding 2: `anomaly_baseline` used an unexplained `0.85/0.15` blend.

Addressed in `src/derived_context.rs`:

- promoted the latest-point weight to
  `PRE_ONSET_WARMUP_BLEND_LATEST_WEIGHT`;
- added a code comment explaining the rule: one pre-onset warm-up point should
  be acknowledged without replacing the earliest stable baseline;
- added a focused test for the dependency fixture baseline.

Finding 3: per-fixture anomaly/window ordering tables remain lower-severity
debt.

Not fully changed in this round. The existing metric ordering tables were not
expanded. For the new log-pattern slice, the generator uses the design's general
stable ordering by first seen, entity, severity, template, and input order. That
exposes one existing fixture-gold wrinkle: the retry-storm expected log pattern
ids are not chronological by first seen. To avoid adding a new per-fixture id
table, log-pattern comparison now matches by natural identity
`entity|severity|template`, reports id drift through
`log_pattern_id_differences`, and keeps the mismatch nonfatal.

## Implementation Summary

Added `derive_log_context(case, store)`:

- reads raw `StoredRecordKind::Log` records from `HotContextStore`;
- only runs for fixtures declaring `log-pattern-clustering`;
- groups by entity, severity, and normalized template;
- normalizes selected variable values that the current corpus exercises:
  leading lock-wait counts, retry attempt numerators, and parenthesized numeric
  queue depths;
- excludes low-signal INFO logs and non-notable WARN logs;
- preserves first/last seen, count, exemplar ids, and runtime source refs;
- assigns deterministic stability labels for `new-since-incident`,
  `transient-trigger`, `overload-symptom`, and `recurring-each-cycle`;
- assigns stable ids from the general ordering rule rather than from fixture
  identity.

The new corpus test compares every fixture that declares
`log-pattern-clustering` and requires runtime provenance. It also checks that
every emitted exemplar id resolves as a raw log source ref through the hot store.

No timeline builder, related-anomaly generator, evidence-ranking logic, MCP
surface, or final full-context API was added in this round.

## Reviewer Focus

Please focus on these points:

1. Does the secondary comparison treatment for window-delta notes resolve the
   review-3 prose overfitting problem without hiding important differences?
2. Is the named pre-onset warm-up blend now acceptable for Slice 2, or should it
   still be replaced by a different baseline rule before more metric work lands?
3. Is matching log patterns by natural identity, while reporting id drift
   separately, the right response to retry-storm's non-chronological gold ids?
4. Are the log template normalization rules general enough for this first slice,
   or are they still too close to the current fixture wording?
5. Is `derive_log_context` correctly bounded to raw hot-store logs plus fixture
   capability metadata, with expected gold used only by tests?
6. If this round is acceptable, should the next round proceed to Slice 4
   timeline construction, or should the lower-severity metric ordering cleanup
   happen first?

## Verification

Commands run:

- `cargo fmt` - passed
- `cargo test derived_context` - passed, 13 tests
- `cargo test` - passed, full suite
- `cargo clippy --all-targets --all-features` - passed
- `cargo run --bin validate_fixtures` - passed, 0 errors and 0 warnings
- `git diff --check` - passed before staging
- `git diff --cached --check` - passed before the implementation commit
- `git rev-parse HEAD` and `git rev-parse '@{u}'` - both resolved to
  `276d520a53d8a901322ac2e899744f3f097ec93f` before this review document was
  created

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude)

### Direction Verdict

**Continue.** Milestone progress first: this round resolves both review-3
blockers correctly and lands Slice 3 (log-pattern derivation) matching the
log-pattern gold across the corpus. I reproduced verification against baseline
`276d520`: 13 `derived_context` tests pass, full suite green, `cargo clippy
--all-targets --all-features` clean. The Verification section is truthful.

I re-checked the generator-integrity invariant for the new slice:
`derive_log_context` reads only `store` (raw `Log` records) plus the
`log-pattern-clustering` capability tag — no `case.expected` access in the
generator region. The "expected gold only in tests" boundary holds (answers Focus
Q5: yes).

Two things I want to credit explicitly, because they are exactly the integrity
behavior this topic is supposed to produce: (1) you applied the secondary-note
channel to *related-anomaly* notes pre-emptively, before that slice can lean on
exact prose; and (2) you surfaced the retry-storm gold id inconsistency honestly
(`log_pattern_id_differences`, nonfatal) instead of hiding it behind a per-fixture
id table. Both are the right call.

Next action: **proceed to Slice 4 (timeline)**. That is the higher-value step and
the one that finally exercises the `non-causal-change` rule and its required
negative test — the original must-fix from rounds 0–1. The lower-severity metric
ordering cleanup (old Finding 3) stays low priority and can ride along later.

### Review-3 Findings — Confirmed Resolved

- **Finding 1 (note overfitting): resolved as recommended.** The
  `(failure_class, entity)` prose lookup is gone; `window_delta_note` now derives
  a compact `"flat"` note purely from the computed `from/to/factor`, and `note`
  comparison moved to `window_delta_note_differences`, which is **excluded** from
  `has_expected_mismatches` (verified). The regression test confirms an editorial
  expected note vs a derived `flat` note is non-fatal. Good.
- **Finding 2 (magic baseline): resolved via the sanctioned path.** The weight is
  now `PRE_ONSET_WARMUP_BLEND_LATEST_WEIGHT`, commented with a real rationale
  (keep the earliest stable point dominant, acknowledge one pre-onset warm-up
  point), and covered by a focused test. It is still corpus-tuned and
  class-gated, but that is now a *named, explained, tested* threshold — within the
  design's allowance. Acceptable.

### Findings

**Finding 1 (medium-low, generalize as the corpus grows) — log clustering's three
rule families are coupled to current fixture wording.**

`include_log_record_for_pattern` (`:744`) decides "notable WARN" with a hardcoded
substring allowlist (`"waiting on lock"`, `"oomkilled"`, `"returning 503"`,
`"retrying attempt"`, …); `normalize_log_template` (`:773`) applies three bespoke
normalizers (leading lock-wait count, retry numerator, parenthesized integer);
`log_pattern_stability` (`:837`) keys labels on template phrases (`"oomkilled"`,
`"transient"`, `"queue full"`). All three are tied to the exact phrasing the
current corpus uses.

This is a real but *lesser* version of the round-3 concern, and I want to be fair
about the difference: these are **heuristic rules over log content, not gold
answer text reproduced verbatim** — a meaningful improvement. The
"preserve-semantic-numbers" instinct (only normalizing known-variable positions,
not blindly collapsing status codes/versions) is correct, and the design permits
a corpus-focused first slice. So this is acceptable to ship. The asks for later:

- the design's general normalization intent (integers, durations, quoted request
  ids, long ids) is not yet met — only three syntactic positions are normalized,
  so two logs differing by an integer elsewhere will split into separate
  templates. Move toward general variable-value normalization as new fixtures
  arrive, or note in code that these are corpus-scoped stopgaps.
- the WARN allowlist fails *closed*: an unrecognized WARN phrase is silently
  dropped. For a small curated corpus that is fine, but a silently-dropped log is
  exactly the kind of missing evidence Janus is supposed to surface, so consider
  making "dropped WARN" visible (count/log) rather than invisible.

**Finding 2 (minor / spec) — retry-storm gold log-pattern ids are
non-chronological, contradicting the design's stated id rule.**

The design's "Stable id assignment" says sort by first seen, entity, severity,
template; the retry-storm gold ids do not follow that. Your nonfatal
natural-identity matching is a reasonable interim response *and* is consistent
with the design — the design's log-pattern comparison field list deliberately does
not include `id`. But the underlying gold/spec mismatch should be resolved rather
than left as standing drift: either correct the retry-storm gold ids to be
chronological, or add a one-line note to the design that log-pattern ids are not
guaranteed chronological. Flagging for resolution; the workaround is fine for now.

**Finding 3 (minor) — `new-since-incident` is the catch-all stability label.**

`log_pattern_stability` returns `new-since-incident` for anything not matching the
three keyword branches. A pattern that genuinely predates the incident would still
be labelled "new-since-incident." Harmless for the current corpus (included
WARN/ERROR are incident-related), but worth a guard or comment before warm-memory
work, where pre-existing patterns become real.

### Answers To Reviewer Focus

1. **Yes** — secondary note treatment resolves the overfitting without hiding
   differences (they remain visible in `window_delta_note_differences`).
2. **Yes, acceptable** — the named, commented, tested warm-up blend is within the
   allowance; no need to replace it now.
3. **Yes** — natural-identity matching with separate id-drift reporting is the
   right response to the non-chronological gold ids (see Finding 2 for the
   follow-up).
4. **Acceptable for this slice, but corpus-coupled** — see Finding 1.
5. **Yes** — `derive_log_context` is correctly bounded to raw logs + capability
   metadata; expected gold is used only by tests (verified).
6. **Proceed to Slice 4 (timeline).** The metric ordering cleanup is genuinely
   low priority; Slice 4 is where the `non-causal-change` rule and its negative
   test finally land, so it should come first.

### Round Termination

This round leaves actionable feedback (Findings 1–3) and the milestone is
incomplete (Slice 4 timeline, Slice 5 related anomalies, final integration all
remain), so a `review-5` round is justified — carrying Slice 4, including the
long-pending `non-causal-change` positive/negative tests.
