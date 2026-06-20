# Roadmap Review 0

- Baseline SHA: `01aba4228d8ab9e10a500de2e10354f311d379a1`
- Current milestone: committed `docs/core/roadmap.md` as the formal roadmap for moving Janus from design and fixtures toward a testable MVP.
- Critical path: yes - the roadmap sets the implementation order that future review topics will follow, starting with the Evidence IR contract.
- Milestone progress: added a milestone-based roadmap that sequences Evidence IR, `get_evidence_bundle`, fixture validation, hot context storage, derived context, evidence compilation, agent surface, comparative eval, ingest, warm/cold memory, and hardening.
- Deferred milestone work: none for this roadmap milestone. Implementation work begins in the next topic, expected to be `evidence-ir-schema`.

This first roadmap review covers `docs/core/roadmap.md`. The document is meant
to be a planning artifact, not an implementation spec or release schedule. It
should help keep Janus focused on the evidence-substrate thesis from
`docs/core/what_and_why.md` while giving implementation rounds concrete
milestones.

The roadmap currently makes these direction choices:

- Evidence IR and schema validation come before storage, ingest, and derivation.
- A narrow fixture loader appears in Milestone 1 because Evidence IR tests need
  fixture gold bundles before the full validation harness exists.
- `get_evidence_bundle` is the first investigation primitive, with fixture-backed
  gold output before real retrieval.
- Full fixture validation is separated into Milestone 3.
- Derived context is split into Milestone 5A for entities and relationships, and
  Milestone 5B for anomalies, log patterns, timelines, `find_related_anomalies`,
  and `compare_windows`.
- `previous_incident` evidence is fixture-sourced until warm/cold memory exists.
- The cold layer is treated as durable understanding plus backlinks, folded into
  the warm-memory and compaction milestone rather than full raw retention.

## Reviewer Focus

Please pay closest attention to direction and milestone boundaries, not wording
polish.

Key questions:

1. Is the roadmap faithful to `what_and_why.md`, especially the boundary that
   Janus is an evidence substrate, not OpenTelemetry replacement, APM UI, or RCA
   agent?
2. Is the sequence right: Evidence IR -> `get_evidence_bundle` -> fixture
   validation -> hot store -> derived context -> evidence compiler -> agent
   surface -> comparative eval?
3. Are Milestones 1 through 4 small enough to land as reviewable implementation
   rounds, or is any dependency still hidden?
4. Is the Milestone 5 split into entity/relationship first and
   anomaly/pattern/timeline second the right cut?
5. Are `find_related_anomalies` and `compare_windows` now given enough of a home
   to prevent fixture-roadmap drift?
6. Is the `previous_incident` boundary clear enough, given that fixture-provided
   prior incidents exist before real warm/cold memory?
7. Does Milestone 10 capture the cold-layer idea from `what_and_why.md` without
   implying long-term full raw telemetry retention?
8. Is anything missing from the roadmap that is required to prove the central
   bet: fewer, more accurate, more auditable evidence under the same agent,
   incident, time, and token budget?

## Verification

No code verification this round. This is a documentation-only roadmap review.

Commands run while preparing the covered roadmap and this review request:

- `Get-Content -Raw -Encoding utf8 docs/core/what_and_why.md`
- `Get-Content -Raw -Encoding utf8 docs/core/evidence-spine.md`
- `Get-Content -Raw -Encoding utf8 docs/process/fixtures.md`
- `Get-Content -Raw -Encoding utf8 docs/core/roadmap.md`
- `rg "Review \\(by Claude\\)|Milestone 2\\.|find_related_anomalies|compare_windows|previous_incident|cold-layer|Milestone 5A|Milestone 5B" docs/core/roadmap.md`
- `git status --short --branch`

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude)

### Direction Verdict

**Continue.** This roadmap is on the critical path — it fixes the implementation
order every later topic inherits — and it moves the Milestone 0 deliverable
forward by making that order concrete. It is faithful to `what_and_why.md`: the
evidence-substrate boundary holds (contract before storage, agent surface before
dashboard, "Deliberately Later" explicitly rejects APM parity, automatic RCA as
the API contract, a custom distributed storage engine, and full raw retention).
The six findings I raised informally last round are all incorporated and verified
against the baseline tree: M0 now points to Milestone 3; a narrow fixture loader
is pulled into M1 and extended in M3; M5 is split into 5A/5B; `find_related_anomalies`
and `compare_windows` have a derived-context home in 5B and a surface in M7;
the `previous_incident` boundary is stated in M6; and the cold layer is folded into
M10 without promising full raw retention. The remaining findings below are
refinements — none change the milestone sequence, so the next action is to apply
what is useful and proceed to `evidence-ir-schema`.

### Answers to the eight reviewer questions

1. **Faithful to the boundary?** Yes. See verdict above.
2. **Sequence right?** Yes. The one thing worth stating explicitly: the *real*
   `get_evidence_bundle` (compiled, ranked) is M6; M2 is a gold-returning stub.
   The eval (M8) correctly lands after the compiler (M6) and surface (M7), so the
   bet is tested against real bundles, not the stub.
3. **M1–M4 small enough / hidden dependencies?** Mostly yes; the previously hidden
   loader dependency is now explicit. One residual boundary nit in F6.
4. **M5 5A/5B cut right?** Yes, with one over-scoped acceptance criterion (F2).
5. **`find_related_anomalies` / `compare_windows` home?** Yes — resolved. But the
   same drift now applies to `rank_suspected_causes` (F1).
6. **`previous_incident` boundary clear?** Yes — M6 Boundary section is sufficient.
7. **M10 cold layer without full-raw-retention implication?** Yes — well captured.
8. **Anything missing to prove the bet?** Yes — the raw-access baseline needs a
   fairness guard (F3), and the scoring source should be named (F4).

### Findings

- **F1 [consistency] `rank_suspected_causes` is now the orphaned primitive.**
  Every fixture ships a gold `suspected_causes` artifact and the registry lists
  `rank_suspected_causes` as a capability, but it appears in no milestone
  deliverable and is absent from the M7 surface list — the exact drift just fixed
  for `find_related_anomalies`/`compare_windows`. Recommend naming
  `rank_suspected_causes` as an M6 output and either adding it to the M7 surface
  list or stating explicitly that it is exposed only as a field inside the
  `get_evidence_bundle` response.

- **F2 [boundary] M5B's timeline acceptance over-promises into M6 territory.**
  "Timeline output can distinguish causal changes from non-causal nearby changes"
  is the same reasoning as the false-causality guard (time alignment, onset-before-
  change), which lands in M6. The `coincidental-deploy-trap` timeline's
  `non-causal-change` marker cannot be justified by 5B alone. Recommend 5B emit the
  timeline structure and *candidate* nearby-change markers, and move the
  causal/non-causal *classification* criterion to M6.

- **F3 [methodology, high] M8's raw-access baseline must be defensible, not a
  strawman.** The entire bet is "better than raw-backend access," so the result is
  only as credible as the baseline. A naive baseline (dump all of `input.json`, or
  a `LIMIT N` truncation that happens to drop the cause) would make Janus win
  trivially and prove nothing. Recommend an explicit M8 requirement that the raw
  baseline approximate realistic raw query access (recency/label/entity slice under
  the same token budget) and that the baseline be reviewed as adversarial to Janus.
  This is the highest-leverage addition to the eval.

- **F4 [minor] Name M8's scoring source and measure token cost.**
  "Suspicious-entity accuracy" should be scored against `scenario.json`
  `ground_truth` (`primary_cause_entity`, `blast_radius`, `not_the_cause`) — say
  so. And the token-cost metric should be *measured* from serialized output, not
  read from the Evidence IR's hand-authored `token_cost` field (using the declared
  field as the metric would be circular).

- **F5 [minor] `explain_symptom` from `what_and_why.md` is unmapped.** The design
  doc's primitive list includes `explain_symptom`; the roadmap never mentions it.
  Likely intentionally folded into question-driven `get_evidence_bundle` — if so,
  one line saying that would close the loop; otherwise it is a gap.

- **F6 [minor] Clarify the M1 narrow loader's addressing.** M2 returns "gold
  bundles by scenario" using the M1 loader, which means the narrow loader must
  resolve a single fixture by scenario id/path (`fixtures/scenarios/<id>/expected.json`).
  M1 says the loader does not do "registry coverage," which is fine, but state that
  single-fixture-by-id resolution *is* in scope for M1 so M2 does not implicitly
  pull M3's registry loader forward.

### Process note

Commit-and-review-gate is compliant: the covered doc was committed first
(`01aba42`), the review request is its own later commit (`a0a4f02`), the baseline
SHA points to the pre-review-document tree, and `roadmap.md` at that baseline
matches the reviewed content.

### Next action

Continue. None of F1–F6 block the sequence; fold them into the roadmap (or a
round-1 response) and proceed to `evidence-ir-schema`. Per Round Termination, a
roadmap `review-1` is only warranted if you want these refinements re-reviewed;
otherwise apply them and move to the first implementation topic.
