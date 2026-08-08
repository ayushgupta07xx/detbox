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
    // A `%` at column zero is a directive only in the directive section: before
    // any content, or after a `...` closes a document. Inside a document it is
    // ordinary content — `%!PS-Adobe-2.0` inside a block scalar, `% : 20` inside
    // a flow mapping. Four documents yaml-test-suite calls valid were rejected
    // for want of this distinction.
    let mut in_content = false;

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
        lex_line_body(input, &mut cur, &mut out, ws_start, in_content);

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

        let end = u32::try_from(out.tokens.len()).unwrap_or(u32::MAX);
        match out
            .tokens
            .get(line_first as usize..end as usize)
            .unwrap_or_default()
            .iter()
            .find(|t| {
                !matches!(
                    t.kind,
                    kind::SPACE | kind::INDENT | kind::NEWLINE | kind::COMMENT
                )
            })
            .map(|t| t.kind)
        {
            Some(kind::DOC_END) => in_content = false,
            Some(kind::DIRECTIVE) | None => {}
            Some(_) => in_content = true,
        }

        out.lines.push(LineSpan {
            indent: if neutral { u32::MAX } else { indent_cols },
            first: line_first,
            end,
        });
    }

    out
}

/// Everything on a line after its indentation, up to the line terminator.
// One dispatch table over YAML's indicator characters. Splitting it into helpers
// would scatter the ordering that makes byte coverage total — the property K1
// rests on — across several functions where it could not be read at a glance.
#[allow(clippy::too_many_lines)]
fn lex_line_body(
    input: &[u8],
    cur: &mut Cursor<'_>,
    out: &mut Lexed,
    line_start: usize,
    in_content: bool,
) {
    // Directives own the whole line, at column zero, and only in the directive
    // section — see `in_content` at the call site.
    if !in_content
        && cur.pos == line_start
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
/// # How rules get added here
///
/// Two ratchets constrain every rule, and together they make this tractable:
///
/// - `yaml-test-suite accept` is pinned at **1.0**. A rule that rejects even one
///   valid document fails CI. This is what a tab-indentation rule violated when
///   it was tried and removed — it rejected 12 valid documents.
/// - konflux **P1** requires K1 on 1,000 corpus files. A rule that refuses a real
///   Helm chart fails that too.
///
/// So rules may only be added where invalidity is unambiguous from the token
/// stream. Anything needing block/flow context tracking waits for
/// `semantic_view` at M2. The reject-rate `conformance/thresholds.tsv` records
/// is whatever this honestly achieves.
#[must_use]
pub fn validate(lexed: &Lexed, input: &[u8]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for token in &lexed.tokens {
        if token.kind == kind::ERROR {
            // Already diagnosed lexically; repeating it adds noise.
            return diagnostics;
        }
    }
    check_block_headers(lexed, input, &mut diagnostics);
    check_comments(lexed, input, &mut diagnostics);
    check_anchors(lexed, &mut diagnostics);
    check_document_structure(lexed, input, &mut diagnostics);
    diagnostics
}

fn text_of(input: &[u8], token: Token) -> &[u8] {
    input
        .get(token.span.start as usize..token.span.end as usize)
        .unwrap_or_default()
}

/// Block scalar headers: `|` and `>` with their indicators.
///
/// The indentation indicator is a single digit `1`–`9` (`0` is not a valid
/// indentation) and at most one chomping indicator may appear. And nothing but
/// whitespace and a comment may follow the header on its line — `folded: > text`
/// is not a folded scalar with content, it is an error.
fn check_block_headers(lexed: &Lexed, input: &[u8], diagnostics: &mut Vec<Diagnostic>) {
    for (idx, token) in lexed.tokens.iter().enumerate() {
        if token.kind == kind::BLOCK_HEADER {
            let text = text_of(input, *token);
            let indicators = text.get(1..).unwrap_or_default();
            let digits = indicators.iter().filter(|b| b.is_ascii_digit()).count();
            let chomps = indicators
                .iter()
                .filter(|b| matches!(b, b'+' | b'-'))
                .count();
            if digits > 1 {
                diagnostics.push(Diagnostic::new(
                    token.span,
                    "block scalar indentation indicator is a single digit 1-9",
                ));
            } else if indicators.contains(&b'0') {
                diagnostics.push(Diagnostic::new(
                    token.span,
                    "block scalar indentation indicator 0 is not valid",
                ));
            }
            if chomps > 1 {
                diagnostics.push(Diagnostic::new(
                    token.span,
                    "block scalar has more than one chomping indicator",
                ));
            }
            // A comment must be separated from the header by whitespace.
            if input.get(token.span.end as usize) == Some(&b'#') {
                diagnostics.push(Diagnostic::new(
                    token.span,
                    "comment must be preceded by whitespace",
                ));
            }
            continue;
        }

        // `folded: > first line` — the lexer could not treat `>` as a header
        // because content follows, so it fell through to a plain scalar. In
        // value position that is not a scalar named `>`, it is an error.
        if token.kind == kind::PLAIN {
            let text = text_of(input, *token);
            if matches!(text.first(), Some(b'|' | b'>'))
                && text.len() <= 3
                && text
                    .iter()
                    .skip(1)
                    .all(|b| b.is_ascii_digit() || matches!(b, b'+' | b'-'))
                && preceded_by_value_indicator(lexed, idx)
                && !followed_by_template(lexed, idx)
            {
                diagnostics.push(Diagnostic::new(
                    token.span,
                    "text after a block scalar indicator must be on the following line",
                ));
            }
        }
    }
}

/// Whether the next significant token is a Go template.
///
/// `config.alloy: |- {{- include "chart.config" . }}` appears in the corpus:
/// the template expands to the block body, so the indicator is doing its job
/// and the text after it is Helm, not stray YAML. ADR-008 puts templates on the
/// verbatim side of the accept/reject line, and this is that line in code.
fn followed_by_template(lexed: &Lexed, idx: usize) -> bool {
    lexed
        .tokens
        .get(idx + 1..)
        .unwrap_or_default()
        .iter()
        .find(|t| !matches!(t.kind, kind::SPACE | kind::INDENT))
        .is_some_and(|t| t.kind == SyntaxKind::VERBATIM)
}

/// Whether the previous significant token opens a value: `:` or `-`.
fn preceded_by_value_indicator(lexed: &Lexed, idx: usize) -> bool {
    lexed
        .tokens
        .get(..idx)
        .unwrap_or_default()
        .iter()
        .rev()
        .find(|t| !matches!(t.kind, kind::SPACE | kind::INDENT))
        .is_some_and(|t| matches!(t.kind, kind::COLON | kind::DASH))
}

/// A `#` only starts a comment when preceded by whitespace or a line start.
/// `key: "value"# nope` is an error, not a comment.
fn check_comments(lexed: &Lexed, input: &[u8], diagnostics: &mut Vec<Diagnostic>) {
    for token in &lexed.tokens {
        if token.kind != kind::COMMENT || token.span.start == 0 {
            continue;
        }
        let before = previous(input, token.span.start as usize);
        if !matches!(before, Some(b' ' | b'\t' | b'\n' | b'\r')) {
            diagnostics.push(Diagnostic::new(
                token.span,
                "comment must be preceded by whitespace",
            ));
        }
    }
}

/// A node takes one anchor, and an alias is a complete node on its own.
fn check_anchors(lexed: &Lexed, diagnostics: &mut Vec<Diagnostic>) {
    // A running `previous` rather than a collected Vec: the allocation gate
    // caught the Vec version costing 290 extra allocations across the golden
    // suite, for a pass that needs to remember exactly one token.
    let mut previous_kind: Option<SyntaxKind> = None;
    for second in lexed
        .tokens
        .iter()
        .filter(|t| !matches!(t.kind, kind::SPACE | kind::INDENT | kind::COMMENT))
    {
        // A newline ends the relationship. `list: &list\n  - a` anchors the
        // sequence that follows and is valid; `&anchor - entry` on one line is
        // not. An earlier version of this loop filtered newlines out and so
        // could not tell them apart — it rejected golden case 120.
        if second.kind == kind::NEWLINE {
            previous_kind = None;
            continue;
        }
        let was_anchor = previous_kind == Some(kind::ANCHOR);
        previous_kind = Some(second.kind);
        if !was_anchor {
            continue;
        }
        match second.kind {
            kind::ALIAS => diagnostics.push(Diagnostic::new(
                second.span,
                "an alias is a complete node and cannot carry an anchor",
            )),
            kind::ANCHOR => diagnostics.push(Diagnostic::new(
                second.span,
                "a node cannot have two anchors",
            )),
            kind::DASH => diagnostics.push(Diagnostic::new(
                second.span,
                "an anchor cannot precede a sequence entry indicator on the same line",
            )),
            _ => {}
        }
    }
}

/// Directives, document markers, and what may follow them.
fn check_document_structure(lexed: &Lexed, input: &[u8], diagnostics: &mut Vec<Diagnostic>) {
    let mut directives_pending = 0usize;
    let mut yaml_directives = 0usize;

    for line in &lexed.lines {
        // Comments are not content. Treating a comment line as content made
        // `# Global` reopen a document that `...` had just closed, and made a
        // wrapped directive comment look like the start of one.
        //
        // Iterated twice rather than collected: the allocation gate caught the
        // Vec version costing one allocation per line of every document.
        let significant = || {
            lexed
                .tokens
                .get(line.first as usize..line.end as usize)
                .unwrap_or_default()
                .iter()
                .copied()
                .filter(|t| {
                    !matches!(
                        t.kind,
                        kind::SPACE | kind::INDENT | kind::NEWLINE | kind::COMMENT
                    )
                })
        };
        let Some(first) = significant().next() else {
            continue;
        };

        match first.kind {
            kind::DIRECTIVE => {
                check_yaml_directive(input, first, &mut yaml_directives, diagnostics);
                directives_pending += 1;
            }
            kind::DOC_START => {
                directives_pending = 0;
                yaml_directives = 0;
            }
            kind::DOC_END => {
                if directives_pending > 0 {
                    diagnostics.push(Diagnostic::new(
                        first.span,
                        "directives must be followed by `---` before the document ends",
                    ));
                    directives_pending = 0;
                }
                if significant().count() > 1 {
                    diagnostics.push(Diagnostic::new(
                        first.span,
                        "nothing but a comment may follow a `...` document-end marker",
                    ));
                }
                yaml_directives = 0;
            }
            _ => {
                if directives_pending > 0 {
                    diagnostics.push(Diagnostic::new(
                        first.span,
                        "directives must be followed by a `---` document start",
                    ));
                    directives_pending = 0;
                }
            }
        }
    }

    if directives_pending > 0
        && let Some(last) = lexed.tokens.last()
    {
        diagnostics.push(Diagnostic::new(
            last.span,
            "directives must be followed by a `---` document start",
        ));
    }
}

/// `%YAML` takes one parameter, appears at most once per document, and may be
/// followed by a comment only if whitespace separates them.
///
/// There is deliberately no rule against a directive appearing after document
/// content. It was tried and the suite disproved it: `---\nscalar\n%YAML 1.2\n`
/// is valid, because once a document has started a leading `%` is content
/// rather than a directive. Distinguishing the two needs document-level state
/// this parser does not keep, so the rule was dropped rather than kept in a
/// form that rejects valid input.
fn check_yaml_directive(
    input: &[u8],
    token: Token,
    yaml_directives: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let full = text_of(input, token);
    // Strip a trailing comment, but only when whitespace precedes the `#`.
    // `%YAML 1.1#...` is an error precisely because it does not.
    let body = full
        .iter()
        .position(|b| *b == b'#')
        .filter(|at| matches!(previous(full, *at), Some(b' ' | b'\t')))
        .map_or(full, |at| full.get(..at).unwrap_or(full));

    let fields: Vec<&[u8]> = body
        .split(|b| matches!(b, b' ' | b'\t'))
        .filter(|f| !f.is_empty())
        .collect();

    if fields.first() != Some(&&b"%YAML"[..]) {
        return;
    }
    *yaml_directives += 1;
    if *yaml_directives > 1 {
        diagnostics.push(Diagnostic::new(
            token.span,
            "a document may carry only one %YAML directive",
        ));
    }
    if fields.len() != 2 {
        diagnostics.push(Diagnostic::new(
            token.span,
            "%YAML takes exactly one parameter, the version",
        ));
    } else if !fields.get(1).is_some_and(|v| {
        v.split(|b| *b == b'.').count() == 2 && v.iter().all(|b| b.is_ascii_digit() || *b == b'.')
    }) {
        diagnostics.push(Diagnostic::new(
            token.span,
            "%YAML version must be `<major>.<minor>`",
        ));
    }
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
    let structural = validate(&lexed, input);
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
    fn structural_violations_are_rejected() {
        // Each of these is a yaml-test-suite must-reject case, reduced to the
        // rule it exercises. They are here as well as in the conformance suite
        // because the suite is fetched and these are committed.
        for bad in [
            &b"--- |0\n"[..],                  // indentation indicator 0
            b"--- |10\n",                      // two-digit indicator
            b"--- |+-\n",                      // two chomping indicators
            b"---\nfolded: > first line\n",    // text after the indicator
            b"block: ># comment\n  scalar\n",  // comment not separated
            b"key: \"value\"# invalid\n",      // comment not separated
            b"key1: &a value\nkey2: &b *a\n",  // anchor on an alias
            b"&anchor - sequence entry\n",     // anchor before `-`
            b"%YAML 1.2\n",                    // directive with no document
            b"%YAML 1.2\n...\n",               // ditto
            b"%YAML 1.2\n%YAML 1.2\n---\n",    // duplicate %YAML
            b"%YAML 1.2 foo\n---\n",           // extra parameter
            b"%YAML 1.1#...\n---\n",           // comment not separated
            b"---\nkey: value\n... invalid\n", // content after `...`
        ] {
            assert!(parse(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn the_valid_neighbours_of_those_rules_are_accepted() {
        // Every rule above was first written in a form that also rejected valid
        // input. These are the documents that caught it — four from
        // yaml-test-suite, one reduced from a Helm chart in the corpus.
        for good in [
            &b"---\nscalar\n%YAML 1.2\n"[..], // `%` is content inside a document
            b"---\n{ matches\n% : 20 }\n...\n", // `%` inside a flow mapping
            b"%YAML 1.2\n---\nDocument\n... # Suffix\n", // comment after `...`
            b"%YAML 1.3 # Attempt parsing\n---\n", // comment after a directive
            b"%FOO  bar baz # ignored\n     # continued\n---\n",
            b"config: |- {{- include \"chart.config\" . }}\n", // Helm after an indicator
            b"list: &list\n  - a\n  - b\ncopy: *list\n",       // anchor, then a sequence NEXT line
        ] {
            assert!(
                round_trips(good),
                "{good:?} should be accepted and round-trip"
            );
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
