# Comparative Eval V1 Review 3

- Baseline SHA: `a12ee76100554eeeee29cc2d1db2787b7c7cd4e9`
- Current milestone: Milestone 8 Comparative Eval V1, a repeatable local harness that compares raw telemetry access with Janus Evidence IR access over the fixture corpus under the same budget and reports wins and regressions honestly.
- Critical path: yes - this round adds the Janus access path that the raw baseline and scorer must compare against.
- Milestone progress: implemented slice 2 from the approved design: the harness now builds Janus eval submissions by constructing an `EvidenceQuery` from scenario question, time window, scenario id, and eval budget; calling the compiled `get_evidence_bundle` path; normalizing the returned `EvidenceBundle`; measuring the serialized context through the shared compact-JSON helper; and writing Janus submissions into the JSON report and text summary.
- Deferred milestone work: slices 3-6 remain incomplete: raw baseline adapter, scoring, false-causality and missing-data regression grouping, `--fail-on-regression`, and documentation examples. They remain deferred because slice 2 first establishes the Janus-side submission shape that slice 3 must match under the same budget.

## Response To Review 2

Review-2's `Direction Verdict` was `CONTINUE - slice 1 lands cleanly and on the critical path; proceed to slice 2 (Janus adapter)`.

I addressed the non-blocking observations as follows:

1. O1, dead public wrapper: removed the unused empty-report loading wrapper and routed the CLI through `load_comparative_eval_report_with_janus`, which now loads the corpus and builds the Janus-backed report.
2. O2, keep F5 honest end-to-end: added `EvalSubmissionInput` plus `EvalSubmission::from_serialized_context`, and the Janus adapter uses that constructor so `measured_tokens` is derived from `serialized_context` at construction.
3. O3, typed report shape when scores land: not implemented yet because this round does not add scores. The open `BTreeMap<String, Value>` fields remain for `summary`, `janus`, `raw`, and `comparison`; this should be revisited in slices 4-5 when score and comparison fields become concrete.

## Implementation Summary

This round adds a Janus adapter inside `src/comparative_eval.rs`.

For each selected fixture, the adapter:

- parses `scenario.json` `time_window` into `TimeWindow`;
- builds an `EvidenceQuery` from scenario question, time window, scenario id, and eval budget;
- sets `require_raw_refs = true`;
- leaves `require_counter_evidence = false` so the runtime query does not use fixture trap labels or scoring metadata;
- calls `get_evidence_bundle`;
- serializes the returned `EvidenceBundle` as the Janus agent context;
- derives `measured_tokens` from that serialized context with the shared compact JSON helper;
- extracts candidate entities from selected evidence item entities, ranked by first selected item and max item strength;
- derives chronologically sorted timeline hints from selected evidence item windows;
- normalizes source refs, counter-evidence refs, and missing-data refs into the common eval shape.

The CLI now emits Janus-backed reports instead of empty reports. The text summary includes per-scenario `janus_tokens`, and the JSON report includes each scenario's `janus.submission` plus a `summary.janus` submission count and measured-token total. The raw and comparison sections are still empty until slices 3-4.

Focused tests were added for:

- Janus report submission population from compiled bundle output;
- measured-token derivation from the serialized context;
- selected counter-evidence ref normalization;
- selected missing-data ref normalization;
- chronological timeline event normalization.

No `expected.json`, `scenario.ground_truth`, suspected-cause gold, or fixture-specific scoring shortcut is used by the Janus adapter. The adapter stays on the public `get_evidence_bundle` boundary, so candidate entities currently use the approved fallback from selected evidence item entities rather than internal suspected-cause output.

## Review Focus

Please focus on these implementation questions:

1. Is using the public `get_evidence_bundle` boundary, with evidence-item entity fallback ranking, acceptable for slice 2, or should the adapter switch to a reviewed internal compiler path to include suspected causes before slice 3 proceeds?
2. Is the query construction fair and design-faithful: scenario question, time window, scenario id, eval budget, `require_raw_refs = true`, and no ground truth, expected artifacts, trap flag, or fixture-specific entity hints?
3. Is it acceptable that `require_counter_evidence` remains `false` for the default Janus eval query, with selected counter evidence normalized when the compiled bundle includes it?
4. Are the normalized Janus fields sufficient for later scoring: serialized EvidenceBundle context, measured tokens, candidate entities, timeline hints, source refs, counter-evidence refs, and missing-data refs?
5. Is `EvalSubmission::from_serialized_context` enough to satisfy the shared-estimator invariant for future adapters, or should `EvalSubmission` fields become private before the raw adapter lands?
6. Should typed score/comparison structs be introduced in slice 4, or earlier in slice 3 before the raw baseline starts writing into `raw`?
7. Should slice 3 proceed next with the raw baseline adapter under the same `EvalSubmission` contract?

## Verification

Commands run successfully:

- `cargo fmt`
- `cargo fmt --check`
- `cargo test comparative_eval`
- `cargo test`
- `cargo clippy --all-targets --all-features`
- `cargo run --bin validate_fixtures`
- `cargo run --bin compare_evidence_access -- --all`
- `git check-ignore -v target/eval/comparative-eval-v1.json`
- `git diff --check`

Observed results:

- `cargo test comparative_eval` passed 8 comparative eval tests.
- `cargo test` passed all current unit, integration, and doc tests.
- `cargo clippy --all-targets --all-features` passed cleanly.
- `cargo run --bin validate_fixtures` reported 12 fixtures, 2 false-causality traps, 0 errors, and 0 warnings.
- `cargo run --bin compare_evidence_access -- --all` loaded all 12 fixtures and emitted Janus token measurements for every scenario under the default `max_items=6`, `max_tokens=1200` budget. The committed-tree run recorded `repo_sha: a12ee76100554eeeee29cc2d1db2787b7c7cd4e9`; per-scenario `janus_tokens` ranged from 876 to 1009.
- `git check-ignore` confirmed `target/eval/comparative-eval-v1.json` is ignored through `target/`.

The covered implementation was committed and pushed first as `a12ee76 Add Janus comparative eval adapter`; this review document's baseline SHA points to that pre-review-document tree and must remain frozen.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
