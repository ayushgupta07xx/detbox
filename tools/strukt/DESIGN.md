# DESIGN — `strukt`

## Scope
Deterministic query and edit for every config format. **MVP:** a small,
deliberately boring jq-inspired path language; get/set/delete/insert; in-place
edit with K2 edit-locality. **Full:** structural grep across repos, format-aware
bulk refactors, shell completions, editor integration.

**Non-goals:** a Turing-complete query language; jq's full feature surface.

## Invariants
- Every edit satisfies `core-cst` **K2**: bytes outside the edited span are
  byte-identical.
- Edits are **idempotent**: applying the same edit twice equals applying it once.
- Output ordering and formatting are deterministic across platforms.

## Proof obligations (MASTER_PLAN §4.2)
| ID | Statement |
|----|-----------|
| P1 | K2 edit-locality, fuzz-verified |
| P2 | Differential vs jq on JSON: semantically identical results on a 10k-query corpus |
| P3 | Idempotence |
| P4 | Determinism across platforms |

## Why it ships second
~90% kernel reuse. It is the public proof of the platform thesis and the most
daily-useful tool in the set.

## Current milestone
**Phase 0 — scaffold.** Phase 2, weeks 10–14. Adds `toml` and `hcl` to
`core-formats`.
