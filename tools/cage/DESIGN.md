# DESIGN — `cage`  — **WING 2, GATED**

## Gate
Same as `replaylab`: ≥2 Wing-1 launches complete AND ≥1 with real adoption
signal (MASTER_PLAN §6). Empty until Ayush confirms.

## Scope
Capability-based sandbox for agent-executed code: WASI preview-2 for portable
workloads plus Landlock/seccomp/namespaces on Linux for native processes;
path-jailed filesystem; egress allowlists with SSRF-safe resolution; CPU/mem/time
limits; MCP tool-call gating; tool-call record/replay on `replaylab`'s cassette
substrate.

## Marketing law — non-negotiable
Claim **"provable default-deny policy enforcement."** Never "unescapable" or
"unbreakable" (Appendix C). No honest security tool claims the latter.
Linux-first, with an explicit platform-support matrix in the README. Never fake
cross-platform security claims.

## Proof obligations (MASTER_PLAN §6.2)
| ID | Statement |
|----|-----------|
| P1 | Adversarial escape suite — secret reads, exfil, process spawn, path traversal, symlink games, DNS rebinding — all denied; a permanent CI regression gate that only grows |
| P2 | Policy determinism — same policy + same request → same allow/deny + same audit log line |
| P3 | Overhead benchmarks vs raw execution, honest |

The P1 suite is **append-only**. Removing an escape case requires
`[NEEDS-AYUSH-APPROVAL]`.

## Current milestone
**Phase 0 — scaffold, gate closed.**
