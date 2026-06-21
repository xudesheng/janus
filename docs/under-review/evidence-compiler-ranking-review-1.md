# Evidence Compiler Ranking Review 1

- Baseline SHA: `738770bd231abf7de05c4b589a6b1128a52e7945`
- Current milestone: reviewer-approved Evidence Compiler V1 design in `docs/core/evidence-compiler-ranking.md`, specifically approval to begin Slice 1 without unresolved token, comparison, or query-transition contradictions
- Critical path: yes - review 0 approved the direction only for Slice 1 and made F1-F5 actionable design feedback before broader implementation
- Milestone progress: updated the formal design doc to resolve review 0 findings F1-F5 and to clarify the exact decisions reviewers must accept before Slice 1 starts
- Deferred milestone work: Rust implementation, fixture token-field migration, compiler model code, and comparison-shell tests are deferred until all active reviewers agree through Direction Verdicts that the updated design is sufficient to start Slice 1

This is a design-only response to `evidence-compiler-ranking-review-0.md`. I did
not start Rust implementation.

## Response To Review 0

F1, token-cost contradiction:

- Resolved in `docs/core/evidence-compiler-ranking.md` by choosing estimator-
  owned fixture regeneration rather than exact comparison to the current
  hand-authored token numbers.
- The design now defines the V1 estimator as
  `ceil(canonical_evidence_item_payload_json_bytes / 4)`.
- The canonical payload is compact JSON with stable field order, sorted map
  keys, compiler-selected array order, deterministic number formatting, and
  `token_cost` omitted to avoid self-reference.
- `EvidenceBudget.tokens_used` is now defined as the sum of selected item
  `token_cost` values. Bundle envelope fields are not counted in the V1
  selection budget.
- Slice 1 now explicitly owns the fixture token-field migration needed before
  token fields become exact comparison targets.

F2, exact-match feasibility for hand-authored free text:

- Resolved by adding field-family comparison modes.
- The design now treats natural-language fields as structural text by default:
  deterministic, non-empty where required, and anchored to compared entities,
  source refs, evidence ids, or reason/check categories.
- Verbatim equality with current hand-authored prose is not required unless a
  later reviewed slice introduces compiler-owned templates and migrates
  fixtures to those templates.

F3, `get_evidence_bundle` transition accuracy:

- Resolved by reframing the transition as preserving the existing acceptance
  checks and swapping only the bundle source from fixture gold to compiled
  evidence.
- Slice 6 now explicitly keeps query validation, budget, raw-ref,
  counter-evidence, source-ref, and query-context checks.

F4, `SuspectedCause` / `NextCheck` and `EvidenceQuery` accuracy:

- Resolved by clarifying that only `StoredRecordKind::SuspectedCause` and
  `StoredRecordKind::NextCheck` exist today.
- The runtime structs, gold parsers, comparison helpers, and store payload
  projections are now called out as Slice 1 work.
- The input section now names the nested `EvidenceQueryIntent` question and/or
  hypothesis and treats `scenario_id` as a fixture adapter rather than a
  production query primitive.

F5, deterministic serialization:

- Resolved in the token estimator definition by requiring compact canonical
  JSON, stable field order, sorted map keys, compiler-selected array order, and
  deterministic number formatting.

## What Changed

Updated `docs/core/evidence-compiler-ranking.md` only. The changes:

- clarified compiler inputs and the role of `EvidenceQuery.scenario_id`;
- corrected the `get_evidence_bundle` transition plan;
- documented that suspected-cause and next-check runtime models are new Slice 1
  work;
- replaced the ambiguous token-cost language with a deterministic estimator and
  exact `tokens_used` rule;
- added explicit comparison modes for exact, structural, numeric-tolerance,
  estimator-owned, and text-structural fields;
- expanded Slice 1 and Slice 6 responsibilities to reflect those decisions.

## Reviewer Focus

Please focus this round on:

1. Whether the token-cost resolution is acceptable: estimator-owned fixture
   regeneration, canonical item payload bytes divided by four, and
   `tokens_used = sum(selected token_cost)`.
2. Whether text-structural comparison is the right default for `claim`,
   suspected-cause prose, and next-check prose, or whether Slice 1 must instead
   introduce compiler-owned templates and migrate fixtures to exact text.
3. Whether the updated comparison modes are precise enough for Slice 1 to build
   the comparison shell without leaking gold into runtime compilation.
4. Whether the updated `get_evidence_bundle` plan correctly preserves existing
   acceptance checks while replacing the gold bundle source.
5. Whether F1-F5 from review 0 are now sufficiently resolved for Slice 1
   implementation to begin after all active reviewers agree.

If implementation should proceed, please say so explicitly in the Direction
Verdict and name the approved scope. The expected next implementation scope is
Slice 1 only: compiler model, suspected-cause and next-check runtime types,
comparison shell, comparison modes, token estimator, errors, proof that gold is
comparison-only, and the fixture token-field migration.

## Verification

No code verification this design-only round.

Checks performed:

- read `docs/core/evidence-compiler-ranking.md`;
- read `docs/under-review/evidence-compiler-ranking-review-0.md`;
- inspected `src/query.rs`, `src/evidence.rs`, `src/hot_context_store.rs`, and
  `fixtures/scenarios/deploy-bad-rollout/expected.json` to verify review 0's
  factual claims;
- ran `git diff --check`;
- confirmed the active branch is `evidence-compiler-ranking`.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
