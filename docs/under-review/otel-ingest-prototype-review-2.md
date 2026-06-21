# OTel Ingest Prototype Review 2

- Baseline SHA: `34ab1c3121788b6959d133488695000460c91bea`
- Current milestone: close the local OTLP JSON/file ingest prototype after addressing Review 1 findings
- Critical path: yes - Review 1 requested one closing round before terminating this approved demo-bridge topic and returning to `entity-resolver-confidence`
- Milestone progress: fixed entity-hint summary counters, reconciled fallback entity documentation with implementation, and added regression coverage for multi-signal low-quality resources
- Deferred milestone work: D4 trace-record time-window derivation is noted as a later derived-context or hot-store polish item; OTLP/HTTP, production ingest, persistence, change-event ingest, derived context, ranking, MCP, and dashboard work remain out of scope

## Response To Review 1

Review 1 found the implementation substantially complete but requested one
closing round for D1-D3.

Addressed findings:

- D1, entity-hint counter over-count: summary counters now count accepted stored
  records that directly carry low-quality or missing-quality entity hints.
  Resource de-duplication no longer increments counters before the resource is
  accepted, child records no longer double-count through a side path, and
  metric-series updates do not increment the counters again.
- D2, unreachable metric reject branch: removed the dead "no stable entity"
  branch. The adapter always derives a deterministic resource-key fallback for
  resources without `service.name`; metric points are still rejected for missing
  or malformed required metric fields.
- D3, unresolved-vs-synthetic fallback doc conflict: updated the design doc to
  state that missing service identity gets a deterministic synthetic resource
  entity, with low-quality or missing-quality summary accounting. Empty-attribute
  resources use a position-scoped resource key based on the OTLP envelope path.

D4, trace record time window, was left as a non-blocking follow-up note as
requested. This topic's Definition Of Done is still source-ref resolution and
file-mode ingest, which remain covered.

## Implementation Summary

Changed `src/otlp_ingest.rs` so normalized events carry their entity-hint
quality into the single event-application path. Counters are updated only when
`HotContextStore::ingest` returns `Inserted`; `Updated` metric-series outcomes
remain successful but do not double-count entity hints.

Updated `docs/core/otel-ingest-prototype.md` to make the fallback rule
unambiguous:

- `service.name` still maps to `service:<name>`;
- resource attributes without `service.name` produce a low-quality
  `resource:attrs:<hash>` fallback;
- attribute-less resources produce a missing-quality, position-scoped
  `resource:attrs:<hash>` fallback;
- summary counters count accepted stored records carrying those hints.

Added a regression test where one low-quality resource appears in
`resourceSpans`, `resourceMetrics`, and `resourceLogs`. The test pins the count
at four accepted stored records carrying the hint: resource, span, metric
series, and log.

## Verification

- `cargo fmt --check` - passed
- `cargo test` - passed, including 67 tests
- `cargo clippy --all-targets --all-features` - passed
- `cargo run --bin validate_fixtures` - passed with 0 errors and 0 warnings
- `cargo run --bin ingest_otlp -- --input fixtures/otel/deploy-bad-rollout.otlp.json --json-summary` - passed; accepted 2 resources, 1 trace, 2 spans, 3 metric points, 2 logs; 0 rejected records; 2 low-quality entity hints; 9 source refs resolved

Reviewers should focus on:

1. Whether D1-D3 are fully resolved.
2. Whether the entity-hint counter definition is now precise and test-covered.
3. Whether D4 can remain deferred outside this topic.
4. Whether the `otel-ingest-prototype` topic should now terminate so the next
   roadmap work returns to `entity-resolver-confidence`.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
