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
| C2 | Exit codes are a contract: 0 clean · 1 finding · 2 usage · >2 internal | CLI golden suite asserting codes |
| C3 | Nothing here reads clock, locale, unseeded randomness, or network | `clippy.toml` disallowed-methods + determinism gate |
| C4 | Errors carry spans, not strings | type-level: the diagnostic type has no free-form-only constructor |

## Non-negotiable
No network access, ever, unless the tool has an explicit `--online` flag and its
README explains why. No telemetry, no phone-home — permanently banned
(Appendix C). Stated in every README (§10).

## Current milestone
**Phase 0 — scaffold.** First real content at konflux M2 (diff output) and M4
(`--check` CI mode).
