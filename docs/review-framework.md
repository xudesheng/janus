# Review Framework

This file defines the review-document workflow for the Janus repository.

Audience: AI agents (primarily Claude Code, and any other coding agent working in
this repo). Optimize for exact execution over prose.

Use this workflow for design documents, implementation plans, review submissions, and implementation review records.

## Quick Rules For AI Agents

1. Create new review documents under `docs/under-review/`.
2. Name review documents as `docs/under-review/{topic}-review-{n}.md`.
3. Compute `n` only from existing same-topic files in the current `docs/under-review/` directory.
4. Do not scan `docs/archived/` when choosing `n`.
5. After creating or updating a review document, report its repo-root-relative path to the User.
6. Push covered code and formal docs before the review document; the baseline SHA points to that pre-review-document tree (see "Baseline SHA Rules").
7. The baseline SHA is frozen once written: never rewrite it to chase HEAD, and never use the review document's own commit as the baseline.
8. The Implementor commits and pushes any review document it creates as its own commit; the Reviewer commits and pushes its own review-section changes (see "Commit And Review Gate").
9. Janus has no `vendor/` directory today. If vendored dependencies are added later, do not modify them as part of this review workflow; document constraints, patch strategy, or upstream strategy instead.
10. Once a file contains any `## Review (by ...)` section, the Implementor MUST NOT edit the pre-review body of that file.
11. Implementor responses to review findings go into the next review document or formal docs, not into an Implementor-only section in the current reviewed file.
12. Reviewers may make non-substantive edits to their own current-round review section; substantive changes need User permission or a new dated note (see "Reviewer Section Format").
13. Do not edit historical review rounds. Archived files are read-only history.
14. Every submitted review round MUST identify the current milestone, state whether the covered work is on the critical path to that milestone, and describe the concrete progress made toward it.
15. Any proposal to defer milestone-critical work MUST explain why it cannot land in the current round and what the current round unblocks.
16. Reviewers MUST start by judging direction and milestone progress before listing local defects.
17. Submit `review-{n+1}.md` only when round `n` left actionable feedback OR the milestone work is incomplete; otherwise stop and report completion (see "Round Termination").

## Roles

- **Implementor**: writes the design, plan, implementation summary, or next-round response. Usually an AI agent.
- **Reviewer**: appends review feedback. Usually the User, Claude, another coding agent, or another User-selected reviewer.

## Directory Rules

```text
docs/
├── review-framework.md         # This file: the top-level process guide.
├── core/                       # Canonical design and vision (what_and_why.md, etc.).
├── process/                    # Supporting process docs (e.g. fixtures.md).
├── under-review/               # Active review documents. New topics start here.
├── archived/                   # Historical review documents.
│   └── <DATETIME_STAMP>/       # New archive batches, when requested by User.
└── <formal-doc>.md             # Other stable docs.

src/                            # Janus crate source (single binary crate today).
```

Janus is currently a single binary crate with no Cargo workspace. If the
responsibility chain in `docs/core/what_and_why.md` grows into real modules, the
natural structure is a Cargo workspace with one crate per responsibility (`crates/`,
and `libs/` for shared subprojects); when that happens, treat those paths as
covered formal records too.

There are no historical review files yet. When archive batches first appear under
`docs/archived/`, do not reorganize them unless the User explicitly asks.

## Review File Naming

Use:

```text
docs/under-review/{topic}-review-{n}.md
```

Rules:

- `topic` should be ASCII kebab-case by default, for example `evidence-ir-schema`, `get-evidence-bundle-contract`, or `entity-resolver-confidence`.
- Avoid spaces, Chinese characters, mixed casing, and special characters unless the User explicitly requests them.
- `n` starts at `0`.
- Pick `n` by scanning only current files in `docs/under-review/` with the same topic.
- Do not continue numbering from `docs/archived/`.

## Commit And Review Gate

Before a review document is submitted to the User or another reviewer:

- The code and formal docs the new review document covers MUST be committed and pushed first, if any. This includes the Implementor's own formal-doc edits for the round, which go with the round's code, not with the review-document commit.
- Reviewers evaluate that pre-review-document tree (the pushed covered changes, or the current pushed branch HEAD when there are none); the review document's baseline SHA points to it.
- Unrelated or out-of-scope files do not need to be included.

At the end of implementation, when a review document is created, the Implementor MUST commit and push it as its own commit after the covered code and formal docs were already pushed. That review-document commit is never the baseline SHA.

At the end of review, the Reviewer MUST commit and push their review-document changes. Because the Reviewer pushes `review-{n}.md` at the end of round n, it is already pushed before round n+1; the Implementor does not re-push the previous review document.

Covered formal records include, when relevant:

- `docs/` formal paths outside `docs/under-review/` and `docs/archived/` (including `docs/core/` and `docs/process/`)
- `./CHANGELOG.md` (once one exists)
- `./Cargo.toml`
- crate manifests under `crates/**/Cargo.toml` and local library manifests under `libs/**/Cargo.toml` (once a workspace exists)
- source code under `src/` (and under `crates/` / `libs/` once a workspace exists)

After creating or updating a review document, tell the User the path, for example:

```text
docs/under-review/evidence-ir-schema-review-0.md
```

## Baseline SHA Rules

Use baseline SHAs only to identify the already-pushed tree under review.

Correct:

- `review-{n}.md` is already pushed by the Reviewer (round n); push only this round's covered code and formal docs, if any.
- Create `review-{n+1}.md` with the baseline SHA = that pushed pre-review-document tree, or the current pushed branch HEAD when there are no covered changes.
- Commit and push `review-{n+1}.md` as its own commit at the end of implementation, then leave the baseline frozen.

Wrong:

- Writing the SHA of the commit that adds `review-{n+1}.md`.
- After pushing the review document, rewriting its baseline to the new HEAD and re-committing; this chases its own SHA in an infinite loop.
- Creating an empty commit just to produce a SHA.
- Using the commit that only adds `review-{n+1}.md` as the baseline SHA.

When reading the baseline SHA, use the current working branch upstream. Do not hard-code `origin/main` unless the current branch is `main`.

A submitted review's baseline MUST identify a pushed-tree SHA that the reviewer can fetch. A review MUST NOT reference a local-only commit as its baseline.

`Local baseline SHA` is draft-only. Remove it before submission; submitted reviews use only `Baseline SHA` for the pre-review-document tree.

When the branch upstream matches the local HEAD before the new review document is committed, a single SHA labeled `Baseline SHA` is sufficient. After the review-document commit is pushed, `Baseline SHA` still points to the pre-review-document tree; it is frozen and does not need to match branch HEAD. Do not update it.

## Milestone And Direction Gate

Each review topic MUST have a current milestone. Milestone source precedence is: explicit User instruction first, then a formal design document, then an existing topic milestone. A review document may propose a milestone only when no higher-precedence milestone exists, and the Reviewer may reject that proposal. If multiple goals exist, use the nearest concrete deliverable as the current milestone.

The milestone MUST be a concrete artifact or measurable outcome (a committed file, a captured number, a shipped behavior), NOT a design-doc section id or "subsystem complete". Critical path is judged against producing that milestone artifact, not against a formal document's internal section order. Measurement, automation, and other tooling is on the critical path only insofar as the current milestone artifact cannot be produced without it; tooling beyond that need is off-critical-path even when a design doc lists it.

Every submitted review document MUST include a short direction statement before implementation details:

- `Current milestone`: the deliverable this round is supposed to move toward.
- `Critical path`: whether this round's covered work is directly required for that milestone.
- `Milestone progress`: what concrete capability, test, artifact, or decision this round adds.
- `Deferred milestone work`: any milestone-critical work not done this round, with a reason it could not land now.

If the covered work is not on the critical path, the Implementor MUST either cite explicit User approval or keep the work out of the review loop until the current milestone is unblocked.

Deferrals are exceptions, not defaults. A deferral is acceptable only when the current round removes a real blocker, reduces meaningful risk, or is explicitly requested by the User. If the same milestone-critical item is deferred in two consecutive rounds, the Reviewer MUST stop or redirect rather than accept a third deferral, unless the User explicitly approves continuing.

## Review Document Structure

Each `review-{n}.md` uses this skeleton. The Implementor writes everything above the reviewer marker; the Reviewer appends below it.

```markdown
# {Topic} Review {n}

- Baseline SHA: `<pushed pre-review-document tree>`
- Current milestone: <deliverable this round moves toward>
- Critical path: yes | no — <why>
- Milestone progress: <concrete capability, test, artifact, or decision added>
- Deferred milestone work: none | <item + why it could not land now>

<response to the previous round's findings, plus this round's implementation summary (a first round has no prior findings)>

## Verification

<commands run and their results, or "no code verification this round">

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by <reviewer-name>)

### Direction Verdict

<on critical path? moves the milestone? next action: continue | redirect | stop>

<review feedback>
```

This skeleton is the single source for the document layout. Header field semantics are defined in "Milestone And Direction Gate"; the reviewer block's rules are defined in "Reviewer Section Format". `## Verification` is required every round; a diagnosis-only round states "no code verification this round".

## Multi-Round Flow

For a topic with multiple rounds:

1. Implementor commits and pushes the round's covered code and formal-doc changes, if any.
2. Implementor creates `topic-review-0.md` with the baseline SHA pointing to the pre-review-document tree.
3. Implementor commits and pushes `topic-review-0.md` as its own commit, then leaves the baseline SHA frozen.
4. Reviewer appends `## Review (by <name>)` and pushes the change.
5. Implementor starts round 1 by pushing the round's covered code and formal-doc changes, if any.
6. Implementor creates `topic-review-1.md` (baseline = the pre-review-document tree) and pushes it as its own commit.
7. Reviewer appends a new review section to `topic-review-1.md` and pushes.
8. Repeat with `topic-review-2.md`, `topic-review-3.md`, and so on, until "Round Termination" applies.

Do not append an Implementor reply below the review section in the same file. Put the response in the next round.

A round may be diagnosis-only when the Implementor wants to align with the Reviewer on contracts, scope, or trade-offs before writing code. The round's `## Verification` section then states "no code verification this round"; the next round carries the implementation and references the diagnosis-only round. Its `Milestone progress` is the decision or alignment it secures, and its critical-path justification is that it unblocks the next round's code.

## Round Termination

The round loop has an explicit exit; it does not run until manually stopped.

Submit `review-{n+1}.md` only when at least one holds:

- Round `n`'s review left actionable feedback: a defect, an open question, or a new requirement.
- The current milestone's covered work is not yet fully implemented.

If round `n`'s review reports no defects and no new requirements AND the milestone work is complete, do NOT submit a new round. Report completion to the User and wait for the next instruction.

If the next step is unclear, do NOT stall and do NOT submit an empty round: submit a diagnosis-only round (see above) that asks the Reviewer the blocking question. "Unclear next step" is a reason to ask, never a reason to stop.

## Locking Rules

### Before Any Review Section Exists

If a review document has no `## Review (by ...)` section:

- The User may ask the Implementor to revise the same file.
- The Implementor may fix facts, structure, typos, or formatting in that same file when explicitly asked.

### After A Review Section Exists

Once the first `## Review (by ...)` section exists:

- The Implementor MUST NOT edit anything before the first review section.
- The Implementor MUST NOT add `### Implementor reply` or similar Implementor-only sections to that file.
- Substantive responses MUST go into the next review document or formal docs.
- The reviewer may make non-substantive edits to their own current-round review section; substantive changes need User permission or a new dated note (see "Reviewer Section Format" for the exact boundary).

Historical rounds are locked records. Do not edit earlier rounds to synchronize wording, baseline SHAs, or conclusions.

## Reviewer Section Format

Reviewers append their section below the reviewer marker, in the layout shown in "Review Document Structure".

Rules:

- Each reviewer gets a separate section.
- A reviewer may append another section with the same name.
- The first substantive reviewer paragraph MUST be the direction verdict. It should answer whether the covered work should be done now, whether it moves the current milestone forward, and whether the next action is continue, redirect, or stop.
- Reviewers may raise direction-level findings such as off-critical-path work, scope creep, over-engineering, or repeated deferral. These findings may be higher severity than local implementation defects.
- If the direction is wrong, lead with that and avoid spending review surface on minor local defects unless they are needed to justify the redirect.
- A reviewer may edit their current-round review section without prior permission for non-substantive changes (typos, formatting, broken links, factual corrections such as wrong test counts, clarifying ambiguous wording). Substantive changes — flipping a finding's severity, withdrawing a recommendation, reversing a conclusion — require User permission or should be appended as a new dated note within the same section.
- Reviewers do not edit the Implementor body.
- Reviewers do not edit historical rounds.

## Archive Rules

Only archive when the User explicitly asks.

Do not infer that a topic is complete and archive it automatically.

Default archive operation:

```text
docs/under-review/{topic}-review-*.md
  -> docs/archived/<DATETIME_STAMP>/
```

`DATETIME_STAMP` format:

```text
YYYY-MM-DDTHHMMSS
```

Example:

```text
2026-04-15T143000
```

This `DATETIME_STAMP` format is authoritative for this workflow. Any older project-specific guidance using a different separator or shape defers to this format.

If the User asks for a different archive layout, follow the User's instruction.

After archiving, treat archived files as read-only history.

## Formal Document Reference Rules

Formal docs are:

- `docs/` files outside `docs/under-review/` and `docs/archived/` (notably `docs/core/` and `docs/process/`)
- `./CHANGELOG.md` (once one exists)
- `./Cargo.toml`
- `AGENTS.md`
- `CLAUDE.md`
- `README.md`
- other stable root-level project records

Formal docs MUST NOT depend on review documents as the source of truth.

Rules:

- Do not add Markdown links from formal docs to specific files under `docs/under-review/` or `docs/archived/`.
- If a conclusion from a review document is stable, copy the conclusion into the formal doc.
- Review and archive paths can move; formal docs must remain stable.

Current Janus policy: there are no formal-doc exceptions. If an exception is needed later, add it explicitly to this file with the exact file, allowed reference style, and boundary.

## References Inside Review Or Archive Docs

Files under `docs/under-review/` and `docs/archived/` may reference other review/archive files.

Those references do not need to be fixed when files are moved or archived. These files are historical records, not live specs.

## Changelog Rules

Janus does not have a `./CHANGELOG.md` yet; create one only when there is a first user-visible release to record. Once it exists, review-document operations do not go into it.

Do NOT add changelog entries for:

- creating `{topic}-review-{n}.md`
- appending `## Review`
- creating `{topic}-review-{n+1}.md`
- archiving review files
- mentioning "based on review-N" inside implementation work

Add changelog entries only for user-visible implementation, formal-doc, feature, fix, or breaking behavior changes.

## Version Cut Rules

Do not bump versions unless the User explicitly asks.

Janus's main version is:

```text
./Cargo.toml [package] version
```

Janus is a single binary crate today, so there is one version field. If a Cargo
workspace is later introduced, the main version becomes
`./Cargo.toml [workspace.package] version`, and any crate under `crates/` or `libs/`
with its own independent version is updated only when the User asks or when the
release task explicitly requires it.

Default behavior:

- Put user-visible implementation changes under `## [Unreleased]` in `./CHANGELOG.md` (once it exists).
- When the User asks for a version cut, move relevant `[Unreleased]` entries into the new version section.
- Synchronize the relevant `Cargo.toml` version fields during the version cut.

## External Material Rules

Formal Janus docs MUST express Janus-owned constraints and acceptance criteria.

If a review round studies external material such as the OpenTelemetry specification,
OTLP, the OpenTelemetry Collector, OTel semantic conventions, or existing AI-native
APM projects:

- Review docs may mention the external source.
- Formal docs should translate the lesson into Janus-specific requirements.
- Do not make an external repository or specification the normative source for Janus behavior. OTel stays the upstream *input contract*; how Janus stores, derives, and exposes evidence is Janus-owned.

## Schema And Agent Surface Checks

When a change affects the contracts Janus exposes to agents or ingests from upstream,
review both runtime behavior and external validator/consumer acceptability. These are
a separate acceptance surface from local runtime correctness.

Examples of Janus-relevant surfaces:

- the Evidence IR JSON Schema (`EvidenceItem`, `EvidenceBundle`)
- MCP tool input/output schemas (e.g. `get_evidence_bundle`)
- OTLP-shaped ingest payloads (traces, metrics, logs, resources) and change-event payloads
- any strict JSON Schema sent to an MCP client or LLM tool-use validator

Validator/consumer compatibility is a separate acceptance surface from local runtime correctness.

Example: if a JSON Schema uses `type: array`, verify that `items` is declared when the external validator (an MCP client, or an LLM tool-use runtime) requires it.

## Shorthand Resolution

When the User mentions:

- "review document"
- "review doc"
- `{topic}-review-{n}.md`
- `{topic}-review-{n}`

Default to:

```text
docs/under-review/{topic}-review-{n}.md
```

Use a different path only when the User explicitly provides one, for example an archived path.

## Working In Git Worktrees

If a topic is being worked on in a dedicated worktree or feature branch (anything other than `main`):

- The Implementor MUST choose an issue-scoped, unique topic. Generic topics like `ingest-fix`, `evidence-bug`, or `cleanup` are not permitted in worktrees. Use a specific scope tied to the issue: `evidence-ir-source-refs`, `entity-resolver-confidence`, `get-evidence-bundle-token-budget`, or include an issue identifier when one exists (e.g., `evidence-ir-issue-1234`).
- `n` is computed from the worktree's own current `docs/under-review/` checkout.
- Because each worktree uses a different topic, concurrent worktrees do not share an `n` namespace and cannot collide on review-document names.
- Baseline SHA references the worktree's branch upstream (already required by "Baseline SHA Rules"). Do not hard-code `origin/main` for worktree branches.
- If a worktree's branch is later merged to `main`, its review documents under `docs/under-review/` come along unchanged. No renumbering is needed because the topics are unique.

## Final Checklist For AI Agents

Before replying to the User, check:

- Did I read this file and the review workflow summary in `AGENTS.md`?
- If I created a review doc, is it under `docs/under-review/`?
- Is the file name exactly `{topic}-review-{n}.md`?
- Did I compute `n` only from current `docs/under-review/` files?
- Did I report the review document path to the User?
- If submitting for review, is the covered code/formal-doc tree (including my own formal-doc edits this round) already pushed before the review document?
- If acting as Implementor and I created a review document, did I push it as its own commit and leave the baseline SHA frozen (not rewritten to the new HEAD)?
- Did the review document state the current milestone, critical-path status, concrete milestone progress, and any milestone-critical deferrals?
- Before submitting `review-{n+1}.md`, did I confirm round `n` left actionable feedback or the milestone work is incomplete? If the work is done with no open feedback, did I stop and report instead of emitting an empty round?
- If acting as Reviewer, did I write a direction verdict before local findings?
- If Janus later adds a `vendor/` directory, did I avoid modifying it in this workflow?
- If the current review file already contains `## Review (by ...)`, did I avoid editing the Implementor body?
- If acting as Reviewer, did I only edit the current-round review section and avoid historical rounds?
- If acting as Reviewer, did I commit and push any review-document changes I made at the end of review?
- If working in a worktree (non-`main` branch), did I choose an issue-scoped, unique topic instead of a generic one?
