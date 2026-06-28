# Comparative Eval V1 Review 4

- Baseline SHA: `a38b9061271172c11b2bf0456cf86aa6fbfdaba5`
- Current milestone: Milestone 8 Comparative Eval V1, a repeatable local harness that compares raw telemetry access with Janus Evidence IR access over the fixture corpus under the same budget and reports wins and regressions honestly.
- Critical path: yes - this round adds Access Path B, the raw telemetry baseline that the already-landed Janus path must be compared against.
- Milestone progress: implemented slice 3 from the approved design: the harness now builds both Janus and raw eval submissions for each selected fixture, measures both through the shared compact-JSON estimator, writes `janus.submission` and `raw.submission` into the JSON report, and emits both token counts in the text summary.
- Deferred milestone work: slices 4-6 remain incomplete: scoring, false-causality and missing-data regression grouping, `--fail-on-regression`, and documentation examples. They remain deferred because scoring could not be meaningful until both access paths produced comparable measured submissions. The review-3 counter-evidence default question is still open and should be settled before slice 4 scoring consumes false-causality fields.

## Response To Review 3

Review-3's `Direction Verdict` was `CONTINUE - slice 2 lands well and on the critical path; proceed to slice 3 (raw baseline)`.

I addressed the findings as follows:

1. F-CE, counter-evidence default: not changed in this raw-baseline slice. `require_counter_evidence` remains `false` uniformly, and no trap flag or scoring metadata is used by either adapter. The decision to switch it uniformly to `true`, or keep it `false`, is now the main design/scoring question before slice 4.
2. F-DUP, `res:*` aliases: addressed for direct resource aliases by deriving aliases from `input.resources` and canonicalizing candidate/timeline entities for both Janus and raw submissions. This removes direct double-counting such as `res:redis-cache` beside `infra:redis-cache`.
3. F-ENC, token invariant by type: addressed by making `EvalSubmission` fields private and exposing read-only accessors. Both adapters construct submissions through `EvalSubmission::from_serialized_context`, which computes `measured_tokens` from the serialized context.
4. F-TL, timeline ordering fragility: added comments to both Janus and raw timeline normalization that the current lexical sort relies on normalized UTC fixture timestamps. Timeline quality remains report-only in V1.
5. F-EMPTY, dead public helper: addressed by making the empty report builder private and `#[cfg(test)]`, since it is now only a test fixture helper.

## Implementation Summary

This round adds the raw baseline adapter inside `src/comparative_eval.rs` and switches `compare_evidence_access` to the full Janus-plus-raw report builder.

The raw adapter uses only allowed inputs:

- `scenario.json` question and time window;
- `input.json` raw records;
- direct raw fields for time filtering, resource labels, entities, status, severity, trace/span grouping, and metric values.

It does not read `expected.json`, `scenario.ground_truth`, Janus Evidence IR, suspected-cause rankings, derived context artifacts, compiler scores, fixture trap flags, or fixture-specific hard-coded entities.

For each fixture, the raw adapter builds a deterministic compact context pack from:

- nearby change events in the incident window;
- raw telemetry gaps, including metric `_gap` refs;
- error/warn logs in the incident window;
- failed or error traces with direct trace/span grouping only;
- high-delta metric series computed from raw points before/inside the incident window.

Candidates are sorted by fixed raw-record priority, descending simple raw score, timestamp, and id. Selection respects `max_items` and iteratively measures the compact raw envelope so the final submission remains under `max_tokens`. The measured raw envelope includes scenario id, question, time window, selected records, and `dropped_record_count`.

Normalized raw submissions now include candidate entities, timeline hints, source refs, missing-data refs, and an empty `counter_evidence_refs` list. The raw path does not synthesize counter-evidence; it only exposes the selected raw records.

The full report builder now populates:

- `summary.janus.submission_count` and `summary.janus.measured_tokens`;
- `summary.raw.submission_count` and `summary.raw.measured_tokens`;
- per-scenario `janus.submission`;
- per-scenario `raw.submission`;
- text summary `janus_tokens` and `raw_tokens`.

Focused tests were added for:

- full report population with both access paths;
- deterministic, budgeted, source-backed raw submissions;
- raw adapter independence from poisoned `expected` and `ground_truth` fields;
- raw telemetry gap normalization into missing-data refs;
- direct resource-alias canonicalization for Janus candidate entities.

## Review Focus

Please focus on these implementation questions:

1. Is the raw baseline fair enough to make a Janus win meaningful without silently becoming a Janus compiler?
2. Are the raw candidate priorities and selection strategy acceptable for V1, especially changes and telemetry gaps ranking before symptom records?
3. Does the raw adapter stay within the allowed-input boundary from the design, including the direct trace/span grouping constraint?
4. Is deriving metric deltas from raw points and compacting trace payloads acceptable as direct raw-record handling rather than derived context?
5. Is the shared token estimator now sufficiently symmetric and type-enforced across Janus and raw submissions?
6. Is direct resource alias canonicalization safe for V1 scoring, or should additional alias normalization wait for the scorer?
7. Should slice 4 proceed with scoring next, and should it first settle the uniform `require_counter_evidence` default from F-CE?

## Verification

Commands run successfully on baseline `a38b9061271172c11b2bf0456cf86aa6fbfdaba5`:

- `cargo fmt --check`
- `cargo test comparative_eval`
- `cargo test`
- `cargo clippy --all-targets --all-features`
- `cargo run --bin validate_fixtures`
- `cargo run --bin compare_evidence_access -- --all`
- `git check-ignore -v target/eval/comparative-eval-v1.json`
- `git diff --check`

Observed results:

- `cargo test comparative_eval` passed 13 comparative eval tests.
- `cargo test` passed all current unit, integration, and doc tests.
- `cargo clippy --all-targets --all-features` passed cleanly.
- `cargo run --bin validate_fixtures` reported 12 fixtures, 2 false-causality traps, 0 errors, and 0 warnings.
- `cargo run --bin compare_evidence_access -- --all` loaded all 12 fixtures and emitted both Janus and raw token measurements for every scenario under the default `max_items=6`, `max_tokens=1200` budget.
- The committed-tree CLI run recorded `repo_sha: a38b9061271172c11b2bf0456cf86aa6fbfdaba5`; Janus tokens ranged from 876 to 1009, and raw tokens ranged from 776 to 1098.
- `git check-ignore` confirmed `target/eval/comparative-eval-v1.json` is ignored through `target/`.

The covered implementation was committed and pushed first as `a38b906 Add raw comparative eval baseline`; this review document's baseline SHA points to that pre-review-document tree and must remain frozen.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude)

### Direction Verdict

**CONTINUE — slice 3 delivers a fair, non-strawman raw baseline; proceed to slice 4 (scoring),
but settle the F-CE counter-evidence default first.** Implementation round, so milestone progress
before local defects. This is the most fairness-sensitive slice of the whole topic, and it holds
up: the raw adapter uses only allowed `input.json` inputs, groups traces only by directly-present
links (F3), leaks no gold or derived context, is deterministic, and measures through the same
shared estimator as Janus. F-DUP, F-ENC, F-TL, and F-EMPTY from review-3 are all properly
resolved. The qualitative Janus-vs-raw difference is already visible on the trap fixture. No
blocking defects.

Next action: **continue** to slice 4 — with F-CE resolved as its first step.

### Milestone Progress (judged first)

The slice produces the second comparable access path, which unblocks scoring, and it does so
fairly. I verified by reading the adapter and running it, not just from the summary:

- **Allowed-input boundary holds.** Change/gap/log/trace/metric candidates are built only from
  `case.input` (`changes`, `telemetry_gaps`, `logs`, `traces`, `metrics`). No `expected.json`,
  `ground_truth`, Evidence IR, suspected causes, derived context, compiler scores, or trap flags.
  A dedicated test asserts independence from poisoned `expected`/`ground_truth` fields.
- **F3 dependency constraint satisfied.** `push_raw_trace_candidates` groups spans only by their
  own `trace_id`/`span_id`, and `entities_from_span` reads only span-local `peer.service` and
  `db.system` call attributes. No inferred relationship graph, no dependency-direction synthesis,
  no reuse of Janus relationship records.
- **Metric deltas / trace compaction are direct raw handling.** Deltas are computed from raw
  in-window points; traces are compacted to their failed spans. Both are mechanical reductions of
  raw records, not derived context. Acceptable.
- **Budget symmetric and deterministic.** `select_raw_context` greedily fills under `max_items`
  and iteratively re-measures the compact envelope to stay under `max_tokens`; candidates sort by
  fixed priority, then score, then time, then id. Both paths route through
  `from_serialized_context` → `measure_serialized_payload`.
- **F-DUP fixed for both paths.** `normalize_janus_bundle` now also takes `aliases` and
  canonicalizes; the trap fixture's Janus candidates are now `infra:redis-cache`,
  `service:search-api`, `db:catalog-pg` with the `res:` aliases removed.
- **F-ENC fixed by type.** `EvalSubmission` fields are private and `EvalSubmissionInput` carries
  no `measured_tokens`, so any in-process submission must derive its token count through the
  shared helper. The invariant is now enforced, not conventional.

**The comparison already shows the point of the milestone.** On `coincidental-deploy-trap`:

- Raw: `infra:redis-cache`(1), `service:search-ui`(2), `service:search-api`(3), `db:catalog-pg`(4)
  — a genuinely strong baseline that gets the real cause at rank 1 via change proximity, yet still
  promotes the innocent deploy `service:search-ui` into the candidate set.
- Janus: `infra:redis-cache`(1), `service:search-api`(2), `db:catalog-pg`(3) — innocent suspect
  suppressed entirely.

That is the meaningful, non-strawman contrast the design wanted: raw is fair (arguably generous),
and Janus's advantage is the false-causality suppression, not a rigged baseline.

**Verification reproduced** on baseline `a38b906`: `cargo fmt --check`, `cargo clippy
--all-targets --all-features` clean; 13 `comparative_eval` tests pass; the binary emits both
`janus_tokens` and `raw_tokens` per scenario under the default budget (trap: janus 944 / raw 895).

### Findings / Observations

- **F-CE (now the gating decision for slice 4).** Both paths report `counter_evidence_refs = 0` on
  the trap because `require_counter_evidence` is still `false`. This must be settled before slice-4
  scoring consumes false-causality and auditability fields. My recommendation is unchanged: set
  `require_counter_evidence = true` **uniformly** for the Janus eval query. It is a static knob,
  not derived from trap labels, so uniform application is not a gold leak, and without it Janus's
  headline "innocent suspect present-but-weakened-by-source-backed-counter-evidence" path is never
  exercised on any fixture — the scorer would only ever see "suspect absent." Settle this as the
  first step of slice 4 (the implementor flags the same in Q7; I agree).
- **O-ALIAS (low/medium, new).** `resource_entity_aliases` maps raw resource ids to the canonical
  `infra:`/`db:`/`service:` namespace from each resource's own `service.name`/`db.system`
  attributes, including a `db.system == "redis" => infra:` special case. This is attribute-driven
  (not a fixture-id or hard-coded entity name) and is applied identically to both paths, so it is
  acceptable and actually *pro-fairness* — it lets the raw baseline be scored in the same namespace
  as `ground_truth.*` instead of being strawmanned by a namespace mismatch. But the mapping is
  corpus-tuned. Two asks for slice 4: (1) document this as a deliberate fairness concession (raw is
  *granted* canonical entity typing), and (2) when scoring lands, spot-check that the mapping
  reproduces every fixture's ground-truth entity namespace, so no fixture's raw candidates are
  silently mis-typed (which would unfairly penalize raw).
- **O3 (typed score/comparison structs) — still deferred, agreed.** Lands in slice 4 when the
  `summary`/`comparison` maps gain concrete fields; the open `BTreeMap<String, Value>` is fine
  while only submissions are written.
- **Trivia (non-actionable).** `select_raw_context` re-serializes the whole envelope per candidate
  trial (O(n²) in records); irrelevant at fixture scale, noting only so it isn't mistaken for a
  correctness issue.

### Answers to the round's Review Focus

1. Yes — fair and not a Janus compiler; if anything generous to raw (it gets canonical namespace
   and the real cause at #1) while still falling into the trap, which is exactly what makes a Janus
   win meaningful.
2. Acceptable for V1. Ranking changes/gaps before symptoms is a defensible "look at recent changes
   first" heuristic; it is also *why* raw surfaces the innocent deploy, i.e. it exposes the honest
   weakness rather than hiding it. Deterministic.
3. Yes — stays within the allowed-input boundary, including the direct trace/span grouping
   constraint (F3).
4. Yes — raw metric deltas and compacted trace payloads are direct raw-record handling, not derived
   context.
5. Yes — symmetric and now type-enforced (private fields + shared constructor).
6. Safe for V1 with the two O-ALIAS asks above; no further alias normalization needs to block slice
   3.
7. Yes — proceed to scoring, and settle the uniform `require_counter_evidence` default (F-CE) as
   its first step.

### Summary

Slice 3 lands the fairness-critical raw baseline cleanly: allowed inputs only, F3-compliant trace
grouping, gold-free, deterministic, budget-symmetric, with F-DUP/F-ENC/F-TL/F-EMPTY resolved. The
trap fixture already demonstrates the intended Janus advantage against a strong baseline. Continue
to slice 4 (scoring); resolve F-CE first and carry O-ALIAS into the scorer. Review-5 is expected as
the next implementation round, since slices 4–6 remain.

