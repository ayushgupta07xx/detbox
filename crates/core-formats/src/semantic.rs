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
    /// A multi-document stream: `---` separated documents, in order.
    ///
    /// Distinct from [`Self::Sequence`] on purpose. Both index the same way, so
    /// they could have shared a variant — but then a single document whose root
    /// is a list would compare *equal in shape* to a two-document file, and a
    /// diff between them would pair document 0 against list item 0. Those are
    /// different things, and a path is an identity (ADR-011). The separate
    /// variant makes the mismatch a replacement instead of a silent alignment.
    Stream(Vec<SemanticNode>),
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
            Self::Stream(docs) => format!("<{} documents>", docs.len()),
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
            (Self::Sequence(a), Self::Sequence(b)) | (Self::Stream(a), Self::Stream(b)) => {
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

// --- YAML -------------------------------------------------------------------
//
// YAML's concrete tree is `STREAM → LINE*`, where a LINE owns its own tokens and
// nests the more-indented LINEs beneath it. So the shape is already there; what
// is missing is *meaning* — which lines are mapping entries, which are sequence
// items, and which are neither.
//
// The rule for "neither" is the load-bearing one. A line this code cannot
// classify makes the whole document [`Unmodelled`], rather than being skipped.
// Skipping would produce a view with fewer keys than the document has, and a
// diff over that view would report "no changes" for an edit inside the part we
// dropped — silently wrong, from the layer whose entire job is to not be
// (ADR-012).
//
// Blank and comment-only lines are the one exception, and they are not an
// exception to that rule: they carry no *semantic* content, so a view without
// them is complete. Their bytes are still owned by the CST, where K1 already
// proves they survive.

/// What a single line means.
enum Line<'a> {
    /// `key: value`, where the value may be inline, nested, or absent.
    Entry {
        /// Resolved key text.
        key: String,
        /// Significant tokens after the colon.
        value: Vec<&'a core_cst::GreenToken>,
    },
    /// `- ...`
    Item {
        /// Significant tokens after the dash.
        rest: Vec<&'a core_cst::GreenToken>,
    },
    /// Blank, or nothing but a comment.
    Blank,
    /// `---` or `...` — a document boundary, meaningful only at the top level.
    Marker,
}

/// Build the semantic view of a YAML tree.
///
/// # Errors
///
/// Returns [`Unmodelled`] for anything this layer does not model yet: multiple
/// documents, flow collections, anchors and aliases, tags, block scalars, and
/// Go templates. Each is a refusal rather than a guess.
pub(crate) fn yaml_view(cst: &Cst) -> Result<SemanticNode, Unmodelled> {
    use crate::yaml::kind;

    let lines = child_lines(cst.root());

    // Split the top level on `---` and `...`. Markers only mean anything here;
    // `node_from_lines` refuses one found deeper, where it cannot.
    let mut documents: Vec<Vec<&GreenNode>> = vec![Vec::new()];
    let mut saw_marker = false;
    for line in lines {
        let is_marker = line.children().iter().any(|child| match child {
            GreenChild::Token(token) => {
                matches!(token.kind(), kind::DOC_START | kind::DOC_END)
            }
            GreenChild::Node(_) => false,
        });
        if is_marker {
            saw_marker = true;
            // A marker line can still own nested lines, because indentation
            // parents them to it. They belong to the document it opens.
            documents.push(child_lines(line));
            continue;
        }
        if let Some(current) = documents.last_mut() {
            current.push(line);
        }
    }

    if !saw_marker {
        let single = documents.first().cloned().unwrap_or_default();
        return node_from_lines(&single);
    }

    // Empty segments are real: `---` at the top of a file opens the first
    // document and leaves nothing before it. Dropping those keeps document
    // indices matching what a reader counts.
    let mut built = Vec::new();
    for document in &documents {
        if document.is_empty() {
            continue;
        }
        built.push(node_from_lines(document)?);
    }

    // One document with explicit markers is still one document. Wrapping it
    // would make `---\na: 1` and `a: 1` compare as different shapes, which is
    // a formatting difference reported as a structural one.
    match built.len() {
        0 => Ok(SemanticNode::Scalar(Scalar {
            text: String::new(),
            value: String::new(),
        })),
        1 => built
            .into_iter()
            .next()
            .ok_or_else(|| unmodelled("a document vanished between counting and taking it")),
        _ => Ok(SemanticNode::Stream(built)),
    }
}

/// The `LINE` children of a node that carry meaning, in source order.
///
/// Comment-only and blank lines are dropped here rather than later, because of
/// where the concrete tree puts them: a line with no indentation of its own
/// attaches to the innermost open line, so the comment in
///
/// ```yaml
/// global:
///   imageRegistry: ""
///   ## E.g. imagePullSecrets:
/// ```
///
/// becomes a *child of `imageRegistry`*, not a sibling. Reading that literally
/// makes a scalar look like a container and refuses the document. It is the
/// single most common shape in the corpus — Helm values files are more comment
/// than value — and dropping these lines costs nothing, because they carry no
/// semantic content and K1 already proves their bytes survive.
fn child_lines(node: &GreenNode) -> Vec<&GreenNode> {
    node.children()
        .iter()
        .filter_map(|child| match child {
            GreenChild::Node(inner) if inner.kind() == crate::yaml::kind::LINE => {
                Some(inner.as_ref())
            }
            _ => None,
        })
        .filter(|line| !is_empty_subtree(line))
        .collect()
}

/// A line with nothing to say and no descendants that do.
///
/// Not recursive on purpose: a comment attaches to the innermost *indented*
/// line, never to another comment, so this nests one level and a loop would be
/// guarding against a shape the lexer cannot produce.
fn is_empty_subtree(line: &GreenNode) -> bool {
    let has_tokens = line.children().iter().any(|child| match child {
        GreenChild::Token(token) => !is_layout(token.kind()),
        GreenChild::Node(_) => false,
    });
    if has_tokens {
        return false;
    }
    !line.children().iter().any(|child| match child {
        GreenChild::Node(inner) => inner.kind() == crate::yaml::kind::LINE,
        GreenChild::Token(_) => false,
    })
}

/// Layout tokens: present in the bytes, absent from the meaning.
fn is_layout(kind: core_cst::SyntaxKind) -> bool {
    use crate::yaml::kind;
    matches!(
        kind,
        kind::INDENT | kind::SPACE | kind::NEWLINE | kind::COMMENT
    )
}

fn unmodelled(reason: &'static str) -> Unmodelled {
    Unmodelled {
        format: "yaml",
        reason,
    }
}

/// Classify one line from its own tokens, ignoring its nested lines.
fn classify(line: &GreenNode) -> Result<Line<'_>, Unmodelled> {
    use crate::yaml::kind;

    let tokens: Vec<&core_cst::GreenToken> = line
        .children()
        .iter()
        .filter_map(|child| match child {
            GreenChild::Token(token) if !is_layout(token.kind()) => Some(token.as_ref()),
            _ => None,
        })
        .collect();

    let Some(first) = tokens.first() else {
        return Ok(Line::Blank);
    };

    match first.kind() {
        kind::DASH => Ok(Line::Item {
            rest: tokens.get(1..).unwrap_or_default().to_vec(),
        }),
        // Everything below is a construct whose meaning this layer does not
        // model. Naming them individually costs nothing and makes the refusal
        // message tell the user what to do about it.
        kind::DOC_START | kind::DOC_END => Ok(Line::Marker),
        kind::DIRECTIVE => Err(unmodelled("directives are not modelled yet (M2)")),
        kind::ANCHOR | kind::ALIAS => {
            Err(unmodelled("anchors and aliases are not modelled yet (M2)"))
        }
        kind::TAG => Err(unmodelled("tags are not modelled yet (M2)")),
        kind::FLOW_PUNCT => Err(unmodelled("flow collections are not modelled yet (M2)")),
        kind::BLOCK_HEADER | kind::BLOCK_BODY => {
            Err(unmodelled("block scalars are not modelled yet (M2)"))
        }
        _ => {
            let colon = tokens
                .iter()
                .position(|token| token.kind() == kind::COLON)
                .ok_or_else(|| {
                    unmodelled("a line that is neither a mapping entry nor a sequence item")
                })?;
            let key_tokens = tokens.get(..colon).unwrap_or_default();
            if key_tokens.len() != 1 {
                return Err(unmodelled(
                    "compound mapping keys are not modelled yet (M2)",
                ));
            }
            let key = key_tokens
                .first()
                .map(|token| resolve(token).value)
                .unwrap_or_default();
            Ok(Line::Entry {
                key,
                value: tokens.get(colon + 1..).unwrap_or_default().to_vec(),
            })
        }
    }
}

/// A scalar's source text and its resolved value.
fn resolve(token: &core_cst::GreenToken) -> Scalar {
    use crate::yaml::kind;
    let text = String::from_utf8_lossy(token.text()).into_owned();
    let value = match token.kind() {
        // `'it''s'` → `it's`. The only escape single quotes have.
        kind::SINGLE_QUOTED => text
            .strip_prefix('\'')
            .and_then(|s| s.strip_suffix('\''))
            .unwrap_or(&text)
            .replace("''", "'"),
        kind::DOUBLE_QUOTED => {
            let inner = text
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(&text);
            unescape(&format!("\"{inner}\""))
        }
        // Plain scalars resolve to themselves. Deciding that `yes` is a boolean
        // is YAML-version-dependent and ADR-008 already refused to make that
        // call one layer down.
        _ => text.clone(),
    };
    Scalar { text, value }
}

/// Turn the tokens after a `:` or a `-`, plus any nested lines, into a value.
fn value_from(
    tokens: &[&core_cst::GreenToken],
    nested: &[&GreenNode],
) -> Result<SemanticNode, Unmodelled> {
    use crate::yaml::kind;

    // Name the construct before falling back to a count. "A value of several
    // tokens" is true and useless; "block scalars are not modelled yet" tells
    // the user which line to look at and what we will not do with it.
    for token in tokens {
        let reason = match token.kind() {
            kind::ANCHOR | kind::ALIAS => "anchors and aliases are not modelled yet (M2)",
            kind::TAG => "tags are not modelled yet (M2)",
            _ => continue,
        };
        return Err(unmodelled(reason));
    }

    // A block scalar is its header and its body, and nothing else can be on the
    // line. Its VALUE is deliberately its source text rather than the string
    // YAML would fold it into: implementing indentation stripping and chomping
    // correctly is a real piece of spec work, and getting it subtly wrong would
    // make two different strings compare equal — a diff that misses an edit.
    //
    // The cost is over-reporting: re-indenting a block body, or switching `|`
    // for `>`, reads as a semantic change when the folded string is unchanged.
    // That is the same trade already taken on numbers, and in the same
    // direction — noisy is recoverable, silent is not. ADR-013 amendment 1.
    if tokens
        .first()
        .is_some_and(|token| token.kind() == kind::BLOCK_HEADER)
    {
        if !nested.is_empty() {
            return Err(unmodelled(
                "a block scalar with further nested lines is not modelled yet (M2)",
            ));
        }
        let mut text = String::new();
        for token in tokens {
            match token.kind() {
                kind::BLOCK_HEADER | kind::BLOCK_BODY => {
                    text.push_str(&String::from_utf8_lossy(token.text()));
                }
                _ => return Err(unmodelled("a block scalar sharing its line with a value")),
            }
        }
        let value = text.clone();
        return Ok(SemanticNode::Scalar(Scalar { text, value }));
    }

    // A flow collection is a whole value in one line's tokens: `{a: 1, b: 2}`
    // or `[1, 2]`, nested arbitrarily. It gets its own small recursive parse
    // rather than being folded into the block reader, because the two disagree
    // about what ends a value — indentation there, a bracket here.
    if tokens
        .first()
        .is_some_and(|token| token.kind() == kind::FLOW_PUNCT)
    {
        if !nested.is_empty() {
            return Err(unmodelled(
                "a flow collection spanning several lines is not modelled yet (M2)",
            ));
        }
        let mut cursor = 0usize;
        let node = flow_value(tokens, &mut cursor)?;
        if cursor != tokens.len() {
            return Err(unmodelled("trailing tokens after a flow collection"));
        }
        return Ok(node);
    }

    for token in tokens {
        if token.kind() == kind::FLOW_PUNCT {
            return Err(unmodelled(
                "a flow collection that does not start the value is not modelled yet (M2)",
            ));
        }
    }

    match tokens.len() {
        // `key:` with children below it, or with nothing at all.
        0 => {
            if nested.is_empty() {
                return Ok(SemanticNode::Scalar(Scalar {
                    text: String::new(),
                    value: String::new(),
                }));
            }
            node_from_lines(nested)
        }
        1 => {
            let scalar = tokens.first().map(|token| resolve(token)).ok_or_else(|| {
                unmodelled("a value token vanished between counting and reading it")
            })?;
            if nested.is_empty() {
                Ok(SemanticNode::Scalar(scalar))
            } else {
                // `key: value` with more-indented lines under it is a multi-line
                // plain scalar or something stranger. Either way, not modelled.
                Err(unmodelled(
                    "a scalar with nested lines beneath it is not modelled yet (M2)",
                ))
            }
        }
        _ => Err(unmodelled(
            "a value of several tokens — an anchor, tag or flow collection — is not modelled yet (M2)",
        )),
    }
}

/// Parse one flow value starting at `*cursor`, leaving it just past the value.
///
/// Recursive, and bounded by bracket nesting rather than by input length. A
/// fuzzer will find a document that is nothing but ten thousand `[`, so the
/// depth is capped and a document past the cap is refused rather than being
/// allowed to take the stack with it — the same failure the deep-nesting test
/// found in `core-cst`'s destructor at M1.
fn flow_value(
    tokens: &[&core_cst::GreenToken],
    cursor: &mut usize,
) -> Result<SemanticNode, Unmodelled> {
    flow_value_at(tokens, cursor, 0)
}

/// How deep a flow collection may nest before we decline to model it.
const FLOW_DEPTH_CAP: usize = 64;

fn flow_value_at(
    tokens: &[&core_cst::GreenToken],
    cursor: &mut usize,
    depth: usize,
) -> Result<SemanticNode, Unmodelled> {
    use crate::yaml::kind;

    if depth > FLOW_DEPTH_CAP {
        return Err(unmodelled(
            "a flow collection nested past the depth this layer models",
        ));
    }
    let token = tokens
        .get(*cursor)
        .ok_or_else(|| unmodelled("a flow collection ends before its value"))?;

    if token.kind() != kind::FLOW_PUNCT {
        *cursor += 1;
        return Ok(SemanticNode::Scalar(resolve(token)));
    }

    let (close, is_mapping) = match token.text() {
        b"[" => (b"]".as_slice(), false),
        b"{" => (b"}".as_slice(), true),
        _ => {
            return Err(unmodelled(
                "a flow collection opens with a bracket we do not model",
            ));
        }
    };
    *cursor += 1;

    let mut entries: Vec<(String, SemanticNode)> = Vec::new();
    let mut items: Vec<SemanticNode> = Vec::new();
    loop {
        let next = tokens
            .get(*cursor)
            .ok_or_else(|| unmodelled("a flow collection is never closed"))?;
        if next.kind() == kind::FLOW_PUNCT && next.text() == close {
            *cursor += 1;
            break;
        }
        if next.kind() == kind::FLOW_PUNCT && next.text() == b"," {
            *cursor += 1;
            continue;
        }

        if is_mapping {
            let key_token = tokens
                .get(*cursor)
                .ok_or_else(|| unmodelled("a flow mapping ends before its key"))?;
            *cursor += 1;
            let colon = tokens
                .get(*cursor)
                .ok_or_else(|| unmodelled("a flow mapping key has no value"))?;
            if colon.kind() != kind::COLON {
                return Err(unmodelled("a flow mapping entry without a colon"));
            }
            *cursor += 1;
            entries.push((
                resolve(key_token).value,
                flow_value_at(tokens, cursor, depth + 1)?,
            ));
        } else {
            items.push(flow_value_at(tokens, cursor, depth + 1)?);
        }
    }

    if is_mapping {
        Ok(SemanticNode::Mapping(entries))
    } else {
        Ok(SemanticNode::Sequence(items))
    }
}

/// Group sibling lines into the collection they describe.
fn node_from_lines(lines: &[&GreenNode]) -> Result<SemanticNode, Unmodelled> {
    let mut entries: Vec<(String, SemanticNode)> = Vec::new();
    let mut items: Vec<SemanticNode> = Vec::new();

    let mut index = 0usize;
    while let Some(line) = lines.get(index) {
        let nested = child_lines(line);
        index += 1;
        match classify(line)? {
            Line::Blank => {}
            // A boundary inside a collection is not a boundary, it is a
            // document marker somewhere it cannot mean anything. Refusing beats
            // guessing which side of it the surrounding keys belong to.
            Line::Marker => {
                return Err(unmodelled("a document marker indented inside a collection"));
            }
            Line::Item { rest } => items.push(item_value(&rest, &nested)?),
            Line::Entry { key, value } => {
                // A zero-indented sequence: `items:` with `- a` beneath it at
                // the SAME indent. YAML allows it and Kubernetes manifests are
                // written that way more often than not, but the indentation
                // that builds the concrete tree makes those dashes *siblings*
                // of the key rather than its children. Reading them as
                // siblings would either refuse the file or, worse, describe a
                // mapping and a sequence sharing one level — so the key adopts
                // the run of items that immediately follows it.
                if value.is_empty() && nested.is_empty() {
                    let mut adopted: Vec<SemanticNode> = Vec::new();
                    while let Some(next) = lines.get(index) {
                        let next_nested = child_lines(next);
                        match classify(next)? {
                            Line::Item { rest } => {
                                adopted.push(item_value(&rest, &next_nested)?);
                                index += 1;
                            }
                            _ => break,
                        }
                    }
                    if !adopted.is_empty() {
                        entries.push((key, SemanticNode::Sequence(adopted)));
                        continue;
                    }
                }
                entries.push((key, value_from(&value, &nested)?));
            }
        }
    }

    match (entries.is_empty(), items.is_empty()) {
        // An empty document. Distinguishable from `key:` with no value only by
        // context, and neither is a collection.
        (true, true) => Ok(SemanticNode::Scalar(Scalar {
            text: String::new(),
            value: String::new(),
        })),
        (false, true) => Ok(SemanticNode::Mapping(entries)),
        (true, false) => Ok(SemanticNode::Sequence(items)),
        // `- a` and `b: 1` as siblings, with the dashes not adopted by any key
        // above them. Not valid YAML, and a view that picked one reading would
        // be inventing a document nobody wrote.
        (false, false) => Err(unmodelled(
            "mapping entries and sequence items at the same level",
        )),
    }
}

/// A sequence item, which may be a scalar or a mapping that starts on the dash.
fn item_value(
    rest: &[&core_cst::GreenToken],
    nested: &[&GreenNode],
) -> Result<SemanticNode, Unmodelled> {
    use crate::yaml::kind;

    // `- name: web` followed by more-indented `image: nginx` is one mapping
    // whose first pair happens to share the dash's line.
    let Some(colon) = rest.iter().position(|token| token.kind() == kind::COLON) else {
        return value_from(rest, nested);
    };

    let key_tokens = rest.get(..colon).unwrap_or_default();
    if key_tokens.len() != 1 {
        return Err(unmodelled(
            "compound mapping keys are not modelled yet (M2)",
        ));
    }
    let key = key_tokens
        .first()
        .map(|token| resolve(token).value)
        .unwrap_or_default();
    let inline = value_from(rest.get(colon + 1..).unwrap_or_default(), &[])?;

    let mut entries = vec![(key, inline)];
    let SemanticNode::Mapping(rest_of_it) = node_from_lines(nested)? else {
        if nested.is_empty() {
            return Ok(SemanticNode::Mapping(entries));
        }
        return Err(unmodelled(
            "a sequence item mixing a mapping entry with non-entry lines",
        ));
    };
    entries.extend(rest_of_it);
    Ok(SemanticNode::Mapping(entries))
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

    // --- YAML -------------------------------------------------------------

    fn yaml_view(src: &str) -> SemanticNode {
        let cst = crate::Yaml.parse(src.as_bytes()).expect("parses");
        crate::Yaml.semantic_view(&cst).expect("has a view")
    }

    fn yaml_refusal(src: &str) -> &'static str {
        let cst = crate::Yaml.parse(src.as_bytes()).expect("parses");
        crate::Yaml
            .semantic_view(&cst)
            .expect_err("should refuse")
            .reason
    }

    #[test]
    fn indentation_becomes_nesting() {
        let SemanticNode::Mapping(pairs) = yaml_view("spec:\n  replicas: 2\n") else {
            panic!("expected a mapping");
        };
        let Some((key, SemanticNode::Mapping(inner))) = pairs.first() else {
            panic!("expected a nested mapping");
        };
        assert_eq!(key, "spec");
        assert_eq!(inner.len(), 1);
    }

    #[test]
    fn a_dash_list_becomes_a_sequence() {
        let SemanticNode::Mapping(pairs) = yaml_view("xs:\n  - a\n  - b\n") else {
            panic!("expected a mapping");
        };
        let Some((_, SemanticNode::Sequence(items))) = pairs.first() else {
            panic!("expected a sequence");
        };
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn a_sequence_item_continues_across_its_indented_lines() {
        // `- name: web` then a deeper `image: ...` is ONE mapping, not two
        // items. Getting this wrong shifts every path underneath it.
        let SemanticNode::Mapping(pairs) =
            yaml_view("containers:\n  - name: web\n    image: nginx:1.25\n")
        else {
            panic!("expected a mapping");
        };
        let Some((_, SemanticNode::Sequence(items))) = pairs.first() else {
            panic!("expected a sequence");
        };
        assert_eq!(items.len(), 1, "the indented line started a second item");
        let Some(SemanticNode::Mapping(entry)) = items.first() else {
            panic!("expected the item to be a mapping");
        };
        let keys: Vec<&str> = entry.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, ["name", "image"]);
    }

    #[test]
    fn a_colon_inside_a_value_is_not_the_separator() {
        // `image: nginx:1.25` has two colons and one of them is punctuation.
        let SemanticNode::Mapping(pairs) = yaml_view("image: nginx:1.25\n") else {
            panic!("expected a mapping");
        };
        let Some((key, SemanticNode::Scalar(value))) = pairs.first() else {
            panic!("expected a scalar");
        };
        assert_eq!(key, "image");
        assert_eq!(value.value, "nginx:1.25");
    }

    #[test]
    fn quoting_style_is_spelling_not_meaning() {
        let plain = yaml_view("k: web\n");
        let quoted = yaml_view("k: \"web\"\n");
        let single = yaml_view("k: 'web'\n");
        assert!(plain.same_value(&quoted));
        assert!(plain.same_value(&single));
        assert_ne!(plain, quoted, "source text must still differ");
    }

    #[test]
    fn comments_and_blank_lines_are_not_content() {
        // They carry no meaning, so a view without them is complete. Their
        // bytes are the CST's business, where K1 already proves they survive.
        assert!(yaml_view("# hi\n\na: 1\n").same_value(&yaml_view("a: 1\n")));
    }

    #[test]
    fn unmodelled_constructs_refuse_and_say_which() {
        // Each of these is a real YAML feature we do not model yet. Refusing
        // names it; dropping it would leave a view with fewer keys than the
        // document has, and a diff over that view would miss real edits.
        for (src, expected) in [
            ("a: {\n  x: 1}\n", "several lines"),
            ("a: &x 1\n", "anchors and aliases"),
            ("a: !!str 1\n", "tags"),
        ] {
            let reason = yaml_refusal(src);
            assert!(
                reason.contains(expected),
                "for {src:?} expected a reason mentioning {expected:?}, got {reason:?}"
            );
        }
    }

    #[test]
    fn a_flow_mapping_is_a_mapping() {
        let SemanticNode::Mapping(pairs) = yaml_view("selector: {app: web, tier: front}\n") else {
            panic!("expected a mapping");
        };
        let Some((_, SemanticNode::Mapping(inner))) = pairs.first() else {
            panic!("expected a nested mapping");
        };
        let keys: Vec<&str> = inner.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, ["app", "tier"]);
    }

    #[test]
    fn a_flow_sequence_is_a_sequence_and_empty_means_empty() {
        let SemanticNode::Mapping(pairs) = yaml_view("ports: [80, 443]\nargs: []\n") else {
            panic!("expected a mapping");
        };
        let Some((_, SemanticNode::Sequence(ports))) = pairs.first() else {
            panic!("expected a sequence");
        };
        assert_eq!(ports.len(), 2);
        let Some((_, SemanticNode::Sequence(args))) = pairs.get(1) else {
            panic!("expected a sequence");
        };
        assert!(args.is_empty(), "`[]` is an empty sequence, not a scalar");
    }

    #[test]
    fn flow_and_block_spell_the_same_mapping() {
        // The point of a semantic view: two spellings, one meaning. A diff
        // between these reports no *semantic* change, which is correct.
        assert!(yaml_view("a: {x: 1}\n").same_value(&yaml_view("a:\n  x: 1\n")));
    }

    #[test]
    fn flow_collections_nest() {
        let SemanticNode::Mapping(pairs) = yaml_view("a: {xs: [1, {y: 2}]}\n") else {
            panic!("expected a mapping");
        };
        let Some((_, SemanticNode::Mapping(inner))) = pairs.first() else {
            panic!("expected a nested mapping");
        };
        let Some((_, SemanticNode::Sequence(xs))) = inner.first() else {
            panic!("expected a nested sequence");
        };
        assert!(matches!(xs.get(1), Some(SemanticNode::Mapping(_))));
    }

    #[test]
    fn a_flow_collection_nested_past_the_cap_is_refused_not_a_stack_overflow() {
        // core-cst's destructor taught this lesson at M1: a fuzzer will send a
        // document that is nothing but brackets, and recursion must decline
        // rather than take the process with it.
        let deep = format!("a: {}{}\n", "[".repeat(200), "]".repeat(200));
        let reason = yaml_refusal(&deep);
        assert!(reason.contains("depth"), "got {reason:?}");
    }

    #[test]
    fn a_block_scalar_is_a_scalar_carrying_its_own_source() {
        let SemanticNode::Mapping(pairs) = yaml_view("script: |\n  echo one\nshell: sh\n") else {
            panic!("expected a mapping");
        };
        let Some((key, SemanticNode::Scalar(scalar))) = pairs.first() else {
            panic!("expected a scalar");
        };
        assert_eq!(key, "script");
        assert_eq!(scalar.text, "|\n  echo one");
        assert_eq!(
            scalar.value, scalar.text,
            "block scalars are deliberately not folded — ADR-013 amendment 1"
        );
    }

    #[test]
    fn block_style_is_not_folded_so_a_restyle_reads_as_semantic() {
        // Documented over-reporting. `|` and `>` fold differently, and deciding
        // they agree would need the folding rules implemented correctly; a
        // subtle error there makes two different strings compare equal, which
        // is a diff that misses an edit. Noisy is recoverable, silent is not.
        assert!(!yaml_view("a: |\n  x\nb: 1\n").same_value(&yaml_view("a: >\n  x\nb: 1\n")));
    }

    #[test]
    fn a_block_scalar_at_end_of_file_owns_the_final_newline() {
        // A lexer wart, pinned rather than papered over: the body absorbs the
        // trailing newline only when nothing follows the block. Same logical
        // content, two different node texts — so comparing a block at end of
        // file against the same content mid-file over-reports. Safe direction,
        // and now it is written down instead of surprising someone at M3.
        let SemanticNode::Mapping(last) = yaml_view("script: |\n  echo one\n") else {
            panic!("expected a mapping");
        };
        let SemanticNode::Mapping(mid) = yaml_view("script: |\n  echo one\nshell: sh\n") else {
            panic!("expected a mapping");
        };
        let (Some((_, SemanticNode::Scalar(at_end))), Some((_, SemanticNode::Scalar(in_mid)))) =
            (last.first(), mid.first())
        else {
            panic!("expected scalars");
        };
        assert_eq!(at_end.text, "|\n  echo one\n");
        assert_eq!(in_mid.text, "|\n  echo one");
    }

    #[test]
    fn two_documents_become_a_stream() {
        let SemanticNode::Stream(docs) = yaml_view("kind: Service\n---\nkind: Deployment\n") else {
            panic!("expected a stream");
        };
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn one_document_with_markers_is_still_one_document() {
        // `---` at the top is a marker, not a second document. Wrapping it
        // would make this compare as a different *shape* from the same file
        // without the marker — a formatting difference reported as structural.
        assert!(yaml_view("---\na: 1\n").same_value(&yaml_view("a: 1\n")));
        assert!(yaml_view("a: 1\n...\n").same_value(&yaml_view("a: 1\n")));
    }

    #[test]
    fn a_stream_never_aligns_against_a_plain_sequence() {
        // The reason Stream is its own variant. Two documents and one document
        // that happens to be a two-item list index identically and mean
        // entirely different things; pairing them would diff document 0
        // against list item 0 and report the difference as ordinary edits.
        let stream = yaml_view("a: 1\n---\nb: 2\n");
        let sequence = yaml_view("- a: 1\n- b: 2\n");
        assert!(matches!(stream, SemanticNode::Stream(_)));
        assert!(matches!(sequence, SemanticNode::Sequence(_)));
        assert!(!stream.same_value(&sequence));
    }

    #[test]
    fn a_document_marker_indented_inside_a_collection_is_refused() {
        // A marker somewhere it cannot mean anything. Guessing which side of it
        // the surrounding keys belong to is exactly the invention ADR-012 bans.
        let reason = yaml_refusal("a:\n  ---\n  b: 1\n");
        assert!(reason.contains("document marker"), "got {reason:?}");
    }

    #[test]
    fn a_line_we_cannot_classify_refuses_the_whole_document() {
        // The load-bearing rule. Skipping the Helm template would produce a
        // view claiming this document has one key, and a diff over it would
        // report "no changes" for an edit inside the template.
        let reason = yaml_refusal("a: 1\n{{- if .Values.x }}\nb: 2\n{{- end }}\n");
        assert!(
            !reason.is_empty(),
            "a template must not be silently dropped"
        );
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
