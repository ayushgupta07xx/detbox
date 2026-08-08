//! YAML, lossless.
//!
//! # What this is, and what it is not
//!
//! This is a **lossless structural** parser, not a YAML semantic parser. It
//! covers every byte, nests lines by indentation, and models the constructs the
//! corpus actually contains. It does not resolve anchors, expand merge keys,
//! interpret tags, or type scalars. Those live in `semantic_view`, which §3.2
//! places at M2 and which reads *this* tree rather than the input bytes.
//!
//! That split is the point. K1 is a claim about bytes, and the way to keep it is
//! to make byte coverage the only thing the parser must get right:
//!
//! - [`lex`] is **total** — every input byte lands in exactly one token.
//! - [`build`] puts every token in the tree, in order.
//! - `Cst::serialize` is an in-order walk with no format knowledge.
//!
//! So K1 follows by construction, and a K1 failure could only be a lexer
//! coverage bug. Getting the *structure* subtly wrong costs us at M2; getting
//! the *bytes* wrong would cost us the platform.
//!
//! # The escape hatch is the main road
//!
//! MASTER_PLAN §3.1 describes [`SyntaxKind::VERBATIM`] as a fallback for "exotic
//! YAML tags, weird encodings". The corpus disagrees about how rare that is: a
//! survey of 750 real files (`cargo xtask corpus-survey`) found Helm's Go
//! templating in **41.2%** of them. `{{ .Values.image | quote }}` is not YAML;
//! it is not even tokenizable as YAML, since it can contain unbalanced quotes
//! and colons that mean nothing. Two files in five are only round-trippable
//! because of the verbatim path, so it is exercised constantly rather than
//! exceptionally.
//!
//! # Accept and reject
//!
//! Per ADR-008, this rejects spec-invalid *structure* and accepts
//! tokenizable-but-unmodelled content as verbatim. The set of structural rules
//! enforced today is small and deliberately honest about it — see
//! [`validate`] and `conformance/thresholds.tsv`, which records the measured
//! reject-rate rather than an aspirational one.

use core_cst::{Cst, GreenChild, GreenNode, GreenToken, Span, SyntaxKind};
use std::rc::Rc;

use crate::{Diagnostic, ParseReport};

/// Syntax kinds for YAML.
pub mod kind {
    use core_cst::SyntaxKind;

    /// The whole stream, all documents.
    pub const STREAM: SyntaxKind = SyntaxKind(0);
    /// One document, from `---` (or the start) to the next separator.
    pub const DOCUMENT: SyntaxKind = SyntaxKind(1);
    /// One source line and everything nested under it.
    pub const LINE: SyntaxKind = SyntaxKind(2);

    /// Leading whitespace. Structural: it decides nesting.
    pub const INDENT: SyntaxKind = SyntaxKind(16);
    /// Spaces or tabs between tokens.
    pub const SPACE: SyntaxKind = SyntaxKind(17);
    /// `\n` or `\r\n`, kept exactly as written.
    pub const NEWLINE: SyntaxKind = SyntaxKind(18);
    /// `#` to end of line.
    pub const COMMENT: SyntaxKind = SyntaxKind(19);
    /// `%YAML` / `%TAG` directive line.
    pub const DIRECTIVE: SyntaxKind = SyntaxKind(20);
    /// `---`
    pub const DOC_START: SyntaxKind = SyntaxKind(21);
    /// `...`
    pub const DOC_END: SyntaxKind = SyntaxKind(22);
    /// A UTF-8 BOM.
    pub const BOM: SyntaxKind = SyntaxKind(23);
    /// `:`
    pub const COLON: SyntaxKind = SyntaxKind(24);
    /// `-` introducing a sequence entry.
    pub const DASH: SyntaxKind = SyntaxKind(25);
    /// `,` `[` `]` `{` `}` in flow context.
    pub const FLOW_PUNCT: SyntaxKind = SyntaxKind(26);
    /// `&anchor`
    pub const ANCHOR: SyntaxKind = SyntaxKind(27);
    /// `*alias`
    pub const ALIAS: SyntaxKind = SyntaxKind(28);
    /// `!tag` or `!!tag`
    pub const TAG: SyntaxKind = SyntaxKind(29);
    /// A single-quoted scalar, possibly spanning lines.
    pub const SINGLE_QUOTED: SyntaxKind = SyntaxKind(30);
    /// A double-quoted scalar, possibly spanning lines.
    pub const DOUBLE_QUOTED: SyntaxKind = SyntaxKind(31);
    /// `|` or `>` with its chomping and indentation indicators.
    pub const BLOCK_HEADER: SyntaxKind = SyntaxKind(32);
    /// The indented body of a block scalar, newlines included.
    pub const BLOCK_BODY: SyntaxKind = SyntaxKind(33);
    /// An unquoted scalar.
    pub const PLAIN: SyntaxKind = SyntaxKind(34);
    /// Bytes the lexer classified but the validator rejected.
    pub const ERROR: SyntaxKind = SyntaxKind(35);
}

/// One token: a kind and the byte range it covers.
#[derive(Clone, Copy, Debug)]
pub struct Token {
    /// What it is.
    pub kind: SyntaxKind,
    /// Where it is.
    pub span: Span,
}

/// A source line: indent width and the token range it owns.
#[derive(Clone, Copy, Debug)]
struct LineSpan {
    /// Indent columns, or `u32::MAX` for a blank or comment-only line, which is
    /// structurally neutral: it must not re-parent everything that follows it.
    indent: u32,
    first: u32,
    end: u32,
}

/// Output of [`lex`]: a total cover of the input.
#[derive(Debug, Default)]
pub struct Lexed {
    /// Every byte, in order, in exactly one token.
    pub tokens: Vec<Token>,
    lines: Vec<LineSpan>,
    /// Lexical problems. Non-empty means reject.
    pub diagnostics: Vec<Diagnostic>,
}

impl Lexed {
    /// Bytes covered by the token stream. Must equal the input length.
    #[must_use]
    pub fn covered(&self) -> usize {
        self.tokens
            .iter()
            .map(|t| (t.span.end - t.span.start) as usize)
            .sum()
    }
}

struct Cursor<'a> {
    src: &'a [u8],
    pos: usize,
}

impl Cursor<'_> {
    fn at(&self, offset: usize) -> Option<u8> {
        self.src.get(self.pos + offset).copied()
    }

    fn peek(&self) -> Option<u8> {
        self.at(0)
    }

    fn done(&self) -> bool {
        self.pos >= self.src.len()
    }

    fn at_line_end(&self) -> bool {
        matches!(self.peek(), None | Some(b'\n' | b'\r'))
    }

    fn starts_with(&self, needle: &[u8]) -> bool {
        self.src
            .get(self.pos..self.pos + needle.len())
            .is_some_and(|s| s == needle)
    }
}

fn span(start: usize, end: usize) -> Span {
    Span {
        start: u32::try_from(start).unwrap_or(u32::MAX),
        end: u32::try_from(end).unwrap_or(u32::MAX),
    }
}

/// Tokenize `input`. Total: never panics, and the tokens cover every byte.
#[must_use]
#[allow(clippy::too_many_lines)] // One line-oriented state machine; splitting it
// across helpers would hide the ordering that makes coverage total.
pub fn lex(input: &[u8]) -> Lexed {
    let mut out = Lexed::default();
    let mut cur = Cursor { src: input, pos: 0 };

    if input.starts_with(&[0xef, 0xbb, 0xbf]) {
        out.tokens.push(Token {
            kind: kind::BOM,
            span: span(0, 3),
        });
        cur.pos = 3;
    }

    while !cur.done() {
        let line_first = u32::try_from(out.tokens.len()).unwrap_or(u32::MAX);

        // --- indentation ----------------------------------------------------
        // Tabs in the leading whitespace are NOT rejected here, and the reason
        // is worth recording. YAML forbids tabs as block *indentation* but
        // permits them in blank lines, as separation after indentation, and
        // before flow indicators. A rule that fired on any tab in the leading
        // run rejected 12 documents yaml-test-suite calls valid — `\t{}`,
        // `foo:\n \tbar`, a blank line containing only a tab. Distinguishing
        // the cases needs the block/flow context tracking that arrives with
        // `semantic_view` at M2. Rejecting valid input is the worse error: a
        // refused file is one konflux cannot help with at all.
        let ws_start = cur.pos;
        while matches!(cur.peek(), Some(b' ' | b'\t')) {
            cur.pos += 1;
        }
        let indent_cols = u32::try_from(cur.pos - ws_start).unwrap_or(u32::MAX);
        if cur.pos > ws_start {
            out.tokens.push(Token {
                kind: kind::INDENT,
                span: span(ws_start, cur.pos),
            });
        }

        // A line that ends right after its indent is structurally neutral, as is
        // a comment-only line: neither should re-parent what follows.
        let neutral = cur.at_line_end() || cur.peek() == Some(b'#');

        // --- line body ------------------------------------------------------
        lex_line_body(input, &mut cur, &mut out, ws_start);

        // --- terminator -----------------------------------------------------
        if !cur.done() {
            let start = cur.pos;
            if cur.peek() == Some(b'\r') {
                cur.pos += 1;
            }
            if cur.peek() == Some(b'\n') {
                cur.pos += 1;
            }
            if cur.pos == start {
                // Neither CR nor LF: the body loop failed to consume. Take one
                // byte so the outer loop always advances. A lexer that can stall
                // is an infinite loop waiting for a fuzz input.
                cur.pos += 1;
                out.tokens.push(Token {
                    kind: kind::PLAIN,
                    span: span(start, cur.pos),
                });
            } else {
                out.tokens.push(Token {
                    kind: kind::NEWLINE,
                    span: span(start, cur.pos),
                });
            }
        }

        out.lines.push(LineSpan {
            indent: if neutral { u32::MAX } else { indent_cols },
            first: line_first,
            end: u32::try_from(out.tokens.len()).unwrap_or(u32::MAX),
        });
    }

    out
}

/// Everything on a line after its indentation, up to the line terminator.
// One dispatch table over YAML's indicator characters. Splitting it into helpers
// would scatter the ordering that makes byte coverage total — the property K1
// rests on — across several functions where it could not be read at a glance.
#[allow(clippy::too_many_lines)]
fn lex_line_body(input: &[u8], cur: &mut Cursor<'_>, out: &mut Lexed, line_start: usize) {
    // Directives own the whole line, and only at column zero.
    if cur.pos == line_start
        && cur.peek() == Some(b'%')
        && line_start_is_column_zero(input, cur.pos)
    {
        let start = cur.pos;
        while !cur.at_line_end() {
            cur.pos += 1;
        }
        out.tokens.push(Token {
            kind: kind::DIRECTIVE,
            span: span(start, cur.pos),
        });
        return;
    }

    while !cur.at_line_end() {
        let start = cur.pos;
        let Some(byte) = cur.peek() else { break };

        // Go templating: not YAML, and not tokenizable as YAML. Kept verbatim,
        // which is the only reason 41.2% of the corpus round-trips at all.
        if cur.starts_with(b"{{") {
            lex_verbatim_template(cur);
            out.tokens.push(Token {
                kind: SyntaxKind::VERBATIM,
                span: span(start, cur.pos),
            });
            continue;
        }

        let kind = match byte {
            b' ' | b'\t' => {
                while matches!(cur.peek(), Some(b' ' | b'\t')) {
                    cur.pos += 1;
                }
                kind::SPACE
            }
            // `#` starts a comment only at the start of a token, which after the
            // whitespace arm above is exactly where we are.
            b'#' => {
                while !cur.at_line_end() {
                    cur.pos += 1;
                }
                kind::COMMENT
            }
            b'-' if cur.pos == line_start_after_indent(input, cur.pos)
                && cur.starts_with(b"---") =>
            {
                cur.pos += 3;
                kind::DOC_START
            }
            b'.' if cur.starts_with(b"...") => {
                cur.pos += 3;
                kind::DOC_END
            }
            b'-' if matches!(cur.at(1), None | Some(b' ' | b'\t' | b'\n' | b'\r')) => {
                cur.pos += 1;
                kind::DASH
            }
            b':' => {
                cur.pos += 1;
                kind::COLON
            }
            b',' | b'[' | b']' | b'{' | b'}' => {
                cur.pos += 1;
                kind::FLOW_PUNCT
            }
            b'&' | b'*' if is_name_byte(cur.at(1)) => {
                let sigil = byte;
                cur.pos += 1;
                while is_name_byte(cur.peek()) {
                    cur.pos += 1;
                }
                if sigil == b'&' {
                    kind::ANCHOR
                } else {
                    kind::ALIAS
                }
            }
            b'!' => {
                cur.pos += 1;
                if cur.peek() == Some(b'<') {
                    while !cur.at_line_end() && cur.peek() != Some(b'>') {
                        cur.pos += 1;
                    }
                    if cur.peek() == Some(b'>') {
                        cur.pos += 1;
                    }
                } else {
                    while matches!(cur.peek(), Some(b) if !b.is_ascii_whitespace()) {
                        cur.pos += 1;
                    }
                }
                kind::TAG
            }
            b'\'' | b'"' if quote_can_open(input, cur.pos) => lex_quoted(cur, byte, out),
            b'|' | b'>' => {
                cur.pos += 1;
                while matches!(cur.peek(), Some(b'0'..=b'9' | b'+' | b'-')) {
                    cur.pos += 1;
                }
                // A block header owns the rest of the line only if nothing but
                // whitespace and a comment follows it.
                let header_end = cur.pos;
                let mut probe = cur.pos;
                while matches!(input.get(probe), Some(b' ' | b'\t')) {
                    probe += 1;
                }
                let is_header = matches!(input.get(probe), None | Some(b'\n' | b'\r' | b'#'));
                if is_header {
                    out.tokens.push(Token {
                        kind: kind::BLOCK_HEADER,
                        span: span(start, header_end),
                    });
                    lex_block_body(input, cur, out, line_start);
                    continue;
                }
                kind::PLAIN
            }
            _ => {
                // A plain scalar runs to a comment, a flow indicator, or the end
                // of the line. `: ` ends it; a bare `:` inside a URL does not.
                while !cur.at_line_end() {
                    if cur.starts_with(b"{{") {
                        break;
                    }
                    match cur.peek() {
                        Some(b':')
                            if matches!(cur.at(1), None | Some(b' ' | b'\t' | b'\n' | b'\r')) =>
                        {
                            break;
                        }
                        Some(b',' | b'[' | b']' | b'{' | b'}') => break,
                        Some(b'#') if matches!(previous(input, cur.pos), Some(b' ' | b'\t')) => {
                            break;
                        }
                        _ => cur.pos += 1,
                    }
                }
                if cur.pos == start {
                    // Never stall.
                    cur.pos += 1;
                }
                // Trailing spaces belong to the following SPACE token, not to
                // the scalar: `a: b  # c` must keep its alignment on serialise,
                // and it does either way, but the split is cleaner for M2.
                while cur.pos > start && matches!(previous(input, cur.pos), Some(b' ' | b'\t')) {
                    cur.pos -= 1;
                }
                kind::PLAIN
            }
        };
        out.tokens.push(Token {
            kind,
            span: span(start, cur.pos),
        });
    }
}

/// Whether a quote at `pos` opens a quoted scalar, or is just a character
/// inside a plain one.
///
/// In YAML a quote only introduces a scalar at a *node* position. Mid-token it
/// is an ordinary character — `couldn't` is one plain scalar, not an unterminated
/// quote, and neither is the closing `"` in
/// `rabbitmq_up{service="{{ template "x" . }}"} == 0`, where the opening quote
/// was already absorbed by the plain run before the template.
///
/// Getting this wrong is expensive rather than merely inaccurate, because YAML
/// permits quoted scalars to span lines: one misread quote ran 230 bytes into
/// the next block scalar and took its header with it. Golden case 045.
fn quote_can_open(input: &[u8], pos: usize) -> bool {
    matches!(
        previous(input, pos),
        None | Some(b' ' | b'\t' | b'\n' | b'\r' | b':' | b'-' | b',' | b'[' | b'{' | b'?' | b'!')
    )
}

fn previous(input: &[u8], pos: usize) -> Option<u8> {
    pos.checked_sub(1).and_then(|p| input.get(p).copied())
}

fn is_name_byte(byte: Option<u8>) -> bool {
    matches!(byte, Some(b) if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

fn line_start_is_column_zero(input: &[u8], pos: usize) -> bool {
    pos == 0 || matches!(previous(input, pos), Some(b'\n'))
}

/// Position of the first non-indent byte on this line.
fn line_start_after_indent(input: &[u8], pos: usize) -> usize {
    let mut start = pos;
    while start > 0 {
        match previous(input, start) {
            Some(b' ' | b'\t') => start -= 1,
            _ => break,
        }
    }
    if start == 0 || matches!(previous(input, start), Some(b'\n')) {
        pos
    } else {
        usize::MAX
    }
}

/// `{{ ... }}`, tracking nesting so `{{ if }}{{ end }}` and `{{ dict "a" 1 }}`
/// both survive. Unterminated templates run to end of line rather than eating
/// the document.
fn lex_verbatim_template(cur: &mut Cursor<'_>) {
    cur.pos += 2;
    let mut depth = 1usize;
    while !cur.done() {
        if cur.starts_with(b"}}") {
            cur.pos += 2;
            depth -= 1;
            if depth == 0 {
                return;
            }
            continue;
        }
        if cur.starts_with(b"{{") {
            cur.pos += 2;
            depth += 1;
            continue;
        }
        if matches!(cur.peek(), Some(b'\n' | b'\r')) {
            return;
        }
        cur.pos += 1;
    }
}

/// A quoted scalar. YAML permits these to span lines, so this does not stop at
/// a newline — but an unterminated quote is reported rather than swallowing the
/// rest of the document silently.
fn lex_quoted(cur: &mut Cursor<'_>, quote: u8, out: &mut Lexed) -> SyntaxKind {
    let start = cur.pos;
    cur.pos += 1;
    while let Some(byte) = cur.peek() {
        // A quote inside a Go template is not a YAML quote. Found the hard way:
        // `service="{{ template "chart.fullname" . }}"` closed the scalar at the
        // template's inner quote, and because YAML lets quoted scalars span
        // lines the mis-parse then ran 230 bytes into the next block scalar.
        // Golden case 045.
        if cur.starts_with(b"{{") {
            lex_verbatim_template(cur);
            continue;
        }
        if byte == quote {
            // In single quotes, `''` is an escaped quote.
            if quote == b'\'' && cur.at(1) == Some(b'\'') {
                cur.pos += 2;
                continue;
            }
            cur.pos += 1;
            return if quote == b'\'' {
                kind::SINGLE_QUOTED
            } else {
                kind::DOUBLE_QUOTED
            };
        }
        if quote == b'"' && byte == b'\\' && cur.at(1).is_some() {
            cur.pos += 2;
            continue;
        }
        cur.pos += 1;
    }
    out.diagnostics.push(Diagnostic::new(
        span(start, cur.pos),
        "unterminated quoted scalar: reached end of input before the closing quote",
    ));
    kind::ERROR
}

/// The body of a block scalar: every following line that is blank or indented
/// deeper than the line that introduced it.
fn lex_block_body(input: &[u8], cur: &mut Cursor<'_>, out: &mut Lexed, header_line_start: usize) {
    let header_indent = indent_width_at(input, header_line_start);
    let body_start = cur.pos;

    // Trailing spaces and a comment may follow the header on its own line.
    while !cur.at_line_end() {
        cur.pos += 1;
    }

    loop {
        let line_break = cur.pos;
        if cur.peek() == Some(b'\r') {
            cur.pos += 1;
        }
        if cur.peek() == Some(b'\n') {
            cur.pos += 1;
        }
        if cur.pos == line_break {
            break; // end of input
        }

        let content_start = cur.pos;
        let mut probe = cur.pos;
        while matches!(input.get(probe), Some(b' ' | b'\t')) {
            probe += 1;
        }
        let blank = matches!(input.get(probe), None | Some(b'\n' | b'\r'));
        let indent = u32::try_from(probe - content_start).unwrap_or(u32::MAX);

        if !blank && indent <= header_indent {
            cur.pos = line_break; // this line belongs to the outer structure
            break;
        }
        cur.pos = probe;
        while !cur.at_line_end() {
            cur.pos += 1;
        }
    }

    if cur.pos > body_start {
        out.tokens.push(Token {
            kind: kind::BLOCK_BODY,
            span: span(body_start, cur.pos),
        });
    }
}

fn indent_width_at(input: &[u8], line_start: usize) -> u32 {
    let mut probe = line_start;
    while matches!(input.get(probe), Some(b' ' | b'\t')) {
        probe += 1;
    }
    u32::try_from(probe - line_start).unwrap_or(u32::MAX)
}

/// Structural checks, over and above the lexical ones [`lex`] already reports.
///
/// The set enforced today is small, and `conformance/thresholds.tsv` records the
/// reject-rate this actually achieves rather than one we would like. Raising it
/// is incremental work with a ratchet behind it.
#[must_use]
pub fn validate(lexed: &Lexed) -> Vec<Diagnostic> {
    // Every rule enforced today is lexical and already reported by `lex`. This
    // function exists so the structural rules that arrive incrementally have a
    // home, and so `parse`'s shape does not change when they do. It returning
    // empty is a statement about how little we currently check, which is what
    // the recorded reject-rate says out loud.
    let _ = lexed;
    Vec::new()
}

/// Parent line index for each line (`u32::MAX` = the document root).
fn nesting(lines: &[LineSpan]) -> Vec<u32> {
    let mut parents = vec![u32::MAX; lines.len()];
    let mut stack: Vec<(u32, u32)> = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        let index = u32::try_from(idx).unwrap_or(u32::MAX);
        if line.indent == u32::MAX {
            if let Some(slot) = parents.get_mut(idx) {
                *slot = stack.last().map_or(u32::MAX, |&(_, i)| i);
            }
            continue;
        }
        while let Some(&(indent, _)) = stack.last() {
            if indent >= line.indent {
                stack.pop();
            } else {
                break;
            }
        }
        if let Some(slot) = parents.get_mut(idx) {
            *slot = stack.last().map_or(u32::MAX, |&(_, i)| i);
        }
        stack.push((line.indent, index));
    }
    parents
}

/// Build the green tree, in token order.
///
/// Iterative: the corpus nests 8+ levels and a fuzzer will find worse.
#[must_use]
pub fn build(input: &[u8], lexed: &Lexed) -> Cst {
    let parents = nesting(&lexed.lines);

    let mut kids: Vec<Vec<u32>> = vec![Vec::new(); lexed.lines.len()];
    let mut roots: Vec<u32> = Vec::new();
    for (idx, &parent) in parents.iter().enumerate() {
        let index = u32::try_from(idx).unwrap_or(u32::MAX);
        if parent == u32::MAX {
            roots.push(index);
        } else if let Some(bucket) = kids.get_mut(parent as usize) {
            bucket.push(index);
        }
    }

    // Pre-order gives source order, because a line's descendants are contiguous
    // and always follow it. Reversed, it is a valid post-order for building.
    let mut order: Vec<u32> = Vec::new();
    let mut stack: Vec<u32> = roots.iter().rev().copied().collect();
    while let Some(line) = stack.pop() {
        order.push(line);
        if let Some(bucket) = kids.get(line as usize) {
            for &kid in bucket.iter().rev() {
                stack.push(kid);
            }
        }
    }

    let mut built: Vec<Option<GreenChild>> = (0..lexed.lines.len()).map(|_| None).collect();
    for &line_idx in order.iter().rev() {
        let Some(line) = lexed.lines.get(line_idx as usize).copied() else {
            continue;
        };
        let mut children: Vec<GreenChild> = Vec::new();
        for token in lexed
            .tokens
            .get(line.first as usize..line.end as usize)
            .unwrap_or_default()
        {
            let bytes = input
                .get(token.span.start as usize..token.span.end as usize)
                .unwrap_or_default();
            children.push(GreenChild::Token(Rc::new(GreenToken::new(
                token.kind, bytes,
            ))));
        }
        if let Some(bucket) = kids.get(line_idx as usize) {
            for &kid in bucket {
                if let Some(slot) = built.get_mut(kid as usize)
                    && let Some(node) = slot.take()
                {
                    children.push(node);
                }
            }
        }
        if let Some(slot) = built.get_mut(line_idx as usize) {
            *slot = Some(GreenChild::Node(Rc::new(GreenNode::new(
                kind::LINE,
                children,
            ))));
        }
    }

    let mut stream: Vec<GreenChild> = Vec::new();
    for &root in &roots {
        if let Some(slot) = built.get_mut(root as usize)
            && let Some(node) = slot.take()
        {
            stream.push(node);
        }
    }

    // The BOM, if present, precedes every line.
    let mut children = Vec::new();
    if let Some(first) = lexed.tokens.first()
        && first.kind == kind::BOM
        && lexed.lines.first().is_none_or(|l| l.first > 0)
    {
        let bytes = input
            .get(first.span.start as usize..first.span.end as usize)
            .unwrap_or_default();
        children.push(GreenChild::Token(Rc::new(GreenToken::new(
            kind::BOM,
            bytes,
        ))));
    }
    children.extend(stream);

    Cst::new(Rc::new(GreenNode::new(kind::STREAM, children)))
}

/// Parse `input` into a lossless tree, or report why not.
///
/// # Errors
///
/// Returns a [`ParseReport`] with spans for the structural violations this
/// parser detects. Per ADR-008, tokenizable-but-unmodelled content — Go
/// templates, exotic tags, unusual encodings — is preserved verbatim rather than
/// rejected.
pub fn parse(input: &[u8]) -> Result<Cst, ParseReport> {
    let lexed = lex(input);
    if !lexed.diagnostics.is_empty() {
        return Err(ParseReport::new(lexed.diagnostics));
    }
    let structural = validate(&lexed);
    if !structural.is_empty() {
        return Err(ParseReport::new(structural));
    }
    Ok(build(input, &lexed))
}

#[cfg(test)]
mod tests {
    use super::{lex, parse};

    fn round_trips(input: &[u8]) -> bool {
        parse(input).is_ok_and(|cst| cst.serialize() == input)
    }

    #[test]
    fn the_lexer_covers_every_byte() {
        for case in [
            &b""[..],
            b"\n",
            b"a: 1\n",
            b"# just a comment\n",
            b"key: |\n  block\n  body\n",
            b"a: {{ .Values.x | quote }}\n",
            b"list:\n  - one\n  - two\n",
            b"---\na: 1\n...\n",
            b"\xef\xbb\xbfa: 1\n",
            b"weird: \xff\xfe\n",
            b"'unterminated\n",
            b"\t\ttabs\n",
        ] {
            let lexed = lex(case);
            assert_eq!(
                lexed.covered(),
                case.len(),
                "lexer dropped bytes in {case:?}"
            );
        }
    }

    #[test]
    fn k1_holds_on_the_constructs_the_corpus_contains() {
        for case in [
            &b"a: 1\n"[..],
            b"empty:\nvalue: 2\n",
            b"# comment\nkey: value  # trailing\n",
            b"quoted: \"a\\nb\"\nsingle: 'it''s'\n",
            b"flow: {a: 1, b: [1, 2]}\n",
            b"block: |\n  line one\n    deeper\n  line three\nafter: 1\n",
            b"folded: >-\n  folded text\nnext: 2\n",
            b"anchors: &a\n  x: 1\nuse:\n  <<: *a\n",
            b"---\ndoc: 1\n---\ndoc: 2\n...\n",
            b"crlf: 1\r\nsecond: 2\r\n",
            b"no_final_newline: true",
            b"deep:\n  a:\n    b:\n      c:\n        d: 1\n",
            b"tags: !!str 123\ncustom: !Foo\n  x: 1\n",
            b"%YAML 1.2\n---\nkey: value\n",
        ] {
            assert!(round_trips(case), "K1 violated for {case:?}");
        }
    }

    #[test]
    fn go_templates_survive_verbatim() {
        // 41.2% of the corpus. Not YAML, not tokenizable as YAML, and the only
        // reason those files round-trip at all.
        for case in [
            &b"image: {{ .Values.image.repository }}:{{ .Values.image.tag }}\n"[..],
            b"{{- if .Values.enabled }}\nkey: 1\n{{- end }}\n",
            b"nested: {{ dict \"a\" 1 | toYaml | nindent 4 }}\n",
            b"unbalanced: {{ \"a colon: here\" }}\n",
            b"labels:\n  {{- include \"chart.labels\" . | nindent 4 }}\n",
        ] {
            assert!(round_trips(case), "K1 violated for template {case:?}");
        }
    }

    #[test]
    fn byte_level_oddities_survive() {
        for case in [
            &b"\xef\xbb\xbfkey: value\n"[..],  // BOM
            b"key: \xff\xfe\x80value\n",       // invalid UTF-8
            b"key: value\x00\n",               // NUL
            b"trailing: spaces   \nnext: 1\n", // trailing whitespace
            b"\n\n\nblank: lines\n\n",
            b"indent:\n   three: 1\n    four: 2\n",
        ] {
            assert!(round_trips(case), "K1 violated for {case:?}");
        }
    }

    #[test]
    fn tabs_are_accepted_and_round_trip() {
        // A tab-indentation rule was tried and removed: it rejected 12 documents
        // yaml-test-suite calls valid. Tabs are legal in blank lines, as
        // separation, and before flow indicators, and telling those apart needs
        // M2's context tracking. Until then they round-trip like any other byte.
        for case in [
            &b"\t{}\n"[..],
            b"foo:\n \tbar\n",
            b"foo: 1\n\t\nbar: 2\n",
            b"key:\n  value\n  \t\n  tabs\n",
            b"a: \"x\ty\"\n",
        ] {
            assert!(round_trips(case), "K1 violated for {case:?}");
        }
    }

    #[test]
    fn unterminated_quotes_are_rejected() {
        assert!(parse(b"a: 'unterminated\n").is_err());
        assert!(parse(b"a: \"unterminated\n").is_err());
    }

    #[test]
    fn the_lexer_always_advances() {
        for byte in 0u8..=255 {
            let _ = parse(&[byte]);
            let _ = parse(&[b'a', b':', b' ', byte]);
            let _ = parse(&[byte, b'\n', byte]);
        }
    }

    #[test]
    fn deep_nesting_does_not_touch_the_stack() {
        let levels = if cfg!(miri) { 200 } else { 20_000 };
        let mut deep = Vec::new();
        for i in 0..levels {
            deep.extend(std::iter::repeat_n(b' ', i));
            deep.extend_from_slice(b"k:\n");
        }
        assert!(round_trips(&deep), "K1 must hold at {levels} levels");
    }
}
