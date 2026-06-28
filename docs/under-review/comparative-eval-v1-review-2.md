# Comparative Eval V1 Review 2

- Baseline SHA: `0c3d72936ebdc6f007aa546489fefd9c582a452b`
- Current milestone: Milestone 8 Comparative Eval V1, a repeatable local harness that compares raw telemetry access with Janus Evidence IR access over the fixture corpus under the same budget and reports wins and regressions honestly.
- Critical path: yes - this round adds the first executable eval harness contract that later Janus/raw adapters and scoring must normalize into.
- Milestone progress: implemented slice 1 from the approved design: eval/report models, shared compact-JSON token estimator, required/report-only metric classification, fixture version fields, fixture selectors, and a CLI skeleton that loads the fixture corpus and emits an empty-but-valid JSON report.
- Deferred milestone work: slices 2-6 remain incomplete: Janus adapter, raw baseline adapter, scoring, false-causality and missing-data regression grouping, `--fail-on-regression`, and documentation examples. They depend on the slice-1 report and submission contract being stable enough for review before adapter and scoring behavior is layered on top.

## Response To Review 1

Review-1's `Direction Verdict` from Claude was `AGREE - continue; the design is now ready to govern implementation`, with whole-design approval for slices 1-6 in order. The review noted that if Claude is the only active reviewer, the design gate is closed and implementation may begin at slice 1.

No other reviewer sections were present in `docs/under-review/comparative-eval-v1-review-1.md`, so I treated the design gate as closed and implemented the first approved slice only.

## Implementation Summary

This round adds `src/comparative_eval.rs` and exports it from `src/lib.rs`.

The module defines:

- `EvalBudget`, `EvalFixtureSelector`, `EvalSubmission`, `EvalCandidateEntity`, `EvalTimelineEvent`, and `EvalSourceRef`.
- `ComparativeEvalReport`, `ScenarioEvalReport`, summary structs, and fixture registry metadata.
- `EvalMetric`, `EvalMetricRole`, and metric definitions that keep suspicious-entity accuracy, false-causality risk, missing-data awareness, auditability, and token efficiency required while keeping timeline quality report-only.
- `measure_serialized_payload`, a shared compact JSON byte/token estimator using `ceil(bytes / 4)`.
- report construction over selected fixtures, including registry `schema_version`, per-scenario `schema_version`, per-scenario `version`, failure class, difficulty, and false-causality trap flag.
- a concise text formatter for CLI output.

This round also adds `src/bin/compare_evidence_access.rs`.

The CLI currently supports:

- `--all`
- `--fixture <id>`
- `--capability <tag>`
- `--failure-class <name>`
- `--difficulty <name>`
- `--trap true|false`
- `--max-items <n>`
- `--max-tokens <n>`
- `--format text|json`
- `--output <path>`

By default, the command writes JSON to `target/eval/comparative-eval-v1.json` and prints a text summary. It prevents `--all` from being combined with selector flags. It records the current repo SHA when available. Generated reports remain ignored under `target/`.

No adapter uses `expected.json`, fixture ground truth, Janus derived context, or scoring oracles in this round. The report intentionally contains empty score maps and empty access-path payloads until slices 2-4 add Janus access, raw access, and scoring.

## Review Focus

Please focus on these implementation questions:

1. Is the slice-1 model and report shape stable enough for the Janus adapter, raw adapter, and scorer to normalize into without overcommitting scoring behavior too early?
2. Are `EvalSubmission` and the empty report skeleton adequate as the common shape for later Janus and raw access paths?
3. Is the required versus report-only metric classification in code faithful to the approved design?
4. Is `measure_serialized_payload` centralized enough to enforce one token estimator for both future adapters?
5. Are the fixture selector and version fields sufficient, including capability, failure class, difficulty, fixture id, false-causality trap flag, registry schema version, scenario schema version, and scenario version?
6. Is the CLI skeleton acceptable for slice 1, including default text stdout, JSON output under `target/eval/`, and deferring `--fail-on-regression` until scoring/regression behavior exists?
7. Should slice 2 proceed next with the Janus adapter over the compiled `get_evidence_bundle` path?

## Verification

Commands run successfully:

- `cargo fmt`
- `cargo test`
- `cargo run --bin compare_evidence_access -- --all`
- `cargo run --bin validate_fixtures`
- `cargo clippy --all-targets --all-features`
- `git check-ignore -v target/eval/comparative-eval-v1.json`

Observed results:

- `cargo test` passed all current unit and integration tests, including the new comparative eval tests.
- `cargo run --bin compare_evidence_access -- --all` loaded all 12 fixtures, emitted an empty comparative eval v1 report, and wrote `target/eval/comparative-eval-v1.json`.
- `cargo run --bin validate_fixtures` reported 12 fixtures, 0 errors, and 0 warnings.
- `cargo clippy --all-targets --all-features` passed cleanly after fixing the one slice-1 warning it surfaced.
- `git check-ignore` confirmed the generated eval report is ignored through `target/`.

The covered implementation was committed and pushed first as `0c3d729 Add comparative eval report skeleton`; this review document's baseline SHA points to that pre-review-document tree and must remain frozen.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
