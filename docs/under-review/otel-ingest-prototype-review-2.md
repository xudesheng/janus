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

## Review (by Claude)

### Direction Verdict

**On critical path:** Yes — this is the single closing round Review 1 asked for
before terminating the approved demo-bridge topic.

**Milestone progress: complete.** All three Review-1 findings are resolved
cleanly, and I verified each one independently rather than from the round's
summary:

- **D1 (must-fix) — fixed and pinned.** Entity-hint counting moved into the
  single `apply` path, keyed by a per-event `entity_hint_quality`, and counted
  only on `IngestOutcome::Inserted`. The over-count in `push_resource` (before
  dedup) and the per-child double-count in `entity_hint` are both gone. I re-ran
  my original Review-1 reproduction (one low-quality resource across
  `resourceSpans` + `resourceMetrics`): it now reports
  `low_quality_entity_hints = 3` (resource + span + metric-series = 3 stored
  records carrying the hint), down from the buggy `4`. The new test
  `low_quality_entity_hint_count_tracks_stored_records_not_envelopes` pins the
  multi-signal case at `4` (adds the log record), so the regression can't
  reappear. The counter now has a precise, defensible definition: accepted
  stored records that directly carry a low/missing-quality hint.
- **D2 — fixed.** `ResourceContext.entity` is now a non-optional `String`, so
  the unreachable `entity.is_none()` branch and the dead "no stable entity"
  metric reject branch are removed. Metric points are still rejected for genuinely
  missing/malformed required fields (`name`, timestamp, value), which is correct.
- **D3 — fixed.** The design doc no longer contains the contradictory "leave the
  record unresolved" / "reject the metric point" language (grep-confirmed clean);
  it now states one rule — missing service identity gets a deterministic
  synthetic resource entity (low-quality for attribute-derived keys,
  missing-quality for envelope-position keys) — and documents the counter
  semantics including the no-double-count guarantee.
- **D4 — appropriately deferred** as a non-blocking follow-up note, as agreed.
  Trace time-window derivation is not in this topic's DoD.

**Independent verification (all green):** `cargo test`, `cargo clippy
--all-targets --all-features`, `cargo fmt --check`, and `cargo run --bin
validate_fixtures` (0 errors) all pass on the baseline tree. The sample CLI run
reproduces the reported summary (2 resources, 1 trace, 2 spans, 3 metric points,
2 logs; 2 low-quality hints; 9 source refs resolved).

**Verdict: stop — topic complete.** The Definition Of Done is met: OTLP JSON is
ingested through the existing `HotIngestEvent` / `HotContextStore` boundary,
source refs are stable and resolvable, metric points accumulate into
metric-series records, entity hints are conservative and accurately counted, the
CLI prints a deterministic summary, and the scope stayed file/JSON-first with no
HTTP/persistence/derivation/ranking/MCP/dashboard work. This round leaves **no
actionable feedback and no new requirements**, so per the framework's Round
Termination rule I am **not** requesting a review-3.

### Next step

Terminate `otel-ingest-prototype` and return to the strict roadmap topic
`entity-resolver-confidence` (Milestone 5A). The boundary-reuse thesis that
justified both `fixture-otel-simulator` and this topic is now validated end to
end by a non-fixture source, so there is no remaining reason to continue down the
Milestone 9 ingest chain (HTTP receiver, persistence, `change-event-ingest`)
before derived context. Archiving the `otel-ingest-prototype-review-*` rounds is
a User decision and should be done only on explicit request.

No further review rounds for this topic.
