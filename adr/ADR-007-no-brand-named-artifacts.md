# ADR-007: No brand-named crate or binary exists until D1

**Date:** 2026-08-07 · **Status:** proposed

## Context

MASTER_PLAN §2 specifies "a single multicall binary `<b>` (BusyBox-style), plus
symlinked names," and §16 makes the umbrella brand and that binary's name
**Decision D1 — Ayush's alone**, gated on a crates.io / npm / GitHub org /
domain / trademark check. Phase 0 has to scaffold a workspace without pre-empting
that. A placeholder name would be the easy move, and it would end up in a
`Cargo.toml`, a doc link, a CI job name and a README — and then it would be
load-bearing.

## Options

- **A — Invent a placeholder crate name now and rename later.** Fast, and rename
  churn is exactly where a placeholder survives into a published artifact.
- **B — Ship no brand-named artifact until D1.** Costs the multicall binary for
  one phase; nothing has to be un-named later.
- **C — Ask Ayush to decide D1 before scaffolding.** Correct in principle, and
  it blocks all of Phase 0 on a decision that is deliberately downstream of the
  validation read.

## Decision

**B.** The workspace root is a **virtual manifest** with no package name. Every
crate is named for what it *is* — `core-cst`, `core-formats`, `konflux`,
`strukt` — all of which are fixed by the master plan and none of which is the
umbrella brand. **No multicall binary crate exists.** The tool crates are
libraries with doc comments; they gain binaries as their milestones arrive.

The repository directory is `detbox` and the remote is
`github.com/ayushgupta07xx/detbox`. That is a working directory name, **not a
brand decision**, and nothing in the workspace derives an identifier from it.

The `<b>` multicall binary lands in the same PR as D1, together with the
symlinked per-tool names.

## Consequences

- No `<b> merge` entry point during Phase 1. konflux ships its own binary at M4
  when it needs the git merge-driver integration; the multicall wrapper is
  additive.
- D1 stays a real decision at the moment it should be made — after the
  validation read, alongside the flagship confirmation (D3) — rather than being
  quietly settled by whatever a scaffolding session typed.
- One extra PR later. That is the whole cost.

## Proof impact

None directly. Protects the §16 decision boundary and keeps §9.3's "every public
API change is a human review point" from being pre-empted by a placeholder.
