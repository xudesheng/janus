# Evidence Compiler Ranking Review 5

- Baseline SHA: `bcb13dc57c45164de63499d4d62625a4529d8483`
- Current milestone: completed Evidence Compiler Slice 4 token-budget selection implementation ready for review
- Critical path: yes - token-budget selection is required before final compiled evidence can be inserted into the store or routed through `get_evidence_bundle`
- Milestone progress: added internal `compile_evidence` and `select_evidence_compilation` paths that select whole scored candidates under item and token budgets, force required counter-evidence, assign final `ev-N` ids, remap selected suspected-cause links, report dropped `cand-*` candidates, and record the Slice 4 and D-OVERFIT decisions in the formal design
- Deferred milestone work: next-check generation, compiled-record store insertion, and `get_evidence_bundle` integration remain deferred because review 4 approved Slice 4 only; the structural causal-ranking refactor for D-OVERFIT remains required before suspected causes become gold-gated in Slice 6

This round implements only the approved Slice 4 scope. It does not generate next
checks, insert compiled evidence records, or route public bundle queries through
the compiler.

## Response To Review 4

F1 was already discharged by review 4 and required no further action.

D-OVERFIT, suspected-cause ranking and reasons looked too fixture-shaped:

- Adopted the reviewer's recommended comparison-shell change. Suspected-cause
  `reasons` are now accepted as a structural non-empty subset of the expected
  derivable category vocabulary instead of requiring exact reproduction of every
  hand-authored reason token.
- Added a test that accepts an expected reason subset and kept a negative test
  that rejects unknown reason tokens.
- Updated `docs/core/evidence-compiler-ranking.md` to state that suspected-cause
  ranking should be judged by structural outcomes and that entity-name
  multipliers must be replaced by structural signals before Slice 6
  gold-gated suspected-cause comparison.
- Did not add new causal-ranking heuristics in this round. That would expand
  beyond the Slice 4 selector scope and should be reviewed as its own targeted
  change before Slice 6.

Smaller review notes:

- Removed the semantically shadowed `ChangeEvent` alternative from the guarded
  `apply_evidence_strength_score` arm; explicit `ChangeEvent` dispatch now owns
  that source family.
- The warning about free-text ranking inputs is recorded under D-OVERFIT. This
  round did not expand free-text dependence, but the structural causal-ranking
  refactor should keep moving toward structured fields before final comparison.

## What Changed

Added `compile_evidence(query, store, derived)` as the internal compiler path
from query plus store plus derived context to selected compilation output.

Added `select_evidence_compilation(input, candidates, suspected_causes)` for
Slice 4 selection. The selector:

- starts from scored `cand-*` candidates;
- reserves `reserve_tokens_for_raw_refs` from `max_tokens`;
- selects whole items only under `max_items` and the effective token limit;
- forces `require_counter_evidence` and `min_counter_evidence_items`, returning
  `EvidenceCompileError::RequirementUnsatisfied` when the budget cannot satisfy
  the requirement;
- prioritizes requested counter-evidence, top-cause support, then deterministic
  fill order;
- assigns selected bundle ids as `ev-1`, `ev-2`, and so on;
- recomputes token costs after final id assignment;
- remaps selected suspected-cause `supporting` and `counter` links from
  `cand-*` ids to selected `ev-*` ids;
- reports generated, selected, and dropped candidates, including stable
  drop reasons for unselected `cand-*` candidates.

Updated tests to cover:

- final selected `ev-N` id assignment;
- source-ref resolution and bundle validation on selected output;
- token-budget whole-item dropping;
- dropped-candidate reporting;
- required counter-evidence priority under tight item budget;
- requirement failure when no counter-evidence item can fit the token budget;
- suspected-cause reason subset comparison for D-OVERFIT.

Updated `docs/core/evidence-compiler-ranking.md` to record:

- the Slice 4 selector contract;
- the continued deferral of next checks, store insertion, and
  `get_evidence_bundle` routing;
- the D-OVERFIT direction decision and the suspected-cause reason comparison
  contract.

## Reviewer Focus

Please focus this round on:

1. Whether `compile_evidence` and `select_evidence_compilation` are the right
   internal Slice 4 boundaries.
2. Whether the selection priority is acceptable: forced counters first,
   top-cause support next, then deterministic fill.
3. Whether the budget behavior is correct enough for this stage: whole-item
   selection, raw-ref reserve handling, token recomputation after `ev-N`
   assignment, and dropped-candidate reporting.
4. Whether counter-evidence enforcement should fail hard with
   `RequirementUnsatisfied` when the requirement cannot fit, rather than
   returning a partial bundle plus report failure.
5. Whether the D-OVERFIT mitigation in this round is enough to proceed to
   Slice 5, or whether the structural causal-ranking refactor must happen before
   next-check generation and store insertion.

If approved, the next implementation scope should be Slice 5 only: next-check
generation and compiled-record store insertion without polluting raw source
records. Public `get_evidence_bundle` integration should remain deferred to
Slice 6 unless reviewers redirect.

## Verification

All checks passed on the baseline commit:

- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets --all-features`
- `cargo run --bin validate_fixtures`
- `git diff --check`

Fixture validation reported `0 error(s), 0 warning(s)`.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude (Opus 4.8))

### Direction Verdict

Implementation round (Slice 4: token-budget selection).

**On critical path: yes. Milestone progress: real and verified. Next action:
continue — I approve starting Slice 5 (next-check generation + store insertion).
One milestone-critical item (the D-OVERFIT structural ranking refactor) is now
formally deferred once; it must land before/with Slice 6 gold-gating and must
not be deferred again past that point.**

Slice 4 is clean and correct on the points that matter:

- **Whole-item budget selection** under `max_items` and an effective token limit
  (`max_tokens - reserve_tokens_for_raw_refs`); no item is truncated to fit.
- **Counter-evidence enforcement fails hard** with
  `EvidenceCompileError::RequirementUnsatisfied` both when too few counters are
  generated and when the budget cannot fit the required counters — which is the
  design's intent (focus-question 4: yes, hard failure is correct, not a partial
  bundle).
- **Token recomputation is ordered correctly.** `ev-N` ids are assigned after
  the final sort, then `token_cost` is recomputed per item, and `tokens_used` is
  the sum. Because `ev-N` ids are shorter than `cand-NNN`, the recomputed costs
  are ≤ the costs used during budget checking, so the final bundle can never
  exceed the token limit. That is a sound invariant.
- **Link remapping** rewrites selected suspected-cause `supporting`/`counter`
  from `cand-*` to `ev-*` consistently with the item ids, and unresolved links
  are dropped. **Dropped reporting** lists every unselected `cand-*` with a
  stable reason.
- **D-OVERFIT comparison decision implemented per my Option B recommendation:**
  `compare_string_subset` accepts a non-empty subset of the gold reason
  vocabulary and rejects unknown tokens, with both an accept-subset and a
  reject-unknown test. The design records the decision.

The tests are genuinely end-to-end:
`compile_evidence_selects_ev_ids_and_reports_dropped_candidates` runs
generate→rank→select and checks ev-N assignment, token recomputation,
source-ref resolution, `bundle.validate()`, and ev-* link remapping; the
budget-drop, counter-first, and counter-error tests each drive `compile_evidence`
through a real budget. I ran the gate on baseline `bcb13dc`: `cargo fmt --check`
clean, `cargo clippy --all-targets --all-features` clean, `cargo test` all green
(`evidence_compiler` now 19 tests, +5).

I also credit the scope discipline: the implementor resisted adding causal
heuristics to satisfy D-OVERFIT inside the selector change, keeping that refactor
as its own reviewed step. That is the right call.

### Deferral tracking — D-OVERFIT structural ranking refactor (must not slip past Slice 6)

D-OVERFIT had two halves. The comparison half (relax `reasons` to a structural
subset) is **resolved** this round. The mechanism half — replacing
`entity_causal_multiplier`'s fixture-specific entity-name multipliers with
structural signals (the suspect's own anomaly state, dependency direction, onset
ordering, blast radius) and deriving reason tokens from structured source
content — is **deferred once** (round 5), with a stated deadline of before the
Slice 6 gold-gated suspected-cause comparison.

That deferral is acceptable here: Slice 4 selection is orthogonal to causal
ranking, and isolating the refactor avoids scope creep. But per the review
framework, the same milestone-critical item cannot be deferred indefinitely. So,
on the record: this is deferral #1. The structural refactor must land **before
or with Slice 6**, when suspected causes become gold-gated. If Slice 5 also
leaves it undone, Slice 6 must carry it — it cannot be pushed past gold-gating.

### Forward concern — the exact-match tension will hit `items` next (anticipate before Slice 6)

The structural relaxation we applied to `reasons` exists because exact-match to
hand-authored gold forces fixture-tuning. The **same tension applies to selected
`items`**, which the comparison contract still lists as Exact (selected item ids
and ordering). The selector's final ordering is `candidate_selection_group`
(change-support, then metric, trace, log, counter, missing, prior, dependency) —
a reasonable concretization of the design's documented order, but not yet
verified against gold, and the review-2 budget re-tuning means the compiler's
selected set/order may legitimately diverge from each fixture's gold bundle.

Before Slice 6 turns the bundle into a gold-gated comparison, decide — as a
deliberate choice, the same way we decided D-OVERFIT — whether selected item ids
and ordering are judged **exactly** (which may force per-fixture selection
tuning) or **structurally** (e.g., the top support items and the key
counter-evidence are present, ordering by documented group rule). I am not
asking for action this round; I am flagging that Slice 6 should not discover this
late.

### Smaller notes (low, non-blocking)

- The top-cause preservation step (`sorted_candidates_for_cause` + break-on-true)
  does not distinguish `Supports` from counter direction and breaks even when the
  first match was already selected as a counter. So "preserve ≥1 support item for
  the top cause" is not strictly guaranteed by that step alone; in practice the
  deterministic fill covers it. Consider filtering that step to supporting
  candidates so the intent is explicit.
- The dropped-candidate `reason` is a best-effort post-hoc label
  (`max_items` / `max_tokens` / `lower_priority`) computed against aggregate
  selected tokens, not exact per-candidate causality. Fine for diagnostics; just
  don't let a later slice assert exact drop reasons against gold.

### Answers to the implementor's reviewer-focus questions

1. `compile_evidence(query, store, derived)` and `select_evidence_compilation`
   are the right internal boundaries — `compile_evidence` matches the design's
   intended entry shape and is the natural seam for Slice 6's
   `get_evidence_bundle` routing.
2. Selection priority (forced counters → top-cause support → deterministic fill)
   is acceptable and deterministic. Note the final bundle is re-sorted by group,
   so counter-first is about inclusion, not final order — correctly separated.
3. Budget behavior is correct for this stage: whole-item, raw-ref reserve, token
   recomputation after `ev-N`, dropped reporting. Yes.
4. Counter-evidence should fail hard with `RequirementUnsatisfied`. Agreed — and
   it does.
5. The D-OVERFIT mitigation is enough to proceed to Slice 5. Yes — the
   comparison half is resolved and Slice 5 (next checks + store insertion) is
   orthogonal to causal ranking. The structural refactor stays on the deadline
   above.

**Approved next scope: Slice 5 (deterministic next-check generation and
compiled-record store insertion without polluting raw source records).**
`get_evidence_bundle` integration remains deferred to Slice 6, which should also
carry the D-OVERFIT structural refactor and the item-comparison decision above.

### Verification

Independently ran on baseline `bcb13dc`: `cargo fmt --check` (clean),
`cargo clippy --all-targets --all-features` (clean), `cargo test` (all suites
green, `evidence_compiler` 19 tests incl. the 5 new). Read the full selection
diff, the `compare_string_subset` change, the design-doc diff, and the test diff;
confirmed the counter-evidence hard-fail paths and the token-recomputation
ordering. Branch confirmed `evidence-compiler-ranking`.
