//! **Candidate B — owned token tree.**
//!
//! The obvious representation: every node owns a `Vec` of children, every token
//! owns its bytes. No refcounting, no interning, no arena, no index
//! indirection. Navigation is a plain `&` walk and an edit is a plain `&mut`
//! walk — the code reads exactly like the data structure.
//!
//! The costs it pays are equally plain: one heap allocation per token's text
//! plus one per node's child vector, no sharing between identical tokens, and
//! no persistence — an edit mutates in place, so any handle taken before the
//! edit is invalidated by the borrow checker (which is at least honest about
//! it), and keeping the pre-edit tree means cloning the whole thing.

use crate::lex::{Kind, Lexed, Tok};

pub struct Token {
    pub kind: Kind,
    pub text: Vec<u8>,
}

pub enum Child {
    Node(Node),
    Token(Token),
}

pub struct Node {
    pub kind: Kind,
    pub children: Vec<Child>,
}

pub struct Tree {
    pub root: Node,
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

    let mut memo: Vec<Option<Node>> = (0..lexed.lines.len()).map(|_| None).collect();
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
        let mut children: Vec<Child> = Vec::new();
        for tok in &lexed.toks[line.first as usize..line.end as usize] {
            let Tok { kind, start, len } = *tok;
            children.push(Child::Token(Token {
                kind,
                text: src[start as usize..(start + len) as usize].to_vec(),
            }));
        }
        for &kid in &kids[line_idx as usize] {
            children.push(Child::Node(memo[kid as usize].take().expect("child built")));
        }
        memo[line_idx as usize] = Some(Node {
            kind: Kind::Line,
            children,
        });
    }

    let mut doc_children = Vec::new();
    for &root in &roots {
        doc_children.push(Child::Node(memo[root as usize].take().expect("root built")));
    }

    Tree {
        root: Node {
            kind: Kind::Document,
            children: doc_children,
        },
    }
}

pub fn serialize(tree: &Tree) -> Vec<u8> {
    let mut out = Vec::new();
    let mut stack: Vec<&Child> = tree.root.children.iter().rev().collect();
    while let Some(child) = stack.pop() {
        match child {
            Child::Token(tok) => out.extend_from_slice(&tok.text),
            Child::Node(node) => {
                for grandchild in node.children.iter().rev() {
                    stack.push(grandchild);
                }
            }
        }
    }
    out
}

pub fn nth_token_path(tree: &Tree, n: usize) -> Option<Vec<usize>> {
    fn walk(node: &Node, n: &mut usize, path: &mut Vec<usize>) -> bool {
        for (idx, child) in node.children.iter().enumerate() {
            match child {
                Child::Token(_) => {
                    if *n == 0 {
                        path.push(idx);
                        return true;
                    }
                    *n -= 1;
                }
                Child::Node(inner) => {
                    path.push(idx);
                    if walk(inner, n, path) {
                        return true;
                    }
                    path.pop();
                }
            }
        }
        false
    }
    let mut n = n;
    let mut path = Vec::new();
    walk(&tree.root, &mut n, &mut path).then_some(path)
}

/// Replace the token at `path`, in place.
///
/// Short, and that is the point — but note what it cannot do: there is no way
/// to hold the pre-edit tree without cloning it wholesale, so undo, "compare
/// before and after", and konflux's three-way merge (which needs base, ours and
/// theirs live at once) all pay full price.
pub fn replace_token(tree: &mut Tree, path: &[usize], text: &[u8]) {
    let mut node = &mut tree.root;
    let Some((&last, prefix)) = path.split_last() else {
        return;
    };
    for &idx in prefix {
        match &mut node.children[idx] {
            Child::Node(inner) => node = inner,
            Child::Token(_) => return,
        }
    }
    if let Child::Token(tok) = &mut node.children[last] {
        tok.text.clear();
        tok.text.extend_from_slice(text);
    }
}

/// Absolute source range of the token at `path`.
///
/// The owned tree stores neither offsets nor lengths, so this has to sum the
/// text of everything that precedes the target — an `O(preceding tokens)` walk
/// for a question that gets asked once per conflict. Caching lengths on nodes
/// would fix it and would also mean invalidating those caches on every edit,
/// which is the trade the green tree already made deliberately.
pub fn locate(tree: &Tree, path: &[usize]) -> (u32, u32) {
    fn text_len(child: &Child) -> u32 {
        match child {
            Child::Token(tok) => tok.text.len() as u32,
            Child::Node(node) => node.children.iter().map(text_len).sum(),
        }
    }
    let mut node = &tree.root;
    let mut at = 0u32;
    let Some((&last, prefix)) = path.split_last() else {
        return (0, tree.root.children.iter().map(text_len).sum());
    };
    for &idx in prefix {
        for child in node.children.iter().take(idx) {
            at += text_len(child);
        }
        match &node.children[idx] {
            Child::Node(inner) => node = inner,
            Child::Token(_) => return (at, at),
        }
    }
    for child in node.children.iter().take(last) {
        at += text_len(child);
    }
    let len = text_len(&node.children[last]);
    (at, at + len)
}

pub fn count_nodes(tree: &Tree) -> (usize, usize) {
    let mut nodes = 0usize;
    let mut tokens = 0usize;
    let mut stack: Vec<&Node> = vec![&tree.root];
    while let Some(node) = stack.pop() {
        nodes += 1;
        for child in &node.children {
            match child {
                Child::Node(inner) => stack.push(inner),
                Child::Token(_) => tokens += 1,
            }
        }
    }
    (nodes, tokens)
}
