# `shell/` — the unified workbench

**Phase 6. The finale. Zero code until then.**

MASTER_PLAN §7:

> A Tauri workbench mounting each tool as a module: konflux's conflict view,
> strukt's query pane, bigsheet's grid, pdfsurgeon's page view, veritas's
> fidelity matrix, lockproof's PR reports, coverify's trace explorer. Ships
> **only** after ≥3 tools have standalone adoption. Its launch story writes
> itself: *"the deterministic toolbox — everything provably lossless, one app."*
> Until then, the shell is a directory with a README and zero code. **No
> exceptions; the tab-bar temptation is how ecosystems die as demos.**

## The gate

Three tools with **standalone adoption** — not three tools that exist. Adoption
is measured per §14, in order of truth: GitHub dependents and merge-driver
installs → organic issues from strangers → crates.io/Homebrew downloads → stars
last.

Shipping this early is a **permanent ban** (Appendix C). It is on the list next
to load-bearing ML and cherry-picked benchmarks, and for the same reason: it
converts a platform into a demo.

## Why the ban is real and not decorative

A shell is the most demo-able artifact in the plan and the least proof-bearing.
Every hour spent on it before the tools have users is an hour not spent on
format-preserving serialization, which is the thing nobody else has done. The
shell is a *reward* for adoption, not a *route* to it.

## What lives here when the gate opens

Nothing but composition: the shell mounts modules, it does not reimplement them.
Any logic that would be written here belongs in the tool crate, where it is
already under the proof harness. **T4 applies at the shell level too** — every
destructive action maps to a CLI invocation the user could have typed.

## Status

Empty. Deliberately.
