//! # `pdfsurgeon` — lossless PDF operations + forms
//!
//! **Mission.** The PDF tasks everyone needs, done provably losslessly,
//! locally.
//!
//! **Pitch.** *"Fill, sign, flatten, merge, and split PDFs on your machine —
//! untouched pages stay byte-identical, and forms render the same in every
//! viewer."*
//!
//! ## Scope
//!
//! **MVP:** merge/split/reorder/rotate with incremental-save (untouched objects
//! byte-preserved); AcroForm fill → flatten with correct appearance-stream
//! generation — the documented open sub-gap: forms that render identically in
//! Chrome, Firefox, Acrobat and Preview.
//! **Full:** PDF/A conversion with veraPDF-verified conformance, redaction
//! (true content removal, verified), attachment/metadata surgery.
//!
//! **Non-goals:** a WYSIWYG editor; content *editing*; OCR (probabilistic —
//! banned as a core promise); out-featuring Stirling-PDF wholesale. We win the
//! narrow lossless/forms lane.
//!
//! ## Architecture note
//!
//! PDF is an object graph plus an xref table, not a text CST. This tool owns
//! its own object-model crate rather than using `core-cst` (MASTER_PLAN §2/§3.2).
//! It still answers to `core-verify` — the harness is shared even when the
//! kernel is not.
//!
//! ## Proof obligations
//!
//! - **P1** qpdf structural-equivalence + byte-preservation checks on untouched
//!   pages.
//! - **P2** Render-diff golden images across ≥2 independent renderers (pdfium +
//!   Poppler/MuPDF) within a pixel threshold, per operation, per corpus file.
//! - **P3** Filled+flattened forms render-diff-identical across the renderer
//!   set.
//! - **P4** veraPDF conformance wherever PDF/A is claimed.
//! - **P5** Fuzzing the parser on a hostile-PDF corpus — never panics, never
//!   mis-saves.
//!
//! ## Status
//!
//! Phase 0 scaffold — no code. Phase 4, and **whether it precedes `veritas` is
//! Decision D5** (Ayush's, decided by user pull from Launches 1–3).
