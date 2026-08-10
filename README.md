# inviolate

> Everything we ship is provably correct, every run.

**Status: konflux M2.** No product has shipped. Nothing here claims to work yet,
and this README will not claim otherwise until a gate says so.

The brand is **inviolate** and the multicall binary will be `invio` — Decision
D1, made 2026-08-09 (ADR-015). *Inviolate:* intact, nothing violated. It is a
claim about the artifact, not about us. The binary itself does not exist yet; it
lands when konflux M4 needs something to install.

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

**MIT OR Apache-2.0**, at your option — [LICENSE-MIT](LICENSE-MIT) and
[LICENSE-APACHE](LICENSE-APACHE). Decision **D4**, settled 2026-08-09.

Unless you state otherwise, any contribution you intentionally submit for
inclusion in this work is dual licensed as above, without additional terms or
conditions.
