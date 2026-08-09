//! Structural diff — konflux **M2**.
//!
//! **The implementation does not exist yet, deliberately.** MASTER_PLAN §8:
//! *"for every milestone, the tests / golden cases / fuzz targets are written
//! and merged before the implementation."* This module is the contract the
//! golden suite is written against, and [`diff`] returns "no changes" so that
//! the suite is red for the one honest reason: nothing computes a diff.
//!
//! # The output shape is the oracle
//!
//! `core-cli` C1 requires every tool to emit stable, schema-versioned `--json`.
//! Rather than test a rendering and retrofit the machine format later, the
//! golden `expected` files **are** the `--json` output. The public contract is
//! therefore under test from before it has an implementation, and any change to
//! it is a reviewed diff against evidence. See ADR-011.
//!
//! # What a change is
//!
//! Two fields, not one, because they answer different questions:
//!
//! - [`ChangeKind`] — *what happened to the tree*: added, removed, changed,
//!   moved.
//! - [`Significance`] — *whether it means anything*: `semantic` if the
//!   document's meaning differs, `formatting` if only its spelling does.
//!
//! Neither determines the other, and the pair is the whole point of konflux. A
//! reordered **mapping** is `moved` + `formatting`; a reordered **sequence** is
//! `moved` + `semantic`, because sequence order is meaning and mapping order is
//! not. A line-based diff renders those two identically, which is precisely the
//! error this tool exists to stop.
//!
//! # Values are source text, never parsed types
//!
//! [`Change::before`] and [`Change::after`] carry the exact source bytes of the
//! node, as text. They are never the "value" of a scalar, because deciding that
//! `yes` is a boolean is a YAML-version-dependent judgement this layer must not
//! make (ADR-008 draws the same line). A diff that says `true → yes` has already
//! lost the argument.

use std::fmt::Write as _;

use core_formats::{Format, ParseReport};

/// Bumped whenever the `--json` shape changes. Consumers pin it (`core-cli` C1).
pub const SCHEMA_VERSION: u32 = 1;

/// What happened to a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// Present in `b`, absent from `a`.
    Added,
    /// Present in `a`, absent from `b`.
    Removed,
    /// Present in both, with different bytes.
    Changed,
    /// A container's children are the same set in a different order.
    Moved,
}

impl ChangeKind {
    /// The stable `--json` spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Removed => "removed",
            Self::Changed => "changed",
            Self::Moved => "moved",
        }
    }
}

/// Whether a change alters what the document *means*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Significance {
    /// The document's meaning differs.
    Semantic,
    /// Only its spelling differs: quoting, ordering of mapping keys, layout.
    Formatting,
}

impl Significance {
    /// The stable `--json` spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Semantic => "semantic",
            Self::Formatting => "formatting",
        }
    }
}

/// One difference between two documents.
#[derive(Debug, Clone)]
pub struct Change {
    /// RFC 6901 JSON Pointer to the node, `""` for the document root.
    ///
    /// A pointer rather than a `.dotted.path` because a path is an identity and
    /// an identity may not be ambiguous. Real Helm charts contain keys like
    /// `kubernetes.io/os`, which a dotted path cannot express and a pointer
    /// escapes as `kubernetes.io~1os`. The pretty form belongs in the human
    /// rendering, where being wrong is cosmetic.
    pub path: String,
    /// What happened.
    pub kind: ChangeKind,
    /// Whether it means anything.
    pub significance: Significance,
    /// Source text of the node in `a`, when it had one.
    pub before: Option<String>,
    /// Source text of the node in `b`, when it has one.
    pub after: Option<String>,
}

/// The result of diffing two documents.
#[derive(Debug, Clone, Default)]
pub struct DiffReport {
    /// Every difference, sorted by `path` bytes then by `kind` (§9.5).
    pub changes: Vec<Change>,
}

impl DiffReport {
    /// Render the stable `--json` form (`core-cli` C1).
    ///
    /// Hand-rolled rather than derived: the workspace carries no serialisation
    /// dependency, and this output is evidence that golden files are compared
    /// against byte-for-byte, so its shape should be visible in one place
    /// rather than emergent from attributes.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push_str("{\n");
        let _ = writeln!(out, "  \"schema_version\": {SCHEMA_VERSION},");
        if self.changes.is_empty() {
            out.push_str("  \"changes\": []\n}\n");
            return out;
        }
        out.push_str("  \"changes\": [\n");
        for (i, change) in self.changes.iter().enumerate() {
            let comma = if i + 1 == self.changes.len() { "" } else { "," };
            out.push_str("    {\n");
            let _ = writeln!(out, "      \"path\": \"{}\",", escape(&change.path));
            let _ = writeln!(out, "      \"kind\": \"{}\",", change.kind.as_str());
            let significance = change.significance.as_str();
            let trailing = change.before.is_none() && change.after.is_none();
            let _ = write!(out, "      \"significance\": \"{significance}\"");
            out.push_str(if trailing { "\n" } else { ",\n" });
            if let Some(before) = &change.before {
                let tail = if change.after.is_none() { "\n" } else { ",\n" };
                let _ = write!(out, "      \"before\": \"{}\"{tail}", escape(before));
            }
            if let Some(after) = &change.after {
                let _ = writeln!(out, "      \"after\": \"{}\"", escape(after));
            }
            let _ = writeln!(out, "    }}{comma}");
        }
        out.push_str("  ]\n}\n");
        out
    }
}

/// Diff two documents of the same format.
///
/// **Not implemented (konflux M2).** Returns an empty report, which is what
/// makes the golden suite red rather than absent: every case that asserts a
/// change fails, and the one case that asserts no change is marked in the suite
/// README as the control it is.
///
/// # Errors
///
/// Returns the first side's [`ParseReport`] when either document does not
/// parse. A diff between a document and an error is not a diff.
pub fn diff<F: Format>(format: &F, a: &[u8], b: &[u8]) -> Result<DiffReport, ParseReport> {
    // Both sides are parsed even though nothing reads the trees yet: a case
    // whose input does not parse must fail here, at the cause, rather than
    // silently scoring an empty diff and looking like agreement.
    let _ = format.parse(a)?;
    let _ = format.parse(b)?;
    Ok(DiffReport::default())
}

/// JSON string escaping, RFC 8259 §7.
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Change, ChangeKind, DiffReport, Significance, escape};

    fn change(path: &str, kind: ChangeKind, significance: Significance) -> Change {
        Change {
            path: path.to_string(),
            kind,
            significance,
            before: None,
            after: None,
        }
    }

    #[test]
    fn an_empty_report_is_still_schema_versioned() {
        let json = DiffReport::default().to_json();
        assert!(json.contains("\"schema_version\": 1"));
        assert!(json.contains("\"changes\": []"));
    }

    #[test]
    fn optional_fields_are_omitted_not_nulled() {
        // A golden is compared byte-for-byte, so "absent" needs exactly one
        // spelling. `"before": null` and no `before` key cannot both be legal.
        let json = DiffReport {
            changes: vec![change("/a", ChangeKind::Moved, Significance::Formatting)],
        }
        .to_json();
        assert!(!json.contains("null"));
        assert!(!json.contains("before"));
        assert!(json.contains("\"significance\": \"formatting\"\n"));
    }

    #[test]
    fn a_pointer_with_a_slash_in_the_key_survives_json_escaping() {
        // RFC 6901 escapes `/` in a key as `~1`; JSON escaping must not touch
        // it. Helm charts really do contain `kubernetes.io/os`.
        let json = DiffReport {
            changes: vec![change(
                "/nodeSelector/kubernetes.io~1os",
                ChangeKind::Changed,
                Significance::Semantic,
            )],
        }
        .to_json();
        assert!(json.contains("\"path\": \"/nodeSelector/kubernetes.io~1os\""));
    }

    #[test]
    fn values_are_escaped_as_json_strings() {
        assert_eq!(escape("a\"b\\c\nd\te"), r#"a\"b\\c\nd\te"#);
        assert_eq!(escape("\u{1}"), "\\u0001");
    }

    #[test]
    fn the_rendered_json_ends_in_exactly_one_newline() {
        // Golden files are byte-compared; trailing whitespace is contract.
        for report in [
            DiffReport::default(),
            DiffReport {
                changes: vec![change("", ChangeKind::Moved, Significance::Formatting)],
            },
        ] {
            let json = report.to_json();
            assert!(json.ends_with("}\n"), "{json:?}");
            assert!(!json.ends_with("}\n\n"));
        }
    }
}
