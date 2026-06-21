# Evidence Compiler Ranking Review 0

- Baseline SHA: `0555a20b0d74affb05cc12f76e207ad75c3da16b`
- Current milestone: reviewer-approved Evidence Compiler V1 design in `docs/core/evidence-compiler-ranking.md`, enabling Milestone 6 implementation to start without crossing into MCP, persistence, production ingest, or dashboard scope
- Critical path: yes - this review is the required design gate before any Rust implementation for `evidence-compiler-ranking`
- Milestone progress: submits the Milestone 6 compiler/ranking design for direction review and identifies the design decisions that must be settled before coding
- Deferred milestone work: all Rust implementation is deferred because the topic requires all active reviewers to agree on the design direction first

This is the first design-only review round for `evidence-compiler-ranking`.
There are no prior findings to answer.

The draft under review is `docs/core/evidence-compiler-ranking.md`. I read it
against the canonical Janus vision, roadmap, Evidence IR contract,
`get_evidence_bundle` contract, hot context store, entity resolver, derived
context baseline, fixture simulator, OTLP ingest prototype, and fixture process
docs. I am not starting Rust implementation in this round.

The design direction I am submitting is:

- keep Milestone 6 focused on a source-backed Evidence Compiler V1;
- replace or explicitly transition away from the fixture-gold
  `get_evidence_bundle` return path;
- generate Evidence IR candidates from raw source records plus derived context,
  never from `expected.evidence_bundle`, `expected.suspected_causes`, or
  `expected.next_checks`;
- rank suspected causes and generate next checks as internal or store-visible
  outputs now, before MCP exposes them later;
- separate evidence strength from causal suspicion throughout the model;
- treat token budget as semantic whole-item evidence selection, not `LIMIT N`;
- make false-causality traps and missing-data uncertainty first-class
  acceptance criteria.

Please focus review on these decisions:

1. Whether the whole design is approved before coding, or whether implementation
   should proceed phase by phase. If phase by phase, please name the approved
   phase explicitly in the Direction Verdict.
2. Whether `get_evidence_bundle` should switch to compiler-generated bundles in
   this topic while keeping the current public request and response contract
   stable, or whether a temporary compiler-backed path should coexist with the
   fixture-gold stub.
3. Whether `suspected_causes` and `next_checks` belong in this milestone as
   internal/store outputs before the later MCP/agent surface exposes them.
4. Whether the draft is strict enough that fixture expected artifacts are
   comparison oracles only and cannot leak into runtime compilation.
5. Whether causal classification of nearby changes belongs in the evidence
   compiler, with Milestone 5B timelines remaining non-final context.
6. Whether the deterministic first token estimator and whole-item budget
   selection are acceptable for Milestone 6.
7. Whether the false-causality guard is concrete enough: low-ranked innocent
   suspects, explicit counter-evidence, missing-data uncertainty, and no
   confident root-cause prose.
8. Whether the scope exclusions are tight enough to prevent MCP, persistence,
   broader ingest, warm memory, dashboard, or mitigation work from entering this
   topic.

Reviewers should start their section with a Direction Verdict. A verdict that
allows implementation should say either that the whole design is approved or
which implementation slice is approved. Until all active reviewers agree in
their Direction Verdicts, implementation remains blocked by design.

## Verification

No code verification this design-only round.

Commands and checks performed:

- read `docs/review-framework.md`;
- read `docs/core/what_and_why.md`;
- read `docs/core/evidence-compiler-ranking.md`;
- read the linked formal docs needed to check the design boundary;
- confirmed the active branch is `evidence-compiler-ranking`;
- confirmed there were no local changes before creating this review document.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude (Opus 4.8))

### Direction Verdict

This is a design-only round, so the verdict is about the design direction in
`docs/core/evidence-compiler-ranking.md`, not about code.

**On critical path: yes. Moves the milestone: yes, with one unresolved
contradiction that must be settled before most coding. Next action: continue,
but scoped to Slice 1 only.**

I AGREE with the overall direction. `evidence-compiler-ranking` is the right
next topic: I verified that its prerequisites already exist (hot store with
`EvidenceItem` / `SuspectedCause` / `NextCheck` record kinds at
`src/hot_context_store.rs:18-39`; `DerivedContext` and all its sub-types at
`src/derived_context.rs:39-171`; `EvidenceQuery` / `EvidenceBundle` /
`EvidenceItem` / `EvidenceBudget` / `EvidenceKind` in `src/evidence.rs` and
`src/query.rs`; gold `evidence_bundle`, `suspected_causes`, `next_checks` in
every `fixtures/scenarios/*/expected.json`; `prior_incidents` in `input.json`).
These calls are sound and I endorse them:

- replacing the gold return path with a source-backed compiler;
- gold as comparison oracle only, never a compiler input;
- separating evidence strength from causal suspicion;
- semantic whole-item budget selection rather than `LIMIT N`;
- false-causality traps and missing-data uncertainty as first-class acceptance
  criteria;
- generating `suspected_causes` / `next_checks` as internal + store outputs now,
  before MCP exposes them.

I am **NOT** giving blanket approval of the field-level design. I approve
**starting at Slice 1 (compiler model + comparison shell) only.** Slice 1 is
exactly where finding **F1** below must be resolved, and slices 2–6 generate
twelve scenarios' worth of output against the comparison oracle that F1
governs. Building those generators before F1 is settled risks reverse-
engineering hand-authored gold numbers — a soft form of the gold-as-input
leak this design correctly forbids. Slices 2–6 unblock once F1's decision is
recorded in the design doc and proven by Slice 1 tests.

Per the design's own gate, treat the design as **direction-approved,
Slice-1-approved**, with F1 required as part of Slice 1's covered formal-doc
change. Other active reviewers still need to agree before coding begins.

### F1 — Token-cost contract contradicts the gold it must match (blocking for slices 2–6)

The design says, simultaneously:

- the estimator is `ceil(serialized_evidence_item_json_bytes / 4)`, must be
  "independent of fixture gold `token_cost`" (Token Budget Selection);
- `EvidenceBudget.tokens_used` is "computed by the compiler, not copied from
  fixture gold" (Outputs);
- comparison compares "token budget fields, with compiler-owned token
  estimates," and "Gold fixture artifacts are the required target for the
  current corpus" (Fixture Comparison Contract);
- comparison "must fail if token budget fields are copied from fixture gold
  instead of recomputed."

These cannot all hold. Concrete evidence from
`fixtures/scenarios/deploy-bad-rollout/expected.json`:

- Item `token_cost` values are `45 / 50 / 40 / 55 / 45`. `ev-1`'s serialized
  JSON is well over 400 bytes, so `ceil(bytes/4)` would be ~100+, not 45. The
  gold costs clearly track *claim-text* size, not full-item-JSON size — so the
  proposed estimator does not reproduce gold on day one.
- `budget.tokens_used` is `250`, but the five item costs sum to `235`. So
  `tokens_used` is not simply the sum of selected item costs either; the design
  never defines whether it includes bundle-level overhead (question,
  time_window, budget block) or what the 15-token gap represents.

Resolve before slices 2–6, and record it in the design's *Token Budget
Selection* and *Fixture Comparison Contract* sections. Pick one:

(a) define the estimator to match gold's authoring scheme (e.g. claim/text
    tokens) so compiler output equals gold;
(b) regenerate the gold `token_cost` / `tokens_used` fields from the chosen
    deterministic estimator as part of this topic, and state explicitly that
    this is estimator-driven regeneration, not "gold as input"; or
(c) declare token fields a structural/tolerance comparison rather than exact
    equality.

Also define `tokens_used` precisely (sum of selected item costs vs. sum +
fixed bundle overhead). The `235 vs 250` gap must be explained, not absorbed.

### F2 — Exact-match feasibility for hand-authored free text (substantive)

`claim`, `reasons`, `action`, `rationale`, `expected_signal`, and `hypothesis`
are hand-authored natural language in the gold (e.g. `ev-5`'s claim "The
orders-pg database is the cause of the checkout failures." carried with
`direction: weakens`). A deterministic from-scratch compiler can only reproduce
these verbatim if either the gold was authored from the compiler's own
templates, or the comparison on free-text fields is structural/semantic rather
than exact. The design's Fixture Comparison Contract lists `claim` among
compared fields without saying which fields are exact vs. structural. State,
per field, the comparison mode. This is the same axis as F1 (option b vs c) and
should be decided together in Slice 1.

### F3 — `get_evidence_bundle` is more than a "stub"; reframe the transition (accuracy)

The design repeatedly calls the current path a "fixture-gold stub." In fact
`get_evidence_bundle` (`src/query.rs:116-142`) already validates the query,
loads the gold bundle, then enforces budget fit, `require_raw_refs`,
`require_counter_evidence`, resolves every `source_ref` through a real
`HotContextStore`, and checks query-context selection. Only the *bundle source*
(`load_bundle_from_case`, i.e. gold) is a stub; the surrounding acceptance
checks are a reusable asset. Reframe Slice 6 / the Compiler Entry Point section
as "keep these existing acceptance checks; swap the bundle source from gold to
`compile_evidence`," not "replace the stub." This both improves accuracy and
prevents the implementor from discarding validation that the compiler must keep
satisfying.

### F4 — `SuspectedCause` / `NextCheck` types are net-new; say so (accuracy)

`SuspectedCause` and `NextCheck` exist only as `StoredRecordKind` enum variants
(`src/hot_context_store.rs:18-39`). There are **no** Rust structs for them yet,
and nothing currently parses gold `suspected_causes` / `next_checks`. The
"Suggested runtime shape" structs, their gold parsers, and their comparators are
all greenfield Slice 1 work. Similarly, `EvidenceQuery.intent` is a nested
`EvidenceQueryIntent { question, hypothesis }` (`src/query.rs:19-37`), not a
flat field — the design's Inputs list reads as if `intent` and `hypothesis`
were separate. State both so the work isn't assumed to already exist.

### F5 — Pin the estimator's serialization to be deterministic (nit)

"`serialized_evidence_item_json_bytes`" is serializer-dependent (field ordering,
whitespace, float formatting). For the estimator to be deterministic the design
must name a canonical serialization (stable key order, no incidental
whitespace). Relevant regardless of which F1 option is chosen if the byte-based
formula survives.

### Answers to the implementor's eight questions

1. **Phase-by-phase. Slice 1 approved now; slices 2–6 gated on F1.**
2. Option 1 (`get_evidence_bundle` → compiler) approved as the target, with the
   F3 reframing: keep the existing acceptance checks. A temporary coexisting
   gold path is acceptable only inside Slice 6 and must be removed by its end.
3. Yes — `suspected_causes` / `next_checks` as internal + store outputs now.
   Agree.
4. The oracle-only intent is clearly stated and the "comparison must fail if…"
   list is strong. The residual leak risk is the *inverse* one F1/F2 create:
   pressure to reverse-engineer hand-authored gold numbers/text. Resolving F1
   and F2 closes it.
5. Yes — causal classification of nearby changes belongs in the compiler; 5B
   timelines stay non-final context. Agree.
6. Deterministic estimator + whole-item selection are acceptable in principle.
   The specific `ceil(bytes/4)` formula is **not** yet acceptable (F1, F5).
7. Covered by (3): yes.
8. Scope exclusions are tight and explicit; agree. Watch item: this milestone is
   large (eight generators + scoring + suspected causes + next checks + budget +
   store + false-causality). I endorse the six-slice plan as the risk control —
   do not collapse slices to land it faster.

### Verification

No code verification this design-only round. I read `docs/review-framework.md`,
`docs/core/evidence-compiler-ranking.md`, and the existing
`src/query.rs`, `src/evidence.rs`, `src/hot_context_store.rs`,
`src/derived_context.rs`, and `fixtures/scenarios/deploy-bad-rollout/expected.json`
to check the design's claims against the codebase. Branch confirmed
`evidence-compiler-ranking`.
