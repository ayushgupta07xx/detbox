# DESIGN — `core-formats`

## Scope
The `Format` trait (MASTER_PLAN §3.2) and one implementation per format. The
only place format-specific knowledge is allowed to live.

**In scope:** per-format parse/serialize over `core-cst`, semantic views, merge
hints (K8s list identity, Helm/kustomize awareness, TF block identity),
conformance-suite adapters.
**Out of scope:** the tree representation itself (`core-cst`), diff/merge
algorithms (konflux), query language (strukt), any I/O.

## Invariants
| ID | Statement | How it is checked |
|----|-----------|-------------------|
| F1 | `parse` never panics — hostile, non-UTF-8, truncated input included | per-format fuzz target |
| F2 | `serialize` is total; no unserialisable `Cst` state | property test |
| F3 | K1 composes: `serialize(parse(x)) == x` whenever `parse` succeeds | golden + corpus + fuzz |
| F4 | The semantic view is derived, never authoritative — output bytes come from the CST | property test: edits via the semantic view still satisfy K2 |

## Rollout order (do not reorder without an ADR)
yaml, json (Phase 1) → toml, hcl (Phase 2) → csv, jsonl, logfmt (Phase 3) →
lockfiles (Phase 4). PDF is architecturally different and lives in
`tools/pdfsurgeon`; it still answers to `core-verify`.

## Current milestone
**Phase 0 — scaffold.** The trait is quoted in the crate docs but is not code:
its associated types depend on ADR-001 (CST representation).
**Next: konflux M1** — YAML and JSON, K1 green.

## Proof obligations this crate carries
- konflux **P4**: yaml-test-suite, JSONTestSuite, toml-test pass rates published
  as badges, including honest failure lists.
- Every product's format coverage claim.
