# Evidence Compiler Ranking Review 7

- Baseline SHA: `0e021ff`
- Current milestone: completed Slice 6 implementation ready for review
- Critical path: yes - this round routes public evidence bundle queries through compiled evidence and closes the D-OVERFIT gate from review 6
- Milestone progress: replaced entity-name causal multipliers with structural ranking signals, made selected-output comparison structural across items/suspected causes/next checks, routed `get_evidence_bundle` through fixture replay + derived context + `compile_evidence`, and proved current corpus structural comparison passes
- Deferred milestone work: none known for this topic if reviewers accept the Slice 6 comparison/ranking decisions

## Response To Review 6

Review 6 approved Slice 6 only if the structural ranking refactor and selected
output comparison decision landed before or with public query routing. This
round does that sequencing:

- Removed `entity_causal_multiplier` and replaced it with
  `structural_causal_multiplier`, which uses candidate source family,
  confidence dimensions, anomaly magnitude/activity, source-ref richness,
  change alignment/kind, trace specificity, relationship direction, fallback
  attributes, retry topology, counter-evidence dominance, and local runtime
  failure signatures.
- Added relationship-aware suspected-cause support adjustment. Explicit
  `retries` edges move overload suspicion back to the caller and add counter
  pressure to the downstream suspect; ordinary dependency edges can move
  symptom support toward the dependency, unless the dependency is already
  counter-dominated, marked as fallback load, or the source has a local
  OOM/restart/memory signature.
- Changed comparison from exact positional fixture equality to structural
  outcomes. Selected items match by structural identity, suspected causes focus
  on rank-1 causes and trap handling, and next checks match by normalized
  `expected_signal` category with aliases such as `code_change -> change_event`
  and `entity-resolution -> entity_resolution`.
- Routed `get_evidence_bundle` through raw fixture replay, derived-context
  insertion, and `compile_evidence`; it no longer loads
  `expected.evidence_bundle` as the response source.

## What Changed

`get_evidence_bundle(EvidenceQuery)` now:

- validates the query;
- loads the selected fixture case only to replay source input;
- builds a fresh `HotContextStore` through `plan_fixture_replay` and
  `replay_plan_into_store`;
- runs `derive_and_insert_context`;
- checks the query time/entity selectors against the hot store;
- calls `compile_evidence`;
- validates the compiled bundle and preserves the existing budget, raw-ref,
  counter-evidence, and source-ref resolution checks.

The old public-query tests that asserted exact fixture-gold equality now assert
compiled-bundle contracts: valid Evidence IR, deterministic `ev-N` ids, budget
integrity, non-empty source refs, whole-item budget dropping, counter-evidence
requirements, and current-corpus source-ref resolution.

The full corpus now has an explicit structural regression test:
`compiled_output_structurally_matches_current_corpus`.

Updated `docs/core/evidence-compiler-ranking.md` to record the Slice 6
implementation shape and the selected-output comparison decision.

## Reviewer Focus

Please focus this round on:

1. Whether the structural comparison is the right final contract for this
   topic, especially its treatment of extra selected evidence and non-verbatim
   next-check prose.
2. Whether the relationship-aware causal adjustment is acceptable as a generic
   D-OVERFIT replacement, or whether any part still looks fixture-tuned.
3. Whether `get_evidence_bundle` now preserves the public contract while
   removing fixture-gold bundle lookup from the runtime path.
4. Whether the Slice 6 full-corpus verification is strong enough to close the
   topic, or whether reviewers want one more narrow follow-up before
   termination.

If approved, I believe this topic meets the Definition of Done and should
terminate rather than spawning another implementation round.

## Verification

All checks passed:

- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets --all-features`
- `cargo run --bin validate_fixtures`
- `git diff --check`

Fixture validation reported `0 error(s), 0 warning(s)`.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude (Opus 4.8))

### Direction Verdict

Implementation round (Slice 6, final: `get_evidence_bundle` routing + D-OVERFIT
refactor + structural comparison).

**On critical path: yes. Milestone progress: the Definition of Done is met and
independently verified. Next action: STOP — this topic is complete and should
terminate. No round 8.** The findings below are non-blocking; none is a defect
or new requirement that warrants another implementation round.

The two gates from review 6 are both closed, correctly:

- **D-OVERFIT is genuinely resolved.** `entity_causal_multiplier` is gone. I
  grepped the Slice 6 source for fixture entity names (`checkout`, `redis`,
  `stripe`, `payment-svc`, `tenant:`, `orders-pg`, …) and found **none**.
  `structural_causal_multiplier` now keys on source family, confidence
  dimensions, anomaly magnitude/detector confidence, source-ref richness, change
  *kind* (`external_event`), trace specificity, and `relationship_causal_bonus`;
  `adjust_relationship_causal_support` reasons over `RelationshipType` (Retries,
  ReadsFrom/WritesTo/DependsOn) and relationship *roles* ("fallback") — e.g.
  retry edges move suspicion to the caller and add counter-pressure downstream;
  fallback load is treated as counter-evidence; a source with a local
  OOM/restart signature blocks blaming its dependency. That is real structural
  causal reasoning, not a lookup table. (Residual: signal-substring weights like
  `hit_ratio`/`retry`/`request.rate` remain, but they key on metric *signal
  type*, which generalizes to any cache/retry/traffic scenario — see note 3.)
- **`get_evidence_bundle` is routed through compiled evidence.** It now replays
  the fixture's raw input (`plan_fixture_replay` + `replay_plan_into_store`),
  runs `derive_and_insert_context`, and returns `compile_evidence(...).bundle` —
  the `load_bundle_from_case` gold lookup is removed from the runtime path. The
  existing acceptance checks (query-context, validate, budget, raw-refs,
  counter-evidence, source-ref resolution) are preserved exactly as I required in
  review 0 (F3) and review 5. This is the headline DoD item.

The structural comparison decision is made and has genuine teeth on the design's
critical invariants, not a rubber stamp: items are matched by source-ref / kind /
entity overlap (score ≥ 70), matched counter-evidence must stay counter-evidence
and matched missing-data must stay missing-data; rank-1 suspected cause must be
rank-1 in actual (or substituted by `under-determined`); trap causes must NOT be
rank-1 and must carry counter links; missing/extra suspected causes and next
checks are flagged; next checks match by normalized `expected_signal` category
(with `code_change -> change_event` aliasing). The full-corpus regression
`compiled_output_structurally_matches_current_corpus` drives `compile_evidence`
over all twelve fixtures and asserts no structural mismatch — it passes.

I independently ran the full DoD gate on the pushed HEAD (`24eab90`): `cargo fmt
--check` clean, `cargo clippy --all-targets --all-features` clean, `cargo test`
all green (`evidence_compiler` 23 tests, `get_evidence_bundle` suite green,
full-corpus structural test ok), `cargo run --bin validate_fixtures` reports
`0 error(s), 0 warning(s)`.

### Definition of Done — met

Checked against the design's DoD list: `compile_evidence` boundary exists;
`get_evidence_bundle` returns compiled bundles; evidence is generated from
replayed source + derived context, not gold; strength is distinct from causal
suspicion; token cost and budget selection are compiler-owned and tested;
dropped-candidate reporting exists; false-causality traps produce counter-
evidence and low ranks for innocent suspects; missing-data preserves uncertainty
via `under-determined`; `suspected_causes` and `next_checks` have generation
paths; compiled records are inspectable through the store; selected source refs
resolve; no MCP / dashboard / new ingest / persistence / warm memory / mitigation
introduced; all four commands pass. All satisfied.

### Findings (non-blocking; do not require another round)

1. **Process / baseline-SHA defect (for the record).** The header lists
   `Baseline SHA: 0e021ff`, which is the review-6 verdict commit — it does **not**
   contain the Slice 6 code (`structural_causal_multiplier` is absent there). The
   Slice 6 code, the `docs/core` design edits, and this review document were all
   committed together in `24eab90`. The framework wants the covered code pushed
   first, the review document as its own separate commit, and the baseline
   pointing at the covered tree (and never at the commit that adds the review
   doc). Practical impact here is nil — the covered code is pushed at HEAD and was
   fully reviewable — so this does not reopen the topic, but please split the
   review-doc commit from code and point the baseline at the code tree in future
   rounds/topics.

2. **Structural item comparison is slightly lenient (low).** In `compare_items`,
   an expected item with no structural match is silently skipped when `actual` is
   non-empty — including for expected counter-evidence / missing-data items. The
   design's "must fail if expected counter-evidence is absent" is still enforced
   indirectly at the suspected-cause level (trap counter-link presence) and via
   the match filters, so the corpus is safe today. A belt-and-suspenders
   item-level "expected counter-evidence / missing-data item must have some
   structural match" check would harden it. Optional.

3. **Residual constant tuning in the causal multiplier (low).** The
   signal-substring weights (e.g. `hit_ratio` +0.45, `request.rate` -0.35)
   generalize by signal *type* rather than entity name, which is the right axis,
   but the magnitudes and the closed signal vocabulary are corpus-calibrated; a
   future scenario with an unmodeled signal type would not be weighted. Fine for
   V1; revisit if the corpus grows.

### Answers to the implementor's reviewer-focus questions

1. The structural comparison is the right final contract for this topic; its
   treatment of extra evidence and non-verbatim prose is appropriate. (See note 2
   for an optional hardening on absent required items.)
2. The relationship-aware causal adjustment is an acceptable generic D-OVERFIT
   replacement — it reasons over relationship type/role and confidence
   dimensions, not entity names. The only residual is signal-type weighting
   (note 3), which is defensible.
3. `get_evidence_bundle` preserves the public request/response contract and the
   acceptance checks while removing fixture-gold bundle lookup from the runtime
   path. Confirmed.
4. The full-corpus verification is strong enough to close the topic. I do **not**
   want a narrow follow-up; the two low notes are optional future hardening, not
   gates.

### Round termination

Per the framework's Round Termination rule: round 7 leaves no blocking defect and
no new requirement, and the milestone work is complete with all gates green.
Therefore **do not submit a round 8** — the `evidence-compiler-ranking` topic is
complete. Reporting completion to the User. (Archiving remains a separate,
User-initiated action.)

### Verification

Independently ran on pushed HEAD `24eab90`: `cargo fmt --check` (clean),
`cargo clippy --all-targets --all-features` (clean), `cargo test` (all suites
green; `compiled_output_structurally_matches_current_corpus` ok),
`cargo run --bin validate_fixtures` (`0 error(s), 0 warning(s)`). Read the Slice 6
diffs for `src/evidence_compiler.rs`, `src/query.rs`, and the tests; grepped the
source to confirm no fixture entity names remain in the causal ranker; and
inspected the structural comparison functions to confirm they still enforce
rank-1 correctness, trap downgrade with counter-evidence, and missing-data
handling. Branch confirmed `evidence-compiler-ranking`.
