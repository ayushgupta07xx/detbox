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
**M2 — structural diff.** M1 is finished: `core-cst` per ADR-001, YAML and JSON
parse/serialize, K1 green on 1,000 corpus files, conformance published
(ADR-009). The two M1 items still open are not M2's to close — P1's 72 fuzz
hours accrue on the clock, and the yaml reject-rate needs the block/flow context
that `semantic_view` brings *here*, at M2.

M2 starts at the oracle: a diff golden suite of hand-built cases where
line-based diff is wrong and structural diff is right. Confirm red first.

## Status of the flagship claim
**Decision D3 is made: konflux is the flagship** (2026-08-09, ADR-010).

It was decided **without** the §11 validation read — the posts in
`docs/validation/` were never sent. So "flagship" is a sequencing decision about
what gets built first, and it is **not** a claim that demand was measured.
Nothing public may imply otherwise, and all public wording is Ayush's regardless
(§16). The claim comes due at §14's kill rule after Launch 1.
