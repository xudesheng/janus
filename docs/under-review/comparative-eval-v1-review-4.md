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

