# OTel Ingest Prototype Review 1

- Baseline SHA: `bd61aaf880566b4265df264b8ddf078887375460`
- Current milestone: completed local OTLP JSON/file ingest prototype slices 1-3, ready for implementation review
- Critical path: yes - Review 0 explicitly approved this one demo-bridge implementation to validate the reusable hot-store ingest boundary before returning to `entity-resolver-confidence`
- Milestone progress: clarified the design contract, implemented OTLP JSON file ingest through `HotIngestEvent` / `HotContextStore`, added a CLI, sample OTLP JSON fixture, and source-ref stability tests
- Deferred milestone work: OTLP/HTTP JSON receiver, production OTLP/gRPC, persistence, change-event ingest, derived context, ranking, MCP, and dashboard work remain out of scope; after this topic closes the roadmap should return to `entity-resolver-confidence`

## Response To Review 0

Review 0 agreed to proceed with JSON/file-first implementation slices 1-3, did
not approve the optional HTTP receiver, and asked that the project return to
derived context after this topic.

Addressed findings:

- M1, fixture-shaped store errors: recorded the design decision that OTLP
  validation is adapter-level. Added `OtlpIssue` and `OtlpIngestError` carrying
  input path and OTLP envelope path; the adapter validates before emitting
  `HotIngestEvent` values and only wraps unexpected store errors.
- M2, metric entity derivation: pinned and implemented the rule that
  `service.name` maps to `service:<name>`, while missing service names fall
  back to deterministic resource keys and count as low-quality entity hints.
- M3, resource key family: documented that OTLP resource keys differ from
  fixture resource ids and verified the resolver does not depend on fixture key
  shape.
- Q1, generated log ids: added separate `explicit_log_ids` and
  `generated_log_ids` summary fields and tests for deterministic generated IDs.
- Q2, provenance placement: added `_janus.provenance` to normalized payloads,
  with metric-point provenance stored on each point so metric-series metadata
  can still merge.
- Q3, true OTLP envelope shape: added
  `fixtures/otel/deploy-bad-rollout.otlp.json` using
  `resourceSpans/scopeSpans/spans`,
  `resourceMetrics/scopeMetrics/metrics/.../dataPoints`, and
  `resourceLogs/scopeLogs/logRecords`.
- Q4, metric merges: `IngestOutcome::Updated` is treated as success, not a
  rejected record or nonzero exit cause.

## Implementation Summary

Added `src/otlp_ingest.rs` as the adapter boundary. It parses OTLP JSON-shaped
trace, metric, and log envelopes, normalizes them into existing
`HotIngestEvent` variants, generates stable source keys, and returns a
machine-readable summary. It supports file input only; no HTTP listener or
long-running service lifecycle was added.

Added `src/bin/ingest_otlp.rs` with:

- repeated `--input <path>`;
- `--json-summary`;
- `--ref <source-ref>` scalar resolution after ingest.

Added focused tests in `tests/otlp_ingest.rs` covering:

- real OTLP JSON envelope names;
- trace/span normalization and lower-case hex source refs;
- metric point accumulation into metric-series records;
- deterministic generated log ids;
- low-quality resource-key fallback;
- post-ingest source-ref resolution;
- structured malformed-id errors;
- CLI smoke behavior.

Also updated `HotContextStore::ingest(HotIngestEvent::Resource)` to extract
entity hints from normalized resource payloads instead of storing resource
events with no entities.

## Verification

- `cargo fmt` - passed
- `cargo test` - passed, including 66 tests
- `cargo clippy --all-targets --all-features` - passed
- `cargo run --bin validate_fixtures` - passed with 0 errors and 0 warnings
- `cargo run --bin ingest_otlp -- --input fixtures/otel/deploy-bad-rollout.otlp.json --json-summary` - passed; accepted 2 resources, 1 trace, 2 spans, 3 metric points, 2 logs; 0 rejected records; 9 source refs resolved

Reviewers should focus on:

1. Whether the implementation satisfies the approved JSON/file-first scope
   without smuggling in HTTP, persistence, derived context, ranking, MCP, or
   dashboard work.
2. Whether the adapter truly reuses `HotIngestEvent` and `HotContextStore`
   rather than creating a parallel ingest path.
3. Whether the metric entity derivation, resource fallback, generated log id,
   and provenance decisions are stable enough for this prototype.
4. Whether the CLI and JSON summary are adequate for the local demo and tests.
5. Whether this topic is now complete; if so, the next roadmap topic should be
   `entity-resolver-confidence`, not another ingest detour.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
