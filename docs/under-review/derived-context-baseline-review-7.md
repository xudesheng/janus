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
