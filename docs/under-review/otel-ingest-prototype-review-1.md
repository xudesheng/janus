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

## Review (by Claude)

### Direction Verdict

**On critical path:** Yes — this is the one demo-bridge implementation Review 0
approved to validate the reusable hot-store ingest boundary before returning to
`entity-resolver-confidence`.

**Milestone progress: strong, essentially complete.** The central bet of the
whole topic — "a non-fixture source can feed `HotIngestEvent` /
`HotContextStore::ingest` and produce resolvable source refs" — is now validated
in code, not just asserted. I independently confirmed:

- All four gates pass: `cargo fmt --check`, `cargo test` (all binaries green),
  `cargo clippy --all-targets --all-features` (clean), `cargo run --bin
  validate_fixtures` (0 errors). The verification table is accurate.
- The adapter genuinely reuses the existing boundary (it emits
  `HotIngestEvent::{Resource,Trace,Span,MetricPoint,Log}` and calls
  `store.ingest`); it does **not** fork a parallel store or write path.
- Metric accumulation works: the sample's 3 points collapse into 2
  metric-series records with `inserted=9, updated=1`, per-point
  `_janus.provenance` preserved, and `Updated` correctly treated as success
  (Q4).
- Scope discipline held: file/stdin-shaped input only, no HTTP listener, no
  persistence, no derivation/ranking/MCP. The CLI returns nonzero on errors.
- Review-0 findings M1–M3 and Q1–Q4 are all addressed, and the stable
  conclusions were correctly written back into the formal design doc.

**Verdict: continue.** I do not declare the topic complete yet, because this
round leaves actionable feedback below — one real correctness defect in the
summary contract (D1) plus tidy-ups. These are small. A single **closing round 2**
that fixes D1 and resolves D2/D3 should converge the topic; after that, with the
Definition Of Done met and no open findings, the topic terminates and the
project returns to `entity-resolver-confidence` (not a third ingest detour).

### Local defects

**D1 — entity-hint quality counters over-count; they are not a clean count of
anything (medium; summary-contract bug).** `low_quality_entity_hints` /
`missing_entity_hints` are incremented in `push_resource` (`src/otlp_ingest.rs`
~808–812) **before** the signature dedup check, *and again* per child record in
`entity_hint` (~840–844). So a resource is counted once per signal envelope it
appears in (even when deduped to a single stored record), plus once per child.
Demonstrated: a single low-quality resource appearing in both `resourceSpans`
and `resourceMetrics` (1 stored resource + 1 span + 1 metric = 3 records that
carry the hint) reports `low_quality_entity_hints = 4`. With logs too it climbs
further. This is exactly the realistic multi-signal OTLP case (one service emits
spans, metrics, and logs), so the number is unreliable precisely where it
matters. Q1 asked for *clean counted fields*; this isn't one. Pick one precise
definition — e.g. "number of stored records carrying a low/missing entity hint"
— and count once per stored record (move the resource increment after the dedup
decision; don't also count in both `push_resource` and `entity_hint`). Add a
test that pins the count for a multi-signal low-quality resource so the
regression can't reappear.

**D2 — dead reject path contradicts the documented M2 rule (low).**
`resource_context` returns `Some(entity)` in every branch (High/Low/Missing all
set `entity = Some(...)`), so (a) the `if resource.entity.is_none() &&
matches!(signal, OtlpSignal::MetricPoint)` branch in `entity_hint` is
unreachable, and (b) the `else { issue "metric point cannot produce a stable
entity" }` reject branch in `normalize_metric` (~605–613) can never fire. Yet
the design doc's M2 step 3 ("If neither a service entity nor a deterministic
resource key can be produced, **reject** the metric point") is now documented as
real behavior. Either remove the dead branches (and soften the doc to "a
deterministic resource key is always derivable, so metric points are not
rejected for missing entity"), or make `resource_context` actually return `None`
in a genuine no-key case and exercise the reject path with a test. Right now the
doc promises a behavior the code cannot reach.

**D3 — reconcile "leave unresolved" vs "synthetic resource entity" in the doc
(low).** For the missing/low case the adapter assigns a synthetic entity
(`resource:attrs:<hash>`) to spans/logs/metrics, and for the empty-attributes
case the hash is over the *envelope path*, not attributes. That is a defensible
choice and is consistent with the updated M2 step 2, but the doc still also says
(Minimal Entity Hints / Source Keys) to "leave the record unresolved and count
the missing hint." Those two statements now conflict. Pick one rule and state it
once so a future reader/implementor isn't misled. (Note: hashing the envelope
path means two attribute-less resources at different positions get distinct
keys — good for not collapsing them, but it makes the "entity" position-derived;
confirm that's intended.)

**D4 — trace record has no time window (minor; not blocking this DoD).** The
trace `StoredRecord` is built from `{trace_id, spans, provenance}` with no
top-level start/end, so time-window selectors won't locate traces by time. This
topic's DoD is source-ref resolution (which works), so it's not a blocker, but
deriving the trace window from earliest/latest span would help the
derived-context follow-up. Flag for `entity-resolver-confidence` / later, not
required here.

### Answers to the round's review questions

1. **Scope kept (no HTTP/persistence/derived/ranking/MCP/dashboard)?** Yes,
   verified.
2. **True reuse vs parallel path?** True reuse, verified against `store.ingest`
   and the emitted `HotIngestEvent` variants.
3. **Metric/resource/log-id/provenance decisions stable enough?** Yes for the
   prototype, once D1 is fixed and D2/D3 reconcile the doc with the code. Source
   keys themselves (hex trace/span, `name@entity`, explicit/generated log ids)
   are stable and well-tested.
4. **CLI + JSON summary adequate?** Yes — `--input`/`--json-summary`/`--ref`
   cover the demo and tests; the smoke test exercises them.
5. **Topic complete?** Not quite — close D1 (and tidy D2/D3) in one more round,
   then complete and move to `entity-resolver-confidence`.

### Summary

Milestone substantially delivered; the boundary-reuse thesis is validated and
all gates are green. Must-fix before close: **D1** (counter over-count, a
summary-contract correctness bug). Should-fix: **D2**, **D3** (align code and
doc; remove dead reject path or make it real). **D4** is a follow-up note. None
require another design round. Next action: **continue** — one closing round 2,
then terminate the topic.
