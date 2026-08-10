# DESIGN — `core-cli`

## Scope
Shared CLI conventions (MASTER_PLAN §3.4): `--json`, `--check`, `NO_COLOR`,
exit codes, span-rich diagnostics, deterministic output ordering.

**In scope:** argument conventions, the diagnostic type, output encoders
(human / JSON / markdown badge), exit-code policy.
**Out of scope:** any tool's own flags, any format knowledge, any TUI.

## Invariants
| ID | Statement | How it is checked |
|----|-----------|-------------------|
| C1 | `--json` is append-only within a schema version | golden suite over serialised output |
| C2 | Exit codes are a contract: 0 clean · 1 finding · 2 usage · **3 refused** · >3 internal | 14 CLI tests running the real binary (`tools/konflux/tests/cli.rs`) |
| C3 | Nothing here reads clock, locale, unseeded randomness, or network | `clippy.toml` disallowed-methods + determinism gate |
| C4 | Errors carry spans, not strings | type-level: the diagnostic type has no free-form-only constructor |

## Non-negotiable
No network access, ever, unless the tool has an explicit `--online` flag and its
README explains why. No telemetry, no phone-home — permanently banned
(Appendix C). Stated in every README (§10).

## Current milestone
**konflux M2 — landed.** Exit-code policy, the shared flag surface, and colour
discipline (ADR-016). No argument-parsing dependency: the surface is small on
purpose and this crate is a leaf every tool imports.

**C2 amended:** `3` is *refused* — the tool cannot model this input. It was
previously folded into ">2 internal", which is wrong: a refusal is a boundary
reported honestly, not a failure. M4's merge driver depends on telling it from
`1`, because "these differ" and "I cannot read this" demand opposite responses.

**Still outstanding: C4**, span-rich diagnostics. A refusal is currently a
sentence, not a span.
