mod syntax;

use alloc::{borrow::Cow, format, string::String, vec::Vec};
use core::fmt::Write;

use regex::syntax::{
    ast::{
        self, parse::Parser, Ast, ClassPerl, ClassPerlKind, ClassSetItem, ErrorKind, Literal,
        LiteralKind, Span, SpecialLiteralKind, Visitor,
    },
    hir::{Class, Hir, HirKind},
};

pub use syntax::is_valid_ecma_regex;

/// Convert ECMA Script 262 regex to Rust regex on the best effort basis.
///
/// NOTE: Patterns with look arounds and backreferences are not supported.
///
/// # Errors
///
/// Errors are returned on unsupported or invalid regular expressions.
#[allow(clippy::result_unit_err)]
pub fn to_rust_regex(pattern: &str) -> Result<Cow<'_, str>, ()> {
    let mut pattern = escape_class_set_syntax(pattern);
    let mut ast = loop {
        match Parser::new().parse(&pattern) {
            Ok(ast) => break ast,
            Err(error) if *error.kind() == ErrorKind::EscapeUnrecognized => {
                let Span { start, end } = error.span();
                let source = error.pattern();
                if &source[start.offset..end.offset] == r"\c" {
                    if let Some(letter) = &source[end.offset..].chars().next() {
                        if letter.is_ascii_alphabetic() {
                            let start = start.offset;
                            let end = end.offset + 1;
                            let replacement = ((*letter as u8) % 32) as char;
                            match pattern {
                                Cow::Borrowed(_) => {
                                    let prefix = &source[..start];
                                    let suffix = &source[end..];
                                    pattern = Cow::Owned(format!("{prefix}{replacement}{suffix}"));
                                }
                                Cow::Owned(ref mut buffer) => {
                                    let mut char_buffer = [0; 4];
                                    let replacement = replacement.encode_utf8(&mut char_buffer);
                                    buffer.replace_range(start..end, replacement);
                                }
                            }
                            continue;
                        }
                    }
                }
                return Err(());
            }
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::UnsupportedLookAround | ErrorKind::UnsupportedBackreference
                ) =>
            {
                // Can't translate patterns with look arounds & backreferences
                return Ok(pattern);
            }
            Err(_) => {
                return Err(());
            }
        };
    };
    let mut has_changes;
    loop {
        let translator = Ecma262Translator::new(pattern);
        (pattern, has_changes) = ast::visit(&ast, translator).map_err(|_| ())?;
        if !has_changes {
            return Ok(pattern);
        }
        match Parser::new().parse(&pattern) {
            Ok(updated_ast) => {
                ast = updated_ast;
            }
            Err(_) => {
                return Err(());
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum ClassState {
    AtItemStart,
    AfterRangeStart,
    ExpectRangeEnd,
}

impl ClassState {
    fn consume_atom(&mut self) {
        *self = match self {
            Self::ExpectRangeEnd => Self::AtItemStart,
            Self::AtItemStart | Self::AfterRangeStart => Self::AfterRangeStart,
        };
    }
}

fn escape_class_set_syntax(pattern: &str) -> Cow<'_, str> {
    let mut escapes = Vec::new();
    let mut in_class = false;
    let mut escaped = false;
    let mut class_state = ClassState::AtItemStart;
    let mut previous_atom_is_class_escape = false;
    let mut class_initial = false;
    let mut chars = pattern.char_indices().peekable();
    while let Some((offset, c)) = chars.next() {
        if escaped {
            escaped = false;
            if in_class {
                class_state.consume_atom();
                previous_atom_is_class_escape = matches!(c, 'd' | 'D' | 's' | 'S' | 'w' | 'W');
                class_initial = false;
            }
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c == '[' {
            if in_class {
                escapes.push(offset);
                class_state.consume_atom();
                previous_atom_is_class_escape = false;
                class_initial = false;
            } else {
                in_class = true;
                class_state = ClassState::AtItemStart;
                previous_atom_is_class_escape = false;
                class_initial = true;
            }
            continue;
        }
        if c == ']' && in_class {
            in_class = false;
            class_state = ClassState::AtItemStart;
            previous_atom_is_class_escape = false;
            continue;
        }
        if !in_class {
            continue;
        }
        if c == '^' && class_initial {
            class_initial = false;
            continue;
        }
        if c == '-'
            && previous_atom_is_class_escape
            && chars.peek().is_some_and(|(_, next)| *next == '-')
        {
            // Without the Unicode flag, a class escape cannot participate in a range. Annex B
            // treats both hyphens in forms such as `[\w--z]` as literals.
            escapes.push(offset);
            let (next_offset, _) = chars.next().expect("the next character was peeked");
            escapes.push(next_offset);
            class_state.consume_atom();
            previous_atom_is_class_escape = false;
            class_initial = false;
            continue;
        }
        if c == '-'
            && class_state == ClassState::AfterRangeStart
            && chars.peek().is_some_and(|(_, next)| *next != ']')
        {
            // This hyphen is an ECMAScript range delimiter. Keep it unescaped so ranges such as
            // the `--b` part of `[a-z--b]` preserve their meaning.
            class_state = ClassState::ExpectRangeEnd;
            previous_atom_is_class_escape = false;
            continue;
        }
        if matches!(c, '&' | '~') && chars.peek().is_some_and(|(_, next)| *next == c) {
            escapes.push(offset);
            let (next_offset, _) = chars.next().expect("the next character was peeked");
            escapes.push(next_offset);
            class_state.consume_atom();
        } else if c == '-'
            && (pattern.as_bytes().get(offset.wrapping_sub(1)) == Some(&b'-')
                || chars.peek().is_some_and(|(_, next)| *next == '-'))
        {
            // Only escape literal hyphens. At least one hyphen in every valid adjacent pair is a
            // class atom, and escaping that atom is sufficient to avoid Rust's `--` operator.
            escapes.push(offset);
        }
        class_state.consume_atom();
        previous_atom_is_class_escape = false;
        class_initial = false;
    }
    if escapes.is_empty() {
        return Cow::Borrowed(pattern);
    }
    let mut translated = String::with_capacity(pattern.len() + escapes.len());
    let mut escapes = escapes.into_iter().peekable();
    for (offset, c) in pattern.char_indices() {
        if escapes.next_if_eq(&offset).is_some() {
            translated.push('\\');
        }
        translated.push(c);
    }
    Cow::Owned(translated)
}

struct Ecma262Translator<'a> {
    pattern: Cow<'a, str>,
    offset: usize,
    has_changes: bool,
}

impl<'a> Ecma262Translator<'a> {
    fn new(input: Cow<'a, str>) -> Self {
        Self {
            pattern: input,
            offset: 0,
            has_changes: false,
        }
    }

    fn replace_impl(&mut self, span: &Span, replacement: &str) {
        let Span { start, end } = span;
        match self.pattern {
            Cow::Borrowed(pattern) => {
                let prefix = &pattern[..start.offset];
                let suffix = &pattern[end.offset..];
                self.pattern = Cow::Owned(format!("{prefix}{replacement}{suffix}"));
            }
            Cow::Owned(ref mut buffer) => {
                buffer.replace_range(
                    start.offset + self.offset..end.offset + self.offset,
                    replacement,
                );
            }
        }
        self.offset += replacement.len() - (end.offset - start.offset);
        self.has_changes = true;
    }

    fn replace(&mut self, cls: &ClassPerl) {
        match cls.kind {
            ClassPerlKind::Digit => {
                let replacement = if cls.negated { "[^0-9]" } else { "[0-9]" };
                self.replace_impl(&cls.span, replacement);
            }
            ClassPerlKind::Word => {
                let replacement = if cls.negated {
                    "[^A-Za-z0-9_]"
                } else {
                    "[A-Za-z0-9_]"
                };
                self.replace_impl(&cls.span, replacement);
            }
            ClassPerlKind::Space => {
                let mut replacement = String::from(if cls.negated { "[^" } else { "[" });
                for (start, end) in ECMA_WHITESPACE_RANGES {
                    // Writing to a `String` cannot fail.
                    let _ = write!(
                        replacement,
                        "\\x{{{:x}}}-\\x{{{:x}}}",
                        start as u32, end as u32
                    );
                }
                replacement.push(']');
                self.replace_impl(&cls.span, &replacement);
            }
        }
    }
}

impl<'a> Visitor for Ecma262Translator<'a> {
    type Output = (Cow<'a, str>, bool);
    type Err = ast::Error;

    fn finish(self) -> Result<Self::Output, Self::Err> {
        Ok((self.pattern, self.has_changes))
    }

    fn visit_class_set_item_pre(&mut self, item: &ast::ClassSetItem) -> Result<(), Self::Err> {
        if let ClassSetItem::Perl(cls) = item {
            self.replace(cls);
        }
        Ok(())
    }
    fn visit_post(&mut self, ast: &Ast) -> Result<(), Self::Err> {
        if self.has_changes {
            return Ok(());
        }
        match ast {
            Ast::ClassPerl(perl) => {
                self.replace(perl);
            }
            Ast::Literal(literal) => {
                if let Literal {
                    kind: LiteralKind::Special(SpecialLiteralKind::Bell),
                    ..
                } = literal.as_ref()
                {
                    // Not possible to create a custom error, hence throw an arbitrary one from a
                    // known invalid pattern.
                    return Parser::new().parse("[").map(|_| ());
                }
            }
            _ => (),
        }
        Ok(())
    }
}

/// The result of analyzing a regex pattern for literal-match optimizations.
#[derive(Debug, PartialEq)]
pub enum PatternAnalysis<'a> {
    /// `^prefix` -> use `starts_with(prefix)`.
    Prefix(Cow<'a, str>),
    /// `^exact$` -> use `== exact`.
    Exact(Cow<'a, str>),
    /// `^(a|b|c)$` -> linear scan over a small sorted set of literals.
    Alternation(Vec<String>),
    /// `^\S*$` -> no ECMA-262 whitespace character (see [`is_ecma_whitespace`]).
    NoWhitespace,
}

/// Inclusive ranges of ECMA-262 `\s`: `WhiteSpace` (TAB, VT, FF, every `Zs`, BOM) and `LineTerminator`.
const ECMA_WHITESPACE_RANGES: [(char, char); 10] = [
    ('\t', '\r'),
    (' ', ' '),
    ('\u{00a0}', '\u{00a0}'),
    ('\u{1680}', '\u{1680}'),
    ('\u{2000}', '\u{200a}'),
    ('\u{2028}', '\u{2029}'),
    ('\u{202f}', '\u{202f}'),
    ('\u{205f}', '\u{205f}'),
    ('\u{3000}', '\u{3000}'),
    ('\u{feff}', '\u{feff}'),
];

/// Returns `true` for ECMA-262 whitespace characters (`\s` in ECMA regex): the union of ASCII
/// whitespace, `\u{00a0}`, and the Unicode space-separator category recognized by the spec.
#[inline]
#[must_use]
pub fn is_ecma_whitespace(c: char) -> bool {
    ECMA_WHITESPACE_RANGES
        .iter()
        .any(|&(start, end)| start <= c && c <= end)
}

/// Whether `input` holds any character `\s` matches.
///
/// All-ASCII input is scanned as bytes, which vectorizes; anything else is decoded, as before.
/// `is_ascii` stops at the first non-ASCII byte, so input taking the decode path pays little for
/// the check.
#[must_use]
#[inline]
pub fn contains_ecma_whitespace(input: &str) -> bool {
    if input.is_ascii() {
        input
            .as_bytes()
            .iter()
            .any(|byte| matches!(byte, b'\t' | b'\n' | 0x0b | 0x0c | b'\r' | b' '))
    } else {
        input.chars().any(is_ecma_whitespace)
    }
}

/// Maps a backslash escape to its literal character, or `None` if the escape requires a regex engine.
fn safe_escape(escaped: char) -> Option<char> {
    matches!(escaped, '/' | '-' | '_' | '$' | '.').then_some(escaped)
}

/// Parse a single literal alternative for use inside `^(a|b|c)$`.
/// Accepts alphanumeric chars, `-`, `_`, `/` and the safe escapes handled by [`safe_escape`].
fn parse_literal_alternative(s: &str) -> Option<String> {
    let mut result = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => result.push(safe_escape(chars.next()?)?),
            c if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '/') => result.push(c),
            _ => return None,
        }
    }
    Some(result)
}

/// Analyze a regex pattern and return a [`PatternAnalysis`] if a literal optimization applies.
///
/// Handles:
/// - `^prefix` -> [`PatternAnalysis::Prefix`] (use `starts_with`)
/// - `^exact$` -> [`PatternAnalysis::Exact`] (use `==`)
/// - `^(a|b|c)$` -> [`PatternAnalysis::Alternation`] (linear scan, sorted)
/// - `^\S*$` -> [`PatternAnalysis::NoWhitespace`] (scan for [`is_ecma_whitespace`])
///
/// Accepts unescaped alphanumeric chars, `-`, `_`, `/` and the safe escapes `\/`, `\-`, `\_`, `\$`, `\.`.
/// Returns `None` if a full regex engine is required.
#[must_use]
pub fn analyze_pattern(pattern: &str) -> Option<PatternAnalysis<'_>> {
    if pattern == r"^\S*$" {
        return Some(PatternAnalysis::NoWhitespace);
    }

    // Fast path: `^(a|b|c)$` alternation.
    if let Some(inner) = pattern
        .strip_prefix("^(")
        .and_then(|s| s.strip_suffix(")$"))
    {
        let mut alternatives: Vec<String> = inner
            .split('|')
            .map(parse_literal_alternative)
            .collect::<Option<_>>()?;
        alternatives.sort_unstable();
        return Some(PatternAnalysis::Alternation(alternatives));
    }

    let suffix = pattern.strip_prefix('^')?;

    // Fast path: no backslashes, borrow directly from the input.
    if !suffix.contains('\\') {
        // Trailing `$` is the end-of-string anchor -> Exact match.
        if let Some(body) = suffix.strip_suffix('$') {
            return if body
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '/'))
            {
                Some(PatternAnalysis::Exact(Cow::Borrowed(body)))
            } else {
                None
            };
        }
        return if suffix
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '/'))
        {
            Some(PatternAnalysis::Prefix(Cow::Borrowed(suffix)))
        } else {
            None
        };
    }

    // Slow path: unescape via `safe_escape`; a bare `$` at end means Exact.
    let mut result = String::with_capacity(suffix.len());
    let mut chars = suffix.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => result.push(safe_escape(chars.next()?)?),
            '$' => {
                // End-of-string anchor: valid only as the last character.
                if chars.peek().is_none() {
                    return Some(PatternAnalysis::Exact(Cow::Owned(result)));
                }
                return None;
            }
            c if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '/') => result.push(c),
            _ => return None,
        }
    }
    Some(PatternAnalysis::Prefix(Cow::Owned(result)))
}

/// A string `pattern` matches, or `None` where the syntax alone is not enough to build one.
///
/// A `pattern` is an unanchored search, so a look-around contributes nothing to the string built
/// here, and the caller checks the result against the pattern anyway.
#[must_use]
pub fn pattern_witness(pattern: &str) -> Option<String> {
    let hir = regex::syntax::parse(&to_rust_regex(pattern).ok()?).ok()?;
    let mut witness = String::new();
    write_witness(&hir, &mut witness).then_some(witness)
}

/// The most repetitions of one sub-expression written out.
const WITNESS_REPETITIONS: u32 = 64;

/// The longest witness built. Without a cap, nested repetitions multiply the length.
const WITNESS_LENGTH: usize = 256;

/// Appends a string matching `hir`, reporting whether one was found. On failure `witness` keeps a
/// partial string, so a caller trying another branch truncates back to its own length first.
fn write_witness(hir: &Hir, witness: &mut String) -> bool {
    if witness.len() > WITNESS_LENGTH {
        return false;
    }
    match hir.kind() {
        HirKind::Empty | HirKind::Look(_) => true,
        HirKind::Literal(literal) => match core::str::from_utf8(&literal.0) {
            Ok(text) => {
                witness.push_str(text);
                true
            }
            Err(_) => false,
        },
        HirKind::Class(class) => {
            let character = match class {
                Class::Unicode(class) => pick(class.ranges().iter().map(|r| (r.start(), r.end()))),
                Class::Bytes(class) => pick(
                    class
                        .ranges()
                        .iter()
                        .map(|r| (char::from(r.start()), char::from(r.end()))),
                ),
            };
            character.is_some_and(|character| {
                witness.push(character);
                true
            })
        }
        HirKind::Repetition(repetition) => {
            if repetition.min > WITNESS_REPETITIONS {
                return false;
            }
            (0..repetition.min).all(|_| write_witness(&repetition.sub, witness))
        }
        HirKind::Capture(capture) => write_witness(&capture.sub, witness),
        HirKind::Concat(parts) => parts.iter().all(|part| write_witness(part, witness)),
        HirKind::Alternation(branches) => {
            let start = witness.len();
            branches.iter().any(|branch| {
                witness.truncate(start);
                write_witness(branch, witness)
            })
        }
    }
}

/// A character from a class, preferring `a` over whatever the class starts with.
fn pick(ranges: impl Iterator<Item = (char, char)>) -> Option<char> {
    let mut first = None;
    for (start, end) in ranges {
        if (start..=end).contains(&'a') {
            return Some('a');
        }
        first.get_or_insert(start);
    }
    first
}

/// Try to extract a simple prefix from a pattern like `^prefix`.
/// Only matches patterns with alphanumeric characters, hyphens, underscores, and forward slashes.
/// The escaped form `\/` is also accepted and normalised to `/`.
#[must_use]
pub fn pattern_as_prefix(pattern: &str) -> Option<Cow<'_, str>> {
    match analyze_pattern(pattern) {
        Some(PatternAnalysis::Prefix(p)) => Some(p),
        _ => None,
    }
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/schema-validation/upstream/jsonschema-regex/unit/src/lib/tests_548.rs"
));
