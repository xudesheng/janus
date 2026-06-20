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

## Review (by Claude)

### Direction Verdict

**Agree — continue to implementation.** I am lifting the conditional hold from review-0.
Design *content* is approved unconditionally: all five round-0 findings (A–E) plus the Q2
seam are closed precisely and correctly in `docs/core/fixture-otel-simulator.md`, and I
re-verified each against the actual hot-store code rather than the response text. There
are no blocking findings this round. Next action: **continue** — implementation may start,
phase-by-phase in the three proposed slices.

On the direction gate I raised in round-0 (the off-roadmap "demo bridge ahead of
`entity-resolver-confidence`"): I am clearing it, with one transparency note. I do not have
a *new, explicit* User sentence saying "build the demo bridge now," and I will not pretend
the procedural instruction to advance the review loop is the same thing. But two facts have
accumulated that I judge sufficient to stop blocking:

1. The User has now twice directed this exact topic forward and has **not** redirected to
   `entity-resolver-confidence` when redirecting was the easy response to my round-0 hold.
2. The User explicitly framed reviewers as the deciding gate ("the implementor must not
   start coding until all reviewers agree"), i.e. delegated the agreement call to me.

Continuing to demand a separate ceremonial go/no-go would now be process-pedantry that
stalls a loop the User keeps pushing forward — which the framework warns against. So I read
the sustained direction as endorsement and clear the gate.

**Final off-ramp for the User (the one thing that would reverse this):** if your intent is
to return to `entity-resolver-confidence` (Milestone 5) and you are only keeping this
branch alive procedurally, say so and this topic stops as a design-only branch per its own
Review Gate. Absent that, I treat the direction as approved and coding may begin.

This answers the implementor's **Q1** (yes, I now treat the renewed direction as sufficient
— no separate User statement required unless the User wants to redirect) and **Q2** (yes,
unconditional `continue` for implementation).

### Findings A–E: verified closed

Checked against the design doc diff `b3eeedd..31c3743`, not just the response summary:

- **A — closed well.** `load_fixture_case` is now a "full-replay source-ref compatibility
  oracle" with the explicit carve-out that byte-for-byte `StoredRecord` equality is *not*
  required except where a shape is pinned. This is the lower-risk bar I recommended and it
  resolves the trace-payload-with-embedded-spans divergence cleanly.
- **B — closed well.** Metric-series convergence shape is pinned (`name`/`entity`/`unit`
  preserved, points appended in replay order, observed-prefix during partial replay, never
  drop/synthesize). This is exactly the target slice-2 tests needed.
- **C — closed precisely.** "Metric series is the only merge-eligible
  `(StoredRecordKind, SourceKey)` pair" and non-metric duplicates "remain errors for all
  non-metric record kinds." The duplicate-key guarantee from the hot-store round is
  preserved with a single named exception. Good.
- **D — closed.** The ambiguous "resolves only after the trace has observed data" line is
  gone; the `Trace` event now makes the trace key resolvable, emitted at earliest span
  time. Matches my recommendation.
- **Q2 — closed.** The source-key-derivation seam is named: the adapter owns key
  normalization, and a future `OtlpReceiverSource` must map real OTLP ids into the same
  write model. The reusability claim is no longer overstated.
- **E — closed.** The round milestone now names the topic-level artifact (the
  `simulate_fixture` CLI + ingest path), so later rounds measure critical path against the
  artifact.

### Answers to Q3–Q5

- **Q3 (A–D adequately closed?)** — Yes, all four, verified above.
- **Q4 (milestone phrased correctly?)** — Yes. The artifact-level phrasing is right for
  implementation review.
- **Q5 (three slices?)** — Yes, proceed in the proposed order. Slice 2 (ingest boundary)
  carries the real risk and should be reviewed most carefully; see the non-blocking notes.

### Non-blocking notes for the implementation rounds (not re-gating)

These are guidance to fold into the relevant slice; none block approval.

1. **Slice 2 must include the A/B/C/D behaviors as named tests**, not just "partial replay
   works": (a) a metric-series record's `points[]` is a strict observed-prefix mid-replay
   and the full set post-replay (Finding B); (b) a non-metric duplicate `(kind,key)` still
   returns `DuplicatePrimaryKey` while a duplicate metric series merges (Finding C); (c) the
   trace ref resolves at the `Trace` event and the span ref does not resolve until its span
   event (Finding D). The existing design Tests list mostly covers these, but call out C's
   negative case explicitly — it is the one most likely to be missed.
2. **Zero-span trace edge case (minor).** "Emit the `Trace` event at the earliest span
   time" is undefined if a trace has no spans. Current fixtures all carry spans, so this is
   not a blocker, but state the fallback (e.g. trace event uses its own start time, or the
   case is asserted not to occur) so the implementation does not silently panic on an
   empty-span trace.
3. **Resolution-equivalence is the acceptance bar (Finding A) — keep it that way under
   test.** When you assert "full replay resolves all current fixture evidence source refs,"
   compare *resolution outcomes*, not stored payloads, against `load_fixture_case`.
   Asserting payload equality would reintroduce the trace-payload divergence A just
   resolved.

### Framework / process check (passes)

- Baseline `31c3743` is pushed and is the parent of the review-1 commit `d89b0d1`
  (pre-review-document tree); frozen. Correct.
- Round-0 locked content is unchanged (`b5a9922..d89b0d1` touches only the design doc and
  the new review-1.md). The Implementor honored the locking rules.
- Header fields complete; milestone names the artifact; design-only round correctly states
  "No code verification this round."

### Bottom line

Design approved, hold lifted, direction gate cleared. **Continue to implementation** in the
three slices, with the negative-path tests called out above. The next round should be an
implementation round (slice 1), and milestone work being incomplete is itself sufficient
grounds to open it.
