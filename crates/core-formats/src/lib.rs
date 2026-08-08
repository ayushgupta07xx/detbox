//! # `core-formats` — one trait, many formats
//!
//! **Mission (MASTER_PLAN §3.2).** Every format the ecosystem speaks implements
//! one trait. Products are heads on this trait; adding a format adds it to
//! every product at once. This is the mechanical form of the platform thesis.
//!
//! ## The trait, and why it arrives in pieces
//!
//! §3.2 specifies the full shape:
//!
//! ```ignore
//! trait Format {
//!     fn parse(&self, input: &[u8]) -> Result<Cst, ParseReport>;   // never panics; report has spans
//!     fn serialize(&self, cst: &Cst) -> Vec<u8>;                   // total function
//!     fn semantic_view(&self, cst: &Cst) -> SemanticTree;          // typed layer for diff/merge/query
//!     fn merge_hints(&self) -> MergeHints;                         // e.g. K8s list-identity keys
//!     fn conformance_suite(&self) -> Option<SuiteAdapter>;         // official test suite hookup
//! }
//! ```
//!
//! [`Format`] declares `parse` and `serialize` today, because those are what K1
//! needs and K1 is konflux M1. `semantic_view` lands at M2 with structural
//! diff, `merge_hints` at M5 with the Kubernetes layer, `conformance_suite`
//! alongside the M1 conformance adapters. Declaring methods with no
//! implementation and no test would be surface area pretending to be progress.
//!
//! ## Invariants
//!
//! - **F1 `parse` never panics.** Not on hostile input, not on non-UTF-8, not
//!   on truncated input. Failure is a [`ParseReport`] carrying spans, never an
//!   abort. Enforced by a per-format fuzz target.
//! - **F2 `serialize` is total.** Every `Cst` this crate can construct
//!   serialises. There is no "unserialisable" state — see
//!   [`Cst::serialize`][core_cst::Cst::serialize], which is an in-order walk
//!   with no failure mode.
//! - **F3 K1 composition.** `serialize(parse(x)) == x` byte-identically
//!   whenever `parse` succeeds. A format that cannot round-trip a construct
//!   models it as [`SyntaxKind::VERBATIM`][core_cst::SyntaxKind::VERBATIM]
//!   instead of normalising it.
//! - **F4 The semantic view is derived, never authoritative.** Output bytes
//!   come from the CST. A semantic view may be lossy; the CST may not.
//!
//! ## Rollout order (MASTER_PLAN §3.2 — do not reorder without an ADR)
//!
//! **yaml, json** (Phase 1) → **toml, hcl** (Phase 2) → **csv, jsonl, logfmt**
//! (Phase 3) → **lockfiles** (Phase 4). PDF is architecturally different
//! (object graph + xref, not a text CST), lives under `tools/pdfsurgeon`, and
//! still answers to `core-verify`.
//!
//! ## Status
//!
//! **konflux M1.** [`Json`] is implemented — see [`json`]. [`Yaml`]'s `parse`
//! still returns [`ParseReport::not_implemented`], so its round-trip, fuzz and
//! conformance oracles remain red by design.

pub mod json;
pub mod yaml;

use core_cst::{Cst, Span};

/// One thing that went wrong while parsing, anchored to the bytes that caused
/// it.
///
/// Span-carrying by construction: there is no constructor that takes only a
/// message, because "invalid YAML" without a location is not a diagnostic
/// (MASTER_PLAN §3.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// The bytes at fault.
    pub span: Span,
    /// What is wrong, in the user's terms.
    pub message: String,
}

impl Diagnostic {
    /// A diagnostic covering `span`.
    #[must_use]
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }
}

/// Why a parse did not produce a tree.
///
/// Returned rather than panicked (**F1**). A `ParseReport` always carries at
/// least one diagnostic: an empty report would say "this failed, and I will not
/// tell you where," which is the failure mode §3.4 exists to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseReport {
    diagnostics: Vec<Diagnostic>,
}

impl ParseReport {
    /// A report from one or more diagnostics.
    ///
    /// If `diagnostics` is empty, a placeholder is inserted rather than
    /// producing a silent, locationless failure.
    #[must_use]
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        if diagnostics.is_empty() {
            return Self {
                diagnostics: vec![Diagnostic::new(
                    Span { start: 0, end: 0 },
                    "parse failed, but no diagnostic was recorded — this is a bug in the parser",
                )],
            };
        }
        Self { diagnostics }
    }

    /// The format has no parser yet.
    ///
    /// This is what makes the M1 round-trip suites red. It is a real error
    /// value rather than a `todo!()` because a panic here would be an F1
    /// violation, and because the workspace denies `todo!`/`unimplemented!`
    /// outright.
    #[must_use]
    pub fn not_implemented(format: &str) -> Self {
        Self::new(vec![Diagnostic::new(
            Span { start: 0, end: 0 },
            format!(
                "`{format}` has no parser yet (konflux M1 is at the oracle stage). \
                 This suite is expected to be red."
            ),
        )])
    }

    /// Every diagnostic, in source order.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

impl std::fmt::Display for ParseReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, diagnostic) in self.diagnostics.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(
                f,
                "bytes {}..{}: {}",
                diagnostic.span.start, diagnostic.span.end, diagnostic.message
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseReport {}

/// A format the ecosystem speaks.
pub trait Format {
    /// The name used in diagnostics and reports. Stable: it appears in `--json`
    /// output, which is schema-versioned (`core-cli` C1).
    fn name(&self) -> &'static str;
    /// File extensions this format claims, lowercase and without the dot.
    fn extensions(&self) -> &'static [&'static str];

    /// Parse `input` into a lossless tree.
    ///
    /// **Never panics** (F1), for any byte sequence whatsoever.
    ///
    /// # Errors
    ///
    /// Returns a [`ParseReport`] with spans when the input cannot be modelled
    /// *even as verbatim nodes*. Note how narrow that is: input the grammar
    /// does not understand is not an error, it is a
    /// [`VERBATIM`][core_cst::SyntaxKind::VERBATIM] node (§3.1). Preserving
    /// beats understanding.
    fn parse(&self, input: &[u8]) -> Result<Cst, ParseReport>;

    /// Emit the exact bytes of a tree. Total (F2).
    ///
    /// Delegates to [`Cst::serialize`]: serialisation is format-independent by
    /// construction, which is what collapses K1 down to a property of `parse`
    /// alone. A format that overrides this to be clever has broken K1.
    fn serialize(&self, cst: &Cst) -> Vec<u8> {
        cst.serialize()
    }
}

/// YAML. konflux M1.
#[derive(Debug, Clone, Copy, Default)]
pub struct Yaml;

impl Format for Yaml {
    fn name(&self) -> &'static str {
        "yaml"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["yaml", "yml"]
    }

    fn parse(&self, input: &[u8]) -> Result<Cst, ParseReport> {
        yaml::parse(input)
    }
}

/// JSON. konflux M1.
#[derive(Debug, Clone, Copy, Default)]
pub struct Json;

impl Format for Json {
    fn name(&self) -> &'static str {
        "json"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["json"]
    }

    fn parse(&self, input: &[u8]) -> Result<Cst, ParseReport> {
        json::parse(input)
    }
}

#[cfg(test)]
mod tests {
    use super::{Diagnostic, Format, Json, ParseReport, Yaml};
    use core_cst::Span;

    #[test]
    fn parse_returns_rather_than_panicking_on_hostile_input() {
        // F1: `parse` must not abort, for any bytes at all. Whether it returns
        // Ok or Err is the accept/reject question ADR-008 answers separately.
        //
        // This assertion used to be `is_err()`, which was written while both
        // parsers were stubs and so encoded the stub's behaviour rather than the
        // property it names. Two of these inputs are now legitimately accepted:
        // a Go template is preserved verbatim (§3.1) and an anchor/alias
        // document is ordinary YAML. Reaching the next line is the test.
        for hostile in [
            &b""[..],
            b"\xff\xfe\x00",
            b"{{ .Values.image | quote }}",
            b"a: &x\n  <<: *x\n",
            b"\x00\x01\x02",
            b"[[[[[[[[[[",
        ] {
            let _ = Yaml.parse(hostile);
            let _ = Json.parse(hostile);
        }
    }

    #[test]
    fn a_report_always_carries_a_diagnostic() {
        // An empty report is a failure that refuses to say where. Guard it at
        // the constructor rather than trusting every future caller.
        let empty = ParseReport::new(Vec::new());
        assert_eq!(empty.diagnostics().len(), 1);
        assert!(empty.to_string().contains("bug in the parser"));
    }

    #[test]
    fn reports_render_with_byte_offsets() {
        let report = ParseReport::new(vec![Diagnostic::new(
            Span { start: 12, end: 19 },
            "unterminated quoted scalar",
        )]);
        assert_eq!(
            report.to_string(),
            "bytes 12..19: unterminated quoted scalar"
        );
    }

    #[test]
    fn format_identity_is_stable() {
        // These strings reach `--json` output, which is a versioned schema
        // (core-cli C1). Changing one is a public API change (§9.3).
        assert_eq!(Yaml.name(), "yaml");
        assert_eq!(Yaml.extensions(), &["yaml", "yml"]);
        assert_eq!(Json.name(), "json");
        assert_eq!(Json.extensions(), &["json"]);
    }
}
