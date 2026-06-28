# Comparative Eval V1 Review 6

- Baseline SHA: `10bebaf142f5571593293d6abdb94ddaba5c4974`
- Current milestone: Milestone 8 Comparative Eval V1, a repeatable local harness that compares raw telemetry access with Janus Evidence IR access over the fixture corpus under the same budget and reports wins and regressions honestly.
- Critical path: yes - review-5 made F-MAT and F-CAND blocking prerequisites before slice 5 regression gating could be built on the score baseline.
- Milestone progress: removed the compiler-side structural counter-evidence materialization, removed the eval query's hard counter-evidence mandate, stopped weakens/counter-only evidence from promoting causal candidates, updated the formal design policy, and re-baselined the all-fixtures score report.
- Deferred milestone work: slices 5-6 remain incomplete: richer false-causality and missing-data regression grouping, `--fail-on-regression`, and documentation examples. They were not advanced in this round because review-5 explicitly required the counter-evidence baseline to be corrected first.

## Response To Review 5

Review-5's `Direction Verdict` was `CONTINUE on the scoring framework, but with one required redirect before slice 5: revert the compiler-side counter-evidence materialization and drop the hard min_counter_evidence_items = 1 mandate from the eval query.`

I addressed the actionable feedback as follows:

1. F-MAT, compiler materialization: removed `push_structural_counter_evidence_candidates` and its call from `compile_evidence`. The compiler no longer relabels low-ranked supporting evidence as counter-evidence for counter-required queries. Existing genuine counter-evidence generation remains untouched.
2. F-MAT, eval query policy: changed the comparative eval Janus query back to no hard counter requirement: `require_counter_evidence = false` and `min_counter_evidence_items = None`. I chose the review-5 allowed fallback rather than changing the global `require_counter_evidence` query semantics, because the existing query contract treats `require_counter_evidence = true` as at least one required counter item.
3. F-MAT, formal design: updated `docs/core/comparative-eval-v1.md` to say V1 should not set a hard `min_counter_evidence_items` mandate and should score counter-evidence only when the reviewed compiler pipeline selects genuine source-backed weakening evidence under the normal budget.
4. F-CAND: changed Janus candidate normalization so `EvidenceKind::CounterEvidence`, `Weakens`, and `Contradicts` items still expose counter refs, audit refs, and timeline hints, but do not promote their entities into the causal candidate list. A focused test now asserts that weakens-only entities are not ranked as causal candidates.
5. Re-baselining: reran the all-fixtures comparison after the fix. The aggregate remains Janus-positive, and the clean false-causality trap result returned: `coincidental-deploy-trap` Janus `false_causality_risk = 1.0`, `risk = 0.0`, `best_avoid_rank = null`.

## Implementation Summary

The code change is intentionally a rollback plus a narrow normalization correction:

- `src/evidence_compiler.rs`: `compile_evidence` once again generates candidates, ranks suspected causes, and selects a compilation without injecting additional structural counter candidates.
- `src/comparative_eval.rs`: `janus_query_for_case` no longer asks for a required counter item; `candidate_entities_from_bundle` skips counter-evidence items when building the candidate ranking.
- `docs/core/comparative-eval-v1.md`: the Janus access section now documents the no-hard-mandate counter policy.

The scorer itself from review-5 was left intact. The changed score profile is caused by removing fabricated counter material and by no longer treating weakens-only entities as promoted candidates.

Notable re-baseline results:

- Aggregate required average: Janus `0.877`, raw `0.828`, delta `0.049`.
- Scenario winners: Janus won 10, raw won 2.
- Raw wins remain visible: `traffic-shift-hotspot` and `missing-data-gap`.
- `coincidental-deploy-trap` improved from Janus `0.887` to `0.949`, with Janus false-causality risk score back to `1.0`.
- `traffic-shift-hotspot` remains a raw win (`0.769` vs `0.875`); the remaining Janus penalty comes from supported `service:orders` evidence, not weakens/counter-only candidate injection.

## Review Focus

Please focus on these questions:

1. Does this fully resolve F-MAT: no compiler-side materialization, no eval hard minimum, and no change to the public `require_counter_evidence` contract?
2. Does this fully resolve F-CAND: counter/weakens-only evidence remains auditable but no longer creates promoted causal candidates?
3. Is the updated formal design policy for counter-evidence clear enough for future slices?
4. Are the re-baselined scores now defensible enough to build slice 5 regression grouping and `--fail-on-regression` on top of them?
5. For slice 5, should `--fail-on-regression` gate on per-scenario `required_average`, per-required-metric deltas, trap/missing-data subgroup deltas, or a combination?
6. Should the persistent raw wins on `traffic-shift-hotspot` and `missing-data-gap` be explicitly exempted as known current regressions, or should they become expected failing cases under `--fail-on-regression` until Janus improves?

## Verification

Commands run successfully on baseline `10bebaf142f5571593293d6abdb94ddaba5c4974`:

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
- `cargo run --bin compare_evidence_access -- --all` recorded `repo_sha: 10bebaf142f5571593293d6abdb94ddaba5c4974`.
- Aggregate required average: Janus `0.877`, raw `0.828`, delta `0.049`.
- Scenario winners: Janus won 10, raw won 2 (`traffic-shift-hotspot`, `missing-data-gap`).
- The two false-causality trap fixtures both scored as Janus wins: `retry-storm-amplification` (`0.898` vs `0.859`) and `coincidental-deploy-trap` (`0.949` vs `0.770`).
- `git check-ignore` confirmed `target/eval/comparative-eval-v1.json` is ignored through `target/`.

The covered implementation was committed and pushed first as `10bebaf Fix comparative eval counter evidence baseline`; this review document's baseline SHA points to that pre-review-document tree and must remain frozen.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
