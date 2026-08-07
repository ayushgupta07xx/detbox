//! **Candidate A — green/red tree (rowan-style).**
//!
//! An immutable, refcounted *green* tree holding kind, text length and children,
//! with no parent pointers and no absolute offsets. Green tokens own their text
//! and are interned, so the thousand `name`/`:`/`\n` tokens in a Helm chart
//! become one allocation each. A *red* layer is materialised on demand and adds
//! the parent chain and the absolute offset that the green tree deliberately
//! omits.
//!
//! Edits are persistent: replacing a token rebuilds only the spine from that
//! token to the root — `O(depth)` new nodes — and shares every untouched
//! subtree. Handles taken before the edit stay valid and keep pointing at the
//! old tree.
//!
//! This is what rust-analyzer uses, and the reason it can hold a whole crate
//! graph and still undo cheaply.

use std::collections::HashMap;
use std::rc::Rc;

use crate::lex::{Kind, Lexed, Tok};

pub struct GreenToken {
    pub kind: Kind,
    pub text: Box<[u8]>,
}

pub enum GreenChild {
    Node(Rc<GreenNode>),
    Token(Rc<GreenToken>),
}

pub struct GreenNode {
    pub kind: Kind,
    pub text_len: u32,
    pub children: Vec<GreenChild>,
}

/// The red layer: parent + absolute offset, computed on demand.
///
/// Note what it costs. Every navigation step allocates an `Rc<Red>`, because
/// the green tree does not know where it lives. That is the ergonomic price of
/// structural sharing, and it is the thing this spike exists to weigh.
pub struct Red {
    pub green: Rc<GreenNode>,
    pub parent: Option<Rc<Red>>,
    pub offset: u32,
}

impl Red {
    pub fn root(green: Rc<GreenNode>) -> Rc<Self> {
        Rc::new(Self {
            green,
            parent: None,
            offset: 0,
        })
    }

    /// Absolute range of this node in the source.
    pub fn range(&self) -> (u32, u32) {
        (self.offset, self.offset + self.green.text_len)
    }

    /// Child nodes, each carrying its computed absolute offset.
    pub fn children(self: &Rc<Self>) -> Vec<Rc<Self>> {
        let mut out = Vec::new();
        let mut at = self.offset;
        for child in &self.green.children {
            match child {
                GreenChild::Node(node) => {
                    out.push(Rc::new(Self {
                        green: Rc::clone(node),
                        parent: Some(Rc::clone(self)),
                        offset: at,
                    }));
                    at += node.text_len;
                }
                GreenChild::Token(tok) => at += tok.text.len() as u32,
            }
        }
        out
    }
}

#[derive(Default)]
struct Interner {
    map: HashMap<(Kind, Box<[u8]>), Rc<GreenToken>>,
    hits: usize,
    misses: usize,
}

impl Interner {
    fn token(&mut self, kind: Kind, text: &[u8]) -> Rc<GreenToken> {
        // Long tokens are not worth interning: they are rarely repeated and the
        // key itself would cost as much as the value.
        if text.len() > 64 {
            self.misses += 1;
            return Rc::new(GreenToken {
                kind,
                text: text.into(),
            });
        }
        let key = (kind, Box::<[u8]>::from(text));
        if let Some(found) = self.map.get(&key) {
            self.hits += 1;
            return Rc::clone(found);
        }
        self.misses += 1;
        let tok = Rc::new(GreenToken {
            kind,
            text: text.into(),
        });
        self.map.insert(key, Rc::clone(&tok));
        tok
    }
}

pub struct Tree {
    pub root: Rc<GreenNode>,
    pub intern_hits: usize,
    pub intern_misses: usize,
}

pub fn build(src: &[u8], lexed: &Lexed, parents: &[u32]) -> Tree {
    let mut interner = Interner::default();

    // Children of each line, and of the document root.
    let mut kids: Vec<Vec<u32>> = vec![Vec::new(); lexed.lines.len()];
    let mut roots: Vec<u32> = Vec::new();
    for (idx, &parent) in parents.iter().enumerate() {
        if parent == u32::MAX {
            roots.push(idx as u32);
        } else {
            kids[parent as usize].push(idx as u32);
        }
    }

    let mut memo: Vec<Option<Rc<GreenNode>>> = vec![None; lexed.lines.len()];
    // Post-order without recursion: a 266 KiB YAML file can nest deeply enough
    // to matter, and blowing the stack in a measurement harness would be a
    // silly way to lose a day.
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
        let mut children: Vec<GreenChild> = Vec::new();
        let mut text_len = 0u32;

        for tok in &lexed.toks[line.first as usize..line.end as usize] {
            let Tok { kind, start, len } = *tok;
            let text = &src[start as usize..(start + len) as usize];
            let green = interner.token(kind, text);
            text_len += len;
            children.push(GreenChild::Token(green));
        }
        for &kid in &kids[line_idx as usize] {
            let node = memo[kid as usize].take().expect("child built first");
            text_len += node.text_len;
            children.push(GreenChild::Node(node));
        }

        memo[line_idx as usize] = Some(Rc::new(GreenNode {
            kind: Kind::Line,
            text_len,
            children,
        }));
    }

    let mut doc_children = Vec::new();
    let mut doc_len = 0u32;
    for &root in &roots {
        let node = memo[root as usize].take().expect("root built");
        doc_len += node.text_len;
        doc_children.push(GreenChild::Node(node));
    }

    Tree {
        root: Rc::new(GreenNode {
            kind: Kind::Document,
            text_len: doc_len,
            children: doc_children,
        }),
        intern_hits: interner.hits,
        intern_misses: interner.misses,
    }
}

pub fn serialize(tree: &Tree) -> Vec<u8> {
    let mut out = Vec::with_capacity(tree.root.text_len as usize);
    let mut stack: Vec<&GreenChild> = Vec::new();
    // Emit the document's children in order.
    for child in tree.root.children.iter().rev() {
        stack.push(child);
    }
    while let Some(child) = stack.pop() {
        match child {
            GreenChild::Token(tok) => out.extend_from_slice(&tok.text),
            GreenChild::Node(node) => {
                for grandchild in node.children.iter().rev() {
                    stack.push(grandchild);
                }
            }
        }
    }
    out
}

/// Path from the root to a token: child indices at each level.
pub type Path = Vec<usize>;

/// Find the path to the `n`-th token in document order.
pub fn nth_token_path(tree: &Tree, n: usize) -> Option<Path> {
    fn walk(node: &GreenNode, n: &mut usize, path: &mut Path) -> bool {
        for (idx, child) in node.children.iter().enumerate() {
            match child {
                GreenChild::Token(_) => {
                    if *n == 0 {
                        path.push(idx);
                        return true;
                    }
                    *n -= 1;
                }
                GreenChild::Node(inner) => {
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
    let mut path = Path::new();
    walk(&tree.root, &mut n, &mut path).then_some(path)
}

/// Replace the token at `path` with `text`, returning a **new** root.
///
/// Persistent: only the spine is rebuilt, every other subtree is shared, and
/// the old root remains valid. This is the whole argument for green/red, and
/// the edit-allocation column of the ADR table measures exactly this.
pub fn replace_token(root: &Rc<GreenNode>, path: &[usize], text: &[u8]) -> Rc<GreenNode> {
    let Some((&idx, rest)) = path.split_first() else {
        return Rc::clone(root);
    };

    let mut children = Vec::with_capacity(root.children.len());
    let mut text_len = 0u32;
    for (i, child) in root.children.iter().enumerate() {
        if i != idx {
            match child {
                GreenChild::Node(node) => {
                    text_len += node.text_len;
                    children.push(GreenChild::Node(Rc::clone(node)));
                }
                GreenChild::Token(tok) => {
                    text_len += tok.text.len() as u32;
                    children.push(GreenChild::Token(Rc::clone(tok)));
                }
            }
            continue;
        }
        match child {
            GreenChild::Token(tok) => {
                let replacement = Rc::new(GreenToken {
                    kind: tok.kind,
                    text: text.into(),
                });
                text_len += replacement.text.len() as u32;
                children.push(GreenChild::Token(replacement));
            }
            GreenChild::Node(node) => {
                let rebuilt = replace_token(node, rest, text);
                text_len += rebuilt.text_len;
                children.push(GreenChild::Node(rebuilt));
            }
        }
    }

    Rc::new(GreenNode {
        kind: root.kind,
        text_len,
        children,
    })
}

/// Absolute source range of the token at `path`, via the red layer.
///
/// This is the green tree's ergonomic tax made concrete. The green tree stores
/// no offsets and no parents, so "where is this token in the file?" — a
/// question konflux asks for every span-anchored conflict — cannot be answered
/// by reading a field. A red node must be materialised for each step down.
///
/// The naive `Rc<Red>` per step below is an **upper bound**: real rowan
/// amortises this with an internal free-list of node data. The shape of the
/// cost is real; the constant is pessimistic, and ADR-001 says so.
pub fn locate(root: &Rc<GreenNode>, path: &[usize]) -> (u32, u32) {
    let Some((&last, prefix)) = path.split_last() else {
        return (0, root.text_len);
    };
    let mut red = Red::root(Rc::clone(root));
    for &idx in prefix {
        let mut at = red.offset;
        let mut next = None;
        for (i, child) in red.green.children.iter().enumerate() {
            match child {
                GreenChild::Node(node) => {
                    if i == idx {
                        next = Some(Rc::new(Red {
                            green: Rc::clone(node),
                            parent: Some(Rc::clone(&red)),
                            offset: at,
                        }));
                        break;
                    }
                    at += node.text_len;
                }
                GreenChild::Token(tok) => at += tok.text.len() as u32,
            }
        }
        match next {
            Some(child) => red = child,
            None => return (red.offset, red.offset),
        }
    }
    let mut at = red.offset;
    for (i, child) in red.green.children.iter().enumerate() {
        let len = match child {
            GreenChild::Node(node) => node.text_len,
            GreenChild::Token(tok) => tok.text.len() as u32,
        };
        if i == last {
            return (at, at + len);
        }
        at += len;
    }
    (at, at)
}

pub fn count_nodes(tree: &Tree) -> (usize, usize) {
    let mut nodes = 0usize;
    let mut tokens = 0usize;
    let mut stack: Vec<&GreenNode> = vec![&tree.root];
    while let Some(node) = stack.pop() {
        nodes += 1;
        for child in &node.children {
            match child {
                GreenChild::Node(inner) => stack.push(inner),
                GreenChild::Token(_) => tokens += 1,
            }
        }
    }
    (nodes, tokens)
}
