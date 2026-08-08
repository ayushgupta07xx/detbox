//! # `core-cst` — lossless concrete syntax trees
//!
//! **Mission (MASTER_PLAN §3.1).** The load-bearing crate of the entire
//! ecosystem. A CST here owns *every byte* of the input: keys, values,
//! comments, whitespace, ordering, YAML anchors/aliases, quoting style, line
//! endings, trailing garbage. Every other crate in this workspace is downstream
//! of this promise.
//!
//! ## Invariants (machine-checked, permanent)
//!
//! - **K1 Round-trip** — `serialize(parse(x)) == x`, byte-identical, for every
//!   `x` in the corpus and every fuzz input that parses. This is the
//!   credibility of the whole platform; it is the first CI gate ever written
//!   and it never comes out.
//! - **K2 Edit locality** — after an edit operation, all bytes outside the
//!   edited span(s) are unchanged.
//! - **K3 Determinism** — identical input + identical operation sequence →
//!   identical output bytes, on every platform.
//!
//! ## Representation
//!
//! A green/red tree, decided by measurement in
//! [ADR-001](https://github.com/ayushgupta07xx/detbox/blob/main/adr/ADR-001-cst-representation.md).
//! The green tree is immutable, refcounted and shared; tokens own their text
//! and are interned. It stores no parent pointers and no absolute offsets — a
//! red layer supplies those on demand — which is what makes an edit cost
//! `O(depth)` new nodes while sharing every untouched subtree.
//!
//! **Why K1 is structural here, not a promise.** [`Cst::serialize`] is a
//! straight in-order walk emitting each token's bytes. It has no format
//! knowledge, no normalisation step, and no opportunity to be clever. So K1
//! reduces entirely to *"did `parse` put every input byte into some token?"* —
//! one question, in one place, instead of a property spread across a
//! serializer.
//!
//! ## Escape hatch for hostile input
//!
//! Anything the modelled grammar cannot represent is preserved as an opaque
//! [`SyntaxKind::VERBATIM`] node rather than normalised. **Preserving beats
//! understanding; K1 outranks elegance.**
//!
//! This is not a rare path. A survey of the 750-file YAML corpus
//! (`cargo xtask corpus-survey`) found Helm's Go templating in **41.2%** of
//! files — text that is not YAML at all and can only be preserved verbatim.
//! The escape hatch is the main road for a large minority of real config.
//!
//! ## Status
//!
//! **konflux M1, oracle stage.** The types below are the contract the K1 oracle
//! tests. There is no parser yet: [`core-formats`] returns a `ParseReport`
//! saying so, and the round-trip suites are red until it exists. That is the
//! point — a test never observed failing is not known to test anything.
//!
//! [`core-formats`]: https://github.com/ayushgupta07xx/detbox/tree/main/crates/core-formats

use std::rc::Rc;

/// A syntax kind, opaque to this crate.
///
/// Each format defines its own constants, exactly as rowan does: the tree
/// machinery never needs to know what a `KEY` or a `BLOCK_SCALAR` is, only that
/// tokens carry bytes and nodes carry children.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SyntaxKind(pub u16);

impl SyntaxKind {
    /// Reserved for the §3.1 escape hatch: a span the grammar cannot model,
    /// preserved byte-for-byte instead of normalised.
    ///
    /// Formats **must not** use this value for a modelled construct. It is the
    /// one kind whose meaning is fixed across every format: *these bytes were
    /// not understood, and are reproduced exactly.*
    pub const VERBATIM: Self = Self(u16::MAX);
}

/// A half-open byte range of the source.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Span {
    /// First byte, inclusive.
    pub start: u32,
    /// One past the last byte.
    pub end: u32,
}

impl Span {
    /// Length in bytes.
    #[must_use]
    pub fn len(&self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    /// Whether the span covers no bytes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }
}

/// A leaf: a kind and the exact bytes it covers.
#[derive(Debug)]
pub struct GreenToken {
    kind: SyntaxKind,
    text: Box<[u8]>,
}

impl GreenToken {
    /// Build a token from its bytes.
    #[must_use]
    pub fn new(kind: SyntaxKind, text: &[u8]) -> Self {
        Self {
            kind,
            text: text.into(),
        }
    }

    /// This token's kind.
    #[must_use]
    pub fn kind(&self) -> SyntaxKind {
        self.kind
    }

    /// The exact bytes this token covers.
    #[must_use]
    pub fn text(&self) -> &[u8] {
        &self.text
    }
}

/// Either a nested node or a token.
#[derive(Debug)]
pub enum GreenChild {
    /// A nested node.
    Node(Rc<GreenNode>),
    /// A leaf.
    Token(Rc<GreenToken>),
}

impl GreenChild {
    /// Bytes covered by this child.
    #[must_use]
    pub fn text_len(&self) -> u32 {
        match self {
            Self::Node(node) => node.text_len(),
            Self::Token(token) => u32::try_from(token.text().len()).unwrap_or(u32::MAX),
        }
    }
}

/// An interior node: a kind, its children, and the byte length it covers.
///
/// Deliberately stores no parent pointer and no absolute offset. That omission
/// is what lets a subtree be shared between two versions of a document, which
/// ADR-001 measured at 1.02x memory to hold both versions against 2.00x for
/// every alternative.
#[derive(Debug)]
pub struct GreenNode {
    kind: SyntaxKind,
    text_len: u32,
    children: Vec<GreenChild>,
}

impl GreenNode {
    /// Build a node from its children, summing their byte lengths.
    #[must_use]
    pub fn new(kind: SyntaxKind, children: Vec<GreenChild>) -> Self {
        let text_len = children.iter().map(GreenChild::text_len).sum();
        Self {
            kind,
            text_len,
            children,
        }
    }

    /// This node's kind.
    #[must_use]
    pub fn kind(&self) -> SyntaxKind {
        self.kind
    }

    /// Bytes covered by this node and everything under it.
    #[must_use]
    pub fn text_len(&self) -> u32 {
        self.text_len
    }

    /// This node's children, in document order.
    #[must_use]
    pub fn children(&self) -> &[GreenChild] {
        &self.children
    }
}

/// Destroy the tree iteratively.
///
/// # Why this exists
///
/// Without it, dropping a `GreenNode` recurses: the `Vec<GreenChild>` drops
/// each `Rc<GreenNode>`, whose own drop drops its children, and so on. On a
/// deeply nested document that overflows the stack and **aborts the process** —
/// `SIGABRT`, not a catchable panic.
///
/// That would defeat both §3.2 F1 (`parse` never panics) and F2 (`serialize` is
/// total): a hostile input could crash the process on the way out, long after
/// parsing "succeeded". For a tool whose first priority is never being silently
/// wrong, crashing during cleanup is not an acceptable failure mode.
///
/// Found by `serialize_does_not_overflow_the_stack_on_deep_nesting`, which was
/// written to check the serializer and caught the destructor instead. ADR-001
/// records it as a consequence of choosing a refcounted tree.
///
/// The loop takes ownership of each child subtree we are the last owner of and
/// pushes its children onto a heap-allocated stack, so each node is dropped with
/// an already-empty child list and cannot recurse. Shared subtrees — the whole
/// point of ADR-001's choice — are left alone: `Rc::into_inner` yields `None`
/// when another version still references them.
impl Drop for GreenNode {
    fn drop(&mut self) {
        let mut stack: Vec<GreenChild> = std::mem::take(&mut self.children);
        while let Some(child) = stack.pop() {
            if let GreenChild::Node(rc) = child
                && let Some(mut node) = Rc::into_inner(rc)
            {
                stack.append(&mut node.children);
            }
        }
    }
}

/// A parsed document: the root of a lossless tree.
///
/// The only way to build one is [`core-formats`]' `Format::parse`, so a `Cst`
/// that exists is a `Cst` that came from real bytes.
///
/// [`core-formats`]: https://github.com/ayushgupta07xx/detbox/tree/main/crates/core-formats
#[derive(Debug)]
pub struct Cst {
    root: Rc<GreenNode>,
}

impl Cst {
    /// Wrap a green root.
    #[must_use]
    pub fn new(root: Rc<GreenNode>) -> Self {
        Self { root }
    }

    /// The root node.
    #[must_use]
    pub fn root(&self) -> &Rc<GreenNode> {
        &self.root
    }

    /// Emit the exact bytes this tree covers.
    ///
    /// This is the `serialize` half of K1, and it is deliberately incapable of
    /// being interesting: an in-order walk that concatenates token text. It
    /// cannot reformat, cannot reorder, cannot normalise quoting, and has no
    /// format knowledge with which to try. Every K1 failure is therefore a
    /// `parse` failure, which is where the difficulty genuinely is.
    ///
    /// Iterative rather than recursive: the corpus contains files nesting 8+
    /// levels deep, and a stack overflow in the serializer would be a panic in
    /// the one function that must be total (§3.2 F2).
    #[must_use]
    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.root.text_len() as usize);
        let mut stack: Vec<&GreenChild> = Vec::new();
        for child in self.root.children().iter().rev() {
            stack.push(child);
        }
        while let Some(child) = stack.pop() {
            match child {
                GreenChild::Token(token) => out.extend_from_slice(token.text()),
                GreenChild::Node(node) => {
                    for grandchild in node.children().iter().rev() {
                        stack.push(grandchild);
                    }
                }
            }
        }
        out
    }
}

/// The K1 identity on the empty grammar.
///
/// # Phase 0 leftover
///
/// ADR-003 introduced this so the golden, fuzz, determinism and miri gates were
/// non-vacuous before a parser existed, and says it is deleted once the real
/// `parse`/`serialize` pair ships. That has not happened yet: `parse` is still
/// a stub, so the Phase 0 gates still point here. It goes when parse lands.
#[must_use]
pub fn roundtrip_identity(input: &[u8]) -> Vec<u8> {
    input.to_vec()
}

#[cfg(test)]
mod tests {
    use super::{Cst, GreenChild, GreenNode, GreenToken, Span, SyntaxKind, roundtrip_identity};
    use std::rc::Rc;

    const ROOT: SyntaxKind = SyntaxKind(0);
    const WORD: SyntaxKind = SyntaxKind(1);

    fn token(text: &[u8]) -> GreenChild {
        GreenChild::Token(Rc::new(GreenToken::new(WORD, text)))
    }

    #[test]
    fn k1_holds_on_the_empty_grammar() {
        for case in [
            &b""[..],
            b"a",
            b"key: value  # comment\n",
            b"\r\n\r\n",
            b"\xff\xfe\x00invalid utf-8",
        ] {
            assert_eq!(roundtrip_identity(case), case, "K1 violated for {case:?}");
        }
    }

    #[test]
    fn serialize_concatenates_tokens_in_document_order() {
        let cst = Cst::new(Rc::new(GreenNode::new(
            ROOT,
            vec![token(b"a: "), token(b"1"), token(b"\n")],
        )));
        assert_eq!(cst.serialize(), b"a: 1\n");
    }

    #[test]
    fn serialize_descends_and_preserves_order() {
        let inner = GreenChild::Node(Rc::new(GreenNode::new(
            ROOT,
            vec![token(b"  b: "), token(b"2\n")],
        )));
        let cst = Cst::new(Rc::new(GreenNode::new(
            ROOT,
            vec![token(b"a:\n"), inner, token(b"c: 3\n")],
        )));
        assert_eq!(cst.serialize(), b"a:\n  b: 2\nc: 3\n");
    }

    #[test]
    fn serialize_is_byte_transparent() {
        // Non-UTF-8, NUL and CRLF must pass through untouched: the serializer
        // sees bytes, never strings.
        let raw: &[u8] = b"k: \xff\xfe\x00v\r\n";
        let cst = Cst::new(Rc::new(GreenNode::new(ROOT, vec![token(raw)])));
        assert_eq!(cst.serialize(), raw);
    }

    fn nested(depth: usize) -> Rc<GreenNode> {
        let mut node = Rc::new(GreenNode::new(ROOT, vec![token(b"x")]));
        for _ in 0..depth {
            node = Rc::new(GreenNode::new(ROOT, vec![GreenChild::Node(node)]));
        }
        node
    }

    #[test]
    fn serialize_does_not_overflow_the_stack_on_deep_nesting() {
        // The corpus nests 8+ levels; a recursive serializer would be a panic
        // waiting for a pathological file. 10_000 levels proves it is iterative.
        assert_eq!(Cst::new(nested(10_000)).serialize(), b"x");
    }

    #[test]
    fn dropping_a_deep_tree_does_not_overflow_the_stack() {
        // The test above found this the hard way: serialisation was already
        // iterative, but the *destructor* recursed and aborted the process with
        // SIGABRT. See the `Drop` impl. 100_000 levels, well past anything the
        // serializer test covers, because a crash on cleanup is not catchable.
        drop(nested(100_000));
    }

    #[test]
    fn dropping_does_not_free_a_shared_subtree() {
        // The iterative drop must not confuse "I am the last owner" with "this
        // is garbage". Structural sharing between versions is the entire reason
        // ADR-001 chose this representation.
        let shared = nested(1_000);
        let a = Cst::new(Rc::new(GreenNode::new(
            ROOT,
            vec![GreenChild::Node(Rc::clone(&shared))],
        )));
        let b = Cst::new(Rc::new(GreenNode::new(
            ROOT,
            vec![GreenChild::Node(Rc::clone(&shared))],
        )));
        drop(a);
        assert_eq!(
            b.serialize(),
            b"x",
            "dropping one version damaged the other"
        );
        drop(b);
        assert_eq!(
            Rc::strong_count(&shared),
            1,
            "shared subtree leaked a reference"
        );
    }

    #[test]
    fn text_len_sums_the_whole_subtree() {
        let inner = GreenChild::Node(Rc::new(GreenNode::new(ROOT, vec![token(b"12345")])));
        let node = GreenNode::new(ROOT, vec![token(b"ab"), inner]);
        assert_eq!(node.text_len(), 7);
    }

    #[test]
    fn verbatim_is_reserved_and_distinct() {
        assert_eq!(SyntaxKind::VERBATIM, SyntaxKind(u16::MAX));
        assert_ne!(SyntaxKind::VERBATIM, WORD);
    }

    #[test]
    fn span_arithmetic_never_underflows() {
        // Spans come from parsers, and a parser with a bug must not turn a
        // reversed span into a panic or a huge length.
        let reversed = Span { start: 10, end: 4 };
        assert_eq!(reversed.len(), 0);
        assert!(reversed.is_empty());
        assert_eq!(Span { start: 4, end: 10 }.len(), 6);
    }
}
