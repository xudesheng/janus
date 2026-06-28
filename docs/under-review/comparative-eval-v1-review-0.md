# Comparative Eval V1 Review 0

- Baseline SHA: `9da1f1f0a73ce5ba2d5b1f364f1b4f3ca7261dd6`
- Current milestone: Milestone 8 Comparative Eval V1, a reviewed local harness that compares raw telemetry access with Janus Evidence IR access over the current fixture corpus under the same budget.
- Critical path: yes - this design approval is required before any Rust implementation, and the topic is the roadmap's first direct test of the central Janus evidence-quality claim.
- Milestone progress: submits the enriched `docs/core/comparative-eval-v1.md` design for review, including explicit approval and completion policy language, and asks reviewers to decide the V1 evaluator, raw baseline, scoring, reporting, and implementation-gate policy.
- Deferred milestone work: all Rust implementation, CLI work, adapters, scoring/report code, generated reports, and tests are deferred until all active reviewers agree on the design direction in their `Direction Verdict` or explicitly approve a named implementation slice.

This is the first review round for `comparative-eval-v1`; there are no prior review findings to answer.

I read `docs/core/comparative-eval-v1.md` against the core Janus context and the milestone chain in `docs/core/what_and_why.md`, `docs/core/roadmap.md`, `docs/core/evidence-ir-schema.md`, `docs/process/fixture-validation-harness.md`, `docs/core/derived-context-baseline.md`, `docs/core/evidence-compiler-ranking.md`, and `docs/core/mcp-agent-surface.md`.

I made one formal design edit before this review document: `docs/core/comparative-eval-v1.md` now has an `Approval And Completion Policy` section. That section makes the no-coding gate explicit, prefers whole-design approval unless reviewers explicitly approve a named slice, and separates harness correctness from the policy question of whether V1 completion must show a Janus metric improvement.

No Rust code, fixture data, schemas, generated reports, or command-line surfaces were changed in this round.

## Direction Request

Reviewers should first decide whether `comparative-eval-v1` is the right Milestone 8 topic after `mcp-agent-surface`, and whether the design is ready to govern implementation.

The direction verdict should say one of:

- continue: the topic is on the critical path and the design is ready for implementation, either whole-topic or phase-by-phase;
- redirect: the topic is premature, too broad, unfair to the raw baseline, or pointed at the wrong eval shape;
- stop: the milestone should not proceed in this form.

If the verdict is `continue` but only for a phase, please name the approved phase explicitly. Without that explicit slice approval from every active reviewer, I will treat the gate as whole-design approval only and will not start coding.

## Review Focus

Please focus on these decisions before local implementation details:

1. Topic fit: is this the right next topic after the MCP agent surface, or should another contract/harness gap come first?
2. Evaluator shape: should V1 be deterministic and local, or does the first credible comparison require an LLM judge or agent-in-the-loop run now?
3. Raw baseline fairness: are the allowed raw selectors competitive enough without using Janus derived context, expected artifacts, ground truth, or fixture-specific shortcuts?
4. Gold boundary: are the tests and design rules strong enough to prove `scenario.json.ground_truth` and `expected.json` are scoring oracles only?
5. Metrics: which dimensions are required for completion versus report-only - suspicious-entity accuracy, timeline quality, false-causality risk, missing-data awareness, auditability, and token efficiency?
6. Completion policy: should V1 require Janus to improve at least one target metric, or is an honest harness/report sufficient even if Janus does not beat raw access yet?
7. Regression policy: should the default CLI fail only on harness/schema/runtime failures, with `--fail-on-regression` as an opt-in stricter mode, or should regression failure be the default?
8. Token budget: is measuring tokens from comparable serialized payload bytes acceptable, and are the raw and Janus envelopes comparable enough?
9. Future agent compatibility: is the normalized `EvalSubmission`/report shape sufficient for a later MCP or agent-in-the-loop adapter without making V1 depend on MCP protocol mechanics?
10. Phase strategy: should implementation wait for whole-design approval, or should reviewers approve specific slices such as models/report schema first?

## Current Recommendation

My recommendation is to continue with a deterministic local evaluator first, using all current fixtures by default and no LLM judge in V1. The raw baseline should be strong enough to retrieve obvious raw symptoms through time, entity, severity, change proximity, metric deltas, and trace/log selectors, but it should not perform Janus's cross-signal derived-context reasoning.

I recommend the completion policy added to the design: harness correctness is mandatory, the report must expose Janus wins and regressions honestly, and the first completed run should show at least one roadmap target-metric improvement without hiding false-causality or auditability failures. The default command should fail on harness failures, while `--fail-on-regression` can enforce a stricter metric gate for CI or a later approved release check.

If reviewers disagree on the evaluator shape, raw baseline, or completion policy, the next round should stay design-only rather than starting implementation.

## Verification

No code verification this round. This is a design-only review submission.

Repository checks performed:

- `git branch --show-current` returned `comparative-eval-v1`.
- `git status --short --branch` showed the branch tracking `origin/comparative-eval-v1` and a clean worktree after the formal design commit was pushed, before this review document was created.
- `git rev-parse HEAD` and `git rev-parse "@{u}"` both returned `9da1f1f0a73ce5ba2d5b1f364f1b4f3ca7261dd6` before this review document was created.
- `docs/under-review/` had no current same-topic review files, so this round is `comparative-eval-v1-review-0.md`.
- The covered formal design change was committed and pushed first as `9da1f1f Clarify comparative eval design gate`; this review document's baseline SHA points to that pre-review-document tree and must remain frozen.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
