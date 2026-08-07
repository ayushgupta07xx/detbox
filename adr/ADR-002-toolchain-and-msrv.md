# ADR-002: Pinned toolchain and MSRV 1.90

**Date:** 2026-08-07 · **Status:** proposed

## Context

MASTER_PLAN §2 requires "Rust stable, MSRV pinned and CI-checked" but does not
name a version. Two separate things need deciding: which compiler the repo
develops against day to day, and how old a compiler a downstream user may have.
Conflating them is the common mistake — it makes CI lint output depend on
whichever compiler a contributor happens to have installed.

## Options

- **A — No `rust-toolchain.toml`, MSRV checked in CI only.** Cheapest, but
  clippy's pedantic output differs between compiler versions, so "green
  locally, red in CI" becomes routine and people learn to ignore the linter.
- **B — Pin the development toolchain *and* check a separate, older MSRV.**
  Two toolchains to install, but a local `cargo clippy` reproduces CI exactly,
  and the MSRV claim is tested rather than asserted.
- **C — Pin one version and make it the MSRV.** Simplest, but forces every
  downstream user onto a compiler released weeks ago, for no proof benefit.

## Decision

**B.** `rust-toolchain.toml` pins **1.97.1** — the compiler this repo is
developed and linted against, so a contributor's `cargo clippy` and CI's are the
same program. `Cargo.toml` declares `rust-version = "1.90"` as the MSRV, roughly
ten months of headroom, and a dedicated CI job builds and tests on exactly that
toolchain via `RUSTUP_TOOLCHAIN`.

Lints run on stable only. Chasing an older clippy's different diagnostics is not
a proof of anything, and it would make the lint gate a version-compatibility
gate wearing a disguise.

Edition 2024 throughout (requires 1.85; comfortably inside the MSRV).

## Consequences

- Two toolchains in CI, and the MSRV job is the one most likely to break on a
  dependency bump. That is the point: it breaks loudly instead of silently.
- Raising the MSRV is a reviewed change with its own ADR, not a side effect of
  reaching for a new language feature.
- Bumping `rust-toolchain.toml` can turn CI red purely from new clippy lints.
  That is a normal, contained PR — and it is far better than the alternative,
  where the lint set drifts per contributor.

## Proof impact

Touches the §8 "Format + lints" and "Docs + MSRV" gates. Indirectly underwrites
**K3**: a pinned compiler is one fewer source of cross-machine divergence when
we claim identical output bytes everywhere.
