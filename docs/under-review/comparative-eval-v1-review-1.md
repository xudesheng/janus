# Comparative Eval V1 Review 1

- Baseline SHA: `4cfdb7a43627c2a7042f479aab02629c72a86349`
- Current milestone: Milestone 8 Comparative Eval V1, a reviewed local harness that compares raw telemetry access with Janus Evidence IR access over the current fixture corpus under the same budget.
- Critical path: yes - review-0 agreed with the direction but left actionable design refinements that should be closed before the design gate is treated as ready for implementation.
- Milestone progress: updates `docs/core/comparative-eval-v1.md` to resolve review-0 findings F1-F6, preserving the roadmap metric-improvement bar, pinning fixture versions in the report schema, constraining raw dependency grouping, defining required versus report-only metrics, requiring one shared token estimator, and making the gold/runtime boundary structural.
- Deferred milestone work: all Rust implementation, CLI work, adapters, scoring/report code, generated reports, and tests remain deferred until every active reviewer agrees in their `Direction Verdict` that the design is ready to govern implementation or explicitly approves a named implementation slice.

## Response To Review 0

Review-0's `Direction Verdict` from Claude was `continue`, with whole-design approval, but it also left six refinements. I treated those as actionable design feedback and kept this round design-only.

Changes made in `docs/core/comparative-eval-v1.md`:

1. F1, completion policy and roadmap: the design now states that the roadmap's Milestone 8 bar still applies. V1 must show Janus improving at least one target metric without hiding regressions. A "harness only" completion would require a roadmap update in the same covered formal-doc tree.
2. F2, fixture-version pinning: the report schema now records the fixture registry `schema_version` and each scenario's `schema_version` and `version`, in addition to `repo_sha`.
3. F3, raw dependency grouping: the raw baseline may group only on dependency links directly visible in raw records, such as span parent/child structure, `peer.service`, `db.system`, call attributes, or a shared trace id. It must not infer a relationship graph or reuse Janus relationship records.
4. F4, required versus report-only metrics: the design now marks suspicious-entity accuracy, false-causality risk, missing-data awareness, auditability, and token efficiency as required V1 metrics. Timeline quality remains structural and report-only in V1.
5. F5, token measurement: both adapters must call one shared compact-JSON serialization and token-estimation helper, with comparable envelope-inclusion rules.
6. F6, gold boundary: the design now requires runtime adapters to be unable to access the scoring-oracle loader where Rust module visibility allows it; tests remain a second guard.

I also propagated those decisions into the implementation slices, tests, Definition of Done, and Review Focus sections so they are not just prose in one place.

No Rust code, fixture data, schemas, generated reports, or command-line surfaces were changed in this round.

## Direction Request

Reviewers should decide whether these refinements close the actionable design feedback from review-0 and whether the design gate is now ready to open for implementation.

The direction verdict should say one of:

- continue: the design is agreed and implementation may start, either whole-topic or with a named first slice;
- redirect: the design still needs changes before implementation;
- stop: the milestone should not proceed in this form.

If the verdict is `continue` but only for a phase, please name the approved phase explicitly. My recommended first implementation phase, if the gate closes, is slice 1 from the design: eval models, report schema, shared token estimator, required/report-only metric classification, fixture-version fields, and a CLI skeleton that emits an empty-but-valid report.

## Review Focus

Please focus on these decisions:

1. Did the F1 response correctly preserve the roadmap's "improve at least one target metric" acceptance bar?
2. Is the report version pinning sufficient: top-level registry `schema_version`, per-scenario `schema_version`, per-scenario `version`, and `repo_sha`?
3. Is raw dependency grouping now constrained tightly enough to avoid rebuilding Janus's relationship graph?
4. Is the required metric set right for V1, with timeline quality structural/report-only for now?
5. Is one shared serializer/token estimator enough to make raw and Janus token measurements comparable?
6. Is the scoring-oracle boundary strong enough if implemented with module visibility plus tests?
7. If the design is agreed, should implementation start with slice 1 only, or may slices 1-6 proceed in order under the whole design?

## Verification

No code verification this round. This is a design-only review submission.

Repository checks performed:

- `git fetch origin comparative-eval-v1` completed before reading review-0.
- `git branch --show-current` returned `comparative-eval-v1`.
- `git status --short --branch` showed the branch tracking `origin/comparative-eval-v1` and a clean worktree before this review document was created.
- `git rev-parse HEAD` and `git rev-parse "@{u}"` both returned `4cfdb7a43627c2a7042f479aab02629c72a86349` before this review document was created.
- `docs/under-review/` contained `comparative-eval-v1-review-0.md`, so this round is `comparative-eval-v1-review-1.md`.
- `git diff --check` passed before the formal design commit.
- The covered formal design change was committed and pushed first as `4cfdb7a Refine comparative eval design after review`; this review document's baseline SHA points to that pre-review-document tree and must remain frozen.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
