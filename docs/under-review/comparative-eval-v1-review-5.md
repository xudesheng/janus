# Comparative Eval V1 Review 5

- Baseline SHA: `f5b59c3559ff64700562bafd2004d2a2761d4f5e`
- Current milestone: Milestone 8 Comparative Eval V1, a repeatable local harness that compares raw telemetry access with Janus Evidence IR access over the fixture corpus under the same budget and reports wins and regressions honestly.
- Critical path: yes - this round implements slice 4 scoring and resolves the review-4 F-CE and O-ALIAS gates needed before scoring could be meaningful.
- Milestone progress: the harness now scores both Janus and raw submissions for every selected fixture, writes typed per-scenario score comparisons under `comparison.scores`, aggregates required-score summaries, prints scenario winners in the text report, sets the Janus eval query to require counter-evidence uniformly, and documents/tests the raw resource-alias fairness concession.
- Deferred milestone work: slices 5-6 remain incomplete: richer false-causality and missing-data regression grouping, `--fail-on-regression`, and documentation examples. The current scoring payload should be the substrate for those slices.

## Response To Review 4

Review-4's `Direction Verdict` was `CONTINUE - slice 3 delivers a fair, non-strawman raw baseline; proceed to slice 4 (scoring), but settle the F-CE counter-evidence default first.`

I addressed the actionable feedback as follows:

1. F-CE, counter-evidence default: changed the V1 Janus eval query to set `require_counter_evidence = true` and `min_counter_evidence_items = 1` uniformly for every fixture. The design doc now states this is a static eval-query requirement, not a trap-label or gold conditional.
2. F-CE implementation consequence: the first uniform-counter run exposed that several fixtures could not satisfy the requirement through explicit counter candidates alone. I added query-gated structural counter-evidence materialization inside `compile_evidence`: it only runs when the query requires counter-evidence, uses already-generated runtime candidates and suspected-cause links, and emits source-backed weakens items without reading fixture gold.
3. O-ALIAS: documented resource aliasing in the design as a deliberate fairness concession for raw access. The aliaser uses direct raw `resources` attributes only (`service.name`, `db.system`, `rollout`, `service.instance.id`, `service.version`, `cluster.name`), applies to both normalized Janus and raw entities, and does not infer relationships. A new test spot-checks the mapping against scored service/db/infra ground-truth entities.
4. O3, typed score/comparison structs: added `EvalMetricScore`, `EvalPathScores`, `EvalScoreDelta`, and `EvalScenarioComparison`. The report still keeps extensible maps at the outer `summary` and `comparison` boundaries, but the score payload itself is typed and round-trips through serde.
5. The raw baseline still does not synthesize counter-evidence. Its `counter_evidence_refs` remain empty unless future raw-record handling explicitly exposes a direct counter record.

## Implementation Summary

Slice 4 adds five required metrics and one report-only metric:

- `suspicious_entity_accuracy`: rewards primary-cause rank, partial credit for visibility, and caps score when a known not-the-cause entity is ranked first.
- `false_causality_risk`: high score means lower risk. It penalizes ranked not-the-cause or innocent-suspect entities, with details exposing the raw `risk` value, avoid-entity rank, and counter-evidence count.
- `missing_data_awareness`: required only for missing-data or under-determined scenarios; rewards visible gap refs and explicit uncertainty.
- `auditability`: rewards source refs that resolve through `ReferenceIndex` and coverage of expected signal families from the expected evidence bundle.
- `token_efficiency`: scales useful score by a modest budget-utilization penalty so utility dominates pure compactness.
- `timeline_quality`: report-only; checks chronological ordering, source-backed events, and expected timeline event coverage.

The full report builder now computes per-scenario Janus/raw scores, score deltas, and a required-score winner. It also aggregates average Janus/raw scores, average deltas, scenario wins, and false-causality trap summaries. The text report now includes top-level required averages and per-scenario `janus_score`, `raw_score`, and `winner`.

The structural counter-evidence helper is intentionally narrow:

- It is called only when `required_counter_evidence_count(query) > 0`.
- It does not change default non-counter-required bundle behavior.
- It skips `under-determined`, already-countered entities, and top-ranked causes without explicit counter links.
- For explicit counter links, it reuses those source refs.
- For low-scoring non-top alternatives without explicit counter links, it can create a weakens item from that alternative's runtime source-backed candidate refs. This is the least settled semantic choice in the round and should get reviewer attention.

## Review Focus

Please focus on these questions:

1. Are the scoring formulas simple and fair enough for V1, especially the rank-based suspicious-entity scoring and the modest token-efficiency penalty?
2. Is `false_causality_risk` clear enough as a high-is-good score, given that its details include a high-is-bad `risk` value?
3. Is the query-gated structural counter-evidence helper acceptable, or is it too much compiler behavior introduced for the eval harness?
4. Does the resource alias mapping remain a fair raw-baseline concession rather than corpus-tuned inference?
5. Is it acceptable for auditability scoring to use expected evidence-bundle source signal families for coverage, while still requiring actual selected source refs to resolve through `ReferenceIndex`?
6. Are raw wins on `traffic-shift-hotspot` and `missing-data-gap` visible enough, and should slice 5 build `--fail-on-regression` around the current `required_average` and winner fields?
7. Should `timeline_quality` remain report-only for V1, or should any part of it move into the required average before regression gating?

## Verification

Commands run successfully on baseline `f5b59c3559ff64700562bafd2004d2a2761d4f5e`:

- `cargo fmt --check`
- `cargo test comparative_eval`
- `cargo test`
- `cargo clippy --all-targets --all-features`
- `cargo run --bin validate_fixtures`
- `cargo run --bin compare_evidence_access -- --all`
- `git check-ignore -v target/eval/comparative-eval-v1.json`
- `git diff --check`

Observed results:

- `cargo test comparative_eval` passed 16 comparative eval tests.
- `cargo test` passed all 36 library tests plus current integration and doc tests.
- `cargo clippy --all-targets --all-features` passed cleanly.
- `cargo run --bin validate_fixtures` reported 12 fixtures, 2 false-causality traps, 0 errors, and 0 warnings.
- `cargo run --bin compare_evidence_access -- --all` recorded `repo_sha: f5b59c3559ff64700562bafd2004d2a2761d4f5e`.
- Aggregate required average: Janus `0.890`, raw `0.828`, delta `0.061`.
- Scenario winners: Janus won 10, raw won 2 (`traffic-shift-hotspot`, `missing-data-gap`).
- The two false-causality trap fixtures both scored as Janus wins: `retry-storm-amplification` (`0.886` vs `0.859`) and `coincidental-deploy-trap` (`0.887` vs `0.770`).
- `git check-ignore` confirmed `target/eval/comparative-eval-v1.json` is ignored through `target/`.

The covered implementation was committed and pushed first as `f5b59c3 Add comparative eval scoring`; this review document's baseline SHA points to that pre-review-document tree and must remain frozen.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude)

### Direction Verdict

**CONTINUE on the scoring framework, but with one required redirect before slice 5: revert the
compiler-side counter-evidence materialization and drop the hard `min_counter_evidence_items = 1`
mandate from the eval query.** Implementation round, so milestone progress before local defects.
The five scorers, typed score structs, symmetric scoring, oracle-only use of `expected`, and the
O-ALIAS documentation are good and should stay. But the F-CE resolution went the wrong way: it
adds new behavior *inside* `compile_evidence` that (a) changes the M6 compiler contract for every
consumer, not just the eval, and (b) empirically degrades rather than strengthens Janus's headline
trap result. Because slice 5 will gate `--fail-on-regression` on these numbers, this must be fixed
and the scores re-baselined first. The milestone is not endangered — Janus's genuine advantage is
*cleaner* without this change.

Net action: **continue**, but treat F-MAT below as blocking for slice 5.

I own part of this: my review-3/4 push to "set `require_counter_evidence = true` uniformly" pointed
here. The implementation revealed that the *hard mandate* is the wrong lever — see F-MAT for the
corrected recommendation.

### Milestone Progress (judged first)

The scoring substrate itself is solid and largely fair:

- Five required metrics + report-only `timeline_quality`, composed into a `required_average`, with
  typed `EvalMetricScore`/`EvalPathScores`/`EvalScoreDelta`/`EvalScenarioComparison` (O3 resolved).
- Scoring is applied identically to both submissions; `expected.json` is used only as a scoring
  oracle (signal-family coverage, `ReferenceIndex` resolution, expected timeline/gap refs) and is
  never fed into either access path. Good.
- O-ALIAS documented in the design as a deliberate, symmetric, attribute-only fairness concession.
- Verified green on baseline `f5b59c3`: `cargo fmt --check`, `cargo clippy --all-targets
  --all-features` clean; 16 `comparative_eval` tests and the full suite pass. Aggregate
  `required_average`: Janus 0.890 vs raw 0.828; Janus wins 10/12, raw wins `traffic-shift-hotspot`
  and `missing-data-gap` (honestly surfaced, not hidden — good).

### F-MAT (high, direction-level) — the structural counter-evidence materialization is net-negative and out of place

`push_structural_counter_evidence_candidates` runs whenever `required_counter_evidence_count(query)
> 0`, so it is **not** an eval-only change — it alters how `compile_evidence`/`get_evidence_bundle`
responds to `require_counter_evidence` for *every* caller (e.g. an MCP agent). That edges into the
design's explicit out-of-scope item, "changing Evidence IR semantics to make the eval easier," and
it was introduced in the same commit as the scorer that rewards it, without its own compiler-review
topic.

The mechanism is also semantically weak: for a low-ranked alternative with no genuine counter link,
it builds a `Weakens` item whose `source_refs` are that entity's own **supporting** candidate refs
(`counter_source_ids = &cause.supporting`). Relabeling supporting evidence as weakening evidence is
not sound counter-evidence — genuine counter-evidence should *discriminate against* causality (flat
metrics, dependency direction, onset-before-change), not reuse the entity's supporting signals.

Most importantly, I measured what it actually does, and it hurts Janus more than it helps:

- `coincidental-deploy-trap`: in review-4 (no counter requirement) Janus *suppressed* the innocent
  suspect → `avoid_rank=None, risk=0, fcr=1.0`. Now the materialized weakens item **injects**
  `service:search-ui` back into Janus's candidate set at rank 4 (candidate entities are derived from
  *all* item entities, including weakens items) → `avoid_rank=4, risk=0.25, fcr=0.75`. The change
  turned the cleanest possible result into a worse, self-referential one (suspect present *because*
  of the counter item, then rescued *by* the counter item).
- `traffic-shift-hotspot` (a raw win): the same injection puts an avoid entity at rank 3
  (`risk=0.25`) that raw never surfaces (`risk=0`). The compiler change directly *caused* a Janus
  loss here.
- `retry-storm-amplification`: Janus's `fcr=1.0` comes from suspect *absence*, not counter-evidence;
  the 6 materialized refs only inflate **auditability** (0.95 vs raw 0.85) via extra resolvable
  refs — a ref-count effect, not an evidence-quality gain.

So the materialization buys a little auditability margin and costs real false-causality quality,
while making the marquee result circular. Janus's genuine win (innocent-suspect suppression) existed
in review-4 *without* any of this.

**Recommended fix:** remove `push_structural_counter_evidence_candidates`; set the eval query to
`require_counter_evidence = true` *without* a hard `min_counter_evidence_items` mandate (i.e. "surface
counter-evidence when the genuine pipeline produces it," not "fabricate one per fixture"), or revert
to the review-4 query. Then re-baseline the report. I expect the two trap fixtures to *improve*
(coincidental-deploy-trap back to `risk=0`) and the numbers to become defensible under a regression
gate. If a real structural-counter-evidence capability is wanted in the compiler, give it its own
reviewed topic (e.g. `false-causality-guard`) with discriminating provenance, not supporting-ref
relabeling.

### F-CAND (medium) — counter/weakens-only entities should not count as promoted candidates

Independent of F-MAT: `candidate_entities_from_bundle` promotes an entity into the candidate list
even when it appears *only* in a `Weakens`/`CounterEvidence` item. But a weakens item is Janus
arguing *against* that entity as a cause, so counting it as a candidate (and then as `avoid_rank`)
contradicts the intent. This is why a properly counter-argued suspect still scores `risk=0.25`
instead of the design's other "best" outcome. The design says the innocent suspect being "absent
**or** explicitly weakened by source-backed counter-evidence" is the best result, yet the scoring
ranks present-and-countered (`0.75`) strictly below absent (`1.0`). Fix one of: (a) exclude
weakens-only entities from the promoted candidate set, or (b) let a present-but-source-backed-
countered suspect reach near-parity with absent in `false_causality_risk`. This should be settled
with F-MAT since they interact.

### Answers to the round's Review Focus

1. Formulas are simple and fair for V1; the rank tiers and modest token penalty are fine.
2. `false_causality_risk` as high-is-good with a high-is-bad `risk` detail is mildly confusing but
   acceptable since the detail is labeled; low priority.
3. **No** — the query-gated structural counter-evidence helper is too much compiler behavior for
   the eval and is net-negative (F-MAT). Remove it; pursue genuine counter-evidence in its own
   compiler topic if desired.
4. Yes — the resource alias mapping is a fair, symmetric, attribute-only concession; it reads only
   raw `resources` attributes and is documented as such. Keep the per-fixture namespace spot-check
   as the corpus grows.
5. Yes — using expected source-signal families for coverage while still requiring selected refs to
   resolve through `ReferenceIndex` is acceptable oracle use, applied to both paths. (Note it
   rewards ref count/coverage, which is why F-MAT's extra refs inflate it — another reason to
   remove the materialization.)
6. The raw wins are visible and honestly reported — good. But do **not** build
   `--fail-on-regression` on the current numbers until F-MAT is resolved and the report is
   re-baselined; otherwise the gate enforces artifacts.
7. Keep `timeline_quality` report-only for V1 (agreed); do not move it into the required average
   before regression gating.

### Summary

The scoring framework is the right shape and mostly fair, with O3 and O-ALIAS resolved and honest
raw wins surfaced. But the F-CE resolution introduced a compiler-contract change
(`push_structural_counter_evidence_candidates`) that is semantically unsound and empirically
degrades Janus's cleanest trap result while only inflating auditability via ref count — and it
changes behavior for all `get_evidence_bundle` consumers, not just the eval. Revert it and drop the
hard counter-evidence mandate, fix the related candidate-from-weakens modeling (F-CAND), and
re-baseline before slice 5 gates on these scores. This strengthens the milestone claim rather than
weakening it. Because this round left actionable feedback, the next round (review-6) should carry
the F-MAT/F-CAND fixes and the re-baselined report, then proceed to slice 5.
