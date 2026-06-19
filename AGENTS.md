# AGENTS.md

This file gives coding agents a compact orientation for this repository.

## Project

Janus is an early Rust project exploring an AI-first evidence backend for
OpenTelemetry-shaped data.

The core question:

> After OpenTelemetry produces and exports telemetry, how should a backend store,
> organize, summarize, and retrieve that data if the main consumer is an AI
> agent rather than a human dashboard user?

## Current State

- The repository is intentionally early.
- `src/main.rs` is still scaffold-level code.
- The main source of truth is `docs/core/what_and_why.md`.
- `README.md` is the public project overview.
- Local research material is ignored by git. Do not cite local-only notes from
  public docs, code comments, prompts, or agent instructions.

## Design Boundary

Janus is not trying to replace OpenTelemetry.

Janus is not trying to be a full APM product first.

Janus is not trying to be the RCA agent itself.

Janus should be an evidence substrate for agents:

- ingest OTel-shaped telemetry;
- preserve source references;
- resolve operational entities;
- derive relationships, anomaly windows, log patterns, timelines, and evidence
  bundles;
- expose investigation APIs and MCP tools;
- keep uncertainty, counter-evidence, and provenance visible.

## Core Terms

- `Evidence IR`: structured operational evidence for agents.
- `Evidence bundle`: a bounded set of evidence selected for a question,
  hypothesis, time window, entity, and token budget.
- `Hot layer`: recent rich working memory for agent investigation.
- `Warm layer`: incident memory, summaries, representative examples, and
  recurring-pattern retrieval.
- `Cold layer`: durable understanding and backlinks, not necessarily full raw
  telemetry retention.

## Implementation Priorities

1. Keep the first implementation small and measurable.
2. Prefer practical storage and clear contracts over novel storage machinery.
3. Do not build dashboard-first features unless they support the evidence
   contract.
4. Preserve source references for anything summarized or ranked.
5. Treat false causality as a core failure mode.
6. Add an eval harness early; the project must prove that the same agent works
   better with Janus evidence than with raw backend access.

## Useful Commands

```bash
cargo build
cargo check
cargo test
cargo fmt
cargo clippy
```

## Before Editing

Read `docs/core/what_and_why.md` first for design work.

Keep generated research material out of git unless explicitly requested.
