# The Deterministic Toolbox

> Everything we ship is provably correct, every run.

**Status: Phase 0 — scaffold.** No product has shipped. Nothing here claims to
work yet, and this README will not claim otherwise until a gate says so.

The brand name and the multicall binary name are **not chosen** (Decision D1,
MASTER_PLAN §16). The repository directory name is a working name, not a brand.

---

## The thesis

We build local-first developer tools whose excellence is *deterministic and
machine-verifiable* — exact correctness, lossless transformations, reproducible
runs — never probabilistic quality.

**Priority order, permanent:** soundness → completeness → performance →
features. When they conflict, the lower number wins. A wrong answer once
destroys the brand.

**Privacy is a feature:** no telemetry, no phone-home, offline by default,
forever.

---

## Layout

| Path | What |
|---|---|
| `MASTER_PLAN.md` | The single source of truth. Read it first. |
| `ENGINEERING.md` | Rules of engagement, distilled from §0/§8/§9/Appendix C |
| `MILESTONES.md` | Live checklist, one section per phase |
| `adr/` | Every non-obvious decision, with its why |
| `crates/core-cst` | Lossless concrete syntax trees — **K1/K2/K3 live here** |
| `crates/core-formats` | One `Format` trait, many formats |
| `crates/core-verify` | The proof harness: golden, fuzz, property, differential |
| `crates/core-cli` `core-tui` | One feel across every tool |
| `tools/` | konflux · strukt · bigsheet · pdfsurgeon · veritas · lockproof · coverify · replaylab · cage |
| `shell/` | Phase 6. README only, deliberately |
| `corpora/` | Fetch scripts at pinned SHAs — never vendored copies |
| `conformance/` | Official suites at pinned revs; [REPORT.md](conformance/REPORT.md) publishes the rates and the full failure list |
| `benches/` `fuzz/` `xtask/` | Criterion · cargo-fuzz · repo automation |
| `.github/workflows/` | The CI law (§8) |

## The invariants

| ID | Statement |
|----|-----------|
| **K1** | `serialize(parse(x)) == x`, byte-identical |
| **K2** | After an edit, all bytes outside the edited span are unchanged |
| **K3** | Same input + same operations → same output bytes, every platform |

K1 is the first CI gate ever written, and it never comes out.

## Build

```bash
cargo test --workspace --all-features
```

```bash
cargo clippy --workspace --all-targets --all-features
```

```bash
corpora/fetch.sh
```

Rust stable; MSRV 1.90, CI-checked. Linux x86_64 is the source of truth; Windows
and macOS are release targets in the matrix.

## Licence

MIT OR Apache-2.0 (final form pending D4).
