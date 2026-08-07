# DESIGN — `lockproof`

## Scope
Lockfile diff plus provenance verification. **MVP:** structural, semantic diff
for `package-lock.json`, `Cargo.lock`, `uv.lock`, as a CLI and a GitHub Action
that comments on PRs. **Full:** yarn/pnpm/`go.sum`; Sigstore signature + SLSA
provenance + hash verification; policy gates.

## The permanent non-goal
**No typosquat heuristics. No "suspiciousness" scores. No behavioural ML.**
Anything that can cry wolf is banned (Appendix C). lockproof states facts and
verifies signatures; it never guesses. A false positive here costs the brand
more than a missed detection.

## Relationship to the kernel
Lockfiles are just JSON/TOML/YAML. **lockproof is a konflux head plus a
verification layer** — the clearest public demonstration that the kernel is real.

## Proof obligations (MASTER_PLAN §4.6)
| ID | Statement |
|----|-----------|
| P1 | Parse fidelity on the top-1,000 real lockfiles per ecosystem — zero parse failures, K1 round-trip |
| P2 | Verification differential vs cosign and npm/cargo native checks — full agreement or triaged divergence |
| P3 | Diff completeness golden suite: each seeded change surfaces exactly once, correctly classified |
| P4 | Deterministic reports |

## Current milestone
**Phase 0 — scaffold.** Phase 4. Corpus lands in `corpora/lockfiles/`.
