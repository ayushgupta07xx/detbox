//! Structural diff — konflux **M2**.
//!
//! **Both formats are implemented.** The golden suite (ADR-011) was written
//! and merged first, per §8, and the implementation caught up to it: 10/10.
//! The algorithm is format-agnostic — it walks [`SemanticNode`], not a CST — so
//! a new format needs only a `semantic_view`. What it does not yet model, it
//! refuses; `cargo xtask semantic-coverage` measures how often (ADR-013).
//!
//! For a format with no view, [`diff`] **refuses**. It does not return an empty
//! report: empty is indistinguishable from "these files agree", and for a merge
//! tool that is the worst available lie (ADR-012).
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

use core_formats::{Format, ParseReport, SemanticNode, Unmodelled};

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

/// Why a diff could not be produced.
///
/// There is no variant meaning "I could not tell". Every failure here is a
/// refusal with a reason, because the alternative — an empty report — is
/// indistinguishable from "these files agree", and that is the silently-wrong
/// answer §0 ranks first (ADR-012).
#[derive(Debug)]
pub enum DiffError {
    /// A side did not parse.
    Parse {
        /// `"a"` or `"b"`, so the message can say which.
        side: &'static str,
        /// The parser's spans and diagnostics.
        report: ParseReport,
    },
    /// The format has no semantic view, so there is nothing to compare.
    Unmodelled(Unmodelled),
}

impl std::fmt::Display for DiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse { side, report } => write!(
                f,
                "konflux: refused — side `{side}` does not parse ({} diagnostic(s))",
                report.diagnostics().len()
            ),
            Self::Unmodelled(why) => write!(f, "konflux: refused — {why}"),
        }
    }
}

impl std::error::Error for DiffError {}

/// Diff two documents of the same format.
///
/// # Errors
///
/// Returns [`DiffError`] when a side does not parse, or when the format has no
/// semantic view. Refusing is the point: konflux would rather tell you it
/// cannot help than hand back an empty diff that reads as agreement.
pub fn diff<F: Format>(format: &F, a: &[u8], b: &[u8]) -> Result<DiffReport, DiffError> {
    let a_cst = format
        .parse(a)
        .map_err(|report| DiffError::Parse { side: "a", report })?;
    let b_cst = format
        .parse(b)
        .map_err(|report| DiffError::Parse { side: "b", report })?;
    let a_view = format
        .semantic_view(&a_cst)
        .map_err(DiffError::Unmodelled)?;
    let b_view = format
        .semantic_view(&b_cst)
        .map_err(DiffError::Unmodelled)?;

    let mut changes = Vec::new();
    walk("", &a_view, &b_view, &mut changes);
    // Sorted by path bytes, then by kind, so output order is a function of the
    // documents and never of traversal order (§9.5).
    changes.sort_by(|x, y| {
        x.path
            .as_bytes()
            .cmp(y.path.as_bytes())
            .then_with(|| x.kind.as_str().cmp(y.kind.as_str()))
    });
    Ok(DiffReport { changes })
}

/// Append every difference between `a` and `b`, rooted at `path`.
fn walk(path: &str, a: &SemanticNode, b: &SemanticNode, out: &mut Vec<Change>) {
    match (a, b) {
        (SemanticNode::Scalar(x), SemanticNode::Scalar(y)) => {
            if x.text == y.text {
                return;
            }
            out.push(Change {
                path: path.to_string(),
                kind: ChangeKind::Changed,
                // Same resolved value, different spelling: `web` and `"web"`.
                significance: if x.value == y.value {
                    Significance::Formatting
                } else {
                    Significance::Semantic
                },
                before: Some(x.text.clone()),
                after: Some(y.text.clone()),
            });
        }
        (SemanticNode::Mapping(x), SemanticNode::Mapping(y)) => walk_mapping(path, x, y, out),
        (SemanticNode::Sequence(x), SemanticNode::Sequence(y)) => walk_sequence(path, x, y, out),
        // A mapping where a sequence was is not two edits, it is one
        // replacement, and describing it as anything finer would be invention.
        _ => out.push(Change {
            path: path.to_string(),
            kind: ChangeKind::Changed,
            significance: Significance::Semantic,
            before: Some(a.text()),
            after: Some(b.text()),
        }),
    }
}

/// Mappings: key identity decides, and key *order* is spelling.
fn walk_mapping(
    path: &str,
    a: &[(String, SemanticNode)],
    b: &[(String, SemanticNode)],
    out: &mut Vec<Change>,
) {
    for (key, value) in a {
        if !b.iter().any(|(k, _)| k == key) {
            out.push(Change {
                path: join(path, &escape_pointer(key)),
                kind: ChangeKind::Removed,
                significance: Significance::Semantic,
                before: Some(value.text()),
                after: None,
            });
        }
    }
    for (key, value) in b {
        match a.iter().find(|(k, _)| k == key) {
            None => out.push(Change {
                path: join(path, &escape_pointer(key)),
                kind: ChangeKind::Added,
                significance: Significance::Semantic,
                before: None,
                after: Some(value.text()),
            }),
            Some((_, previous)) => walk(&join(path, &escape_pointer(key)), previous, value, out),
        }
    }

    // Same keys in a different order. Reported at the mapping itself, not at
    // the keys: asking *which* key moved has no unambiguous answer, and
    // inventing one would put a guess into evidence.
    let a_keys: Vec<&String> = a.iter().map(|(k, _)| k).collect();
    let b_keys: Vec<&String> = b.iter().map(|(k, _)| k).collect();
    if a_keys.len() == b_keys.len() && a_keys != b_keys {
        let mut sorted_a = a_keys.clone();
        let mut sorted_b = b_keys.clone();
        sorted_a.sort();
        sorted_b.sort();
        if sorted_a == sorted_b {
            out.push(Change {
                path: path.to_string(),
                kind: ChangeKind::Moved,
                // Mapping order is not meaning. This is the half of the pair
                // that a line-based diff gets wrong.
                significance: Significance::Formatting,
                before: None,
                after: None,
            });
        }
    }
}

/// Sequences: position is identity, and order *is* meaning.
fn walk_sequence(path: &str, a: &[SemanticNode], b: &[SemanticNode], out: &mut Vec<Change>) {
    // A permutation is one reorder, not a pile of unrelated edits — and unlike
    // a mapping's, this one changes what the document means.
    if a.len() == b.len() && is_permutation(a, b) && !equal_in_order(a, b) {
        out.push(Change {
            path: path.to_string(),
            kind: ChangeKind::Moved,
            significance: Significance::Semantic,
            before: None,
            after: None,
        });
        return;
    }

    // Otherwise align on the longest common subsequence, so inserting one item
    // mid-list is one `added` rather than a cascade of positional "changes" —
    // which is exactly what a line diff produces and why it is unreadable.
    //
    // LCS matches on equality, so an item that was *edited* matches nothing and
    // falls out as a removal beside an addition. That is technically true and
    // practically useless: it throws away the path to what actually changed,
    // which is the entire product. So each run of unmatched items is paired up
    // positionally and recursed into — a modified container is one edit, not
    // two — and only the leftovers past the shorter side are a real add or
    // remove. This is the cheap end of the Chawathe/GumTree family; §4.1 wants
    // similarity-based matching here eventually, and this is the part of it
    // that the golden suite can currently justify.
    let mut gap_a: Vec<usize> = Vec::new();
    let mut gap_b: Vec<usize> = Vec::new();
    let flush = |gap_a: &mut Vec<usize>, gap_b: &mut Vec<usize>, out: &mut Vec<Change>| {
        for pair in 0..gap_a.len().max(gap_b.len()) {
            match (gap_a.get(pair), gap_b.get(pair)) {
                (Some(&i), Some(&j)) => {
                    if let (Some(x), Some(y)) = (a.get(i), b.get(j)) {
                        walk(&join(path, &j.to_string()), x, y, out);
                    }
                }
                (Some(&i), None) => {
                    if let Some(x) = a.get(i) {
                        out.push(Change {
                            path: join(path, &i.to_string()),
                            kind: ChangeKind::Removed,
                            significance: Significance::Semantic,
                            before: Some(x.text()),
                            after: None,
                        });
                    }
                }
                (None, Some(&j)) => {
                    if let Some(y) = b.get(j) {
                        out.push(Change {
                            path: join(path, &j.to_string()),
                            kind: ChangeKind::Added,
                            significance: Significance::Semantic,
                            before: None,
                            after: Some(y.text()),
                        });
                    }
                }
                (None, None) => {}
            }
        }
        gap_a.clear();
        gap_b.clear();
    };

    for step in lcs_align(a, b) {
        match step {
            Step::Both(i, j) => {
                flush(&mut gap_a, &mut gap_b, out);
                if let (Some(x), Some(y)) = (a.get(i), b.get(j)) {
                    walk(&join(path, &j.to_string()), x, y, out);
                }
            }
            Step::OnlyA(i) => gap_a.push(i),
            Step::OnlyB(j) => gap_b.push(j),
        }
    }
    flush(&mut gap_a, &mut gap_b, out);
}

/// One position in an alignment.
enum Step {
    /// Matched: `a[i]` with `b[j]`.
    Both(usize, usize),
    /// Present only in `a`.
    OnlyA(usize),
    /// Present only in `b`.
    OnlyB(usize),
}

/// Longest-common-subsequence alignment over semantic equality.
///
/// Textbook dynamic programming. Sequences in config files are short — a pod
/// has a handful of containers — so the quadratic table is the right trade
/// against the complexity of anything smarter, and it is deterministic, which
/// a heuristic would have to prove.
fn lcs_align(left: &[SemanticNode], right: &[SemanticNode]) -> Vec<Step> {
    let (left_len, right_len) = (left.len(), right.len());
    let same = |li: usize, ri: usize| {
        left.get(li)
            .zip(right.get(ri))
            .is_some_and(|(x, y)| x.same_value(y))
    };

    // table[li][ri] = length of the LCS of left[li..] and right[ri..]
    let mut table = vec![vec![0usize; right_len + 1]; left_len + 1];
    let get = |t: &Vec<Vec<usize>>, li: usize, ri: usize| {
        t.get(li).and_then(|row| row.get(ri)).copied().unwrap_or(0)
    };
    for li in (0..left_len).rev() {
        for ri in (0..right_len).rev() {
            let value = if same(li, ri) {
                get(&table, li + 1, ri + 1) + 1
            } else {
                get(&table, li + 1, ri).max(get(&table, li, ri + 1))
            };
            if let Some(cell) = table.get_mut(li).and_then(|row| row.get_mut(ri)) {
                *cell = value;
            }
        }
    }

    let mut steps = Vec::new();
    let (mut li, mut ri) = (0usize, 0usize);
    while li < left_len && ri < right_len {
        if same(li, ri) {
            steps.push(Step::Both(li, ri));
            li += 1;
            ri += 1;
        } else if get(&table, li + 1, ri) >= get(&table, li, ri + 1) {
            steps.push(Step::OnlyA(li));
            li += 1;
        } else {
            steps.push(Step::OnlyB(ri));
            ri += 1;
        }
    }
    while li < left_len {
        steps.push(Step::OnlyA(li));
        li += 1;
    }
    while ri < right_len {
        steps.push(Step::OnlyB(ri));
        ri += 1;
    }
    steps
}

/// Same items, possibly in a different order.
fn is_permutation(a: &[SemanticNode], b: &[SemanticNode]) -> bool {
    let mut taken = vec![false; b.len()];
    for item in a {
        let found = b.iter().enumerate().position(|(index, candidate)| {
            taken.get(index) == Some(&false) && item.same_value(candidate)
        });
        match found {
            Some(index) => {
                if let Some(slot) = taken.get_mut(index) {
                    *slot = true;
                }
            }
            None => return false,
        }
    }
    true
}

/// Pairwise equal, position for position.
fn equal_in_order(a: &[SemanticNode], b: &[SemanticNode]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.same_value(y))
}

/// Append one segment to an RFC 6901 pointer.
fn join(path: &str, segment: &str) -> String {
    format!("{path}/{segment}")
}

/// RFC 6901 §3: `~` becomes `~0` and `/` becomes `~1`, in that order.
fn escape_pointer(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
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

    // --- The algorithm ----------------------------------------------------
    //
    // The two JSON golden cases cover mapping reorder and a nested scalar
    // change, and nothing else. Sequences — the LCS alignment and the
    // permutation check — are exercised only by the YAML cases, which `diff`
    // currently refuses. Without these, that code would ship untested behind a
    // green suite, which is precisely the vacuity this repo keeps legislating
    // against. JSON has arrays, so it can carry them today.

    use core_formats::Json;

    fn diff_of(a: &str, b: &str) -> Vec<(String, &'static str, &'static str)> {
        super::diff(&Json, a.as_bytes(), b.as_bytes())
            .expect("json diffs")
            .changes
            .into_iter()
            .map(|c| (c.path, c.kind.as_str(), c.significance.as_str()))
            .collect()
    }

    #[test]
    fn inserting_mid_sequence_is_one_addition_not_a_cascade() {
        // The failure that makes line diff unreadable: without alignment this
        // reports "b changed to c, and c was added".
        assert_eq!(
            diff_of(r#"{"xs":["a","c"]}"#, r#"{"xs":["a","b","c"]}"#),
            [("/xs/1".to_string(), "added", "semantic")]
        );
    }

    #[test]
    fn removing_mid_sequence_is_one_removal() {
        assert_eq!(
            diff_of(r#"{"xs":["a","b","c"]}"#, r#"{"xs":["a","c"]}"#),
            [("/xs/1".to_string(), "removed", "semantic")]
        );
    }

    #[test]
    fn a_reordered_sequence_is_one_semantic_move() {
        // The other half of the pair in ADR-011: sequence order IS meaning, so
        // this is `moved` + semantic where a mapping reorder is + formatting.
        assert_eq!(
            diff_of(r#"{"xs":["a","b"]}"#, r#"{"xs":["b","a"]}"#),
            [("/xs".to_string(), "moved", "semantic")]
        );
    }

    #[test]
    fn a_reordered_mapping_is_one_formatting_move() {
        assert_eq!(
            diff_of(r#"{"a":1,"b":2}"#, r#"{"b":2,"a":1}"#),
            [(String::new(), "moved", "formatting")]
        );
    }

    #[test]
    fn changing_a_nodes_type_is_one_replacement_not_two_edits() {
        assert_eq!(
            diff_of(r#"{"k":[1]}"#, r#"{"k":{"a":1}}"#),
            [("/k".to_string(), "changed", "semantic")]
        );
    }

    #[test]
    fn identical_documents_produce_no_changes() {
        assert!(diff_of(r#"{"a":[1,2]}"#, r#"{"a":[1,2]}"#).is_empty());
    }

    #[test]
    fn output_order_is_a_function_of_the_documents_not_the_walk() {
        // §9.5: sorted by path bytes then kind. Two keys removed and one added,
        // deliberately out of alphabetical order in the source.
        let changes = diff_of(r#"{"z":1,"a":1,"m":1}"#, r#"{"m":1,"b":9}"#);
        let paths: Vec<&str> = changes.iter().map(|(p, _, _)| p.as_str()).collect();
        let mut sorted = paths.clone();
        sorted.sort_by(|x, y| x.as_bytes().cmp(y.as_bytes()));
        assert_eq!(paths, sorted, "changes are not in sorted path order");
    }

    #[test]
    fn a_construct_without_a_semantic_view_is_refused_never_answered() {
        // ADR-012. An empty report would read as "these files agree", so a
        // construct we cannot model must refuse. Flow collections are not
        // modelled yet, and the two documents here genuinely differ — so a
        // silent `[]` would be a wrong answer, not merely an unhelpful one.
        let refusal = super::diff(&core_formats::Yaml, b"a: {x: 1}\n", b"a: {x: 2}\n")
            .expect_err("flow collections are not modelled yet");
        let rendered = refusal.to_string();
        assert!(rendered.contains("refused"), "{rendered}");
        assert!(rendered.contains("flow"), "{rendered}");
    }

    #[test]
    fn yaml_and_json_agree_on_the_same_document() {
        // The algorithm is format-agnostic, so the same edit expressed in each
        // format must produce the same paths and the same significance. If
        // these ever diverge it is a semantic_view bug, not a diff bug.
        let yaml = super::diff(&core_formats::Yaml, b"a:\n  b: 1\n", b"a:\n  b: 2\n")
            .expect("yaml diffs")
            .to_json();
        let json = super::diff(&Json, br#"{"a":{"b":1}}"#, br#"{"a":{"b":2}}"#)
            .expect("json diffs")
            .to_json();
        assert_eq!(yaml, json);
    }

    #[test]
    fn a_side_that_does_not_parse_is_refused_with_the_side_named() {
        let refusal = super::diff(&Json, b"{", b"{}").expect_err("side a is malformed");
        assert!(refusal.to_string().contains("side `a`"), "{refusal}");
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
