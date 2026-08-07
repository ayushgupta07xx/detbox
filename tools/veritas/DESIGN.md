# DESIGN — `veritas`

## Scope
A converter whose product *is* the fidelity report. Re-parses its own output,
structurally diffs it against the input's semantic tree, and emits a structured
receipt: `perfect` | `lossy-with-itemized-losses` | `refused`.

**Admission rule:** a conversion pair is admitted only when a **decidable**
fidelity report is possible. Depth over breadth, permanently.

**Non-goals:** "convert anything" breadth; media/codec formats (patent trap —
banned).

## Relationship to the kernel
`veritas` is `core-verify`'s report emitter productized (§3.3). Improvements
flow both ways; the receipt schema is shared.

## Proof obligations (MASTER_PLAN §4.5)
| ID | Statement |
|----|-----------|
| P1 | Round-trip on every pair claiming lossless |
| P2 | Report-completeness golden suite: seeded known-loss inputs produce exactly the expected loss items — **the report itself is under test** |
| P3 | Determinism: same input → same output bytes + same receipt |

## Current milestone
**Phase 0 — scaffold.** Phase 4. Order vs `pdfsurgeon` is **Decision D5**.
