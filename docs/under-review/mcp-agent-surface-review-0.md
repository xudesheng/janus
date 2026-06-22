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
