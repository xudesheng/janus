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

## Review (by Claude)

### Direction Verdict

**AGREE — continue, with whole-design approval.** This is a design-discussion round, so
the gate is agreement, not defect-counting. I agree with the design direction and the
proposed V1 shape. The covered work is on the critical path: `comparative-eval-v1` is
roadmap Milestone 8, the correct next topic after `mcp-agent-surface` (M7), and it is the
roadmap's first direct test of the central Janus bet ("fewer, more accurate, more auditable
evidence under the same budget"). The design is internally coherent, grounded in the real
fixtures and the real Evidence IR types, and ready to govern implementation.

Approval scope: **whole design**, slices 1–6 may proceed in their stated order. I am not
restricting this to a named slice. Two findings below (F1 completion-policy/roadmap, F2
fixture-version pinning) should be resolved in the formal design or answered in review-1
**before the scoring slice (4) and reporting slice (5)**; they do not block slice 1 (models
/report schema) or slices 2–3 (adapters).

Gate caveat: this is one reviewer's agreement. The design's own Design Review Gate requires
**every active reviewer** to agree before Rust starts. My `continue` does not by itself
unblock coding — the Implementor must still collect agreement from all other active
reviewers (and resolve any of their findings) before slice 1.

Next action: **continue.**

### Why I agree (grounded checks)

I verified the design is not hand-wavy:

- **Topic fit / critical path:** roadmap `Milestone 8` is `comparative-eval-v1`, listed #9
  (last) in the near-term review order after `mcp-agent-surface`. Correct sequencing.
- **Oracle fields exist:** `coincidental-deploy-trap/scenario.json` carries
  `false_causality_trap: true`, `ground_truth.primary_cause_entity`, `innocent_suspect`,
  `not_the_cause`, `blast_radius`, `failure_class`, `difficulty`. `missing-data-gap` carries
  `primary_cause_entity: "under-determined"` and `inputs: [... "telemetry_gaps"]`. The
  design's scoring inputs map onto fields that actually exist, including the
  `under-determined` sentinel the missing-data scoring relies on.
- **Required fixtures exist:** all five named ids (`deploy-bad-rollout`,
  `coincidental-deploy-trap`, `retry-storm-amplification`, `ambiguous-entity-resolution`,
  `missing-data-gap`) are present, plus blast-radius scenarios
  (`downstream-outage-cascade`, `dependency-db-degradation`). The corpus is 12 fixtures,
  matching the report example's `fixture_count`.
- **Janus adapter is buildable:** `EvidenceKind::CounterEvidence`/`MissingData` and
  `EvidenceDirection::Weakens`/`Contradicts` exist in `src/evidence.rs`; `source_refs` and
  suspected-cause output exist in the compiler. The planned Evidence IR extraction is real,
  not aspirational.
- **Baseline SHA discipline:** baseline `9da1f1f` is the pre-review-document tree; HEAD
  `4886610` (this review-0 commit) is pushed and tracks `origin/comparative-eval-v1`. The
  baseline correctly points at the prior tree and is frozen. Good.

On the design's explicit gate questions: deterministic local evaluator (no LLM judge) for
V1 — **agree** (matches "evaluation before scale"; avoids prompt drift, keeps the harness
the artifact). Raw baseline fair, not a strawman — **agree** with one refinement (F3).
Default fails on harness/schema/runtime, `--fail-on-regression` opt-in — **agree**.
Token cost from serialized payload bytes / 4 (not fixture gold `token_cost`) — **agree**
with one refinement (F5).

### Findings (refinements, none blocking my agreement)

**F1 — medium — Completion policy must not silently weaken a roadmap acceptance criterion.**
The roadmap (a formal doc, higher milestone-source precedence than this design doc) already
commits Milestone 8 acceptance to "Janus improves at least one target metric without hiding
regressions in others." The design's `Approval And Completion Policy` proposes the same bar
but adds that reviewers "may" make the topic "harness only" even if Janus improves no metric.
That option, if taken, would contradict `roadmap.md` and is therefore not a mere verdict note
— it would require editing the roadmap too. I **accept the proposed policy as written**
(metric-improvement bar required) precisely because it matches the roadmap. Recommendation:
keep the metric bar; if any reviewer wants harness-only, treat it as a roadmap change, not a
review-verdict footnote.

**F2 — medium — Report schema does not yet satisfy "results tied to fixture versions."**
Roadmap M8 acceptance requires results tied to fixture versions; the report schema records
`repo_sha` but no per-fixture version. Each `scenario.json` has `version` and
`schema_version`. The report should pin each scenario's `version` (and the registry
`schema_version`) so a run is reproducible against fixture versions. Add this to the report
schema in slice 1 so it is not retrofitted later.

**F3 — low/medium — Constrain raw-baseline "dependency" grouping to directly-present edges.**
The allowed raw selectors include "simple grouping for records that already share the same
raw entity, service, route, dependency, or trace id." Grouping by *dependency* is borderline:
dependency *direction* is exactly the cross-signal reasoning Janus is supposed to win on
(e.g. `retry-storm-amplification`). Clarify that the baseline may group only on a dependency
edge **directly present on a raw record** (span parent/child, a db-client call attribute),
never an inferred or derived edge. Otherwise the baseline can quietly reconstruct Janus's
relationship graph and the amplification-direction win becomes meaningless.

**F4 — low — Pin required-vs-report-only metrics before the scoring slice.**
The design lists six dimensions but does not crisply split scored-required from report-only.
Recommend: suspicious-entity accuracy, false-causality risk, auditability, missing-data
awareness, and token efficiency are scored/required; timeline quality stays structural and
**report-only in V1** (exact structural timeline scoring is the fuzziest and most overfit-
prone). Lock this before slice 4 so the completion bar (F1) has a concrete metric set.

**F5 — low — Use one shared serializer + token estimator for both paths.**
Comparability of `measured_tokens` depends on both envelopes serializing at the same
granularity. Have both adapters route through a single shared serialize+estimate helper so
the measurement cannot drift between paths (e.g. one path including pretty-printing or extra
envelope fields the other omits).

**F6 — low — Enforce the gold boundary with module visibility, not just a test.**
The design's rule ("if a helper loads expected artifacts for scoring, the runtime adapters
should not be able to access that helper") is the right invariant. Make it structural:
scoring/oracle loading lives in a module the adapter modules cannot import, so the boundary
is a compile-time fact, with the planned test as a second guard.

### Summary

Design is sound, on critical path, and well-grounded. I agree with the direction and grant
whole-design approval for slices 1–6. F1 and F2 should land in the design or review-1 before
the scoring/reporting slices; F3–F6 are refinements for the relevant slices. Coding still
waits on agreement from every other active reviewer per the design's Review Gate.
