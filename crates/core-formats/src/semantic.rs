//! The typed layer — MASTER_PLAN §3.2's `semantic_view`.
//!
//! `core-cst` owns every byte; this owns what those bytes *mean*. Diff, merge
//! and query all need to ask "same value?" without asking "same spelling?", and
//! that question has no answer in a lossless tree, where `"web"` and `web` are
//! plainly different token sequences.
//!
//! # A view is a claim, and a format may decline to make one
//!
//! §3.2 sketches `semantic_view` as total: `fn semantic_view(&self, cst) ->
//! SemanticTree`. It cannot be. YAML's concrete tree is a flat list of lines
//! with indentation tokens — deliberately, since M1 built it lossless and
//! structural rather than semantic — so producing a view means *inferring*
//! block structure, which is M2 work that does not exist yet.
//!
//! The signature therefore returns a `Result`, and a format that cannot model
//! its input says so. Returning an empty view instead would let konflux report
//! "no changes" for a document it never understood, which is the silently-wrong
//! failure §0 ranks first and the worst possible lie for a merge tool. ADR-012.
//!
//! # What is normalised, and what deliberately is not
//!
//! [`Scalar::value`] is the *resolved* form and [`Scalar::text`] the source
//! bytes. Equal `value` with differing `text` is a formatting change; differing
//! `value` is a semantic one.
//!
//! Only **strings** are resolved, by unescaping. Numbers, booleans and nulls
//! carry `value == text`, so `1.0` and `1.00` read as a semantic change. That
//! is deliberate: calling them equal requires a numeric interpretation this
//! layer refuses to make, for the same reason ADR-008 refused to decide whether
//! `yes` is a boolean. Over-reporting a change is noisy; under-reporting one
//! loses an edit in a merge. Only one of those is recoverable.

use core_cst::{Cst, GreenChild, GreenNode};

/// Why a format cannot produce a semantic view of this input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unmodelled {
    /// Format that declined, for the diagnostic.
    pub format: &'static str,
    /// Why, in a sentence a user can act on.
    pub reason: &'static str,
}

impl std::fmt::Display for Unmodelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} has no semantic view: {}", self.format, self.reason)
    }
}

impl std::error::Error for Unmodelled {}

/// A scalar, in both of its forms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scalar {
    /// Exact source bytes, as text. What a diff reports.
    pub text: String,
    /// Resolved form. What a diff compares.
    pub value: String,
}

/// The meaning of a document, with spelling deliberately discarded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticNode {
    /// A leaf.
    Scalar(Scalar),
    /// Key-value pairs **in source order**, which is preserved because losing
    /// it would make "the keys were reordered" unaskable — and that question is
    /// the difference between a formatting change and a semantic one.
    Mapping(Vec<(String, SemanticNode)>),
    /// Ordered items. Sequence order *is* meaning.
    Sequence(Vec<SemanticNode>),
}

impl SemanticNode {
    /// The source text this node came from, for a diff's `before`/`after`.
    #[must_use]
    pub fn text(&self) -> String {
        match self {
            Self::Scalar(scalar) => scalar.text.clone(),
            // Containers report their shape rather than their bytes: a diff
            // that inlined a 400-line subtree into one JSON field would be
            // unreadable, and the interesting change is always deeper.
            Self::Mapping(pairs) => format!("{{{} keys}}", pairs.len()),
            Self::Sequence(items) => format!("[{} items]", items.len()),
        }
    }

    /// Whether two nodes mean the same thing, ignoring spelling.
    #[must_use]
    pub fn same_value(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Scalar(a), Self::Scalar(b)) => a.value == b.value,
            (Self::Mapping(a), Self::Mapping(b)) => {
                a.len() == b.len()
                    && a.iter().all(|(key, value)| {
                        b.iter()
                            .find(|(k, _)| k == key)
                            .is_some_and(|(_, v)| value.same_value(v))
                    })
            }
            (Self::Sequence(a), Self::Sequence(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.same_value(y))
            }
            _ => false,
        }
    }
}

// --- JSON ------------------------------------------------------------------

/// Build the semantic view of a JSON tree.
///
/// # Errors
///
/// Returns [`Unmodelled`] when the tree holds no value at all, which the parser
/// only produces for input it already rejected.
pub(crate) fn json_view(cst: &Cst) -> Result<SemanticNode, Unmodelled> {
    let root = cst.root();
    first_value(root.children()).ok_or(Unmodelled {
        format: "json",
        reason: "the document contains no value",
    })
}

/// Tokens that carry no meaning: whitespace, byte-order marks, punctuation.
fn is_trivia(kind: core_cst::SyntaxKind) -> bool {
    use crate::json::kind;
    matches!(
        kind,
        kind::WHITESPACE
            | kind::BOM
            | kind::COLON
            | kind::COMMA
            | kind::L_BRACE
            | kind::R_BRACE
            | kind::L_BRACKET
            | kind::R_BRACKET
    )
}

/// The first meaningful child, as a semantic node.
fn first_value(children: &[GreenChild]) -> Option<SemanticNode> {
    children.iter().find_map(child_value)
}

/// A child as a semantic node, or `None` if it is trivia.
fn child_value(child: &GreenChild) -> Option<SemanticNode> {
    use crate::json::kind;
    match child {
        GreenChild::Node(node) => match node.kind() {
            kind::OBJECT => Some(object_view(node)),
            kind::ARRAY => Some(array_view(node)),
            _ => None,
        },
        GreenChild::Token(token) => {
            if is_trivia(token.kind()) {
                return None;
            }
            let text = String::from_utf8_lossy(token.text()).into_owned();
            let value = if token.kind() == kind::STRING {
                unescape(&text)
            } else {
                // Numbers, booleans and nulls resolve to themselves. See the
                // module docs: normalising `1.0` and `1.00` needs a numeric
                // interpretation this layer will not make.
                text.clone()
            };
            Some(SemanticNode::Scalar(Scalar { text, value }))
        }
    }
}

/// Pair each `STRING` key with the value that follows its colon.
fn object_view(node: &GreenNode) -> SemanticNode {
    use crate::json::kind;
    let mut pairs: Vec<(String, SemanticNode)> = Vec::new();
    let mut pending_key: Option<String> = None;

    for child in node.children() {
        // A key is a STRING token appearing where no key is pending. Anything
        // meaningful arriving while a key IS pending is that key's value —
        // including a STRING, which is why the two cases cannot be merged.
        if pending_key.is_none()
            && let GreenChild::Token(token) = child
            && token.kind() == kind::STRING
        {
            pending_key = Some(unescape(&String::from_utf8_lossy(token.text())));
            continue;
        }
        if let Some(value) = child_value(child)
            && let Some(key) = pending_key.take()
        {
            pairs.push((key, value));
        }
    }
    SemanticNode::Mapping(pairs)
}

/// Every meaningful child, in order.
fn array_view(node: &GreenNode) -> SemanticNode {
    SemanticNode::Sequence(node.children().iter().filter_map(child_value).collect())
}

/// Resolve a JSON string literal to the characters it denotes (RFC 8259 §7).
///
/// Lossy by design and only in one direction: this is the *comparison* form,
/// never serialised back. `Scalar::text` keeps the bytes.
fn unescape(literal: &str) -> String {
    let inner = literal
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(literal);

    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('b') => out.push('\u{8}'),
            Some('f') => out.push('\u{c}'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                // A lone surrogate has no `char`. Keeping the escape verbatim
                // means two spellings of the same unpaired surrogate compare
                // unequal — conservative, and the conservative direction
                // reports a change rather than hiding one.
                if let Some(decoded) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    out.push(decoded);
                } else {
                    out.push_str("\\u");
                    out.push_str(&hex);
                }
            }
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{SemanticNode, unescape};
    use crate::{Format, Json};

    fn view(src: &str) -> SemanticNode {
        let cst = Json.parse(src.as_bytes()).expect("parses");
        Json.semantic_view(&cst).expect("json has a view")
    }

    #[test]
    fn an_object_becomes_a_mapping_in_source_order() {
        let SemanticNode::Mapping(pairs) = view(r#"{"b": 1, "a": 2}"#) else {
            panic!("expected a mapping");
        };
        let keys: Vec<&str> = pairs.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, ["b", "a"], "source order must survive the view");
    }

    #[test]
    fn key_order_does_not_change_the_value() {
        // The whole reason Mapping keeps order AND same_value ignores it.
        assert!(view(r#"{"a":1,"b":2}"#).same_value(&view(r#"{"b":2,"a":1}"#)));
    }

    #[test]
    fn escape_form_is_spelling_not_meaning() {
        // RFC 8259 lets `/` be written plain or escaped, and both denote the
        // same string. The K1 tree must keep each spelling; the semantic view
        // must stop distinguishing them.
        let plain = view("{\"k\": \"/\"}");
        let escaped = view("{\"k\": \"\\/\"}");
        assert!(
            plain.same_value(&escaped),
            "same string, different escape form"
        );
        assert_ne!(plain, escaped, "but the source text must still differ");
    }

    #[test]
    fn numbers_are_not_normalised() {
        // Deliberate. Calling these equal needs a numeric interpretation this
        // layer refuses to make; over-reporting a change is the safe direction.
        assert!(!view(r#"{"n": 1.0}"#).same_value(&view(r#"{"n": 1.00}"#)));
    }

    #[test]
    fn nesting_and_arrays_survive() {
        let SemanticNode::Mapping(pairs) = view(r#"{"xs": [1, {"y": null}]}"#) else {
            panic!("expected a mapping");
        };
        let Some((_, SemanticNode::Sequence(items))) = pairs.first() else {
            panic!("expected a sequence");
        };
        assert_eq!(items.len(), 2);
        assert!(matches!(items.get(1), Some(SemanticNode::Mapping(_))));
    }

    #[test]
    fn sequence_order_is_meaning() {
        assert!(!view(r#"{"xs":[1,2]}"#).same_value(&view(r#"{"xs":[2,1]}"#)));
    }

    #[test]
    fn yaml_declines_rather_than_inventing_a_view() {
        // The point of ADR-012: no view is an explicit refusal, never an empty
        // one. An empty view would make konflux report "no changes".
        let cst = crate::Yaml.parse(b"a: 1\n").expect("parses");
        let err = crate::Yaml
            .semantic_view(&cst)
            .expect_err("yaml has no semantic view yet");
        assert_eq!(err.format, "yaml");
        assert!(!err.reason.is_empty());
    }

    #[test]
    fn unescape_handles_the_awkward_forms() {
        assert_eq!(unescape(r#""a\nb""#), "a\nb");
        assert_eq!(unescape(r#""A""#), "A");
        assert_eq!(unescape(r#""\/""#), "/");
        // Lone surrogate: kept verbatim rather than replaced.
        assert_eq!(unescape(r#""\ud800""#), r"\ud800");
    }
}
