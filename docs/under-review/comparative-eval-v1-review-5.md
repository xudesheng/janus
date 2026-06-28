# Comparative Eval V1 Review 5

- Baseline SHA: `f5b59c3559ff64700562bafd2004d2a2761d4f5e`
- Current milestone: Milestone 8 Comparative Eval V1, a repeatable local harness that compares raw telemetry access with Janus Evidence IR access over the fixture corpus under the same budget and reports wins and regressions honestly.
- Critical path: yes - this round implements slice 4 scoring and resolves the review-4 F-CE and O-ALIAS gates needed before scoring could be meaningful.
- Milestone progress: the harness now scores both Janus and raw submissions for every selected fixture, writes typed per-scenario score comparisons under `comparison.scores`, aggregates required-score summaries, prints scenario winners in the text report, sets the Janus eval query to require counter-evidence uniformly, and documents/tests the raw resource-alias fairness concession.
- Deferred milestone work: slices 5-6 remain incomplete: richer false-causality and missing-data regression grouping, `--fail-on-regression`, and documentation examples. The current scoring payload should be the substrate for those slices.

## Response To Review 4

Review-4's `Direction Verdict` was `CONTINUE - slice 3 delivers a fair, non-strawman raw baseline; proceed to slice 4 (scoring), but settle the F-CE counter-evidence default first.`

I addressed the actionable feedback as follows:

1. F-CE, counter-evidence default: changed the V1 Janus eval query to set `require_counter_evidence = true` and `min_counter_evidence_items = 1` uniformly for every fixture. The design doc now states this is a static eval-query requirement, not a trap-label or gold conditional.
2. F-CE implementation consequence: the first uniform-counter run exposed that several fixtures could not satisfy the requirement through explicit counter candidates alone. I added query-gated structural counter-evidence materialization inside `compile_evidence`: it only runs when the query requires counter-evidence, uses already-generated runtime candidates and suspected-cause links, and emits source-backed weakens items without reading fixture gold.
3. O-ALIAS: documented resource aliasing in the design as a deliberate fairness concession for raw access. The aliaser uses direct raw `resources` attributes only (`service.name`, `db.system`, `rollout`, `service.instance.id`, `service.version`, `cluster.name`), applies to both normalized Janus and raw entities, and does not infer relationships. A new test spot-checks the mapping against scored service/db/infra ground-truth entities.
4. O3, typed score/comparison structs: added `EvalMetricScore`, `EvalPathScores`, `EvalScoreDelta`, and `EvalScenarioComparison`. The report still keeps extensible maps at the outer `summary` and `comparison` boundaries, but the score payload itself is typed and round-trips through serde.
5. The raw baseline still does not synthesize counter-evidence. Its `counter_evidence_refs` remain empty unless future raw-record handling explicitly exposes a direct counter record.

## Implementation Summary

Slice 4 adds five required metrics and one report-only metric:

- `suspicious_entity_accuracy`: rewards primary-cause rank, partial credit for visibility, and caps score when a known not-the-cause entity is ranked first.
- `false_causality_risk`: high score means lower risk. It penalizes ranked not-the-cause or innocent-suspect entities, with details exposing the raw `risk` value, avoid-entity rank, and counter-evidence count.
- `missing_data_awareness`: required only for missing-data or under-determined scenarios; rewards visible gap refs and explicit uncertainty.
- `auditability`: rewards source refs that resolve through `ReferenceIndex` and coverage of expected signal families from the expected evidence bundle.
- `token_efficiency`: scales useful score by a modest budget-utilization penalty so utility dominates pure compactness.
- `timeline_quality`: report-only; checks chronological ordering, source-backed events, and expected timeline event coverage.

The full report builder now computes per-scenario Janus/raw scores, score deltas, and a required-score winner. It also aggregates average Janus/raw scores, average deltas, scenario wins, and false-causality trap summaries. The text report now includes top-level required averages and per-scenario `janus_score`, `raw_score`, and `winner`.

The structural counter-evidence helper is intentionally narrow:

- It is called only when `required_counter_evidence_count(query) > 0`.
- It does not change default non-counter-required bundle behavior.
- It skips `under-determined`, already-countered entities, and top-ranked causes without explicit counter links.
- For explicit counter links, it reuses those source refs.
- For low-scoring non-top alternatives without explicit counter links, it can create a weakens item from that alternative's runtime source-backed candidate refs. This is the least settled semantic choice in the round and should get reviewer attention.

## Review Focus

Please focus on these questions:

1. Are the scoring formulas simple and fair enough for V1, especially the rank-based suspicious-entity scoring and the modest token-efficiency penalty?
2. Is `false_causality_risk` clear enough as a high-is-good score, given that its details include a high-is-bad `risk` value?
3. Is the query-gated structural counter-evidence helper acceptable, or is it too much compiler behavior introduced for the eval harness?
4. Does the resource alias mapping remain a fair raw-baseline concession rather than corpus-tuned inference?
5. Is it acceptable for auditability scoring to use expected evidence-bundle source signal families for coverage, while still requiring actual selected source refs to resolve through `ReferenceIndex`?
6. Are raw wins on `traffic-shift-hotspot` and `missing-data-gap` visible enough, and should slice 5 build `--fail-on-regression` around the current `required_average` and winner fields?
7. Should `timeline_quality` remain report-only for V1, or should any part of it move into the required average before regression gating?

## Verification

Commands run successfully on baseline `f5b59c3559ff64700562bafd2004d2a2761d4f5e`:

- `cargo fmt --check`
- `cargo test comparative_eval`
- `cargo test`
- `cargo clippy --all-targets --all-features`
- `cargo run --bin validate_fixtures`
- `cargo run --bin compare_evidence_access -- --all`
- `git check-ignore -v target/eval/comparative-eval-v1.json`
- `git diff --check`

Observed results:

- `cargo test comparative_eval` passed 16 comparative eval tests.
- `cargo test` passed all 36 library tests plus current integration and doc tests.
- `cargo clippy --all-targets --all-features` passed cleanly.
- `cargo run --bin validate_fixtures` reported 12 fixtures, 2 false-causality traps, 0 errors, and 0 warnings.
- `cargo run --bin compare_evidence_access -- --all` recorded `repo_sha: f5b59c3559ff64700562bafd2004d2a2761d4f5e`.
- Aggregate required average: Janus `0.890`, raw `0.828`, delta `0.061`.
- Scenario winners: Janus won 10, raw won 2 (`traffic-shift-hotspot`, `missing-data-gap`).
- The two false-causality trap fixtures both scored as Janus wins: `retry-storm-amplification` (`0.886` vs `0.859`) and `coincidental-deploy-trap` (`0.887` vs `0.770`).
- `git check-ignore` confirmed `target/eval/comparative-eval-v1.json` is ignored through `target/`.

The covered implementation was committed and pushed first as `f5b59c3 Add comparative eval scoring`; this review document's baseline SHA points to that pre-review-document tree and must remain frozen.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
