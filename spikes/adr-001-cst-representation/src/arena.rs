//! **Candidate C — flat arena of spans.**
//!
//! One `Vec<Elem>` for the whole tree. Children are a `first_child` /
//! `next_sibling` linked list of `u32` indices, so there are no per-node
//! allocations at all: building a tree is one growing vector. Crucially, a
//! token does not *own* its text — it is a `(start, len)` span into the source
//! buffer, which the tree keeps alive. An unedited file therefore stores **zero
//! bytes of text** beyond the input itself.
//!
//! Edits write an owned override into a side table and point the token at it,
//! which is `O(1)` and touches nothing else.
//!
//! What it gives up is real. Indices are untyped `u32`s the compiler will not
//! check, so a stale index from a previous tree is a silent wrong answer rather
//! than a borrow error — precisely the class of bug MASTER_PLAN §0 puts first.
//! And it is not persistent: base, ours and theirs each need their own arena.

use crate::lex::{Kind, Lexed, Tok};

pub const NONE: u32 = u32::MAX;

#[derive(Clone, Copy)]
pub struct Elem {
    pub kind: Kind,
    pub is_token: bool,
    /// Span into the source (tokens), or the covering span (nodes).
    pub start: u32,
    pub len: u32,
    pub first_child: u32,
    pub next_sibling: u32,
    /// Index into `overrides`, or [`NONE`] when the token still reads from src.
    pub over: u32,
}

pub struct Tree {
    pub elems: Vec<Elem>,
    pub overrides: Vec<Vec<u8>>,
    pub root: u32,
    /// The source buffer. The tree borrows nothing and copies nothing: it holds
    /// exactly one copy of the input and indexes into it.
    pub src: Vec<u8>,
}

pub fn build(src: &[u8], lexed: &Lexed, parents: &[u32]) -> Tree {
    let mut kids: Vec<Vec<u32>> = vec![Vec::new(); lexed.lines.len()];
    let mut roots: Vec<u32> = Vec::new();
    for (idx, &parent) in parents.iter().enumerate() {
        if parent == u32::MAX {
            roots.push(idx as u32);
        } else {
            kids[parent as usize].push(idx as u32);
        }
    }

    // One element per line node, plus one per token, plus the document.
    let mut elems: Vec<Elem> = Vec::with_capacity(lexed.toks.len() + lexed.lines.len() + 1);
    let mut line_elem: Vec<u32> = vec![NONE; lexed.lines.len()];

    let mut order: Vec<u32> = Vec::new();
    let mut stack: Vec<u32> = roots.iter().rev().copied().collect();
    while let Some(line) = stack.pop() {
        order.push(line);
        for &kid in kids[line as usize].iter().rev() {
            stack.push(kid);
        }
    }

    for &line_idx in order.iter().rev() {
        let line = lexed.lines[line_idx as usize];
        let mut children: Vec<u32> = Vec::new();
        let mut start = u32::MAX;
        let mut len = 0u32;

        for tok in &lexed.toks[line.first as usize..line.end as usize] {
            let Tok { kind, start: s, len: l } = *tok;
            start = start.min(s);
            len += l;
            elems.push(Elem {
                kind,
                is_token: true,
                start: s,
                len: l,
                first_child: NONE,
                next_sibling: NONE,
                over: NONE,
            });
            children.push(elems.len() as u32 - 1);
        }
        for &kid in &kids[line_idx as usize] {
            let id = line_elem[kid as usize];
            let elem = elems[id as usize];
            start = start.min(elem.start);
            len += elem.len;
            children.push(id);
        }

        // Thread the sibling list.
        for pair in children.windows(2) {
            elems[pair[0] as usize].next_sibling = pair[1];
        }
        elems.push(Elem {
            kind: Kind::Line,
            is_token: false,
            start: if start == u32::MAX { 0 } else { start },
            len,
            first_child: children.first().copied().unwrap_or(NONE),
            next_sibling: NONE,
            over: NONE,
        });
        line_elem[line_idx as usize] = elems.len() as u32 - 1;
    }

    let doc_children: Vec<u32> = roots.iter().map(|&r| line_elem[r as usize]).collect();
    for pair in doc_children.windows(2) {
        elems[pair[0] as usize].next_sibling = pair[1];
    }
    let total: u32 = doc_children.iter().map(|&c| elems[c as usize].len).sum();
    elems.push(Elem {
        kind: Kind::Document,
        is_token: false,
        start: 0,
        len: total,
        first_child: doc_children.first().copied().unwrap_or(NONE),
        next_sibling: NONE,
        over: NONE,
    });
    let root = elems.len() as u32 - 1;

    Tree {
        elems,
        overrides: Vec::new(),
        root,
        src: src.to_vec(),
    }
}

impl Tree {
    pub fn text_of(&self, id: u32) -> &[u8] {
        let elem = self.elems[id as usize];
        if elem.over == NONE {
            &self.src[elem.start as usize..(elem.start + elem.len) as usize]
        } else {
            &self.overrides[elem.over as usize]
        }
    }
}

pub fn serialize(tree: &Tree) -> Vec<u8> {
    let mut out = Vec::with_capacity(tree.src.len());
    let mut stack: Vec<u32> = vec![tree.root];
    while let Some(id) = stack.pop() {
        let elem = tree.elems[id as usize];
        if elem.is_token {
            out.extend_from_slice(tree.text_of(id));
            continue;
        }
        // Push children in reverse so they pop in document order.
        let mut children = Vec::new();
        let mut child = elem.first_child;
        while child != NONE {
            children.push(child);
            child = tree.elems[child as usize].next_sibling;
        }
        for child in children.into_iter().rev() {
            stack.push(child);
        }
    }
    out
}

/// Id of the `n`-th token in document order.
pub fn nth_token(tree: &Tree, n: usize) -> Option<u32> {
    let mut seen = 0usize;
    let mut stack: Vec<u32> = vec![tree.root];
    while let Some(id) = stack.pop() {
        let elem = tree.elems[id as usize];
        if elem.is_token {
            if seen == n {
                return Some(id);
            }
            seen += 1;
            continue;
        }
        let mut children = Vec::new();
        let mut child = elem.first_child;
        while child != NONE {
            children.push(child);
            child = tree.elems[child as usize].next_sibling;
        }
        for child in children.into_iter().rev() {
            stack.push(child);
        }
    }
    None
}

/// Replace a token's text. `O(1)`: one push into the override table, one field
/// write. Nothing else in the tree moves.
///
/// Note the silent cost: ancestor `len` fields are now stale. Keeping them
/// correct means walking to the root, which is the same `O(depth)` the green
/// tree pays — so the honest `O(1)` claim only holds for a representation that
/// does not cache covering spans.
pub fn replace_token(tree: &mut Tree, id: u32, text: &[u8]) {
    tree.overrides.push(text.to_vec());
    tree.elems[id as usize].over = tree.overrides.len() as u32 - 1;
    tree.elems[id as usize].len = text.len() as u32;
}

/// Absolute source range of a token. One array read, no allocation, no walk —
/// the span is what the element *is*.
pub fn locate(tree: &Tree, id: u32) -> (u32, u32) {
    let elem = tree.elems[id as usize];
    (elem.start, elem.start + elem.len)
}

pub fn count_nodes(tree: &Tree) -> (usize, usize) {
    let nodes = tree.elems.iter().filter(|e| !e.is_token).count();
    let tokens = tree.elems.iter().filter(|e| e.is_token).count();
    (nodes, tokens)
}
