//! # `core-tui` — shared ratatui components and the one theme
//!
//! **Mission (MASTER_PLAN §3.4).** Terminal UI components shared by every tool
//! that has one, wearing a single theme, so that konflux's conflict resolver
//! and bigsheet's grid are visibly the same product family.
//!
//! ## Theme
//!
//! Crimson `#e5484d` on near-black `#0a0a0c` — Ayush's existing identity
//! system, carried through every tool, the docs site, social cards and GIF
//! frames. The colour values live in a design-tokens file consumed by the TUI
//! theme, the docs site and the social-card generator (§16), so there is one
//! source of truth for the palette.
//!
//! > **[AYUSH owns all visual design.]** No colour, glyph, layout or spacing
//! > decision is made in this crate without his sign-off. Implementation here
//! > consumes tokens; it does not choose them.
//!
//! ## Invariants
//!
//! - **T1 `NO_COLOR` and non-TTY are honoured before anything renders.**
//! - **T2 Every view is reachable and legible without colour** — colour is
//!   emphasis, never the sole carrier of meaning (a conflict side must be
//!   identifiable in a monochrome terminal).
//! - **T3 Rendering is a pure function of state.** No clock reads, no
//!   animation that changes what a screenshot of a given state looks like —
//!   this is what makes TUI golden-image tests possible.
//! - **T4 The TUI never mutates a file the CLI would not.** Every destructive
//!   action in the TUI maps to a CLI invocation that could have been typed.
//!
//! ## Status
//!
//! Phase 0 scaffold. First real content lands at konflux M6 (TUI conflict
//! view). GUI work (Tauri, Decision D2) is a separate stack and does not live
//! here.
