# DESIGN — `core-tui`

## Scope
Shared ratatui components and the single theme (MASTER_PLAN §3.4).

**In scope:** reusable widgets (side-by-side pane, conflict block, virtualised
list), key-binding conventions, the theme loaded from design tokens.
**Out of scope:** choosing any colour, glyph or layout — **[AYUSH owns all
visual design]**. Also out of scope: the Tauri/GUI stack (Decision D2), which is
a different surface entirely.

## Invariants
| ID | Statement | How it is checked |
|----|-----------|-------------------|
| T1 | `NO_COLOR` and non-TTY honoured before anything renders | unit test on the theme constructor |
| T2 | Every view is legible without colour; colour is never the sole carrier of meaning | golden-image test rendered with colour disabled |
| T3 | Rendering is a pure function of state — no clock reads, no state-independent animation | golden-image tests are only possible if this holds |
| T4 | Every destructive TUI action maps to a CLI invocation the user could have typed | reviewed per-action; the TUI is a front-end, not a second implementation |

## Theme
Crimson `#e5484d` on near-black `#0a0a0c`, from a design-tokens file shared with
the docs site and social cards (§16). One palette, one source of truth.

## Current milestone
**Phase 0 — scaffold.** First real content at konflux M6 (TUI conflict view).
