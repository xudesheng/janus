# Fixture OTel Simulator Review 1

- Baseline SHA: `31c37436ae5689e5f9845579bf91bf57a470f2df`
- Current milestone: a `simulate_fixture` CLI and ingest-like replay path that replays a registered fixture into a fresh `HotContextStore`, validates source refs after replay, and proves partial-replay ref availability with tests.
- Critical path: yes - this design-only round tightens the contract required before that simulator artifact can be implemented, and coding remains blocked until reviewers agree on direction.
- Milestone progress: updated `docs/core/fixture-otel-simulator.md` to address review-0 findings on source-ref equivalence, metric accumulation, duplicate-key semantics, trace availability, and future OTLP source-key normalization.
- Deferred milestone work: Rust implementation, CLI, and tests remain deferred because review-0's Direction Verdict was conditional and reviewers have not yet unconditionally agreed that implementation should start.

This round is design-only. I read the current design and the latest review-0 feedback, including Claude's conditional direction verdict. I did not start coding.

## Response To Review 0

Claude's direction verdict accepted the design content conditionally but required an explicit go/no-go on building the demo bridge ahead of `entity-resolver-confidence`. The User's current instruction keeps the TOPIC as `fixture-otel-simulator` and asks this branch to advance per the review framework. I treated that as approval to continue the design review loop for this topic, not as permission to start Rust implementation before reviewers give a clear `continue` verdict.

I updated the formal design to close the actionable specification findings:

- Finding A: reframed `HotContextStore::load_fixture_case` as a full-replay source-ref compatibility oracle, not a byte-for-byte stored-record oracle.
- Finding B: specified metric-series accumulation payload shape: preserve metric metadata, append points in replay order, keep observed-point prefixes during partial replay, and never drop or synthesize points.
- Finding C: stated that metric series is the only merge-eligible `(StoredRecordKind, SourceKey)` pair; non-metric duplicates remain hard errors.
- Finding D: defined trace ref availability: the `Trace` event makes the trace source key resolvable and should be emitted when the trace first has observable data.
- Q2 tightening: added that source-specific adapters own source-key derivation, so fixture ids and future real OTLP ids are normalized by their respective sources before hitting the hot-store write model.
- Finding E: this review round names the topic-level artifact as the current milestone rather than using design approval as the milestone.

I did not change review-0, and I did not edit any locked review content.

## Review Focus

Please focus this round on direction first:

1. Does the User's renewed instruction to continue the `fixture-otel-simulator` topic satisfy the requested go/no-go on building the demo bridge ahead of `entity-resolver-confidence`, or is a separate explicit User statement still required?
2. If direction is acceptable, can reviewers now give an unconditional `continue` verdict for implementation after this round?
3. Are the review-0 specification findings A-D adequately closed in the formal design?
4. Is the topic-level milestone phrased correctly for later implementation review?
5. If implementation is approved, should it proceed in the three proposed slices: replay planning/dry run, hot-store ingest boundary, then CLI and validation surface?

## Verification

No code verification this round. Design-only checks performed:

- `git diff --check -- .\docs\core\fixture-otel-simulator.md` passed before the formal design-doc commit.
- Confirmed the working branch is `fixture-otel-simulator`.
- Confirmed the formal design-doc change was committed and pushed before this review document was created.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
