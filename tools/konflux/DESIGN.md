# DESIGN — `konflux`

## Scope
Structural diff and 3-way merge for structured config. **MVP:** YAML + JSON
diff/merge as a git merge-driver + `git mergetool`, with byte-identical
round-trip. **Full:** TOML + HCL, K8s semantic merging, Helm/kustomize
awareness, TUI conflict resolver, `--check` CI mode.

**Non-goals:** general text merge (delegate to git); auto-resolving semantic
conflicts it cannot prove safe; any AI-assisted resolution.

## Deep core
`core-cst` parsing → semantic-tree matching (Chawathe/GumTree-class, Dijkstra-style
structural diff à la difftastic) → true 3-way merge over trees with path-based
conflict detection → **format-preserving serialisation of the merged result**
(the genuinely hard part, and the reason the gap exists) → conflict presentation
as structured, span-anchored blocks.

## Invariants
- **Conflict-on-uncertainty.** If soundness cannot be proven, emit a conflict.
  Never resolve to be impressive.
- **Auto-resolution rate is measured, published, and never bought with
  soundness.** Including the cells where diff3 and Mergiraf beat us.

## Milestones and their proof obligations (MASTER_PLAN §4.1)
| M | Work | Proof obligation |
|---|------|------------------|
| M1 | CST + K1 for YAML/JSON | **P1** round-trip; **P4** conformance suites wired |
| M2 | Structural diff, side-by-side CLI output | **P4** + differential runner online |
| M3 | 3-way merge core + P2/P3 harness | **P2** merge algebra; **P3** soundness suite |
| M4 | git merge-driver + mergetool + install one-liner | **P3** on replayed OSS merges |
| M5 | K8s semantic layer | **P3** extended: list-by-key cases |
| M6 | TUI conflict view | T1–T4 of `core-tui` |

**Launch gate:** P1–P4 green in public CI, benchmark table (including where
Mergiraf/diff3 win), 60-second screencast, README per §12.

## Current milestone
**Phase 0 — scaffold.** M1 does not begin until Ayush accepts Phase 0. M1 is
blocked on ADR-001 (CST representation), which follows a 2-day spike.

## Status of the flagship claim
**Decision D3, Ayush's alone.** Confirmed or overturned by the Phase 0
validation read (§11). Do not assert "flagship" in any public artifact until D3.
