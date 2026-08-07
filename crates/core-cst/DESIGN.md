# DESIGN — `core-cst`

> Per MASTER_PLAN §9.1, every crate carries scope, invariants, and its current
> milestone. Read this and `ENGINEERING.md` before touching this crate.

## Scope

Lossless concrete syntax trees. Parsing and serialisation of *bytes* — never
semantics. The typed layer lives in `core-formats::semantic_view`.

**In scope:** token/trivia representation, span arithmetic, verbatim escape
nodes, edit application with byte-level locality, cursor/navigation API.

**Out of scope:** anything format-specific (that is `core-formats`), diffing,
merging, querying, I/O, CLI concerns.

## Invariants

| ID | Statement | How it is checked |
|----|-----------|-------------------|
| K1 | `serialize(parse(x)) == x`, byte-identical | golden suite + fuzz target + corpus sweep |
| K2 | After an edit, bytes outside the edited span(s) are unchanged | property tests + fuzz target |
| K3 | Identical input + identical op sequence → identical output bytes, every platform | determinism double-run gate + `clippy.toml` bans |

**Escape hatch.** Input the modelled grammar cannot represent is preserved as an
opaque verbatim node, never normalised. Preserving beats understanding.

**Panic policy.** `parse` never panics on any input, including hostile and
non-UTF-8 input. It returns a `ParseReport` with spans. `serialize` is a total
function. Enforced by the workspace `panic`/`unwrap_used`/`indexing_slicing`
denials and by the fuzz targets.

## Current milestone

**Phase 0 — scaffold.** No parser exists. `roundtrip_identity` exists solely to
make the K1 gates non-vacuous before there is anything to parse (ADR-003).

**Next: konflux M1** — CST + K1 for YAML and JSON.
Blocked on **ADR-001** (green/red tree vs owned token tree), which is written
after a 2-day spike comparing edit ergonomics and memory footprint. ADR-001 is
reserved; do not number anything else 001.

## Proof obligations this crate carries

- konflux **P1** (MASTER_PLAN §4.1): K1 on ≥1,000 real-world files plus ≥72
  cumulative fuzz-hours with zero violations.
- strukt **P1**: K2 edit-locality, fuzz-verified.
- Every product's determinism proof bottoms out in K3 here.
