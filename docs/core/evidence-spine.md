# Evidence Spine: The First Implementation Plan

This document is a concrete, near-term implementation plan for Janus. It is
grounded in [`what_and_why.md`](what_and_why.md) and assumes that doc as the
canonical source of truth. Where the two disagree, `what_and_why.md` wins and
this file should be corrected.

Status: exploratory plan, not a frozen spec. It exists to turn the design doc's
"初始赌注" (initial bet, `what_and_why.md` §"初始赌注") into a sequence of small,
verifiable steps.

## Where we are

- The canonical design doc (`docs/core/what_and_why.md`) exists and is the map.
- The code is still scaffold-level: `src/main.rs` prints "Hello, world!" and
  `Cargo.toml` has no dependencies.
- There is no Evidence IR, no primitive, no fixture, and no storage. That is the
  correct starting point — the design doc is deliberately a contract document,
  not an implementation spec.

## The thesis this plan serves

The design doc makes one falsifiable bet (`what_and_why.md` §"初始赌注", §"评估标准"):

> Given the same agent, the same incident, and the same time/token budget, Janus
> should put **fewer, more accurate, more auditable** evidence into the agent's
> context than raw-backend access does.

Everything below is the minimum needed to make that bet *testable*. It is not an
attempt to rebuild an observability backend.

The corollary that drives sequencing: **right now the contract is the product.**
The Evidence IR (the data shape) and `get_evidence_bundle` (the query shape) are
the load-bearing spine. Storage, ingestion, and derivation are downstream of the
spine and must not be built first.

## The plan (fused P1 + P2, with P3 and P4)

Four pieces, built together as one thin vertical slice rather than four
horizontal layers.

### 1. Evidence IR as Rust types + JSON Schema

Define the Evidence IR exactly as enumerated in `what_and_why.md` §"Evidence IR".
An `EvidenceItem` carries at least:

| Field | Meaning |
|---|---|
| `claim` | the statement this evidence supports or weakens |
| `kind` | metric anomaly, trace exemplar, log cluster, change event, dependency edge, profile hotspot, previous incident, counter-evidence |
| `time_window` | the interval over which the evidence holds |
| `entities` | related service / route / host / pod / deployment / tenant / dependency |
| `source_refs` | pointers back to raw spans / logs / metrics / profiles / change records (provenance) |
| `strength` | evidence strength — **not** the same as causal confidence |
| `direction` | `supports` \| `weakens` \| `contradicts` \| `neutral` |
| `freshness` | whether the evidence is still changing |
| `missing_data` | data this evidence depends on but currently lacks |
| `token_cost` | approximate cost of placing this item in agent context |
| `privacy_scope` | permission / tenant / sensitive-field redaction info |

Deliverable:

- `serde`-derived Rust structs and enums for the IR.
- A generated JSON Schema and one serialized example `EvidenceItem`.

Design constraints from the doc that must be representable, not optional:

- `source_refs` must be a first-class field, never elided. A summary that drops
  its source refs is not an evidence item (`what_and_why.md` §"Evidence IR":
  "Summary 只能是 evidence item 的一种展示或压缩形式，不能切断 source refs").
- `direction` + `missing_data` + `source_refs` are the **false-causality guard**
  (`what_and_why.md` §"设计优先级" point 7, §"从相关工作中提炼出的约束" point 2). An
  absent counter-evidence path or an empty `source_refs` must be visible in the
  data, not a silent default.

This is P1, and it is the spine's vertebra. Pin it first because every other
piece references it.

### 2. `get_evidence_bundle` as the first primitive

Define one investigation primitive end-to-end. Choose **`get_evidence_bundle`**,
not `build_timeline`, because the design doc over-specifies it and it is the only
primitive whose full request contract is written out (`what_and_why.md`
§"Token Budget 是查询约束", §205).

The request (`EvidenceQuery`) must carry, per that section:

- `question` and/or `hypothesis`
- `time_window`
- `entities`
- `max_items`
- `max_tokens`
- `require_counter_evidence`
- `require_raw_refs`
- `freshness` requirement
- `privacy_scope`

The response (`EvidenceBundle`) is **not** a filtered list. It is the answer to a
budgeted optimization problem (`what_and_why.md`: "不应只是 `LIMIT N`"):

- an *ordered* set of `EvidenceItem`s,
- both supporting and counter-evidence,
- a `missing_data` channel for "what I could not show you,"
- token-budget accounting (what was spent, what was dropped).

Deliverable for this step: the request/response Rust types plus a **stub**
implementation that returns a hand-built bundle. The retrieval can be faked at
first — the win is a frozen contract and JSON flowing end to end.

This is P2.

### 3. One hand-built incident fixture

Author a single small incident by hand — for example *a deploy that triggers a
downstream 5xx spike* — as:

- canned OTLP-shaped signals (a few traces, log records, metric points, and one
  change event), and
- the "gold" `EvidenceBundle` that a good agent should receive for a question
  like "why did service X start failing?"

The doc demands the eval harness appear early (`what_and_why.md`
§"从相关工作中提炼出的约束" point 5: "eval harness 必须尽早出现"; §"初始赌注" step 9). This
single fixture triples as:

1. the test data for the walking skeleton,
2. the first datapoint of the incident eval corpus,
3. a forcing function that keeps the IR and the primitive honest — if the gold
   bundle can't be expressed in the IR, the IR is wrong.

This is P3, promoted to co-equal with P1 because the whole value proposition *is*
the comparison it enables.

### 4. The thin walking skeleton

Wire the three pieces into the thinnest possible vertical slice:

```text
canned OTLP-shaped signals
      -> trivial in-memory store
      -> get_evidence_bundle (stub retrieval)
      -> Evidence IR (serialized to JSON)
      -> (optional) one MCP tool exposing get_evidence_bundle
```

Deliberately **out of scope** for the skeleton:

- a real storage engine (columnar / time-series / vector),
- entity resolution, anomaly detection, pattern clustering,
- ingestion from a live Collector,
- warm/cold layers and compaction.

The design doc explicitly endorses this restraint: components "首先是设计职责"
and a small implementation can start from "少量 tables、indexes、background jobs 和
MCP tools" (`what_and_why.md` §"架构含义"). The README adds: "Prefer practical
storage over novelty… start with a columnar store or simple local implementation
before inventing new storage machinery." Building the storage engine first is the
named trap; this plan walks around it.

This is P4.

## Why this sequencing

- **Contract before storage.** The bet is about evidence *shape and selection*,
  not write throughput or query latency (those are constraints, not the product
  — `what_and_why.md` §"评估标准"). Freezing the IR and the primitive contract is
  the highest-leverage work; storage can be swapped under a stable contract later.
- **`get_evidence_bundle` before `build_timeline`.** It is the only primitive the
  doc fully specifies, and it forces the token-budget and counter-evidence
  thinking that distinguishes Janus from "raw query with a LIMIT."
- **Fixture early, not last.** A gold bundle is the cheapest way to discover that
  the IR can't express something it must. It also seeds the eval corpus that the
  doc says must exist early.
- **Fake the retrieval first.** End-to-end JSON through frozen contracts is worth
  more than a real retriever behind an unstable contract. Real retrieval is a
  later substitution that the contract makes safe.

## Definition of done for this slice

The slice is done when:

1. `EvidenceItem` and `EvidenceBundle` exist as `serde` types with a JSON Schema
   and at least one serialized example.
2. `get_evidence_bundle(EvidenceQuery) -> EvidenceBundle` compiles and returns the
   gold bundle for the fixture.
3. The hand-built incident fixture is committed and used as a test.
4. The skeleton runs end-to-end and emits Evidence IR JSON; ideally one MCP tool
   exposes it.

None of this proves Janus can do RCA. It proves the one thing the design doc asks
the MVP to prove: that Janus can put structured, source-backed, budget-aware
evidence into an agent's context. If that holds, the storage and derivation work
that follows has a stable target to aim at.

## What comes after (not part of this slice)

In rough order, still grounded in `what_and_why.md` §"初始赌注" and §"架构含义":
replace stub retrieval with a real recent-window store → entity resolver (with
confidence) → relationship builder → anomaly windows → log/error pattern
clustering → a second primitive (`build_timeline` or `expand_entity_context`) →
the comparative eval (raw backend vs Janus) on a small grown corpus.
