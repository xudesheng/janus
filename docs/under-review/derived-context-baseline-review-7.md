# Derived Context Baseline Review 7

- Baseline SHA: `4da0926ea6ba40e7ed41ef16f70498c627b5203d`
- Current milestone: Milestone 5B derived-context baseline, with Slice 6 final integration now implemented
- Critical path: yes - this round closes the final integration work required for the milestone
- Milestone progress: added a full derived-context pipeline, hot-store insertion for derived entity/relationship/context records, full-corpus capability-projected comparison, resolvable relationship-backed provenance, and the formal Slice 6 timeline marker decision
- Deferred milestone work: none for Milestone 5B; the earlier review-4 cleanup items remain non-critical post-milestone cleanup if reviewers still want them

This round follows review 6's Direction Verdict: continue to Slice 6, carrying
store integration, relationship-ref resolution, and the timeline generalization
decision.

## Response To Review 6

Relationship refs did not resolve as store scalar refs.

Fixed. `derive_and_insert_context(case, store)` now resolves entities and
relationships from the raw store, inserts derived entity and relationship
records first, then inserts anomaly windows, log patterns, timeline events,
related anomalies, and window comparisons. Tests now treat `relationship:` refs
as `SourceSignal::Relationship` and assert that relationship-backed
related-anomaly provenance resolves through the same store boundary as anomaly
and log-pattern refs.

Seed selection is failure-class keyed.

Recorded in `docs/core/derived-context-baseline.md` as a current milestone
boundary. `find_related_anomalies` still has no public caller-provided seed or
query surface in this topic, so seed choice remains deterministic fixture-scope
selection while relationship traversal, lag calculation, and prior-incident
matching remain source-driven derivation logic.

Timeline marker generalization should be decided in Slice 6.

Implemented and recorded. Timeline event candidate selection remains
corpus-bounded for this milestone, but supported anomaly marker roles now use
source-derived context where safe: retry-source anomaly windows become
`amplification` from resolved `retries` relationships, and dependent
latency/duration anomaly windows become `propagation` only when a downstream
anomaly on a `calls`, `reads-from`, or `depends-on` relationship starts strictly
earlier. Other anomaly windows remain `symptom` unless a named source-backed
rule applies, preserving the false-causality fixtures.

## Implementation Summary

Added `derive_full_context(case, store)` in `src/derived_context.rs`.

The full pipeline derives:

- anomaly windows and window comparison from canonical metric series;
- log patterns from raw log records;
- timeline events from raw changes/logs/spans/gaps plus derived anomaly windows;
- related anomalies from anomaly windows, resolved relationships, and prior
  incidents.

Added `derive_and_insert_context(case, store)`.

The insertion order is:

1. resolve entities and relationships from raw source records;
2. insert derived `Entity` and `Relationship` records;
3. insert derived `AnomalyWindow`, `LogPattern`, `TimelineEvent`,
   `RelatedAnomaly`, and `WindowComparison` records.

This order makes related-anomaly relationship provenance resolvable before the
related-anomaly record is inserted and keeps all derived records out of
`raw_source_records()`.

Added final integration tests:

- full-corpus derived context comparison against capability-projected fixture
  gold;
- full-pipeline insertion over raw replay stores for every fixture;
- derived record inspectability for anomaly windows, log patterns, timeline
  events, related anomalies, and window comparisons;
- scalar source-ref resolution for anomaly windows, log patterns, log exemplars,
  timeline refs, related-anomaly refs, and relationship refs.

Updated `docs/core/derived-context-baseline.md` to record:

- the scoped timeline marker generalization decision;
- the caller-provided related-anomaly seed boundary;
- derived relationship insertion as part of Store Integration.

## Reviewer Focus

Please focus on these points:

1. Does `derive_full_context` plus `derive_and_insert_context` satisfy Slice 6's
   final integration requirement?
2. Is inserting derived relationship records before related anomalies the right
   resolution for relationship-backed provenance?
3. Is the timeline marker decision acceptable: generalized for supported
   relationship/onset roles, while broader event selection remains
   milestone-scoped scaffolding?
4. Is capability-projected full-corpus comparison the right interpretation of
   the design contract, given some gold files contain artifacts whose capability
   tag is not declared?
5. If these are acceptable, is Milestone 5B complete?

## Verification

Commands run:

- `cargo fmt` - passed
- `cargo test derived_context -- --nocapture` - passed, 19 tests
- `cargo test` - passed, full suite
- `cargo clippy --all-targets --all-features` - passed
- `cargo run --bin validate_fixtures` - passed, 0 errors and 0 warnings
- `git diff --check` - passed before staging
- `git diff --cached --check` - passed before the implementation commit
- `git rev-parse HEAD` and `git rev-parse '@{u}'` - both resolved to
  `4da0926ea6ba40e7ed41ef16f70498c627b5203d` before this review document was
  created

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude)

### Direction Verdict

**Continue — milestone progress is excellent, but Milestone 5B is not quite
complete: one coverage hole must close first.** Judging progress before defects:
Slice 6 delivers the full pipeline (`derive_full_context`) and store integration
(`derive_and_insert_context`) correctly, resolves the round-6 relationship-ref
finding, and makes the timeline-generalization decision explicit and recorded —
exactly the close-out I asked for. I reproduced verification against baseline
`4da0926`: 19 `derived_context` tests pass, full suite green, clippy clean,
`validate_fixtures` 0 errors / 0 warnings. The three formal-doc updates match
their claims.

This is genuinely strong final-integration work. I am withholding a clean
"complete" for one reason only — a silent comparison gap (Finding 1) — and once
that closes, the topic is done. So: **answering Focus Q5 directly — not yet; one
focused fix remains**, then 5B is complete.

### What This Round Resolves (confirmed)

- **Relationship-ref resolution (Focus Q2): correct.** `derive_and_insert_context`
  resolves entities/relationships and inserts `Entity` + `Relationship` records
  *before* the derived context records, so `relationship:` provenance resolves via
  `SourceSignal::Relationship`, like anomaly/log-pattern refs. A test asserts it.
  This closes round-6 Finding 1, and the design's Store Integration section was
  updated to require the ordering.
- **Timeline marker decision (Focus Q3): acceptable and well-scoped.** Amplification
  is now derived from resolved `retries` edges, and propagation only when a
  downstream anomaly on a `calls`/`reads-from`/`depends-on` edge starts strictly
  earlier; everything else stays `symptom`. That is real source-derived role
  assignment (the Slice-5 approach applied to markers), it preserves the
  false-causality fixtures, and the remaining corpus-bounded *event selection* is
  now an explicit, recorded decision in the design doc rather than silent
  overfitting. This is the outcome I required in rounds 5–6.
- **Insertion keeps derived records out of `raw_source_records()`** — the
  Milestone-5A separation still holds.

### Findings

**Finding 1 (must close before declaring 5B complete) — capability projection
silently hides a legitimate gold artifact in `traffic-shift-hotspot`.**

I audited every fixture's declared capabilities against the artifacts actually
present in its `expected.json`. Eleven of twelve align. The exception:
`traffic-shift-hotspot` declares `anomaly-windows`, `build_timeline`,
`compare_windows` — but **not** `log-pattern-clustering` — yet its gold contains a
well-formed `log_patterns` entry:

```
lp-1  WARN  shard:orders-shard-3
"queue depth high on shard 3 (<n>), processing delayed"  count 2  exemplars [log-1]
```

This is not stale junk — the template even reflects the Slice-3
parenthesized-integer normalization, and the underlying WARN ("queue depth high")
is already in the log-notability allowlist. But because the capability tag is
missing, three things compound:

- `derive_log_patterns` gates on `log-pattern-clustering` and returns empty for
  this fixture;
- `capability_expected_context` projects the gold `log_patterns` out;
- so the full-corpus test compares empty-vs-empty and **passes vacuously** — a
  real gold log pattern is neither derived nor verified.

Capability projection is a defensible reading of the design ("follow capability
tags"), so this is not a code-logic bug per se — but it converts a fixture
inconsistency into invisible lost coverage, which directly undercuts DoD item
"current fixture gold artifacts are compared according to capability tags." Two
fixes, ideally both:

1. Add `log-pattern-clustering` to `traffic-shift-hotspot`'s capabilities (the
   gold pattern is legitimate), then confirm `derive_full_context` actually
   produces `lp-1` and the comparison checks it. This is the substantive fix.
2. Make the comparison **surface present-but-undeclared gold** (a report list /
   nonfatal channel, like the existing `extra_*` and `*_differences` fields) so a
   future capability-tag omission cannot silently zero out coverage again. This
   prevents recurrence.

This is the single blocker to completion; it is narrow and the fix is small.

### Definition Of Done — Status

Checked each DoD item against the implementation:

- anomaly windows / log patterns / timeline / related-anomalies / window-comparison
  derived with bounded windows, source refs, confidence, stability, exemplars,
  ordering — **met**;
- derived anomaly + log-pattern refs resolve through the hot store; timeline /
  related / window-comparison records inspectable via the store + comparison path
  — **met** (and relationship refs now resolve too);
- no evidence ranking, suspected-cause scoring, MCP, persistence, dashboard, or new
  ingest introduced — **met**;
- `cargo fmt` / `cargo test` / `cargo clippy --all-targets --all-features` /
  `cargo run --bin validate_fixtures` pass — **met** (reproduced);
- gold compared according to capability tags — **met in form, but with the
  traffic-shift coverage hole of Finding 1**.

So DoD is met on every axis except the one Finding-1 gap. That is why this is
"one fix from done," not "done."

### Answers To Reviewer Focus

1. **Yes, with the Finding-1 caveat** — the pipeline and insertion satisfy Slice 6
   integration; the only gap is the hidden traffic-shift log-pattern coverage.
2. **Yes** — inserting derived relationship records before related anomalies is the
   right resolution; provenance now resolves through the store boundary.
3. **Yes** — the timeline marker decision is acceptable and properly recorded.
4. **Defensible, but it creates the silent hole** — see Finding 1. Keep
   capability-driven comparison, but surface present-but-undeclared gold and fix
   the traffic-shift capability tag.
5. **Not yet — close Finding 1 first.** Everything else meets the Definition Of
   Done. With the traffic-shift gold log pattern derived and compared (and the
   present-but-undeclared gold surfaced), Milestone 5B is complete.

### Round Termination

This round leaves actionable feedback (Finding 1), so per the framework a
`review-8` round is justified to carry the fix. It should be small: declare the
`traffic-shift-hotspot` log-pattern capability (or otherwise resolve the gold),
make present-but-undeclared gold visible in the comparison, and re-run the corpus
suite. If round 8 closes Finding 1 with no new findings, **Milestone 5B is
complete and no further round should be opened** — report completion to the User
rather than emitting an empty round.
