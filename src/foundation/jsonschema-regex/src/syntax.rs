//! ECMA-262 regular expression syntax check.
//!
//! The grammar is the `Pattern[+UnicodeMode, +NamedCaptureGroups]` production of ECMA-262,
//! including the pattern modifiers of ES2025, widened with the identity escapes that patterns
//! without the `u` flag allow: a backslash before any character that is not alphanumeric. Drafts 4
//! to 2019-09 cite the ES5.1 dialect, which has no `u` flag. Unicode property names and values are
//! checked for shape only, not against the Unicode tables.

use std::{borrow::Cow, cmp::Ordering};

/// Whether `pattern` is a syntactically valid ECMA-262 regular expression under the `u` flag, or
/// differs from one only by escaping non-alphanumeric characters.
#[must_use]
pub fn is_valid_ecma_regex(pattern: &str) -> bool {
    Parser::new(pattern).parse().is_ok()
}

struct SyntaxError;

type Parsed<T> = Result<T, SyntaxError>;

/// What the cursor follows, which decides whether a quantifier may appear.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Previous {
    /// Start of the pattern, an alternative, or a group.
    Nothing,
    Atom,
    Assertion,
    Quantifier,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GroupKind {
    Capturing,
    NonCapturing,
    Lookaround,
}

/// An open group. Group names are tracked per alternative: two groups may share a name when
/// they can never both take part in one match, i.e. when they sit in different alternatives of
/// some enclosing disjunction.
struct Frame<'a> {
    kind: GroupKind,
    /// Names declared in the alternative being parsed, including those of groups closed inside it.
    current_alternative: Vec<Cow<'a, str>>,
    /// Names declared in the alternatives already finished.
    finished_alternatives: Vec<Cow<'a, str>>,
}

impl Frame<'_> {
    fn new(kind: GroupKind) -> Self {
        Self {
            kind,
            current_alternative: Vec::new(),
            finished_alternatives: Vec::new(),
        }
    }
}

struct Parser<'a> {
    pattern: &'a str,
    position: usize,
    previous: Previous,
    /// The pattern itself is the outermost frame, so this is never empty.
    frames: Vec<Frame<'a>>,
    capturing_groups: u64,
    /// The largest `\N` seen, checked against `capturing_groups` once the whole pattern is read.
    largest_backreference: u64,
    /// Every `\k<name>` seen, checked against the declared names once the whole pattern is read.
    referenced_names: Vec<Cow<'a, str>>,
    declared_names: Vec<Cow<'a, str>>,
}

impl<'a> Parser<'a> {
    fn new(pattern: &'a str) -> Self {
        Self {
            pattern,
            position: 0,
            previous: Previous::Nothing,
            frames: vec![Frame::new(GroupKind::NonCapturing)],
            capturing_groups: 0,
            largest_backreference: 0,
            referenced_names: Vec::new(),
            declared_names: Vec::new(),
        }
    }

    fn peek(&self) -> Option<char> {
        self.pattern[self.position..].chars().next()
    }

    fn peek_second(&self) -> Option<char> {
        self.pattern[self.position..].chars().nth(1)
    }

    fn bump(&mut self) -> Parsed<char> {
        let c = self.peek().ok_or(SyntaxError)?;
        self.position += c.len_utf8();
        Ok(c)
    }

    fn eat(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.position += expected.len_utf8();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: char) -> Parsed<()> {
        if self.eat(expected) {
            Ok(())
        } else {
            Err(SyntaxError)
        }
    }

    fn current_frame(&mut self) -> &mut Frame<'a> {
        self.frames
            .last_mut()
            .expect("the outermost frame is never popped")
    }

    fn parse(mut self) -> Parsed<()> {
        while let Some(c) = self.peek() {
            self.position += c.len_utf8();
            match c {
                '|' => {
                    let frame = self.current_frame();
                    frame
                        .finished_alternatives
                        .append(&mut frame.current_alternative);
                    self.previous = Previous::Nothing;
                }
                '(' => {
                    self.open_group()?;
                    self.previous = Previous::Nothing;
                }
                ')' => self.previous = self.close_group()?,
                '*' | '+' | '?' => self.quantified()?,
                '{' => {
                    self.braced_quantifier()?;
                    self.quantified()?;
                }
                '}' | ']' => return Err(SyntaxError),
                '^' | '$' => self.previous = Previous::Assertion,
                '\\' => self.previous = self.atom_escape()?,
                '[' => {
                    self.class()?;
                    self.previous = Previous::Atom;
                }
                _ => self.previous = Previous::Atom,
            }
        }
        if self.frames.len() != 1 || self.largest_backreference > self.capturing_groups {
            return Err(SyntaxError);
        }
        if self
            .referenced_names
            .iter()
            .any(|name| !self.declared_names.contains(name))
        {
            return Err(SyntaxError);
        }
        Ok(())
    }

    /// A quantifier was consumed: it must follow an atom and may take a lazy marker.
    fn quantified(&mut self) -> Parsed<()> {
        if self.previous != Previous::Atom {
            return Err(SyntaxError);
        }
        self.eat('?');
        self.previous = Previous::Quantifier;
        Ok(())
    }

    /// `{n}`, `{n,}` or `{n,m}` with `n <= m`; the opening brace is consumed.
    fn braced_quantifier(&mut self) -> Parsed<()> {
        let lower = self.decimal_digits()?;
        if self.eat(',') && self.peek() != Some('}') {
            let upper = self.decimal_digits()?;
            if compare_decimal(lower, upper) == Ordering::Greater {
                return Err(SyntaxError);
            }
        }
        self.expect('}')
    }

    fn decimal_digits(&mut self) -> Parsed<&'a str> {
        let start = self.position;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.position += 1;
        }
        if self.position == start {
            return Err(SyntaxError);
        }
        Ok(&self.pattern[start..self.position])
    }

    /// The opening paren is consumed.
    fn open_group(&mut self) -> Parsed<()> {
        let kind = if self.eat('?') {
            match self.bump()? {
                ':' => GroupKind::NonCapturing,
                '=' | '!' => GroupKind::Lookaround,
                '<' => {
                    if matches!(self.peek(), Some('=' | '!')) {
                        self.position += 1;
                        GroupKind::Lookaround
                    } else {
                        let name = self.group_name()?;
                        self.declare_name(name)?;
                        self.capturing_groups += 1;
                        GroupKind::Capturing
                    }
                }
                first => {
                    self.modifiers(first)?;
                    GroupKind::NonCapturing
                }
            }
        } else {
            self.capturing_groups += 1;
            GroupKind::Capturing
        };
        self.frames.push(Frame::new(kind));
        Ok(())
    }

    /// `(?ims-ims:` with the first character after `?` consumed: at least one flag, none repeated.
    fn modifiers(&mut self, first: char) -> Parsed<()> {
        let mut seen = [false; 3];
        let mut flags = 0;
        let mut dashes = 0;
        let mut c = first;
        while c != ':' {
            match c {
                'i' | 'm' | 's' => {
                    let slot = &mut seen[match c {
                        'i' => 0,
                        'm' => 1,
                        _ => 2,
                    }];
                    if std::mem::replace(slot, true) {
                        return Err(SyntaxError);
                    }
                    flags += 1;
                }
                '-' if dashes == 0 => dashes = 1,
                _ => return Err(SyntaxError),
            }
            c = self.bump()?;
        }
        if flags == 0 {
            return Err(SyntaxError);
        }
        Ok(())
    }

    /// The closing paren is consumed. Returns what the closed group counts as for a quantifier.
    fn close_group(&mut self) -> Parsed<Previous> {
        if self.frames.len() == 1 {
            return Err(SyntaxError);
        }
        let mut frame = self.frames.pop().expect("a nested frame is open");
        let parent = self.current_frame();
        parent
            .current_alternative
            .append(&mut frame.current_alternative);
        parent
            .current_alternative
            .append(&mut frame.finished_alternatives);
        Ok(match frame.kind {
            GroupKind::Lookaround => Previous::Assertion,
            GroupKind::Capturing | GroupKind::NonCapturing => Previous::Atom,
        })
    }

    fn declare_name(&mut self, name: Cow<'a, str>) -> Parsed<()> {
        if self
            .frames
            .iter()
            .any(|frame| frame.current_alternative.contains(&name))
        {
            return Err(SyntaxError);
        }
        self.declared_names.push(name.clone());
        self.current_frame().current_alternative.push(name);
        Ok(())
    }

    /// `name>` of a group name; the opening angle bracket is consumed.
    fn group_name(&mut self) -> Parsed<Cow<'a, str>> {
        let start = self.position;
        let mut decoded: Option<String> = None;
        let mut first = true;
        loop {
            let char_start = self.position;
            let c = match self.bump()? {
                '>' => break,
                '\\' => {
                    self.expect('u')?;
                    let c = char::from_u32(self.unicode_escape()?).ok_or(SyntaxError)?;
                    decoded.get_or_insert_with(|| self.pattern[start..char_start].to_owned());
                    c
                }
                c => c,
            };
            let allowed = if first {
                is_identifier_start(c)
            } else {
                is_identifier_part(c)
            };
            if !allowed {
                return Err(SyntaxError);
            }
            if let Some(name) = &mut decoded {
                name.push(c);
            }
            first = false;
        }
        if first {
            return Err(SyntaxError);
        }
        Ok(match decoded {
            Some(name) => Cow::Owned(name),
            None => Cow::Borrowed(&self.pattern[start..self.position - 1]),
        })
    }

    /// An escape outside a class; the backslash is consumed.
    fn atom_escape(&mut self) -> Parsed<Previous> {
        match self.bump()? {
            'b' | 'B' => return Ok(Previous::Assertion),
            'd' | 'D' | 's' | 'S' | 'w' | 'W' => {}
            'p' | 'P' => self.unicode_property()?,
            'k' => {
                self.expect('<')?;
                let name = self.group_name()?;
                self.referenced_names.push(name);
            }
            '1'..='9' => {
                self.position -= 1;
                let number = self.decimal_digits()?.parse().unwrap_or(u64::MAX);
                self.largest_backreference = self.largest_backreference.max(number);
            }
            c => {
                self.character_escape(c)?;
            }
        }
        Ok(Previous::Atom)
    }

    /// `CharacterEscape` with its first character consumed; returns the code point.
    fn character_escape(&mut self, c: char) -> Parsed<u32> {
        Ok(match c {
            'f' => 0x0C,
            'n' => 0x0A,
            'r' => 0x0D,
            't' => 0x09,
            'v' => 0x0B,
            'c' => {
                let letter = self.bump()?;
                if !letter.is_ascii_alphabetic() {
                    return Err(SyntaxError);
                }
                letter as u32 % 32
            }
            '0' => {
                if self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    return Err(SyntaxError);
                }
                0
            }
            'x' => {
                let high = self.hex_digit()?;
                let low = self.hex_digit()?;
                high * 16 + low
            }
            'u' => self.unicode_escape()?,
            c if !c.is_alphanumeric() => c as u32,
            _ => return Err(SyntaxError),
        })
    }

    fn hex_digit(&mut self) -> Parsed<u32> {
        self.bump()?.to_digit(16).ok_or(SyntaxError)
    }

    /// `RegExpUnicodeEscapeSequence` after `\u`; a lead surrogate joins a following trail one.
    fn unicode_escape(&mut self) -> Parsed<u32> {
        if self.eat('{') {
            let mut value: u32 = 0;
            let mut digits = 0;
            while !self.eat('}') {
                value = value.saturating_mul(16).saturating_add(self.hex_digit()?);
                digits += 1;
            }
            if digits == 0 || value > 0x10_FFFF {
                return Err(SyntaxError);
            }
            return Ok(value);
        }
        let lead = self.hex4()?;
        if (0xD800..=0xDBFF).contains(&lead) && self.pattern[self.position..].starts_with("\\u") {
            let position = self.position;
            self.position += 2;
            match self.hex4() {
                Ok(trail) if (0xDC00..=0xDFFF).contains(&trail) => {
                    return Ok(0x10000 + ((lead - 0xD800) << 10) + (trail - 0xDC00));
                }
                _ => self.position = position,
            }
        }
        Ok(lead)
    }

    fn hex4(&mut self) -> Parsed<u32> {
        let mut value = 0;
        for _ in 0..4 {
            value = value * 16 + self.hex_digit()?;
        }
        Ok(value)
    }

    /// `{Name}` or `{Name=Value}` after `\p` or `\P`.
    fn unicode_property(&mut self) -> Parsed<()> {
        self.expect('{')?;
        self.property_word()?;
        if self.eat('=') {
            self.property_word()?;
        }
        self.expect('}')
    }

    fn property_word(&mut self) -> Parsed<()> {
        let start = self.position;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            self.position += 1;
        }
        if self.position == start {
            return Err(SyntaxError);
        }
        Ok(())
    }

    /// A character class; the opening bracket is consumed.
    fn class(&mut self) -> Parsed<()> {
        self.eat('^');
        loop {
            if self.eat(']') {
                return Ok(());
            }
            let start = self.class_atom()?;
            // A dash right before the closing bracket is a literal.
            if self.peek() == Some('-') && self.peek_second() != Some(']') {
                self.position += 1;
                let end = self.class_atom()?;
                match (start, end) {
                    (Some(start), Some(end)) if start <= end => {}
                    _ => return Err(SyntaxError),
                }
            }
        }
    }

    /// One class atom: `Some` code point, or `None` for a class escape such as `\d`.
    fn class_atom(&mut self) -> Parsed<Option<u32>> {
        let c = self.bump()?;
        if c != '\\' {
            return Ok(Some(c as u32));
        }
        Ok(match self.bump()? {
            'b' => Some(0x08),
            '-' => Some('-' as u32),
            'd' | 'D' | 's' | 'S' | 'w' | 'W' => None,
            'p' | 'P' => {
                self.unicode_property()?;
                None
            }
            c => Some(self.character_escape(c)?),
        })
    }
}

/// `ID_Start` plus `$` and `_`, approximated by the Alphabetic property.
fn is_identifier_start(c: char) -> bool {
    c.is_alphabetic() || matches!(c, '$' | '_')
}

/// `ID_Continue` plus `$`, `_`, ZWNJ and ZWJ, approximated by the Alphabetic and Numeric properties.
fn is_identifier_part(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '$' | '_' | '\u{200C}' | '\u{200D}')
}

/// Compares two digit strings by value.
fn compare_decimal(left: &str, right: &str) -> Ordering {
    let left = left.trim_start_matches('0');
    let right = right.trim_start_matches('0');
    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/jsonschema-regex/unit/src/syntax/tests_508.rs"));
