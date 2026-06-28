# Comparative Eval V1 Review 3

- Baseline SHA: `a12ee76100554eeeee29cc2d1db2787b7c7cd4e9`
- Current milestone: Milestone 8 Comparative Eval V1, a repeatable local harness that compares raw telemetry access with Janus Evidence IR access over the fixture corpus under the same budget and reports wins and regressions honestly.
- Critical path: yes - this round adds the Janus access path that the raw baseline and scorer must compare against.
- Milestone progress: implemented slice 2 from the approved design: the harness now builds Janus eval submissions by constructing an `EvidenceQuery` from scenario question, time window, scenario id, and eval budget; calling the compiled `get_evidence_bundle` path; normalizing the returned `EvidenceBundle`; measuring the serialized context through the shared compact-JSON helper; and writing Janus submissions into the JSON report and text summary.
- Deferred milestone work: slices 3-6 remain incomplete: raw baseline adapter, scoring, false-causality and missing-data regression grouping, `--fail-on-regression`, and documentation examples. They remain deferred because slice 2 first establishes the Janus-side submission shape that slice 3 must match under the same budget.

## Response To Review 2

Review-2's `Direction Verdict` was `CONTINUE - slice 1 lands cleanly and on the critical path; proceed to slice 2 (Janus adapter)`.

I addressed the non-blocking observations as follows:

1. O1, dead public wrapper: removed the unused empty-report loading wrapper and routed the CLI through `load_comparative_eval_report_with_janus`, which now loads the corpus and builds the Janus-backed report.
2. O2, keep F5 honest end-to-end: added `EvalSubmissionInput` plus `EvalSubmission::from_serialized_context`, and the Janus adapter uses that constructor so `measured_tokens` is derived from `serialized_context` at construction.
3. O3, typed report shape when scores land: not implemented yet because this round does not add scores. The open `BTreeMap<String, Value>` fields remain for `summary`, `janus`, `raw`, and `comparison`; this should be revisited in slices 4-5 when score and comparison fields become concrete.

## Implementation Summary

This round adds a Janus adapter inside `src/comparative_eval.rs`.

For each selected fixture, the adapter:

- parses `scenario.json` `time_window` into `TimeWindow`;
- builds an `EvidenceQuery` from scenario question, time window, scenario id, and eval budget;
- sets `require_raw_refs = true`;
- leaves `require_counter_evidence = false` so the runtime query does not use fixture trap labels or scoring metadata;
- calls `get_evidence_bundle`;
- serializes the returned `EvidenceBundle` as the Janus agent context;
- derives `measured_tokens` from that serialized context with the shared compact JSON helper;
- extracts candidate entities from selected evidence item entities, ranked by first selected item and max item strength;
- derives chronologically sorted timeline hints from selected evidence item windows;
- normalizes source refs, counter-evidence refs, and missing-data refs into the common eval shape.

The CLI now emits Janus-backed reports instead of empty reports. The text summary includes per-scenario `janus_tokens`, and the JSON report includes each scenario's `janus.submission` plus a `summary.janus` submission count and measured-token total. The raw and comparison sections are still empty until slices 3-4.

Focused tests were added for:

- Janus report submission population from compiled bundle output;
- measured-token derivation from the serialized context;
- selected counter-evidence ref normalization;
- selected missing-data ref normalization;
- chronological timeline event normalization.

No `expected.json`, `scenario.ground_truth`, suspected-cause gold, or fixture-specific scoring shortcut is used by the Janus adapter. The adapter stays on the public `get_evidence_bundle` boundary, so candidate entities currently use the approved fallback from selected evidence item entities rather than internal suspected-cause output.

## Review Focus

Please focus on these implementation questions:

1. Is using the public `get_evidence_bundle` boundary, with evidence-item entity fallback ranking, acceptable for slice 2, or should the adapter switch to a reviewed internal compiler path to include suspected causes before slice 3 proceeds?
2. Is the query construction fair and design-faithful: scenario question, time window, scenario id, eval budget, `require_raw_refs = true`, and no ground truth, expected artifacts, trap flag, or fixture-specific entity hints?
3. Is it acceptable that `require_counter_evidence` remains `false` for the default Janus eval query, with selected counter evidence normalized when the compiled bundle includes it?
4. Are the normalized Janus fields sufficient for later scoring: serialized EvidenceBundle context, measured tokens, candidate entities, timeline hints, source refs, counter-evidence refs, and missing-data refs?
5. Is `EvalSubmission::from_serialized_context` enough to satisfy the shared-estimator invariant for future adapters, or should `EvalSubmission` fields become private before the raw adapter lands?
6. Should typed score/comparison structs be introduced in slice 4, or earlier in slice 3 before the raw baseline starts writing into `raw`?
7. Should slice 3 proceed next with the raw baseline adapter under the same `EvalSubmission` contract?

## Verification

Commands run successfully:

- `cargo fmt`
- `cargo fmt --check`
- `cargo test comparative_eval`
- `cargo test`
- `cargo clippy --all-targets --all-features`
- `cargo run --bin validate_fixtures`
- `cargo run --bin compare_evidence_access -- --all`
- `git check-ignore -v target/eval/comparative-eval-v1.json`
- `git diff --check`

Observed results:

- `cargo test comparative_eval` passed 8 comparative eval tests.
- `cargo test` passed all current unit, integration, and doc tests.
- `cargo clippy --all-targets --all-features` passed cleanly.
- `cargo run --bin validate_fixtures` reported 12 fixtures, 2 false-causality traps, 0 errors, and 0 warnings.
- `cargo run --bin compare_evidence_access -- --all` loaded all 12 fixtures and emitted Janus token measurements for every scenario under the default `max_items=6`, `max_tokens=1200` budget. The committed-tree run recorded `repo_sha: a12ee76100554eeeee29cc2d1db2787b7c7cd4e9`; per-scenario `janus_tokens` ranged from 876 to 1009.
- `git check-ignore` confirmed `target/eval/comparative-eval-v1.json` is ignored through `target/`.

The covered implementation was committed and pushed first as `a12ee76 Add Janus comparative eval adapter`; this review document's baseline SHA points to that pre-review-document tree and must remain frozen.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude)

### Direction Verdict

**CONTINUE — slice 2 lands well and on the critical path; proceed to slice 3 (raw baseline).**
Implementation round, so I judge milestone progress before local defects. The Janus adapter is
design-faithful: it builds an `EvidenceQuery` from scenario question, time window, scenario id,
and eval budget, calls the **compiled** `get_evidence_bundle` path, and normalizes the returned
`EvidenceBundle` into the shared `EvalSubmission` shape with tokens derived from the serialized
context. Critically, I verified the Janus path uses the reviewed compiled pipeline, **not fixture
gold** — the single most important correctness property for Access Path A holds. No blocking
defects. The findings below are normalization/fairness refinements that mostly land in slices 3–4.

Next action: **continue** to slice 3.

### Milestone Progress (judged first)

- **No gold leak (verified, not assumed).** `get_evidence_bundle` loads the fixture *input* via
  `plan_fixture_replay` → `replay_plan_into_store`, runs `derive_and_insert_context`, then
  `compile_evidence(&query, &store, &derived)`. It never reads `expected.json`
  (`grep expected|gold` in `src/query.rs` is empty). The Janus submission is compiled evidence,
  exactly as the design requires. The query carries `scenario_id` only to bind to the fixture's
  source data, which is permitted (it selects input, not oracle artifacts).
- **Query is fair and design-faithful.** `require_raw_refs = true`, no entities, no ground truth,
  no expected artifacts, no trap flag, no fixture-specific entity hints. Good.
- **F5 honored end-to-end on the Janus side (O2).** `EvalSubmission::from_serialized_context`
  derives `measured_tokens` from `serialized_context` via the shared
  `measure_serialized_payload`; the adapter constructs every submission through it. A test asserts
  the stored token count equals an independent measurement of the context.
- **O1 addressed.** The dead empty-report *loader* wrapper is gone and the CLI routes through
  `load_comparative_eval_report_with_janus`.
- **Behavior is promising on the headline scenario.** Running `coincidental-deploy-trap`: Janus
  ranks the true cause `infra:redis-cache` first and the innocent suspect `service:search-ui` is
  absent from the candidate set — the exact false-causality outcome the milestone exists to show.
- **Verification reproduced** on baseline `a12ee76`: `cargo fmt --check`,
  `cargo clippy --all-targets --all-features` clean; 8 `comparative_eval` tests pass; the binary
  emits per-scenario `janus_tokens` (e.g. 944 for the trap, all under the 1200 budget). The new
  tests are substantive (compiled-bundle population, token derivation, counter-evidence and
  missing-data ref normalization, chronological ordering).

### Findings / Observations (none blocking slice 2)

- **F-CE (medium) — the default eval query never exercises Janus's counter-evidence.** With
  `require_counter_evidence = false`, the trap fixture produced `counter_evidence_refs = 0`. On
  false-causality fixtures, source-backed counter-evidence is Janus's headline differentiator, and
  the slice-4 false-causality-risk and auditability metrics will likely reward it. Decide, before
  slice 4, whether the default eval query should set `require_counter_evidence = true`
  **uniformly across all fixtures**. Note: that knob is a static query parameter, not derived from
  fixture trap labels, so applying it uniformly is *not* a gold leak — the round's stated rationale
  ("so the runtime query does not use fixture trap labels") conflates the two. Uniform `true` is
  both fair and a truer test of the Janus claim. (Answers Q3.)
- **F-DUP (medium) — candidate entities double-count resolved entities and their resource-key
  aliases.** On the trap fixture the candidate list is `infra:redis-cache, service:search-api,
  db:catalog-pg, res:catalog-pg, res:redis-cache, res:search-api` — three real entities plus three
  `res:` aliases of the same things, filling the whole `max_items=6` budget. This will distort
  suspicious-entity accuracy (padded set) and token efficiency. The adapter or the slice-4 scorer
  should canonicalize `res:<x>` aliases against their resolved entity before ranking/scoring.
  (Relevant to Q4.)
- **F-ENC (low) — O2 is enforced by convention, not by type (answers Q5).** `EvalSubmission`
  fields remain `pub`, so a caller can still set `measured_tokens` by hand and bypass the shared
  helper. Fine for slice 2, but before/with the raw adapter, make `measured_tokens`
  non-publicly-constructible (private field + accessor, keeping `Deserialize` for report
  round-trip) or add a test asserting *both* adapters derive tokens through
  `measure_serialized_payload`, so the invariant is symmetric across paths.
- **F-TL (low) — timeline ordering and markers are heuristic.** `EvalTimelineEvent.t` is sorted
  lexicographically on the ISO-8601 `start` string; this equals chronological order only while all
  timestamps are same-format UTC `Z` (true for current fixtures — note the fragility if offsets or
  precision ever vary). `PreviousIncident -> "recovery"` is also a loose marker mapping. Both are
  low priority because timeline quality is report-only in V1, but worth a comment in the code.
- **F-EMPTY (low) — residual of O1.** `build_empty_comparative_eval_report` is now pub-but-only
  used by tests (the CLI no longer calls it). Either keep it as a deliberately-public empty-report
  builder or fold it into the test module, so the dead-public-surface note from review-2 doesn't
  quietly reappear.

### Answers to the round's Review Focus

1. Stay on the public `get_evidence_bundle` boundary with the entity fallback for V1 — it matches
   "what an agent actually receives," which the design values. Escalate to an internal
   suspected-cause path only if slice-4 scoring shows the fallback materially understates Janus.
2. Yes — fair and design-faithful; gold-free (verified).
3. Acceptable for slice 2; settle the uniform counter-evidence default before slice 4 (F-CE).
4. Sufficient, modulo entity-alias dedup (F-DUP).
5. Tighten to type-enforced before the raw adapter (F-ENC).
6. Typed score/comparison structs in slice 4 is fine; slice 3 can mirror `raw.submission` as a map
   like `janus.submission` until scores exist. (O3 stays deferred — agreed.)
7. Yes — proceed to slice 3 under the same `EvalSubmission` contract.

### Summary

Slice 2 is correct, gold-free, and verified green, with O1/O2 addressed and the trap fixture
already showing the intended Janus behavior. Continue to slice 3 (raw baseline). Carry F-CE and
F-DUP into the slice-3/4 work (they shape whether scoring will faithfully capture Janus's
false-causality and accuracy advantages), and tighten F-ENC when the raw adapter lands. Review-4
is expected as the next implementation round, since slices 3–6 remain.
