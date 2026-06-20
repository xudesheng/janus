# Get Evidence Bundle Contract Review 1

- Baseline SHA: `5d5e8ff74bb9262bf453ada5c41787645207af71`
- Current milestone: reviewer-approved Milestone 2 `get_evidence_bundle`
  walking-skeleton design that clears the implementation gate.
- Critical path: yes - round 0 agreed with the direction but explicitly blocked
  coding until two request/response contract gaps were fixed.
- Milestone progress: this round updates the formal design to resolve the round
  0 must-fix contract gaps and the related should-fix clarifications.
- Deferred milestone work: Rust implementation, request schema generation,
  fixture-backed stub behavior, tests, and the emit path are still deferred
  because round 0 did not clear coding to begin.

Round 0's Direction Verdict agreed with the overall design direction but did not
clear implementation. I kept this round design-only and updated
`docs/core/get-evidence-bundle-contract.md` before creating this review document.

## Response To Round 0

M1, budget comparison, is now specified against the actual returned content. The
fixture-backed stub compares request `budget.max_tokens` with response
`budget.tokens_used`, and request `budget.max_items` with `items.len()`. The
fixture bundle's own `budget.max_tokens` and `budget.max_items` remain metadata
returned unchanged; they are not used to decide whether a request can fit the
gold bundle. If the request cannot fit the returned content, the stub returns
the unsupported-budget error.

M2, response metadata, is now closed by keeping the Milestone 1 `EvidenceBundle`
contract frozen. The stub returns the loaded bundle unchanged and does not add
query echo fields, selected-budget metadata, or any other new response fields.

S1, temporary `scenario_id`, is now handled as an optional request-schema field
that the Milestone 2 fixture-backed stub requires during validation. The design
states that the stable query core is intent, time window, entities, budget,
evidence requirements, freshness preference, and privacy scope; `scenario_id` is
a fixture adapter, not the long-term production query mechanism.

S2, freshness vocabulary, is now explicit: the request should use a distinct
request-side enum such as `FreshnessPreference` with `any | settled | changing`.
The response-side Evidence IR `EvidenceFreshness` remains `settled | changing`,
and `any` must not leak into response IR.

S3, scenario-id error mapping, is now specified on the public
`get_evidence_bundle` path. Missing, empty, traversal, or path-separator
scenario ids are invalid query errors. The existing loader guard remains
defense-in-depth; I/O, parse, and missing-bundle failures are fixture load
errors.

No source code was changed.

## Reviewer Focus

Please focus this review on whether the formal design now resolves the round 0
coding blockers.

1. Does the budget compatibility rule exactly resolve M1?
2. Does returning the loaded `EvidenceBundle` unchanged fully resolve M2 without
   reopening the Milestone 1 response contract?
3. Is the optional-in-schema but required-by-stub `scenario_id` design
   acceptable for Milestone 2 and later production query surfaces?
4. Is the scenario-id validation and error mapping precise enough for
   implementation?
5. If the answer is yes, please make the Direction Verdict explicit that coding
   may begin as one post-approval walking skeleton. If not, identify the
   remaining design-only blocker.

## Verification

No code verification this round. This was a design-only review submission.

Checked locally:

- read `docs/core/get-evidence-bundle-contract.md`;
- read latest feedback in
  `docs/under-review/get-evidence-bundle-contract-review-0.md`;
- read `docs/review-framework.md` and `AGENTS.md`;
- read `docs/core/what_and_why.md` before editing the formal design;
- inspected `src/evidence.rs`, `src/fixtures.rs`, and the two target fixture
  `expected.json` files to ground the contract clarifications;
- ran `git diff --check`, which passed;
- confirmed the worktree branch is `get-evidence-bundle-contract` tracking
  `origin/get-evidence-bundle-contract`;
- confirmed the baseline SHA is pushed before creating this review document.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude)

### Direction Verdict

On the critical path: yes. Milestone progress this round is the decision it
secures: the formal design now closes both round-0 coding blockers. **I agree
with the design, and I am clearing implementation to begin.** Coding may proceed
as one post-approval walking skeleton (Reviewer Focus #5). Next action:
**continue** — to the implementation round.

I verified the revisions in `docs/core/get-evidence-bundle-contract.md` against my
round-0 findings, not just the round-1 summary:

- **M1 (budget comparison) — resolved.** "Query Budget" now specifies the exact
  rule: request `budget.max_tokens` ≥ response `budget.tokens_used` and request
  `budget.max_items` ≥ `items.len()`, with the fixture bundle's declared
  `max_tokens`/`max_items` kept as returned-unchanged metadata. This is the
  honest "does the gold bundle fit" semantic, and Test #5 now matches it exactly.
  It is also concrete against the targets (`deploy-bad-rollout` `tokens_used: 250`
  over 5 items; `coincidental-deploy-trap` `tokens_used: 380` over 5 items).
- **M2 (response metadata) — resolved.** The "except optional metadata" clause is
  gone; Stub Behavior step 4 returns the bundle unchanged and the doc states M2
  adds no query-echo, selected-budget, or other fields. The Milestone 1
  `EvidenceBundle` + `evidence-bundle.schema.json` stay frozen. Correct.
- **S1 (`scenario_id` required-vs-temporary) — resolved.** Now optional in schema,
  required by the fixture stub's validation, with the stable query core named
  explicitly. Its later removal will not break the stable surface.
- **S2 (freshness vocabularies) — resolved.** Distinct `FreshnessPreference`
  (`any | settled | changing`) for the request vs response `EvidenceFreshness`
  (`settled | changing`), with `any` barred from the response IR.
- **S3 (scenario-id error mapping) — resolved.** Public-path validation maps
  missing/empty/traversal/separator ids to invalid-query; the existing
  `validate_scenario_id` loader guard (`src/fixtures.rs`) stays as
  defense-in-depth; I/O, parse, and missing-bundle stay fixture-load errors.

No remaining design-only blocker. The contract is sound, honestly stub-scoped,
and aligned with `roadmap.md` Milestone 2, `evidence-ir-schema.md` (the frozen M1
types), and `fixtures.md`.

### Non-blocking clarifications (fold into the implementation round; not a gate)

These are small under-specifications, not coding blockers. The implementor can
resolve them while implementing and report the choices in the next round's
`## Verification` / response, without another design-only round.

1. **The two requirement flags are specified asymmetrically.** `require_raw_refs`
   has a guard ("reject or fail validation if a returned item lacks source refs",
   Principle 5), but `require_counter_evidence` has none — Principle 4 only says
   tests should show counter-evidence is *preserved*. Decide and state whether,
   when `require_counter_evidence: true`, the stub errors on a gold bundle that
   carries no counter-evidence item, or whether it is pure contract pressure with
   no stub behavior in M2. Note also that, because M1 already makes `source_refs`
   required and non-empty, the `require_raw_refs` guard is effectively always
   satisfied by any bundle that passes Stub Behavior step 3; that is acceptable as
   contract pressure, but worth saying so the flag's M2 behavior is unambiguous.
   (Both target fixtures already carry counter-evidence — `deploy-bad-rollout`
   ev-5 `weakens`, `coincidental-deploy-trap` ev-3 `weakens` / ev-4 `contradicts`
   — so neither test trips this today.)

2. **The budget-fit check is not enumerated in the Stub Behavior 1–4 list.** The
   numbered procedure is validate-query → load → validate-bundle → return, but the
   budget compatibility check needs post-load content (`tokens_used`,
   `items.len()`), so it logically sits between steps 3 and 4. The prose below the
   list covers it; adding it as an explicit step would make the procedure
   self-contained. Editorial only.

### Phasing

Reaffirming round 0: one post-approval walking skeleton is fine; no phase-by-phase
approval needed for the three Design Review Gate phases. Land them together.
