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
