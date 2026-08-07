# DESIGN — `replaylab`  — **WING 2, GATED**

## Gate
Wing 2 opens only when **≥2 Wing-1 launches are complete AND at least one shows
real adoption signal** (MASTER_PLAN §6, §14). Until Ayush confirms the gate has
passed, this crate stays empty. Do not start work here.

## Scope
An OpenAI/Anthropic-compatible local proxy that records full request/response
streams as **cassettes** and replays them byte-identically; exact-prefix caching
only; a deterministic token/cost ledger (arithmetic, not estimates); streaming
with backpressure.

## The ban that defines this tool
**Semantic/similarity caches are permanently banned** (Appendix C). An
embedding-similarity cache hit that returns a subtly wrong answer is exactly the
probabilistic failure this brand exists to reject. Exact prefix match, or a miss.

## Proof obligations (MASTER_PLAN §6.1)
| ID | Statement |
|----|-----------|
| P1 | Replay determinism — replayed transcript hash equals recorded hash, always |
| P2 | Passthrough transparency — proxied vs direct byte-identical modulo an allow-listed header set, differentially tested |
| P3 | Cassette schema versioned, with migration tests |
| P4 | Ledger exactness against provider-reported usage on golden traces |

## Current milestone
**Phase 0 — scaffold, gate closed.** Launches jointly with `cage`.
