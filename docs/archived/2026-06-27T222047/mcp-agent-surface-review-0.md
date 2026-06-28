# MCP Agent Surface Review 0

- Baseline SHA: `4b4a8e664b755770ffd41ccf0b671dd919f9ede9`
- Current milestone: Milestone 7 Agent Surface V1, a reviewed local agent-facing `get_evidence_bundle` tool surface with strict MCP-facing schemas and a deterministic fixture-backed smoke path.
- Critical path: yes - this design approval is required before any Rust implementation, and this topic decides the first external agent boundary for Janus evidence.
- Milestone progress: submits `docs/core/mcp-agent-surface.md` for design review, confirms this round is design-only, and asks reviewers to resolve the V1 scope, transport, schema, selector, and output-shape choices.
- Deferred milestone work: all implementation, schema artifacts, runtime handlers, and smoke tests are deferred until all active reviewers agree on the design direction in their `Direction Verdict`.

This is the first review round for `mcp-agent-surface`; there are no prior review findings to answer.

I read the current design draft in `docs/core/mcp-agent-surface.md` against the core Janus context and the milestone chain in `docs/core/roadmap.md`, `docs/core/evidence-ir-schema.md`, `docs/core/get-evidence-bundle-contract.md`, `docs/core/evidence-compiler-ranking.md`, `docs/core/fixture-otel-simulator.md`, and `docs/core/otel-ingest-prototype.md`.

I did not change the formal design document in this round. The draft already states the necessary implementation gate: no Rust implementation should start until every active reviewer agrees on the design direction. This review should decide whether the draft is ready to become the implementation contract, or whether another design-only round is needed first.

## Direction Request

Reviewers should first decide whether this topic is the right Milestone 7 next step after `evidence-compiler-ranking`.

The direction verdict should say one of:

- continue: the topic is on the critical path and the design is ready for implementation, either whole-topic or phase-by-phase;
- redirect: the topic is premature, too broad, or pointed at the wrong agent surface;
- stop: the milestone should not proceed in this form.

If the verdict is `continue` but only for a phase, please name the approved phase explicitly.

## Review Focus

Please focus on these decisions before local implementation details:

1. V1 tool scope: should Agent Surface V1 expose only `get_evidence_bundle`, or may it include a small read-only companion surface? The draft prefers `get_evidence_bundle` only unless reviewers approve extra output.
2. Runtime shape: should the first implementation be a real stdio MCP server, a protocol-shaped command that lists/calls the same tool, or both? If a command-shaped slice is accepted, reviewers should state exactly what remains before the topic is complete.
3. Schema boundary: is the proposed `schemas/mcp/` tool-facing schema layer strict enough for external tool validators without forcing a broad project-wide schema dialect migration?
4. Temporary selector: may `scenario_id` remain in the V1 tool input as a clearly labeled fixture/demo selector? If yes, what production replacement name should the design reserve, such as `context_selector`, `workspace_selector`, or another reviewed term?
5. Output envelope: should V1 return only `{ "bundle": EvidenceBundle }`, or should it also expose compiler-produced `suspected_causes` and `next_checks` in the same envelope?
6. Error model: are the proposed stable tool error categories sufficient, and are any categories missing for a real MCP client or strict tool-use runtime?
7. Phase strategy: should reviewers approve the whole V1 design before any coding, or approve implementation phase by phase starting with schema artifacts and handler boundaries?
8. Evidence boundary: does the draft keep Janus from becoming an RCA prose API and preserve the contract that the agent receives structured, source-backed Evidence IR?

## Implementation Position

My recommended implementation path, if reviewers agree with the design direction, is phase-by-phase:

1. Tool schema model and committed/generated MCP schema artifacts.
2. Transport-independent tool handler that maps arguments to the existing `EvidenceQuery` path and stable tool errors.
3. Local invocation surface, with stdio MCP preferred if the required protocol subset stays small; otherwise a reviewed protocol-shaped command plus a tracked remaining MCP-server gap.
4. Smoke test using an existing fixture-backed query and validating structured Evidence IR JSON.

I recommend keeping V1 output to `{ "bundle": EvidenceBundle }` unless reviewers explicitly want `suspected_causes` and `next_checks` in the first agent-facing envelope. That keeps the first external surface small while still proving the agent can call Janus and receive auditable evidence.

## Verification

No code verification this round. This is a design-only review submission.

Repository checks performed:

- `git branch --show-current` returned `mcp-agent-surface`.
- `git status --short --branch` showed a clean worktree tracking `origin/mcp-agent-surface` before this review document was created.
- `git log -1 --oneline --decorate` showed `4b4a8e6 (HEAD -> mcp-agent-surface, origin/mcp-agent-surface) docs: add mcp agent surface design`.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude)

### Direction Verdict

Continue. This is the right topic, on the critical path, and the design is sound.
I **agree with the design direction** with three clarifications that should land in
the formal design doc before the implementation gate closes (none changes the
direction; all are containable in one design-only round 1).

- On critical path: yes. Roadmap "Near-Term Review Order" puts `mcp-agent-surface`
  immediately after `evidence-compiler-ranking` (now archived/complete), and
  Milestone 7's first deliverable is exactly "first MCP tool for
  `get_evidence_bundle`". The internal evidence path is real today
  (`src/query.rs:121-154` compiles a bundle through fixture replay -> derived
  context -> `compile_evidence`), so the missing layer genuinely is the agent-facing
  boundary, not more ingestion.
- Moves the milestone: yes. As a design-only round its milestone progress is the
  alignment it secures; the header correctly frames that.
- Next action: continue, **phase-by-phase**, starting with schema artifacts +
  transport-independent handler (the implementor's recommended path), but the
  design gate should only be treated as closed once the clarifications below are
  folded in and every other active reviewer (notably the User) has also recorded
  agreement. I can only speak for one reviewer.

I verified the design's factual claims against the tree at the baseline SHA:
`get_evidence_bundle` routes through the compiled path (not gold bundles);
`compile_evidence` already produces `suspected_causes` and `next_checks`
(`src/evidence_compiler.rs:49-52`) which the bundle path discards today; schemas
emit draft-07 (`schemas/evidence-ir/*.schema.json`, `$schema` =
`http://json-schema.org/draft-07/schema#`). All three claims hold.

### Answers To The Eight Decisions

1. **V1 tool scope** — `get_evidence_bundle` only. Agree. Smallest honest surface,
   and the `{ "bundle": ... }` envelope already leaves room to grow.
2. **Runtime shape** — stdio MCP server is the completion target; a
   protocol-shaped command is acceptable only as an intermediate slice. See
   finding F1: the roadmap acceptance criterion is "an external agent can call
   Janus", and a bespoke command does not satisfy that on its own.
3. **Schema boundary** — `schemas/mcp/` is the right separation. Approve, with the
   acceptance caveat in F2 (the validator used to certify the schemas must be
   representative of a real MCP/tool-use runtime, not a self-chosen lenient one).
4. **Temporary selector** — `scenario_id` may remain, but fix the
   optional-in-schema / required-at-runtime mismatch (F3) and reserve the
   production name now. Recommended reserved name: `context_selector`.
5. **Output envelope** — `{ "bundle": EvidenceBundle }` only. Agree with the
   implementor. Defer `suspected_causes` / `next_checks` to the optional slice.
6. **Error model** — categories are mostly sufficient, but one is missing (F4):
   `require_raw_refs` failures have no dedicated code, yet in code both raw-refs
   and counter-evidence collapse into `UnsatisfiedRequirement` (`src/query.rs:104`).
7. **Phase strategy** — approve the direction whole, implement phase-by-phase.
   Agree.
8. **Evidence boundary** — preserved. Scope and Out-of-scope correctly exclude RCA
   prose; consistent with `roadmap.md` "Deliberately Later" and `what_and_why.md`.

### Findings (conditions on closing the gate, by descending importance)

- **F1 — Pin the completion bar for the runtime surface (Decision 2).** The DoD
  accepts "a local agent-compatible or reviewer-approved MCP-shaped invocation",
  but Milestone 7's roadmap acceptance is concretely "an external agent can call
  Janus and receive structured Evidence IR JSON". A protocol-shaped command alone
  does not let an external agent call over MCP. Keep Option B as a permitted
  *intermediate* slice, but state in the DoD that the topic is **not** complete
  until a stdio MCP server answers a real `initialize` / `tools/list` /
  `tools/call` exchange. The design already gestures at this ("complete only when
  an external agent can call Janus through an MCP-compatible ... surface"); make it
  unambiguous that command-only does not satisfy DoD.

- **F2 — Schema acceptance must not be self-certifying (Decision 3).** "Verify the
  emitted schemas are accepted by the chosen local validator" lets the topic pass
  by picking a lenient validator. Per the framework's "Schema And Agent Surface
  Checks", validator/consumer acceptability is a separate acceptance surface.
  Name the validator and justify it as representative of a real MCP client / LLM
  tool-use runtime, document the emitted dialect (draft-07) explicitly in the
  committed artifacts, and record the concrete trigger for a future 2020-12
  migration. Keeping draft-07 for V1 is fine; self-selecting the judge is not.

- **F3 — `scenario_id` schema honesty (Decision 4).** The design carries forward
  "optional in schema, required by runtime" from the Milestone 2 contract. At an
  internal Rust boundary that is tolerable; at the agent tool boundary it is
  actively misleading — an agent reading the input schema sees an optional field
  that, if omitted, *always* errors (`src/query.rs:126-129` `expect`s it). Either
  mark `scenario_id` **required in the V1 tool input schema** (cleanest, since the
  fixture-backed runtime requires it) with a description that flags it as a
  temporary demo selector to be replaced by `context_selector`, or keep it
  optional but have the tool *description* state it is currently mandatory. I
  prefer required-in-V1. Also reserve `context_selector` in the doc so the eventual
  swap is a documented, not improvised, schema change.

- **F4 — Error model gap for raw-refs (Decision 6).** `require_raw_refs:true`
  failures are real and map to `UnsatisfiedRequirement` alongside counter-evidence
  (`src/query.rs:104-107`), but the proposed categories only name
  `counter_evidence_unsatisfied`. Either add `raw_refs_unsatisfied`, or replace
  both with one `requirement_unsatisfied` code carrying a `requirement` field
  (which mirrors the existing struct). Pick one so the mapping from
  `GetEvidenceBundleError` to tool error is total. While here, confirm the intended
  collapse of `FixtureReplay` / `HotStore` / `EvidenceCompile` /
  `MissingFixtureBundle` / `FixtureBundleParse` into `internal_error` is
  deliberate — it is defensible (those are server-side faults), but the design
  should say so rather than leave the mapping implicit.

### Nits (non-blocking)

- The output envelope is `{ "bundle": EvidenceBundle }`, but `compile_evidence`
  already computes `suspected_causes`/`next_checks` and the bundle path throws them
  away (`src/query.rs:140-142`). No action needed for V1; just note in the design
  that the optional slice (5) is a pure surfacing change with no new computation,
  which strengthens the case for deferring it cheaply later.
- "Schema generation is repeatable and tested" — please make the `schemas/mcp/`
  artifacts checked-in and diffed against freshly generated output in CI, the same
  way `generate_schemas.rs` covers evidence-ir today, so the committed agent
  contract cannot silently drift from the Rust types.

### Process / Framework Check

- Baseline SHA `4b4a8e6` correctly points to the pushed design-doc commit (the
  pre-review-document tree), not to the review-0 commit; it is frozen. Compliant.
- Covered formal doc (`docs/core/mcp-agent-surface.md`) was pushed before, and
  review-0 as its own commit after. Compliant.
- Header carries milestone / critical-path / progress / deferred, and
  `## Verification` correctly records "no code verification this round" for a
  design-only round. Compliant.
- Because round 0 leaves actionable feedback (F1-F4), a round 1 is warranted under
  "Round Termination". Round 1 may be another design-only round that folds F1-F4
  into `docs/core/mcp-agent-surface.md`; implementation should not begin until the
  design gate is closed by all active reviewers.
