//! One lossless tokenizer, shared by all three candidate representations.
//!
//! The spike compares *tree representations*, so the tokenizer must be a
//! constant. It is deliberately not a YAML parser — it is an
//! indentation-structured lexer that is **total and lossless**: every byte of
//! the input lands in exactly one token, so `concat(tokens) == input` before any
//! tree is built. That makes a K1 failure downstream unambiguously the tree's
//! fault rather than the lexer's.
//!
//! Lines nest by indentation, which is what gives the tree realistic depth on
//! real Helm and Kubernetes files — the property the memory comparison depends
//! on. A flat token list would make all three representations look identical and
//! prove nothing.

/// Token and node kinds. One enum for both, as all three representations do.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub enum Kind {
    // Tokens
    Indent,
    Word,
    Str,
    Punct,
    Space,
    Comment,
    Newline,
    Other,
    // Nodes
    Document,
    Line,
}

/// A token: a half-open byte range of the source and its kind.
#[derive(Clone, Copy, Debug)]
pub struct Tok {
    pub kind: Kind,
    pub start: u32,
    pub len: u32,
}

/// A source line: its indent width and the half-open token range it owns.
#[derive(Clone, Copy, Debug)]
pub struct LineSpan {
    /// Indent columns, or `u32::MAX` for a blank line (structurally neutral:
    /// a blank line must not re-parent everything that follows it).
    pub indent: u32,
    pub first: u32,
    pub end: u32,
}

pub struct Lexed {
    pub toks: Vec<Tok>,
    pub lines: Vec<LineSpan>,
}

impl Lexed {
    /// Total bytes covered by the token stream. Must equal `src.len()`.
    pub fn covered(&self) -> usize {
        self.toks.iter().map(|t| t.len as usize).sum()
    }
}

fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-' | b'/' | b'+' | b'@' | b'%')
}

fn is_punct(b: u8) -> bool {
    matches!(
        b,
        b':' | b',' | b'{' | b'}' | b'[' | b']' | b'&' | b'*' | b'|' | b'>' | b'!' | b'?' | b'='
    )
}

/// Tokenize `src`. Total: every byte is covered exactly once.
pub fn lex(src: &[u8]) -> Lexed {
    let mut toks: Vec<Tok> = Vec::new();
    let mut lines: Vec<LineSpan> = Vec::new();
    let mut i = 0usize;

    while i < src.len() {
        let line_tok_start = toks.len() as u32;

        // Leading whitespace becomes the Indent token and sets nesting depth.
        let ws_start = i;
        while i < src.len() && (src[i] == b' ' || src[i] == b'\t') {
            i += 1;
        }
        let indent_cols = (i - ws_start) as u32;
        if i > ws_start {
            toks.push(Tok {
                kind: Kind::Indent,
                start: ws_start as u32,
                len: (i - ws_start) as u32,
            });
        }

        // A line that ends right after its indent is blank: structurally neutral.
        let blank = i >= src.len() || src[i] == b'\n' || src[i] == b'\r';

        // Body of the line, up to but not including the terminator.
        while i < src.len() && src[i] != b'\n' && src[i] != b'\r' {
            let start = i;
            let b = src[i];
            let kind = if b == b'#' {
                while i < src.len() && src[i] != b'\n' && src[i] != b'\r' {
                    i += 1;
                }
                Kind::Comment
            } else if b == b'"' || b == b'\'' {
                let quote = b;
                i += 1;
                while i < src.len() && src[i] != quote && src[i] != b'\n' && src[i] != b'\r' {
                    // Backslash escapes only matter inside double quotes.
                    if quote == b'"' && src[i] == b'\\' && i + 1 < src.len() {
                        i += 1;
                    }
                    i += 1;
                }
                if i < src.len() && src[i] == quote {
                    i += 1;
                }
                Kind::Str
            } else if b == b' ' || b == b'\t' {
                while i < src.len() && (src[i] == b' ' || src[i] == b'\t') {
                    i += 1;
                }
                Kind::Space
            } else if is_word(b) {
                while i < src.len() && is_word(src[i]) {
                    i += 1;
                }
                Kind::Word
            } else if is_punct(b) {
                i += 1;
                Kind::Punct
            } else {
                i += 1;
                Kind::Other
            };
            toks.push(Tok {
                kind,
                start: start as u32,
                len: (i - start) as u32,
            });
        }

        // Line terminator: CRLF or LF, kept verbatim. Normalising it here would
        // be a K1 violation before the tree was even built.
        if i < src.len() {
            let start = i;
            if src[i] == b'\r' {
                i += 1;
            }
            if i < src.len() && src[i] == b'\n' {
                i += 1;
            }
            toks.push(Tok {
                kind: Kind::Newline,
                start: start as u32,
                len: (i - start) as u32,
            });
        }

        lines.push(LineSpan {
            indent: if blank { u32::MAX } else { indent_cols },
            first: line_tok_start,
            end: toks.len() as u32,
        });
    }

    Lexed { toks, lines }
}

/// The nesting plan: for each line, its parent line index (`u32::MAX` = the
/// document root). Computed once and shared, so all three representations build
/// structurally identical trees and the comparison is about representation only.
pub fn nesting(lines: &[LineSpan]) -> Vec<u32> {
    let mut parents = vec![u32::MAX; lines.len()];
    // Stack of (indent, line index) of currently open ancestors.
    let mut stack: Vec<(u32, u32)> = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        if line.indent == u32::MAX {
            // Blank line: child of whatever is currently open, no re-parenting.
            parents[idx] = stack.last().map_or(u32::MAX, |&(_, i)| i);
            continue;
        }
        while let Some(&(indent, _)) = stack.last() {
            if indent >= line.indent {
                stack.pop();
            } else {
                break;
            }
        }
        parents[idx] = stack.last().map_or(u32::MAX, |&(_, i)| i);
        stack.push((line.indent, idx as u32));
    }
    parents
}
