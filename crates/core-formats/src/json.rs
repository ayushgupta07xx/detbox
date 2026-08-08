//! JSON, lossless. RFC 8259.
//!
//! Three passes, deliberately separate:
//!
//! 1. [`lex`] — total and lossless. Every byte of the input lands in exactly one
//!    lexeme. This is where K1 is won: if the lexeme stream covers the input and
//!    the builder puts every lexeme in the tree, then `serialize` — an in-order
//!    walk with no format knowledge — reproduces the input by construction.
//! 2. [`validate`] — RFC 8259's grammar over the *significant* lexemes,
//!    iteratively. Decides accept or reject (ADR-008).
//! 3. [`build`] — the green tree, in lexeme order. Runs only after validation,
//!    so it has no error paths to get wrong.
//!
//! # Iterative, everywhere
//!
//! `n_structure_100000_opening_arrays.json` in JSONTestSuite is 100,000 `[`
//! characters. A recursive-descent parser meets the stack there and **aborts the
//! process**, which is an F1 violation ("parse never panics") of the worst kind:
//! not a wrong answer, a dead process. So validation and building both use an
//! explicit heap stack, and `core-cst`'s `Drop` is iterative for the same reason.
//!
//! # What is not modelled yet
//!
//! Objects and arrays hold a **flat** sequence of children — braces, keys,
//! colons, commas, values and trivia, in source order. There are no `MEMBER`
//! nodes grouping a key with its value.
//!
//! That is deliberate. K1 is the property under test today, and flat is
//! sufficient for it. Grouping is what `semantic_view` needs, which §3.2 places
//! at M2, and it will be built then against the diff goldens that actually
//! constrain its shape. Inventing tree structure that no test constrains is how
//! a CST ends up with a shape nobody can justify.

use core_cst::{Cst, GreenChild, GreenNode, GreenToken, Span, SyntaxKind};
use std::rc::Rc;

use crate::{Diagnostic, ParseReport};

/// Syntax kinds for JSON.
///
/// Node kinds are `0..16`, token kinds `16..`. `SyntaxKind::VERBATIM`
/// (`u16::MAX`) is reserved by `core-cst` and never used here: JSON has no
/// unmodelled-but-valid constructs, so anything this grammar cannot represent
/// is a rejection rather than a verbatim span (ADR-008).
pub mod kind {
    use core_cst::SyntaxKind;

    /// The whole document, including leading and trailing trivia.
    pub const DOCUMENT: SyntaxKind = SyntaxKind(0);
    /// `{ ... }`.
    pub const OBJECT: SyntaxKind = SyntaxKind(1);
    /// `[ ... ]`.
    pub const ARRAY: SyntaxKind = SyntaxKind(2);

    /// Space, tab, CR or LF between tokens.
    pub const WHITESPACE: SyntaxKind = SyntaxKind(16);
    /// A leading UTF-8 BOM.
    pub const BOM: SyntaxKind = SyntaxKind(17);
    /// `{`
    pub const L_BRACE: SyntaxKind = SyntaxKind(18);
    /// `}`
    pub const R_BRACE: SyntaxKind = SyntaxKind(19);
    /// `[`
    pub const L_BRACKET: SyntaxKind = SyntaxKind(20);
    /// `]`
    pub const R_BRACKET: SyntaxKind = SyntaxKind(21);
    /// `:`
    pub const COLON: SyntaxKind = SyntaxKind(22);
    /// `,`
    pub const COMMA: SyntaxKind = SyntaxKind(23);
    /// A quoted string, escapes unexpanded.
    pub const STRING: SyntaxKind = SyntaxKind(24);
    /// A number literal, exactly as written.
    pub const NUMBER: SyntaxKind = SyntaxKind(25);
    /// `true`
    pub const TRUE: SyntaxKind = SyntaxKind(26);
    /// `false`
    pub const FALSE: SyntaxKind = SyntaxKind(27);
    /// `null`
    pub const NULL: SyntaxKind = SyntaxKind(28);
    /// Bytes the lexer could not classify. Always accompanied by a diagnostic.
    pub const ERROR: SyntaxKind = SyntaxKind(29);
}

/// One lexeme: a kind and the byte range it covers.
#[derive(Clone, Copy, Debug)]
pub struct Lexeme {
    /// What it is.
    pub kind: SyntaxKind,
    /// Where it is.
    pub span: Span,
}

impl Lexeme {
    fn is_trivia(self) -> bool {
        self.kind == kind::WHITESPACE || self.kind == kind::BOM
    }
}

/// Output of [`lex`]: a total cover of the input, plus anything wrong with it.
#[derive(Debug, Default)]
pub struct Lexed {
    /// Every byte of the input, in order, in exactly one lexeme.
    pub lexemes: Vec<Lexeme>,
    /// Lexical problems. Non-empty means reject.
    pub diagnostics: Vec<Diagnostic>,
}

fn at(input: &[u8], pos: usize) -> Option<u8> {
    input.get(pos).copied()
}

/// Tokenize `input`. Total: never panics, and the lexemes cover every byte.
#[must_use]
pub fn lex(input: &[u8]) -> Lexed {
    let mut out = Lexed::default();

    // RFC 8259 §8.1: "JSON text SHALL be encoded in UTF-8". Checked once over
    // the whole input rather than per-string, because a stray continuation byte
    // between tokens is just as invalid as one inside a scalar.
    if let Err(e) = std::str::from_utf8(input) {
        let start = u32::try_from(e.valid_up_to()).unwrap_or(u32::MAX);
        out.diagnostics.push(Diagnostic::new(
            Span {
                start,
                end: start.saturating_add(1),
            },
            "input is not valid UTF-8 (RFC 8259 §8.1)",
        ));
    }

    let mut pos = 0usize;
    // A UTF-8 BOM is not part of the JSON grammar. RFC 8259 §8.1 says a parser
    // MAY ignore one; we keep it as trivia so the bytes survive K1 rather than
    // being silently eaten.
    if input.starts_with(&[0xef, 0xbb, 0xbf]) {
        out.lexemes.push(Lexeme {
            kind: kind::BOM,
            span: Span { start: 0, end: 3 },
        });
        pos = 3;
    }

    while let Some(byte) = at(input, pos) {
        let start = pos;
        let kind = match byte {
            b' ' | b'\t' | b'\n' | b'\r' => {
                while matches!(at(input, pos), Some(b' ' | b'\t' | b'\n' | b'\r')) {
                    pos += 1;
                }
                kind::WHITESPACE
            }
            b'{' => {
                pos += 1;
                kind::L_BRACE
            }
            b'}' => {
                pos += 1;
                kind::R_BRACE
            }
            b'[' => {
                pos += 1;
                kind::L_BRACKET
            }
            b']' => {
                pos += 1;
                kind::R_BRACKET
            }
            b':' => {
                pos += 1;
                kind::COLON
            }
            b',' => {
                pos += 1;
                kind::COMMA
            }
            b'"' => lex_string(input, &mut pos, &mut out.diagnostics),
            b'-' | b'0'..=b'9' => lex_number(input, &mut pos, &mut out.diagnostics),
            b't' => lex_literal(input, &mut pos, b"true", kind::TRUE, &mut out.diagnostics),
            b'f' => lex_literal(input, &mut pos, b"false", kind::FALSE, &mut out.diagnostics),
            b'n' => lex_literal(input, &mut pos, b"null", kind::NULL, &mut out.diagnostics),
            _ => {
                // Unknown byte. Consume exactly one so the loop always advances:
                // a lexer that can fail to move is an infinite loop waiting for a
                // fuzz input.
                pos += 1;
                out.diagnostics.push(Diagnostic::new(
                    span(start, pos),
                    format!("unexpected byte {byte:#04x}"),
                ));
                kind::ERROR
            }
        };
        out.lexemes.push(Lexeme {
            kind,
            span: span(start, pos),
        });
    }

    out
}

fn span(start: usize, end: usize) -> Span {
    Span {
        start: u32::try_from(start).unwrap_or(u32::MAX),
        end: u32::try_from(end).unwrap_or(u32::MAX),
    }
}

fn lex_string(input: &[u8], pos: &mut usize, diagnostics: &mut Vec<Diagnostic>) -> SyntaxKind {
    let start = *pos;
    *pos += 1; // opening quote
    loop {
        let Some(byte) = at(input, *pos) else {
            diagnostics.push(Diagnostic::new(
                span(start, *pos),
                "unterminated string: reached end of input before the closing quote",
            ));
            return kind::ERROR;
        };
        match byte {
            b'"' => {
                *pos += 1;
                return kind::STRING;
            }
            // RFC 8259 §7: unescaped characters below 0x20 are not permitted.
            0x00..=0x1f => {
                diagnostics.push(Diagnostic::new(
                    span(*pos, *pos + 1),
                    format!("unescaped control character {byte:#04x} in string"),
                ));
                *pos += 1;
            }
            b'\\' => {
                *pos += 1;
                match at(input, *pos) {
                    Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => *pos += 1,
                    Some(b'u') => {
                        *pos += 1;
                        for _ in 0..4 {
                            match at(input, *pos) {
                                Some(hex) if hex.is_ascii_hexdigit() => *pos += 1,
                                _ => {
                                    diagnostics.push(Diagnostic::new(
                                        span(*pos, *pos + 1),
                                        "\\u must be followed by four hex digits",
                                    ));
                                    break;
                                }
                            }
                        }
                    }
                    Some(other) => {
                        diagnostics.push(Diagnostic::new(
                            span(*pos, *pos + 1),
                            format!("invalid escape `\\{}`", char::from(other)),
                        ));
                        *pos += 1;
                    }
                    None => {
                        diagnostics.push(Diagnostic::new(
                            span(start, *pos),
                            "unterminated string: input ends inside an escape",
                        ));
                        return kind::ERROR;
                    }
                }
            }
            _ => *pos += 1,
        }
    }
}

/// RFC 8259 §6: `-? ( 0 | [1-9][0-9]* ) ( '.' [0-9]+ )? ( [eE] [+-]? [0-9]+ )?`
///
/// The bytes are kept exactly as written and never converted. `1`, `1.0` and
/// `1e0` are three different documents, and a parser that routes them through
/// an `f64` has already lost K1 — and, for a 30-digit integer, the value too.
fn lex_number(input: &[u8], pos: &mut usize, diagnostics: &mut Vec<Diagnostic>) -> SyntaxKind {
    let start = *pos;
    let bad = |d: &mut Vec<Diagnostic>, at_pos: usize, msg: &str| {
        d.push(Diagnostic::new(span(start, at_pos + 1), msg.to_string()));
    };

    if at(input, *pos) == Some(b'-') {
        *pos += 1;
    }

    match at(input, *pos) {
        Some(b'0') => {
            *pos += 1;
            // Leading zeros are invalid: `01` is not a number.
            if matches!(at(input, *pos), Some(b'0'..=b'9')) {
                bad(diagnostics, *pos, "number has a leading zero");
                while matches!(at(input, *pos), Some(b'0'..=b'9')) {
                    *pos += 1;
                }
                return kind::ERROR;
            }
        }
        Some(b'1'..=b'9') => {
            while matches!(at(input, *pos), Some(b'0'..=b'9')) {
                *pos += 1;
            }
        }
        _ => {
            bad(diagnostics, *pos, "number has no integer part");
            return kind::ERROR;
        }
    }

    if at(input, *pos) == Some(b'.') {
        *pos += 1;
        if !matches!(at(input, *pos), Some(b'0'..=b'9')) {
            bad(diagnostics, *pos, "number has a trailing decimal point");
            return kind::ERROR;
        }
        while matches!(at(input, *pos), Some(b'0'..=b'9')) {
            *pos += 1;
        }
    }

    if matches!(at(input, *pos), Some(b'e' | b'E')) {
        *pos += 1;
        if matches!(at(input, *pos), Some(b'+' | b'-')) {
            *pos += 1;
        }
        if !matches!(at(input, *pos), Some(b'0'..=b'9')) {
            bad(diagnostics, *pos, "number has an empty exponent");
            return kind::ERROR;
        }
        while matches!(at(input, *pos), Some(b'0'..=b'9')) {
            *pos += 1;
        }
    }

    kind::NUMBER
}

fn lex_literal(
    input: &[u8],
    pos: &mut usize,
    word: &[u8],
    ok: SyntaxKind,
    diagnostics: &mut Vec<Diagnostic>,
) -> SyntaxKind {
    let start = *pos;
    if input.get(start..start + word.len()) == Some(word) {
        *pos += word.len();
        return ok;
    }
    // Consume the alphabetic run so the diagnostic points at the whole word
    // (`nul`, `tru`, `nan`) rather than at one byte.
    while matches!(at(input, *pos), Some(b) if b.is_ascii_alphabetic()) {
        *pos += 1;
    }
    if *pos == start {
        *pos += 1;
    }
    let seen = String::from_utf8_lossy(input.get(start..*pos).unwrap_or_default()).into_owned();
    let expected = String::from_utf8_lossy(word).into_owned();
    diagnostics.push(Diagnostic::new(
        span(start, *pos),
        format!("expected `{expected}`, found `{seen}`"),
    ));
    kind::ERROR
}

/// What the grammar permits next.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Expect {
    Value,
    ValueOrEndArray,
    KeyOrEndObject,
    Key,
    Colon,
    CommaOrEnd,
    Eof,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Container {
    Object,
    Array,
}

/// Check RFC 8259's grammar over the significant lexemes.
///
/// Iterative: the container stack lives on the heap, so 100,000 nested arrays
/// cost memory rather than the process.
#[must_use]
pub fn validate(lexed: &Lexed) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut stack: Vec<Container> = Vec::new();
    let mut expect = Expect::Value;

    for lexeme in lexed.lexemes.iter().filter(|l| !l.is_trivia()) {
        if lexeme.kind == kind::ERROR {
            // Already diagnosed by the lexer; reporting it twice adds noise.
            return diagnostics;
        }
        let k = lexeme.kind;
        let after_value = |stack: &Vec<Container>| {
            if stack.is_empty() {
                Expect::Eof
            } else {
                Expect::CommaOrEnd
            }
        };

        expect = match expect {
            Expect::Value | Expect::ValueOrEndArray => match k {
                kind::L_BRACE => {
                    stack.push(Container::Object);
                    Expect::KeyOrEndObject
                }
                kind::L_BRACKET => {
                    stack.push(Container::Array);
                    Expect::ValueOrEndArray
                }
                kind::R_BRACKET if expect == Expect::ValueOrEndArray => {
                    stack.pop();
                    after_value(&stack)
                }
                kind::STRING | kind::NUMBER | kind::TRUE | kind::FALSE | kind::NULL => {
                    after_value(&stack)
                }
                _ => {
                    diagnostics.push(Diagnostic::new(lexeme.span, describe(k, "a value")));
                    return diagnostics;
                }
            },
            Expect::KeyOrEndObject => match k {
                kind::R_BRACE => {
                    stack.pop();
                    after_value(&stack)
                }
                kind::STRING => Expect::Colon,
                _ => {
                    diagnostics.push(Diagnostic::new(
                        lexeme.span,
                        describe(k, "a string key or `}`"),
                    ));
                    return diagnostics;
                }
            },
            Expect::Key => {
                if k != kind::STRING {
                    diagnostics.push(Diagnostic::new(lexeme.span, describe(k, "a string key")));
                    return diagnostics;
                }
                Expect::Colon
            }
            Expect::Colon => {
                if k != kind::COLON {
                    diagnostics.push(Diagnostic::new(lexeme.span, describe(k, "`:`")));
                    return diagnostics;
                }
                Expect::Value
            }
            Expect::CommaOrEnd => match (k, stack.last()) {
                (kind::COMMA, Some(Container::Object)) => Expect::Key,
                (kind::COMMA, Some(Container::Array)) => Expect::Value,
                (kind::R_BRACE, Some(Container::Object))
                | (kind::R_BRACKET, Some(Container::Array)) => {
                    stack.pop();
                    after_value(&stack)
                }
                _ => {
                    diagnostics.push(Diagnostic::new(lexeme.span, describe(k, "`,`, `}` or `]`")));
                    return diagnostics;
                }
            },
            Expect::Eof => {
                diagnostics.push(Diagnostic::new(
                    lexeme.span,
                    "a JSON document holds exactly one value; found a second one",
                ));
                return diagnostics;
            }
        };
    }

    check_end(lexed, &stack, expect, &mut diagnostics);
    diagnostics
}

/// A document must end with every container closed and exactly one value seen.
fn check_end(
    lexed: &Lexed,
    stack: &[Container],
    expect: Expect,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let end = lexed
        .lexemes
        .last()
        .map_or(Span { start: 0, end: 0 }, |l| Span {
            start: l.span.end,
            end: l.span.end,
        });

    if !stack.is_empty() {
        diagnostics.push(Diagnostic::new(
            end,
            format!("input ends with {} container(s) still open", stack.len()),
        ));
    } else if expect != Expect::Eof {
        diagnostics.push(Diagnostic::new(
            end,
            "input ends before a complete value (an empty document is not JSON)",
        ));
    }
}

fn describe(found: SyntaxKind, expected: &str) -> String {
    let name = match found {
        kind::L_BRACE => "`{`",
        kind::R_BRACE => "`}`",
        kind::L_BRACKET => "`[`",
        kind::R_BRACKET => "`]`",
        kind::COLON => "`:`",
        kind::COMMA => "`,`",
        kind::STRING => "a string",
        kind::NUMBER => "a number",
        kind::TRUE => "`true`",
        kind::FALSE => "`false`",
        kind::NULL => "`null`",
        _ => "unexpected input",
    };
    format!("expected {expected}, found {name}")
}

/// Build the green tree, in lexeme order.
///
/// Runs only on validated input, so brackets are balanced and there is nothing
/// to recover from. Iterative for the same reason as everything else here.
#[must_use]
pub fn build(input: &[u8], lexed: &Lexed) -> Cst {
    let mut stack: Vec<(SyntaxKind, Vec<GreenChild>)> = vec![(kind::DOCUMENT, Vec::new())];

    for lexeme in &lexed.lexemes {
        let bytes = input
            .get(lexeme.span.start as usize..lexeme.span.end as usize)
            .unwrap_or_default();
        let token = GreenChild::Token(Rc::new(GreenToken::new(lexeme.kind, bytes)));

        match lexeme.kind {
            kind::L_BRACE => stack.push((kind::OBJECT, vec![token])),
            kind::L_BRACKET => stack.push((kind::ARRAY, vec![token])),
            kind::R_BRACE | kind::R_BRACKET => {
                // `stack.len() > 1` keeps the DOCUMENT frame; validation
                // guarantees it, and checking costs nothing.
                if stack.len() > 1
                    && let Some((node_kind, mut children)) = stack.pop()
                {
                    children.push(token);
                    let node = GreenChild::Node(Rc::new(GreenNode::new(node_kind, children)));
                    if let Some((_, parent)) = stack.last_mut() {
                        parent.push(node);
                    }
                } else if let Some((_, parent)) = stack.last_mut() {
                    parent.push(token);
                }
            }
            _ => {
                if let Some((_, parent)) = stack.last_mut() {
                    parent.push(token);
                }
            }
        }
    }

    // Unwind anything still open. Validation makes this unreachable; leaving it
    // total means `build` has no panic path even if a future caller skips
    // validation.
    while stack.len() > 1 {
        if let Some((node_kind, children)) = stack.pop() {
            let node = GreenChild::Node(Rc::new(GreenNode::new(node_kind, children)));
            if let Some((_, parent)) = stack.last_mut() {
                parent.push(node);
            }
        }
    }

    let (root_kind, children) = stack
        .pop()
        .unwrap_or((kind::DOCUMENT, Vec::<GreenChild>::new()));
    Cst::new(Rc::new(GreenNode::new(root_kind, children)))
}

/// Parse `input` into a lossless tree, or report why not.
///
/// # Errors
///
/// Returns a [`ParseReport`] with spans for lexical or grammatical violations of
/// RFC 8259. Per ADR-008 JSON has no verbatim escape hatch: invalid JSON is
/// rejected so konflux falls back to git's line merge rather than merging a
/// document it misread.
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
    use super::{lex, parse, validate};

    /// K1 in its most direct form.
    fn round_trips(input: &[u8]) -> bool {
        parse(input).is_ok_and(|cst| cst.serialize() == input)
    }

    #[test]
    fn the_lexer_covers_every_byte() {
        // K1's real precondition. If this holds and the builder keeps every
        // lexeme, round-tripping is structural rather than hopeful.
        for case in [
            &b"{}"[..],
            b"  {\t\"a\" : [1, 2.5e-3, true, null]  }\r\n",
            b"\xef\xbb\xbf{\"bom\":1}",
            b"{\"bad\": 01}",
            b"@!$",
            b"\"unterminated",
        ] {
            let lexed = lex(case);
            let covered: usize = lexed
                .lexemes
                .iter()
                .map(|l| (l.span.end - l.span.start) as usize)
                .sum();
            assert_eq!(covered, case.len(), "lexer dropped bytes in {case:?}");
        }
    }

    #[test]
    fn k1_holds_on_the_shapes_that_break_normalising_parsers() {
        for case in [
            &b"{}"[..],
            b"[]",
            b"{\"zebra\":1,\"apple\":2}", // key order
            b"{\"dup\":1,\"dup\":2}",     // duplicate keys
            b"[1,1.0,1e5,1E+5,-0,-0.0]",  // number spelling
            b"[\"a\\/b\",\"\\u0041\",\"\\t\",\"\\ud83d\\ude00\"]", // escape form
            b"{ \"a\" : 1 ,\n\n  \"b\" : 2 }", // irregular whitespace
            b"\xef\xbb\xbf{\"bom\":true}", // BOM
            b"{\r\n  \"crlf\": true\r\n}\r\n", // CRLF
            b"{\"noFinalNewline\":true}",
            b"{\"trailing\":1}\n\n\n",
        ] {
            assert!(round_trips(case), "K1 violated for {case:?}");
        }
    }

    #[test]
    fn numbers_are_bytes_not_values() {
        // A 30-digit integer and a 34-digit float survive exactly; routing them
        // through an f64 would lose both the spelling and the value.
        let case = b"[123456789012345678901234567890,0.1000000000000000055511151231257827]";
        assert!(round_trips(case));
    }

    #[test]
    fn rfc8259_number_grammar_is_enforced() {
        for bad in [
            &b"01"[..],
            b"-01",
            b"1.",
            b".1",
            b"+1",
            b"1e",
            b"1e+",
            b"-",
            b"0x1",
        ] {
            assert!(parse(bad).is_err(), "{bad:?} should be rejected");
        }
        for good in [&b"0"[..], b"-0", b"1", b"1.0", b"1e5", b"1E+5", b"-1.5e-10"] {
            assert!(parse(good).is_ok(), "{good:?} should be accepted");
        }
    }

    #[test]
    fn string_grammar_is_enforced() {
        for bad in [
            &b"\"\x01\""[..], // raw control character
            b"\"\\x\"",       // invalid escape
            b"\"\\u00\"",     // short \u
            b"\"\\uZZZZ\"",   // non-hex \u
            b"\"unterminated",
            b"'single quoted'",
        ] {
            assert!(parse(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn structural_errors_are_rejected() {
        for bad in [
            &b""[..],
            b"   ",
            b"{",
            b"}",
            b"[1,]",
            b"{\"a\":1,}",
            b"{\"a\"1}",
            b"{a:1}",
            b"[1 2]",
            b"{} {}",
            b"[}",
        ] {
            assert!(parse(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn invalid_utf8_is_rejected() {
        // RFC 8259 §8.1. Note this differs from YAML, whose K1 suite requires
        // invalid UTF-8 to round-trip — the accept/reject line follows each
        // format's own spec (ADR-008).
        assert!(parse(b"[\"\xff\xfe\"]").is_err());
    }

    /// Depth for the stack-safety tests, reduced under miri.
    ///
    /// Miri interprets every operation and needs orders of magnitude longer, so
    /// 100,000 brackets there is a job that runs for hours. This scopes a tool
    /// to what it can check rather than weakening a proof: the property at stake
    /// is native stack depth, which miri does not model, and the full depth
    /// still runs in `gate/tests`, `gate/msrv` and `gate/platform`. What miri is
    /// here for — the absence of undefined behaviour — is identical at 500.
    fn depth(full: usize) -> usize {
        if cfg!(miri) { 500 } else { full }
    }

    #[test]
    fn deep_nesting_does_not_touch_the_stack() {
        // JSONTestSuite ships 100,000 opening brackets. A recursive parser dies
        // here, and dying is an F1 violation of the worst kind: not a wrong
        // answer, a dead process.
        let open = vec![b'['; depth(100_000)];
        assert!(parse(&open).is_err(), "unclosed brackets must be rejected");

        let levels = depth(50_000);
        let mut balanced = Vec::new();
        balanced.extend(std::iter::repeat_n(b'[', levels));
        balanced.push(b'1');
        balanced.extend(std::iter::repeat_n(b']', levels));
        assert!(round_trips(&balanced), "K1 must hold at {levels} levels");
    }

    #[test]
    fn the_lexer_always_advances() {
        // A lexer that can fail to move is an infinite loop waiting for a fuzz
        // input. Every byte value, alone and in pairs with a quote.
        for byte in 0u8..=255 {
            let _ = parse(&[byte]);
            let _ = parse(&[b'"', byte]);
            let _ = parse(&[b'[', byte, b']']);
        }
    }

    #[test]
    fn validation_reports_the_first_problem_with_a_span() {
        let lexed = lex(b"{\"a\" 1}");
        let diagnostics = validate(&lexed);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("expected `:`"));
        assert_eq!(diagnostics[0].span.start, 5);
    }
}
