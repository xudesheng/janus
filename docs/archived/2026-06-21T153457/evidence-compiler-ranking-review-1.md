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

## Review (by Claude (Opus 4.8))

### Direction Verdict

Design-only round, so the verdict is about the updated design in
`docs/core/evidence-compiler-ranking.md`.

**On critical path: yes. Resolves the round-0 blockers: yes. Next action:
continue — I AGREE the updated design is sufficient to start, and I approve
Slice 1 as scoped.**

I checked the design doc as it stands at the round-1 baseline
(`738770b`, diffed against round 0 `0555a20`) and confirmed F1–F5 are genuinely
resolved, not merely acknowledged:

- **F1 (token contradiction): resolved.** The doc now picks estimator-owned
  fixture regeneration, defines `tokens_used = sum(selected item token_cost)`
  with the bundle envelope excluded, and omits `token_cost` from the estimator
  payload to break self-reference. This dissolves the contradiction I raised:
  the `deploy-bad-rollout` `250 vs 235` gap disappears because the gold token
  fields are regenerated from the estimator (sum = 235), and the estimator is no
  longer required to match the old hand-authored numbers. Framing the migration
  as "estimator-driven regeneration, not gold-as-input" is the right call.
- **F2 (free-text): resolved.** The new per-field-family comparison table
  (Exact / Set-or-ordered-structural / Numeric-tolerance / Estimator-owned /
  Text-structural) is exactly what I asked for, and text-structural-by-default
  for `claim`/prose is the correct default for V1.
- **F3 (`get_evidence_bundle`): resolved.** The doc now states the existing
  acceptance checks and scopes Slice 6 to preserve them while swapping only the
  bundle source, with the temporary gold path named and removed/quarantined by
  slice end.
- **F4 (net-new types): resolved.** Runtime `SuspectedCause`/`NextCheck`
  structs, gold parsers, comparison helpers, and store projections are now
  called out as Slice 1 work; nested `EvidenceQueryIntent` and `scenario_id` as
  a fixture adapter are stated.
- **F5 (determinism): resolved** via the canonical compact-JSON definition
  (stable field order, sorted keys, compiler-selected array order, deterministic
  number formatting).

**Approved scope:** Slice 1 only, as the implementor named it — compiler model,
suspected-cause and next-check runtime types, comparison shell, the comparison
modes, the token estimator, errors, the proof that gold is comparison-only, and
the fixture token-field migration. Slices 2–6 remain unapproved and unblock as
their prerequisites land; this is consistent with the design's own gate. As one
reviewer I agree; per that gate, coding begins once all active reviewers
(including the User) concur.

The three notes below are **non-blocking** clarifications for Slice 1 to settle
inside the comparison shell. None of them gates my agreement; I am flagging them
so the comparison oracle is unambiguous before slices 2–5 build against it.

### N1 — Pin the evidence-item id scheme (Slice 1, non-blocking)

The comparison table lists "selected item ids and ordering" as **Exact**, but
gold ids are the literal sequence `ev-1 … ev-5`. "Exact id match" only works if
the compiler either adopts that `ev-N` scheme or the ids are migrated/normalized
like the token fields. A content-hash id scheme would never equal `ev-1`. Slice 1
should state which: compiler adopts the gold id convention, or ids join the
fixture migration, or comparison maps ids positionally. This is the same
"migrate the fixture vs. match the fixture" decision F1 already made for token
fields — apply it deliberately to ids rather than leaving it implicit.

### N2 — Disambiguate `reasons` comparison (Slice 1, non-blocking)

Suspected-cause `reasons` appears under both "Set or ordered-structural" and
"Text structural." In the gold they are category tokens
(`time_alignment`, `change_proximity`, `error_signature`,
`downstream_db_healthy`), not prose. Those read as an enum/category set, so
exact-token set comparison is the right model, not free-text-structural. Slice 1
should classify `reasons` as a category set (with a defined vocabulary) and
reserve text-structural for genuinely free-form fields (`claim`, `note`,
`trap_note`, next-check `action`/`rationale`).

### N3 — Float formatting in the canonical payload (Slice 1, nit)

"Deterministic number formatting from the chosen JSON serializer" is fine as
long as one serializer is fixed and exercised in tests; serde_json is
deterministic but its float rendering (e.g. trailing-zero handling) is an
implementation detail the byte count depends on. Slice 1's estimator test should
lock the exact serializer/config so the byte count — and therefore every
`token_cost` — can't drift under a dependency bump.

### Forward note (not a blocker)

With token and text tensions removed, the real difficulty now lands squarely in
slices 2–5: the compiler's scoring, selection, and ordering must reproduce
gold's *selected set and order* exactly (the Exact family). That coupling is the
intended acceptance bar — where the compiler and gold disagree, it surfaces a
real defect in one or the other — so I endorse keeping it exact. I just want it
on record that this, not the token math, is where the corpus will be won or
lost, and the six-slice plan should not be collapsed to rush it.

### Verification

No code verification this design-only round. I diffed
`docs/core/evidence-compiler-ranking.md` between `0555a20` (round-0 baseline)
and `738770b` (round-1 baseline) to confirm each F1–F5 resolution, and
re-checked `fixtures/scenarios/deploy-bad-rollout/expected.json` to confirm the
`235 vs 250` gap is the one the new `tokens_used` rule closes. Branch confirmed
`evidence-compiler-ranking`.
