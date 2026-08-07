# ADR-005: Determinism = double-build + double-run output-hash compare, with an in-repo SHA-256

**Date:** 2026-08-07 · **Status:** proposed

## Context

MASTER_PLAN §8 specifies a "double-build + double-run output-hash compare" gate,
blocking. Two things need pinning down. First, what exactly is hashed —
"double-build" could mean bit-identical *binaries* (a much harder claim than
K3 makes) or identical *program output* from two independent builds. Second,
what computes the hash, on Linux, Windows and macOS runners, given that K3 is a
cross-platform claim and `sha256sum` is not.

## Options

- **A — Hash the built binaries.** The strongest claim, but Rust binaries are
  not byte-reproducible without `--remap-path-prefix`, `SOURCE_DATE_EPOCH` and
  care about `-Cmetadata`. It would be claiming reproducible builds, which is a
  different (and unclaimed) property from K3.
- **B — Hash program output from two independent builds.** Exactly K3: identical
  input + identical operations → identical output bytes. Weaker than A, and it
  is the thing the plan actually promises.
- **C — Shell out to `sha256sum` / `shasum` / `certutil` per platform.** No code
  to write; three dialects, and the gate that proves cross-platform determinism
  would itself behave differently per platform.

## Decision

**B for what is hashed, plus an in-repo SHA-256 for what computes it.**

CI builds `xtask` twice into separate target directories, runs each binary,
hashes both outputs and byte-compares them — on Linux, Windows and macOS,
because K3 is a cross-platform claim and testing it only on the cheapest runner
would prove the cheapest thing. **We do not claim reproducible builds.** If we
ever want that, it is a separate ADR and a separate badge.

SHA-256 is implemented in `xtask/src/sha256.rs` with no dependencies, validated
against the FIPS 180-4 published vectors. Rationale: `xtask` is the tool whose
entire job is to be trustworthy about determinism, and giving it a dependency
tree to prove that is backwards. Zero third-party dependencies also makes
`cargo-deny` trivially green for it.

**This is explicitly not a security primitive**, and its module doc says so. The
moment a hash guards a *security* decision — lockproof P2, signature and
provenance verification — that code uses an audited implementation
(`RustCrypto`'s `sha2`), not this one.

Phase 0's hashed output is `xtask scaffold-report`: a deterministic JSON walk of
the repository tree. It was chosen because it exercises every §9.5 failure mode
at once — sorted iteration, no wall-clock field, stable sorts, `/`-normalised
paths, locale-independent comparison — each of which is also a unit test. At M1
the gate additionally runs the real tool binaries; the report stays as the
repo's own canary.

## Consequences

- We can be asked "are your builds reproducible?" and must answer "no, and we
  do not claim it — our claim is output determinism, here is the gate." That is
  a better position than a claim we cannot defend.
- A hand-written SHA-256 is code we own. It is fixed-size, fully specified, and
  covered by the published vectors, so the risk is bounded — but it is real, and
  the security carve-out above is why it stays bounded.
- Adding a determinism case is cheap, which is the point: every tool's output
  should end up under this gate.

## Proof impact

Implements the §8 determinism gate. Directly enforces **K3** and, through the
`clippy.toml` bans, the §9.5 hygiene rules. Underwrites strukt **P4**, veritas
**P3**, lockproof **P4**, replaylab **P1** and cage **P2** — every
determinism-shaped proof obligation in the plan bottoms out here.
