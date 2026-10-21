// Copyright 2016 The Fancy Regex Authors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

//! A regex parser yielding an AST.

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::{format, vec};

use bit_set::BitSet;
use regex_syntax::escape_into;

use crate::parse_flags::*;
use crate::{codepoint_len, Error, Expr, ParseError, Result, MAX_RECURSION};
use crate::{
    Absent, Assertion, AstNode, BacktrackingControlVerb, CaptureGroupTarget, LookAround::*,
};

#[cfg(not(feature = "std"))]
pub(crate) type NamedGroups = alloc::collections::BTreeMap<String, usize>;
#[cfg(feature = "std")]
pub(crate) type NamedGroups = std::collections::HashMap<String, usize>;

#[derive(Debug, Clone)]
pub struct ExprTree {
    pub expr: Expr,
    pub backrefs: BitSet,
    pub named_groups: NamedGroups,
    pub(crate) numeric_capture_group_references: bool,
    pub contains_subroutines: bool,
    pub(crate) self_recursive: bool,
    /// Total number of capture groups in the pattern, as counted by the resolver.
    /// Used by the analyzer to validate that backreferences refer to groups that exist.
    pub(crate) total_groups: usize,
    /// The first backreference group number that exceeds `total_groups`, if any.
    /// Such numbers are not inserted into `backrefs` (to avoid allocating memory proportional
    /// to an arbitrarily large value); the analyzer reads this field instead to report the error.
    pub(crate) out_of_range_backref: Option<usize>,
    /// Whether numbered groups were ignored (treated as non-capturing) during parsing
    pub(crate) numbered_groups_ignored: bool,
}

#[derive(Debug)]
pub(crate) struct Parser<'a> {
    re: &'a str, // source
    flags: u32,
    numeric_capture_group_references: bool,
    contains_subroutines: bool,
    self_recursive: bool,
    has_named_groups: bool,
}

/// anything which involves a capture group name will be parsed into an AstNode
/// which will then get resolved into a standard Expr variant after parsing is otherwise complete.
/// This allows us to only have to handle numbered capture groups internally during analysis and compilation.
impl<'a> Parser<'a> {
    pub(crate) fn parse_with_flags(re: &str, flags: u32) -> Result<ExprTree> {
        let mut p = Parser::new(re, flags);
        let (ix, mut expr) = p.parse_re(0, 0)?;
        if ix < re.len() {
            return Err(Error::ParseError(
                ix,
                ParseError::GeneralParseError("end of string not reached".to_string()),
            ));
        }

        let mut resolver = Resolver {
            named_groups: NamedGroups::default(),
            named_group_positions: NamedGroups::default(),
            ignore_numbered_groups: p.flag(FLAG_IGNORE_NUMBERED_GROUPS_WHEN_NAMED_GROUPS_EXIST)
                && p.has_named_groups,
            next_group_index: 1,
            total_groups: 0,
            out_of_range_backref: None,
            curr_group: 0,
            backrefs: Default::default(),
        };
        resolver.resolve_groups(&mut expr);
        resolver.total_groups = resolver.next_group_index - 1;
        resolver.next_group_index = 1;
        resolver.curr_group = 0;
        resolver.resolve_capture_group_targets(&mut expr)?;

        Ok(ExprTree {
            expr,
            backrefs: resolver.backrefs,
            named_groups: resolver.named_groups,
            numeric_capture_group_references: p.numeric_capture_group_references,
            contains_subroutines: p.contains_subroutines,
            self_recursive: p.self_recursive,
            total_groups: resolver.total_groups,
            out_of_range_backref: resolver.out_of_range_backref,
            numbered_groups_ignored: resolver.ignore_numbered_groups,
        })
    }

    fn new(re: &str, flags: u32) -> Parser<'_> {
        Parser {
            re,
            numeric_capture_group_references: false,
            flags,
            contains_subroutines: false,
            self_recursive: false,
            has_named_groups: false,
        }
    }

    fn parse_re(&mut self, ix: usize, depth: usize) -> Result<(usize, Expr)> {
        let (ix, child) = self.parse_branch(ix, depth)?;
        let mut ix = self.optional_whitespace(ix)?;
        if self.re[ix..].starts_with('|') {
            let mut children = vec![child];
            while self.re[ix..].starts_with('|') {
                ix += 1;
                let (next, child) = self.parse_branch(ix, depth)?;
                children.push(child);
                ix = self.optional_whitespace(next)?;
            }
            return Ok((ix, Expr::Alt(children)));
        }
        Ok((ix, child))
    }

    fn parse_branch(&mut self, ix: usize, depth: usize) -> Result<(usize, Expr)> {
        let mut children = Vec::new();
        let mut ix = ix;
        while ix < self.re.len() {
            let (next, child) = self.parse_piece(ix, depth)?;
            if next == ix {
                break;
            }
            if child != Expr::Empty {
                children.push(child);
            }
            ix = next;
        }
        match children.len() {
            0 => Ok((ix, Expr::Empty)),
            1 => Ok((ix, children.pop().unwrap())),
            _ => Ok((ix, Expr::Concat(children))),
        }
    }

    fn parse_quantifier(
        &mut self,
        ix: usize,
        start_ix: usize,
        child: Expr,
    ) -> Result<(usize, Expr)> {
        let mut ix = self.optional_whitespace(ix)?;
        if ix < self.re.len() {
            let (mut lo, mut hi, user_specified) = match self.re.as_bytes()[ix] {
                b'?' => (0, 1, false),
                b'*' => (0, usize::MAX, false),
                b'+' => (1, usize::MAX, false),
                b'{' => {
                    match self.parse_repeat(ix) {
                        Ok((next, lo, hi)) => {
                            ix = next - 1;
                            (lo, hi, true)
                        }
                        Err(_) => {
                            // Invalid repeat syntax, which results in `{` being treated as a literal
                            return Ok((ix, child));
                        }
                    }
                }
                _ => return Ok((ix, child)),
            };
            let mut skip = false;
            if !self.is_repeatable(&child) {
                // In Oniguruma mode, `(?:)` followed by a quantifier is a
                // zero-width no-op that Oniguruma silently accepts. Skip
                // past the quantifier and its modifiers, returning Empty.
                // We check `ix > start_ix` to ensure the atom actually consumed
                // input (e.g. a `(?:)` group), as opposed to a bare
                // quantifier with no preceding atom.
                if child == Expr::Empty && ix > start_ix && self.flag(FLAG_ONIGURUMA_MODE) {
                    skip = true;
                } else {
                    return Err(Error::ParseError(ix, ParseError::TargetNotRepeatable));
                }
            }
            ix = self.optional_whitespace(ix + 1)?;
            let mut greedy = true;
            if ix < self.re.len() && self.re.as_bytes()[ix] == b'?' {
                greedy = false;
                ix += 1;
            }
            greedy ^= self.flag(FLAG_SWAP_GREED);
            let mut atomic_group = false;
            if self.flag(FLAG_ONIGURUMA_MODE) {
                if hi < lo {
                    (lo, hi) = (hi, lo);
                    atomic_group = true;
                } else if !user_specified && ix < self.re.len() && self.re.as_bytes()[ix] == b'+' {
                    ix += 1;
                    atomic_group = true;
                }
            } else if ix < self.re.len() && self.re.as_bytes()[ix] == b'+' {
                ix += 1;
                atomic_group = true;
            }
            if skip {
                return Ok((ix, child));
            } else {
                let mut node = Expr::Repeat {
                    child: Box::new(child),
                    lo,
                    hi,
                    greedy,
                };
                if atomic_group {
                    node = Expr::AtomicGroup(Box::new(node));
                }
                return Ok((ix, node));
            }
        }
        Ok((ix, child))
    }

    fn parse_piece(&mut self, ix: usize, depth: usize) -> Result<(usize, Expr)> {
        let start = ix;
        let (ix, child) = self.parse_atom(ix, depth)?;
        let (mut ix, mut repeat) = self.parse_quantifier(ix, start, child)?;
        if self.flag(FLAG_ONIGURUMA_MODE) {
            // Oniguruma allows expressions like `\s{1,2}{0,1}` to mean `(?:\s{1,2})?`.
            // Each additional adjacent quantifier wraps the previous result in
            // another `Expr::Repeat` (and `Expr::AtomicGroup` if a possessive
            // `+` modifier follows), increasing AST depth. We count them
            // against the parse recursion budget AND enforce a tighter
            // per-piece cap, because the AST depth interacts multiplicatively
            // with subroutine-call inlining (`\g<...>`) during compile and
            // can otherwise blow the stack on adversarial inputs like
            // Oniguruma test #139.
            const MAX_ADJACENT_QUANTIFIERS: usize = 4;
            let mut depth = depth;
            let mut adjacent = 0;
            loop {
                let before = ix;
                let (next_ix, next_repeat) = self.parse_quantifier(ix, before, repeat)?;
                ix = next_ix;
                repeat = next_repeat;
                if ix == before {
                    break;
                }
                adjacent += 1;
                depth += 1;
                if adjacent > MAX_ADJACENT_QUANTIFIERS || depth >= MAX_RECURSION {
                    return Err(Error::ParseError(ix, ParseError::RecursionExceeded));
                }
            }
        }
        Ok((ix, repeat))
    }

    fn is_repeatable(&self, child: &Expr) -> bool {
        match child {
            Expr::LookAround(_, _) => false,
            Expr::Empty => false,
            // In Oniguruma mode, repetition after assertions is not allowed
            Expr::Assertion(_) => !self.flag(FLAG_ONIGURUMA_MODE),
            Expr::KeepOut => false,
            Expr::ContinueFromPreviousMatchEnd => false,
            Expr::BackrefExistsCondition { .. } => false,
            Expr::BacktrackingControlVerb(_) => false,
            _ => true,
        }
    }

    // ix, lo, hi
    fn parse_repeat(&self, ix: usize) -> Result<(usize, usize, usize)> {
        let ix = self.optional_whitespace(ix + 1)?; // skip opening '{'
        let bytes = self.re.as_bytes();
        if ix == self.re.len() {
            return Err(Error::ParseError(ix, ParseError::InvalidRepeat));
        }
        let mut end = ix;
        let lo = if bytes[ix] == b',' {
            0
        } else if let Some((next, lo)) = parse_usize(self.re, ix) {
            end = next;
            lo
        } else {
            return Err(Error::ParseError(ix, ParseError::InvalidRepeat));
        };
        let ix = self.optional_whitespace(end)?; // past lo number
        if ix == self.re.len() {
            return Err(Error::ParseError(ix, ParseError::InvalidRepeat));
        }
        end = ix;
        let hi = match bytes[ix] {
            b'}' => lo,
            b',' => {
                end = self.optional_whitespace(ix + 1)?; // past ','
                if let Some((next, hi)) = parse_usize(self.re, end) {
                    end = next;
                    hi
                } else {
                    usize::MAX
                }
            }
            _ => return Err(Error::ParseError(ix, ParseError::InvalidRepeat)),
        };
        let ix = self.optional_whitespace(end)?; // past hi number
        if ix == self.re.len() || bytes[ix] != b'}' {
            return Err(Error::ParseError(ix, ParseError::InvalidRepeat));
        }
        Ok((ix + 1, lo, hi))
    }

    fn parse_atom(&mut self, ix: usize, depth: usize) -> Result<(usize, Expr)> {
        let ix = self.optional_whitespace(ix)?;
        if ix == self.re.len() {
            return Ok((ix, Expr::Empty));
        }
        match self.re.as_bytes()[ix] {
            b'.' => Ok((
                ix + 1,
                Expr::Any {
                    newline: self.flag(FLAG_DOTNL),
                    crlf: self.flag(FLAG_CRLF),
                },
            )),
            b'^' => Ok((
                ix + 1,
                if self.flag(FLAG_MULTI) {
                    if self.flag(FLAG_ONIGURUMA_MODE) {
                        Expr::Assertion(Assertion::StartLineOniguruma {
                            crlf: self.flag(FLAG_CRLF),
                        })
                    } else {
                        Expr::Assertion(Assertion::StartLine {
                            crlf: self.flag(FLAG_CRLF),
                        })
                    }
                } else {
                    Expr::Assertion(Assertion::StartText)
                },
            )),
            b'$' => Ok((
                ix + 1,
                if self.flag(FLAG_MULTI) {
                    Expr::Assertion(Assertion::EndLine {
                        crlf: self.flag(FLAG_CRLF),
                    })
                } else {
                    Expr::Assertion(Assertion::EndText)
                },
            )),
            b'(' => self.parse_group(ix, depth),
            b'\\' => self.parse_escape(ix, false),
            b'+' | b'*' | b'?' | b'|' | b')' => Ok((ix, Expr::Empty)),
            b'[' => self.parse_class(ix),
            b => {
                // TODO: maybe want to match multiple codepoints?
                let next = ix + codepoint_len(b);
                Ok((
                    next,
                    Expr::Literal {
                        val: String::from(&self.re[ix..next]),
                        casei: self.flag(FLAG_CASEI),
                    },
                ))
            }
        }
    }

    fn parse_delimited_backref(
        &mut self,
        ix: usize,
        open: &str,
        close: &str,
        allow_relative: bool,
    ) -> Result<(usize, Expr)> {
        if let Some(ParsedId { id, relative, skip }) =
            parse_id(&self.re[ix..], open, close, allow_relative)
        {
            let (target, relative_recursion_level) = match (id.is_empty(), relative) {
                // backref with nothing before the + or -, so it is purely a relative group backref
                (true, Some(relative)) => (CaptureGroupTarget::Relative(relative), None),
                (_, relative) => (
                    if let Ok(num) = id.parse::<usize>() {
                        CaptureGroupTarget::ByNumber(num)
                    } else {
                        CaptureGroupTarget::ByName(id.to_string())
                    },
                    relative,
                ),
            };

            Ok((
                ix + skip,
                Expr::AstNode(
                    AstNode::Backref {
                        target,
                        casei: self.flag(FLAG_CASEI),
                        relative_recursion_level,
                    },
                    ix,
                ),
            ))
        } else {
            Err(Error::ParseError(ix, ParseError::InvalidGroupName))
        }
    }

    fn parse_delimited_subroutine_call(
        &mut self,
        ix: usize,
        open: &str,
        close: &str,
        allow_relative: bool,
    ) -> Result<(usize, Expr)> {
        // First: try unrestricted name (no relative suffix)
        if let Some((name, skip)) = parse_unrestricted_name(&self.re[ix..], open, close) {
            // Check it's not a +N/-N or bare +/- (those should go through the relative path)
            let is_pure_relative = name == "+"
                || name == "-"
                || ((name.starts_with('+') || name.starts_with('-'))
                    && name[1..].parse::<usize>().is_ok());

            if !is_pure_relative {
                let target = if let Ok(num) = name.parse::<usize>() {
                    self.numeric_capture_group_references = true;
                    if num == 0 {
                        self.self_recursive = true;
                    }
                    CaptureGroupTarget::ByNumber(num)
                } else {
                    CaptureGroupTarget::ByName(name.to_string())
                };
                self.contains_subroutines = true;
                return Ok((
                    ix + skip,
                    Expr::AstNode(AstNode::SubroutineCall(target), ix),
                ));
            }
        }

        // Second: try as relative reference (+N / -N)
        if allow_relative {
            if let Some(ParsedId {
                id,
                relative: Some(rel),
                skip,
            }) = parse_id(&self.re[ix..], open, close, true)
            {
                if id.is_empty() {
                    let target = CaptureGroupTarget::Relative(rel);
                    self.contains_subroutines = true;
                    return Ok((
                        ix + skip,
                        Expr::AstNode(AstNode::SubroutineCall(target), ix),
                    ));
                }
            }
        }

        Err(Error::ParseError(ix, ParseError::InvalidGroupName))
    }

    fn parse_unrestricted_subroutine_call(
        &mut self,
        ix: usize,
        open: &str,
        close: &str,
    ) -> Result<(usize, Expr)> {
        if let Some((name, skip)) = parse_unrestricted_name(&self.re[ix..], open, close) {
            let target = if let Ok(num) = name.parse::<usize>() {
                self.numeric_capture_group_references = true;
                if num == 0 {
                    self.self_recursive = true;
                }
                CaptureGroupTarget::ByNumber(num)
            } else {
                CaptureGroupTarget::ByName(name.to_string())
            };
            self.contains_subroutines = true;
            Ok((
                ix + skip,
                Expr::AstNode(AstNode::SubroutineCall(target), ix),
            ))
        } else {
            Err(Error::ParseError(ix, ParseError::InvalidGroupName))
        }
    }

    fn parse_numbered_backref(&mut self, ix: usize) -> Result<(usize, Expr)> {
        let (end, group) = self.parse_numbered_backref_or_subroutine_call(ix)?;
        self.numeric_capture_group_references = true;
        Ok((
            end,
            Expr::AstNode(
                AstNode::Backref {
                    target: CaptureGroupTarget::ByNumber(group),
                    casei: self.flag(FLAG_CASEI),
                    relative_recursion_level: None,
                },
                ix,
            ),
        ))
    }

    fn parse_numbered_subroutine_call(&mut self, ix: usize) -> Result<(usize, Expr)> {
        let (end, group) = self.parse_numbered_backref_or_subroutine_call(ix)?;
        self.numeric_capture_group_references = true;
        self.contains_subroutines = true;
        if group == 0 {
            self.self_recursive = true;
        }
        Ok((end, Expr::SubroutineCall(group)))
    }

    fn parse_numbered_backref_or_subroutine_call(&self, ix: usize) -> Result<(usize, usize)> {
        parse_usize(self.re, ix).ok_or(Error::ParseError(ix, ParseError::InvalidBackref))
    }

    // ix points to \ character
    fn parse_escape(&mut self, ix: usize, in_class: bool) -> Result<(usize, Expr)> {
        let bytes = self.re.as_bytes();
        let Some(b) = bytes.get(ix + 1).copied() else {
            return Err(Error::ParseError(ix, ParseError::TrailingBackslash));
        };
        let end = ix + 1 + codepoint_len(b);
        Ok(if b.is_ascii_digit() {
            return self.parse_numbered_backref(ix + 1);
        } else if matches!(b, b'k') && !in_class {
            // Named backref: \k<name>
            if bytes.get(end) == Some(&b'\'') {
                return self.parse_delimited_backref(end, "'", "'", true);
            } else {
                return self.parse_delimited_backref(end, "<", ">", true);
            }
        } else if b == b'A' && !in_class {
            (end, Expr::Assertion(Assertion::StartText))
        } else if b == b'z' && !in_class {
            (end, Expr::Assertion(Assertion::EndText))
        } else if b == b'Z' && !in_class {
            // \Z matches at the end of the string, or before any number of newlines at the end
            (
                end,
                Expr::Assertion(Assertion::EndTextIgnoreTrailingNewlines {
                    crlf: self.flag(FLAG_CRLF),
                }),
            )
        } else if (b == b'b' || b == b'B') && !in_class {
            let check_pos = self.optional_whitespace(end)?;
            if bytes.get(check_pos) == Some(&b'{') {
                let next_open_brace_pos = self.optional_whitespace(check_pos + 1)?;
                let is_repetition = matches!(
                    bytes.get(next_open_brace_pos),
                    Some(&ch) if ch.is_ascii_digit() || ch == b','
                );
                if !is_repetition {
                    return self.parse_word_boundary_brace(ix);
                }
            }
            let expr = if b == b'b' {
                Expr::Assertion(Assertion::WordBoundary)
            } else {
                Expr::Assertion(Assertion::NotWordBoundary)
            };
            (end, expr)
        } else if b == b'<' && !in_class {
            let expr = if self.flag(FLAG_ONIGURUMA_MODE) {
                make_literal("<")
            } else {
                Expr::Assertion(Assertion::LeftWordBoundary)
            };
            (end, expr)
        } else if b == b'>' && !in_class {
            let expr = if self.flag(FLAG_ONIGURUMA_MODE) {
                make_literal(">")
            } else {
                Expr::Assertion(Assertion::RightWordBoundary)
            };
            (end, expr)
        } else if matches!(b | 32, b'd' | b's' | b'w') {
            (
                end,
                Expr::Delegate {
                    inner: String::from(&self.re[ix..end]),
                    casei: self.flag(FLAG_CASEI),
                },
            )
        } else if (b | 32) == b'h' {
            let s = if b == b'h' {
                "[0-9A-Fa-f]"
            } else {
                "[^0-9A-Fa-f]"
            };
            (
                end,
                Expr::Delegate {
                    inner: String::from(s),
                    casei: false,
                },
            )
        } else if b == b'x' {
            let end = self.optional_whitespace(end)?;
            return self.parse_hex(end, 2);
        } else if b == b'u' {
            let end = self.optional_whitespace(end)?;
            return self.parse_hex(end, 4);
        } else if b == b'U' {
            let end = self.optional_whitespace(end)?;
            return self.parse_hex(end, 8);
        } else if (b | 32) == b'p' && end != bytes.len() {
            let mut end = end;
            let b = bytes[end];
            end += codepoint_len(b);
            if b == b'{' {
                loop {
                    if end == self.re.len() {
                        return Err(Error::ParseError(ix, ParseError::UnclosedUnicodeName));
                    }
                    let b = bytes[end];
                    if b == b'}' {
                        end += 1;
                        break;
                    }
                    end += codepoint_len(b);
                }
            }
            (
                end,
                Expr::Delegate {
                    inner: remap_unicode_property_if_necessary(
                        &self.re[ix..end],
                        self.flag(FLAG_UNICODE),
                        in_class,
                    ),
                    casei: self.flag(FLAG_CASEI),
                },
            )
        } else if b == b'K' && !in_class {
            (end, Expr::KeepOut)
        } else if b == b'G' && !in_class {
            (end, Expr::ContinueFromPreviousMatchEnd)
        } else if b == b'R' && !in_class {
            (
                end,
                Expr::GeneralNewline {
                    unicode: self.flag(FLAG_UNICODE),
                },
            )
        } else if b == b'O' && !in_class {
            (
                end,
                Expr::Any {
                    newline: true,
                    crlf: true,
                },
            )
        } else if b == b'N' && !in_class {
            // \N always means "not a newline" regardless of dot mode flags
            // - just like \n is a literal, this is the inverse
            (
                end,
                Expr::Any {
                    newline: false,
                    crlf: false,
                },
            )
        } else if b == b'g' && !in_class {
            if end == self.re.len() {
                return Err(Error::ParseError(
                    ix,
                    ParseError::InvalidEscape("\\g".to_string()),
                ));
            }
            let b = bytes[end];
            if b.is_ascii_digit() {
                self.parse_numbered_subroutine_call(end)?
            } else if b == b'\'' {
                self.parse_delimited_subroutine_call(end, "'", "'", true)?
            } else {
                self.parse_delimited_subroutine_call(end, "<", ">", true)?
            }
        } else {
            // printable ASCII (including space, see issue #29)
            (
                end,
                make_literal(match b {
                    b'a' => "\x07", // BEL
                    b'b' => "\x08", // BS
                    b'f' => "\x0c", // FF
                    b'n' => "\n",   // LF
                    b'r' => "\r",   // CR
                    b't' => "\t",   // TAB
                    b'v' => "\x0b", // VT
                    b'e' => "\x1b", // ESC
                    b' ' => " ",
                    b => {
                        let s = &self.re[ix + 1..end];
                        if b.is_ascii_alphabetic()
                            && !matches!(
                                b,
                                b'k' | b'A' | b'z' | b'b' | b'B' | b'<' | b'>' | b'K' | b'G' | b'R'
                            )
                        {
                            return Err(Error::ParseError(
                                ix,
                                ParseError::InvalidEscape(format!("\\{}", s)),
                            ));
                        } else {
                            s
                        }
                    }
                }),
            )
        })
    }

    // ix points after '\x', eg to 'A0' or '{12345}', or after `\u` or `\U`
    fn parse_hex(&self, ix: usize, digits: usize) -> Result<(usize, Expr)> {
        if ix >= self.re.len() {
            // Incomplete escape sequence
            return Err(Error::ParseError(ix, ParseError::InvalidHex));
        }
        let bytes = self.re.as_bytes();
        let b = bytes[ix];
        // Parse fixed-width hex (e.g., \xAB)
        if ix + digits <= self.re.len() && bytes[ix..ix + digits].iter().all(|&b| is_hex_digit(b)) {
            let hex_str = &self.re[ix..ix + digits];
            return self.hex_to_literal(ix, ix + digits, hex_str);
        }
        // Parse brace-enclosed hex (e.g., \u{00AB})
        if b == b'{' {
            let mut pos = ix + 1;
            let mut hex_chars = String::new();
            while pos < self.re.len() {
                // Skip whitespace/comments if FLAG_IGNORE_SPACE is set
                pos = self.optional_whitespace(pos)?;
                if pos >= self.re.len() {
                    return Err(Error::ParseError(ix, ParseError::InvalidHex));
                }
                let b = bytes[pos];
                if b == b'}' && !hex_chars.is_empty() {
                    return self.hex_to_literal(ix, pos + 1, &hex_chars);
                }
                if is_hex_digit(b) && hex_chars.len() < 8 {
                    hex_chars.push(b as char);
                    pos += 1;
                } else {
                    return Err(Error::ParseError(ix, ParseError::InvalidHex));
                }
            }
        }
        Err(Error::ParseError(ix, ParseError::InvalidHex))
    }

    fn hex_to_literal(&self, ix: usize, end: usize, hex_str: &str) -> Result<(usize, Expr)> {
        let codepoint = u32::from_str_radix(hex_str, 16).unwrap();
        if let Some(c) = char::from_u32(codepoint) {
            Ok((
                end,
                Expr::Literal {
                    val: c.to_string(),
                    casei: self.flag(FLAG_CASEI),
                },
            ))
        } else {
            Err(Error::ParseError(ix, ParseError::InvalidCodepointValue))
        }
    }

    // ix points before '\b' or '\B'
    fn parse_word_boundary_brace(&self, ix: usize) -> Result<(usize, Expr)> {
        let bytes = self.re.as_bytes();

        // Verify that we have '\b' or '\B'
        if !matches!(bytes.get(ix..ix + 2), Some([b'\\', b'b' | b'B'])) {
            return Err(Error::ParseError(
                ix,
                ParseError::InvalidEscape("\\b{...}".to_string()),
            ));
        }
        // Skip whitespace/comments after \b or \B if FLAG_IGNORE_SPACE is set
        let brace_start = self.optional_whitespace(ix + 2)?;
        // Verify we have '{'
        if bytes.get(brace_start) != Some(&b'{') {
            return Err(Error::ParseError(
                ix,
                ParseError::InvalidEscape("\\b{...}".to_string()),
            ));
        }
        // Extract content between braces
        let mut pos = brace_start + 1;
        let mut content = String::new();
        while pos < self.re.len() {
            let b = bytes[pos];
            if b == b'}' {
                break;
            }
            // Skip whitespace/comments if FLAG_IGNORE_SPACE is set
            let next_pos = self.optional_whitespace(pos)?;
            if next_pos > pos {
                // Whitespace was skipped
                pos = next_pos;
                if pos >= self.re.len() || bytes[pos] == b'}' {
                    break;
                }
            }
            // Add non-whitespace character to content
            let b = bytes[pos];
            if b != b'}' {
                content.push(b as char);
                pos += codepoint_len(b);
            }
        }

        let end_brace = pos;
        if end_brace >= self.re.len() || bytes[end_brace] != b'}' {
            return Err(Error::ParseError(
                ix,
                ParseError::InvalidEscape("\\b{...}".to_string()),
            ));
        }

        // \B{...} is not supported
        if bytes[ix + 1] == b'B' {
            return Err(Error::ParseError(
                ix,
                ParseError::InvalidEscape(format!("\\B{{{}}}", content)),
            ));
        }

        let expr = match content.as_str() {
            "start" => Expr::Assertion(Assertion::LeftWordBoundary),
            "end" => Expr::Assertion(Assertion::RightWordBoundary),
            "start-half" => Expr::Assertion(Assertion::LeftWordHalfBoundary),
            "end-half" => Expr::Assertion(Assertion::RightWordHalfBoundary),
            _ => {
                return Err(Error::ParseError(
                    ix,
                    ParseError::InvalidEscape(format!("\\b{{{}}}", content)),
                ));
            }
        };

        Ok((end_brace + 1, expr))
    }

    fn parse_class(&mut self, ix: usize) -> Result<(usize, Expr)> {
        let bytes = self.re.as_bytes();
        let mut ix = ix + 1; // skip opening '['
        let mut class = String::new();
        let mut nest = 1;
        let mut seen_negated_property = false;
        class.push('[');

        // Negated character class
        if bytes.get(ix) == Some(&b'^') {
            class.push('^');
            ix += 1;
        }

        // `]` does not have to be escaped after opening `[` or `[^`
        if bytes.get(ix) == Some(&b']') {
            class.push(']');
            ix += 1;
        }

        loop {
            if ix == self.re.len() {
                return Err(Error::ParseError(ix, ParseError::InvalidClass));
            }
            let end = match bytes[ix] {
                b'\\' => {
                    // We support more escapes than regex, so parse it ourselves before delegating.
                    let (end, expr) = self.parse_escape(ix, true)?;
                    match expr {
                        Expr::Literal { val, .. } => {
                            debug_assert_eq!(val.chars().count(), 1);
                            escape_into(&val, &mut class);
                        }
                        Expr::Delegate { inner, .. } => {
                            // Check if this is a negated property that needs && prefix
                            // A negated property is a nested character class that starts with [^ and contains
                            // Unicode property patterns or POSIX classes
                            let is_negated_prop = inner.starts_with("[^")
                                && (inner.contains(r"\p{")
                                    || inner.contains(r"\P{")
                                    || inner.contains("[:"));
                            if is_negated_prop {
                                if seen_negated_property {
                                    // This is not the first negated property, add &&
                                    class.push_str("&&");
                                }
                                // Mark that we've seen a negated property
                                seen_negated_property = true;
                            }
                            class.push_str(&inner);
                        }
                        _ => {
                            return Err(Error::ParseError(ix, ParseError::InvalidClass));
                        }
                    }
                    end
                }
                b'[' => {
                    nest += 1;
                    class.push('[');
                    let mut end = ix + 1;

                    // Handle `]` after `[` or `[^` in nested classes
                    // Check for negated character class
                    if bytes.get(end) == Some(&b'^') {
                        class.push('^');
                        end += 1;
                    }

                    // `]` does not have to be escaped after opening `[` or `[^`
                    if bytes.get(end) == Some(&b']') {
                        class.push(']');
                        end += 1;
                    }

                    end
                }
                b']' => {
                    nest -= 1;
                    class.push(']');
                    if nest == 0 {
                        break;
                    }
                    ix + 1
                }
                b => {
                    let end = ix + codepoint_len(b);
                    class.push_str(&self.re[ix..end]);
                    end
                }
            };
            ix = end;
        }
        let class = Expr::Delegate {
            inner: class,
            casei: self.flag(FLAG_CASEI),
        };
        let ix = ix + 1; // skip closing ']'
        Ok((ix, class))
    }

    fn parse_group(&mut self, ix: usize, depth: usize) -> Result<(usize, Expr)> {
        let depth = depth + 1;
        if depth >= MAX_RECURSION {
            return Err(Error::ParseError(ix, ParseError::RecursionExceeded));
        }
        let open_paren_ix = ix; // position of the opening `(`
        let ix = self.optional_whitespace(ix + 1)?;
        let mut group_name = None;
        let (la, skip) = if self.re[ix..].starts_with("?=") {
            (Some(LookAhead), 2)
        } else if self.re[ix..].starts_with("?!") {
            (Some(LookAheadNeg), 2)
        } else if self.re[ix..].starts_with("?<=") {
            (Some(LookBehind), 3)
        } else if self.re[ix..].starts_with("?<!") {
            (Some(LookBehindNeg), 3)
        } else if self.re[ix..].starts_with("?<") || self.re[ix..].starts_with("?'") {
            // Named capture group using Oniguruma syntax: (?<name>...) or (?'name'...)
            let (open, close) = if self.re[ix..].starts_with("?<") {
                ("<", ">")
            } else {
                ("'", "'")
            };
            if let Some((name, skip)) = parse_unrestricted_name(&self.re[ix + 1..], open, close) {
                group_name = Some(name.to_string());
                self.has_named_groups = true;
                (None, skip + 1)
            } else {
                return Err(Error::ParseError(ix, ParseError::InvalidGroupName));
            }
        } else if self.re[ix..].starts_with("?P<") {
            // Named capture group using Python syntax: (?P<name>...)
            if let Some((name, skip)) = parse_unrestricted_name(&self.re[ix + 2..], "<", ">") {
                group_name = Some(name.to_string());
                self.has_named_groups = true;
                (None, skip + 2)
            } else {
                return Err(Error::ParseError(ix, ParseError::InvalidGroupName));
            }
        } else if self.re[ix..].starts_with("?P=") {
            // Backref using Python syntax: (?P=name)
            return self.parse_delimited_backref(ix + 3, "", ")", false);
        } else if self.re[ix..].starts_with("?~") {
            return self.parse_absent(ix + 1, depth);
        } else if self.re[ix..].starts_with("?>") {
            (None, 2)
        } else if self.re[ix..].starts_with("?(") {
            return self.parse_conditional(ix + 2, depth);
        } else if self.re[ix..].starts_with("?P>") {
            return self.parse_unrestricted_subroutine_call(ix + 3, "", ")");
        } else if self.re[ix..].starts_with("*") {
            return self.parse_backtracking_control_verb(ix);
        } else if self.re[ix..].starts_with('?') {
            return self.parse_flags(ix, depth);
        } else {
            (None, 0)
        };
        let ix = ix + skip;
        let (ix, child) = self.parse_re(ix, depth)?;
        let ix = self.check_for_close_paren(ix)?;
        let result = match (la, skip) {
            (Some(la), _) => Expr::LookAround(Box::new(child), la),
            (None, 2) => Expr::AtomicGroup(Box::new(child)),
            _ => make_ast_group(child, group_name, open_paren_ix),
        };
        Ok((ix, result))
    }

    fn check_for_close_paren(&self, ix: usize) -> Result<usize> {
        let ix = self.optional_whitespace(ix)?;
        if ix == self.re.len() {
            return Err(Error::ParseError(ix, ParseError::UnclosedOpenParen));
        } else if self.re.as_bytes()[ix] != b')' {
            return Err(Error::ParseError(
                ix,
                ParseError::GeneralParseError("expected close paren".to_string()),
            ));
        }
        Ok(ix + 1)
    }

    // ix points to `?` in `(?`
    fn parse_flags(&mut self, ix: usize, depth: usize) -> Result<(usize, Expr)> {
        let start = ix + 1;

        fn unknown_flag(re: &str, start: usize, end: usize) -> Error {
            let after_end = end + codepoint_len(re.as_bytes()[end]);
            let s = format!("(?{}", &re[start..after_end]);
            Error::ParseError(start, ParseError::UnknownFlag(s))
        }

        let mut ix = start;
        let mut neg = false;
        let oldflags = self.flags;
        loop {
            ix = self.optional_whitespace(ix)?;
            if ix == self.re.len() {
                return Err(Error::ParseError(ix, ParseError::UnclosedOpenParen));
            }
            let b = self.re.as_bytes()[ix];
            match b {
                b'i' => self.update_flag(FLAG_CASEI, neg),
                b'm' => self.update_flag(FLAG_MULTI, neg),
                b'R' => self.update_flag(FLAG_CRLF, neg),
                b's' => self.update_flag(FLAG_DOTNL, neg),
                b'U' => self.update_flag(FLAG_SWAP_GREED, neg),
                b'x' => self.update_flag(FLAG_IGNORE_SPACE, neg),
                b'u' => {
                    if neg == self.flag(FLAG_UNICODE) {
                        return Err(Error::ParseError(
                            ix,
                            ParseError::ChangingUnicodeModeUnsupported,
                        ));
                    }
                }
                b'-' => {
                    if neg {
                        return Err(unknown_flag(self.re, start, ix));
                    }
                    neg = true;
                }
                b')' => {
                    if ix == start || neg && ix == start + 1 {
                        return Err(unknown_flag(self.re, start, ix));
                    }
                    return Ok((ix + 1, Expr::Empty));
                }
                b':' => {
                    if neg && ix == start + 1 {
                        return Err(unknown_flag(self.re, start, ix));
                    }
                    ix += 1;
                    let (ix, child) = self.parse_re(ix, depth)?;
                    if ix == self.re.len() {
                        return Err(Error::ParseError(ix, ParseError::UnclosedOpenParen));
                    } else if self.re.as_bytes()[ix] != b')' {
                        return Err(Error::ParseError(
                            ix,
                            ParseError::GeneralParseError("expected close paren".to_string()),
                        ));
                    };
                    self.flags = oldflags;
                    return Ok((ix + 1, child));
                }
                _ => return Err(unknown_flag(self.re, start, ix)),
            }
            ix += 1;
        }
    }

    /// Parse the target of a backref-exists condition — e.g. `1`, `name`, `+1`, `-1` — from
    /// the delimited form used in `(?(...)...)`.  Returns an `AstNode::BackrefExistsCondition`
    /// expression on success.
    fn parse_condition_target(
        &mut self,
        ix: usize,
        open: &str,
        close: &str,
        allow_relative: bool,
    ) -> Result<(usize, Expr)> {
        if let Some(ParsedId { id, relative, skip }) =
            parse_id(&self.re[ix..], open, close, allow_relative)
        {
            let (target, relative_recursion_level) = match (id.is_empty(), relative) {
                // purely relative group reference: `(?(-1)...)` or `(?(+2)...)`
                (true, Some(relative)) => (CaptureGroupTarget::Relative(relative), None),
                (_, relative) => {
                    if let Ok(num) = id.parse::<usize>() {
                        (CaptureGroupTarget::ByNumber(num), relative)
                    } else {
                        (CaptureGroupTarget::ByName(id.to_string()), relative)
                    }
                }
            };
            Ok((
                ix + skip,
                Expr::AstNode(
                    AstNode::BackrefExistsCondition {
                        target,
                        relative_recursion_level,
                    },
                    ix,
                ),
            ))
        } else {
            Err(Error::ParseError(ix, ParseError::InvalidGroupName))
        }
    }

    // ix points to after the last ( in (?(
    fn parse_conditional(&mut self, ix: usize, depth: usize) -> Result<(usize, Expr)> {
        if ix >= self.re.len() {
            return Err(Error::ParseError(ix, ParseError::UnclosedOpenParen));
        }
        let bytes = self.re.as_bytes();
        // get the character after the open paren
        let b = bytes[ix];

        // Check for DEFINE condition first - (?(DEFINE)...)
        if self.re[ix..].starts_with("DEFINE)") {
            let end = ix + "DEFINE)".len();
            let (end, definitions) = self.parse_re(end, depth)?;
            let after = self.check_for_close_paren(end)?;
            return Ok((
                after,
                Expr::DefineGroup {
                    definitions: Box::new(definitions),
                },
            ));
        }

        let (next, condition) = if b == b'\'' {
            self.parse_condition_target(ix, "'", "')", true)?
        } else if b == b'<' {
            self.parse_condition_target(ix, "<", ">)", true)?
        } else if b == b'+' || b == b'-' || b.is_ascii_digit() {
            self.parse_condition_target(ix, "", ")", true)?
        } else if b == b'*' {
            self.parse_backtracking_control_verb(ix)?
        } else {
            let (next, condition) = self.parse_re(ix, depth)?;
            (self.check_for_close_paren(next)?, condition)
        };
        let (end, child) = self.parse_re(next, depth)?;
        if end == next {
            // The condition had no branches, so it is a backref-exists-only condition
            // (e.g. `(?(1))`). The delimited paths already produce AstNode::BackrefExistsCondition
            // directly; for the general parse_re path we require an expression.
            if matches!(
                condition,
                Expr::AstNode(AstNode::BackrefExistsCondition { .. }, _)
            ) {
                let after = self.check_for_close_paren(end)?;
                return Ok((after, condition));
            } else {
                return Err(Error::ParseError(
                    end,
                    ParseError::GeneralParseError(
                        "expected conditional to be a backreference or at least an expression for when the condition is true".to_string()
                    )
                ));
            }
        }
        let if_true: Expr;
        let mut if_false: Expr = Expr::Empty;
        if let Expr::Alt(mut alternatives) = child {
            // the truth branch will be the first alternative
            if_true = alternatives.remove(0);
            // if there is only one alternative left, take it out the Expr::Alt
            if alternatives.len() == 1 {
                if_false = alternatives.pop().expect("expected 2 alternatives");
            } else {
                // otherwise the remaining branches become the false branch
                if_false = Expr::Alt(alternatives);
            }
        } else {
            // there is only one branch - the truth branch. i.e. "if" without "else"
            if_true = child;
        }
        let inner_condition = condition;

        let after = self.check_for_close_paren(end)?;
        Ok((
            after,
            if if_true == Expr::Empty && if_false == Expr::Empty {
                inner_condition
            } else {
                Expr::Conditional {
                    condition: Box::new(inner_condition),
                    true_branch: Box::new(if_true),
                    false_branch: Box::new(if_false),
                }
            },
        ))
    }

    // ix points to * after (
    fn parse_backtracking_control_verb(&mut self, ix: usize) -> Result<(usize, Expr)> {
        if self.re[ix..].starts_with("*FAIL)") {
            Ok((
                ix + "*FAIL)".len(),
                Expr::BacktrackingControlVerb(BacktrackingControlVerb::Fail),
            ))
        } else if self.re[ix..].starts_with("*F)") {
            Ok((
                ix + "*F)".len(),
                Expr::BacktrackingControlVerb(BacktrackingControlVerb::Fail),
            ))
        } else if self.re[ix..].starts_with("*ACCEPT)") {
            Ok((
                ix + "*ACCEPT)".len(),
                Expr::BacktrackingControlVerb(BacktrackingControlVerb::Accept),
            ))
        } else if self.re[ix..].starts_with("*COMMIT)") {
            Ok((
                ix + "*COMMIT)".len(),
                Expr::BacktrackingControlVerb(BacktrackingControlVerb::Commit),
            ))
        } else if self.re[ix..].starts_with("*SKIP)") {
            Ok((
                ix + "*SKIP)".len(),
                Expr::BacktrackingControlVerb(BacktrackingControlVerb::Skip),
            ))
        } else if self.re[ix..].starts_with("*PRUNE)") {
            Ok((
                ix + "*PRUNE)".len(),
                Expr::BacktrackingControlVerb(BacktrackingControlVerb::Prune),
            ))
        } else {
            Err(Error::ParseError(ix, ParseError::TargetNotRepeatable))
        }
    }

    // ix points to ~ after (?
    fn parse_absent(&mut self, ix: usize, depth: usize) -> Result<(usize, Expr)> {
        let ix = ix + 1; // skip `~` character
        if ix >= self.re.len() {
            return Err(Error::ParseError(ix, ParseError::UnclosedOpenParen));
        }

        if self.re[ix..].starts_with('|') {
            // (?~|...) - either absent expression, absent stopper, or range clear
            let ix = ix + 1; // skip `|` character

            // Parse the absent part (up to | or ))
            let (ix, absent) = self.parse_branch(ix, depth)?;

            if ix >= self.re.len() {
                return Err(Error::ParseError(ix, ParseError::UnclosedOpenParen));
            }

            if self.re.as_bytes()[ix] == b'|' {
                // (?~|absent|exp) - absent expression
                let ix = ix + 1; // skip |
                let (ix, exp) = self.parse_branch(ix, depth)?;
                let ix = self.check_for_close_paren(ix)?;
                Ok((
                    ix,
                    Expr::Absent(Absent::Expression {
                        absent: Box::new(absent),
                        exp: Box::new(exp),
                    }),
                ))
            } else if self.re.as_bytes()[ix] == b')' {
                // (?~|absent) - absent stopper, or (?~|) - range clear
                if absent == Expr::Empty {
                    Ok((ix + 1, Expr::Absent(Absent::Clear)))
                } else {
                    Ok((ix + 1, Expr::Absent(Absent::Stopper(Box::new(absent)))))
                }
            } else {
                Err(Error::ParseError(
                    ix,
                    ParseError::GeneralParseError(
                        "expected '|' or ')' in absent expression".to_string(),
                    ),
                ))
            }
        } else {
            // (?~absent) - absent repeater
            let (ix, absent) = self.parse_re(ix, depth)?;
            let ix = self.check_for_close_paren(ix)?;
            Ok((ix, Expr::Absent(Absent::Repeater(Box::new(absent)))))
        }
    }

    fn flag(&self, flag: u32) -> bool {
        let v = self.flags & flag;
        v == flag
    }

    fn update_flag(&mut self, flag: u32, neg: bool) {
        if neg {
            self.flags &= !flag;
        } else {
            self.flags |= flag;
        }
    }

    fn optional_whitespace(&self, mut ix: usize) -> Result<usize> {
        let bytes = self.re.as_bytes();
        loop {
            if ix == self.re.len() {
                return Ok(ix);
            }
            match bytes[ix] {
                b'#' if self.flag(FLAG_IGNORE_SPACE) => {
                    match bytes[ix..].iter().position(|&c| c == b'\n') {
                        Some(x) => ix += x + 1,
                        None => return Ok(self.re.len()),
                    }
                }
                b' ' | b'\r' | b'\n' | b'\t' if self.flag(FLAG_IGNORE_SPACE) => ix += 1,
                b'(' if bytes[ix..].starts_with(b"(?#") => {
                    ix += 3;
                    loop {
                        if ix >= self.re.len() {
                            return Err(Error::ParseError(ix, ParseError::UnclosedOpenParen));
                        }
                        match bytes[ix] {
                            b')' => {
                                ix += 1;
                                break;
                            }
                            b'\\' => ix += 2,
                            _ => ix += 1,
                        }
                    }
                }
                _ => return Ok(ix),
            }
        }
    }
}

struct Resolver {
    named_groups: NamedGroups,
    backrefs: BitSet,
    /// Maps each named group's name to the byte offset of its opening `(` in the pattern.
    /// Used to enforce that named backrefs (`\k<name>`) cannot refer to groups that appear
    /// later in the pattern (forward references by name are not supported for backrefs,
    /// only for subroutine calls).
    named_group_positions: NamedGroups,
    ignore_numbered_groups: bool,
    next_group_index: usize,
    /// Total number of capture groups in the pattern, set after the first resolution pass.
    /// Used to guard `backrefs` BitSet insertions against unreasonably large group numbers.
    total_groups: usize,
    /// The first backreference group number that exceeds `total_groups`, if any. Such numbers
    /// are not inserted into `backrefs` to avoid allocating memory proportional to the raw value.
    out_of_range_backref: Option<usize>,
    /// The number of the most recently opened capture group, mirroring the old single-pass
    /// parser's `curr_group` counter. It is updated each time a `Group` node is entered
    /// during `resolve_capture_group_targets`, and never decremented, so that a relative
    /// backref at any position sees the same value the old parser would have had at that
    /// point in its left-to-right scan.
    curr_group: usize,
}

impl Resolver {
    // resolving is a two step process.
    // On pass 1, we resolve named and numbered groups.
    // On pass 2, we resolve subroutine calls.
    // We can't do this in one pass because they may be forward references by name, and we don't know what index
    // that named group will be assigned yet in pass 1
    // It is okay if a backref or subroutine call is to a capture group index which doesn't exist
    // - the analyzer will detect it
    fn resolve_groups(&mut self, expr: &mut Expr) {
        if let Expr::AstNode(AstNode::AstGroup { .. }, _) = expr {
            // Move the AstNode out of `expr` by swapping in a sentinel, freeing the borrow so
            // we can destructure and move the inner Box<Expr> without cloning.
            let node = core::mem::replace(expr, Expr::Empty);
            if let Expr::AstNode(AstNode::AstGroup { name, inner }, ix) = node {
                let group_index = if let Some(name) = name {
                    self.named_groups
                        .insert(name.clone(), self.next_group_index);
                    self.named_group_positions.insert(name, ix);
                    Some(self.next_group_index)
                } else if !self.ignore_numbered_groups {
                    Some(self.next_group_index)
                } else {
                    None
                };

                if group_index.is_some() {
                    self.next_group_index += 1;
                }

                let mut inner = inner;
                self.resolve_groups(&mut inner);

                // If we have a group index, create a capturing group
                // If not, just use the resolved inner expression directly
                *expr = if group_index.is_some() {
                    Expr::Group(Arc::new(*inner))
                } else {
                    *inner // Use the inner expression directly
                };
            }
        } else if !expr.is_leaf_node() {
            // recursively resolve in inner expressions
            for child in expr.children_iter_mut() {
                self.resolve_groups(child);
            }
        }
    }

    /// Resolve a `CaptureGroupTarget` to a concrete group index using the groups seen so far.
    ///
    /// `backref_ix` should be `Some(ix)` for backreferences, which enforces that a by-name
    /// target must have been defined *before* the backref's position in the pattern (forward
    /// named backrefs are not permitted). Pass `None` for subroutine calls and backref-exists
    /// conditions, which do support forward references by name.
    ///
    /// Returns `None` if the target cannot be resolved (unknown name, forward named backref,
    /// or a relative offset that underflows before group 1).
    fn resolve_target(
        &self,
        target: &CaptureGroupTarget,
        backref_ix: Option<usize>,
    ) -> Option<usize> {
        match target {
            CaptureGroupTarget::ByNumber(group_index) => Some(*group_index),
            CaptureGroupTarget::Relative(relative_group) => {
                let relative_group = *relative_group;
                self.curr_group.checked_add_signed(if relative_group < 0 {
                    relative_group + 1
                } else {
                    relative_group
                })
            }
            CaptureGroupTarget::ByName(name) => {
                let group = self.named_groups.get(name.as_str()).copied();
                // For backrefs, reject forward references: the group's opening `(` must appear
                // before the backref in the pattern.
                if let (Some(backref_ix), Some(group_ix)) = (
                    backref_ix,
                    self.named_group_positions.get(name.as_str()).copied(),
                ) {
                    if group_ix >= backref_ix {
                        return None;
                    }
                }
                group
            }
        }
    }

    fn resolve_capture_group_targets(&mut self, expr: &mut Expr) -> Result<()> {
        if let Expr::AstNode(astnode, ix) = expr {
            match astnode {
                AstNode::AstGroup { .. } => unreachable!(),
                AstNode::Backref {
                    target,
                    casei,
                    relative_recursion_level,
                } => {
                    // TODO: if multiple groups with the same name, ideally we would
                    // return an Expr::Alt with all the backrefs to be compatible with Oniguruma.
                    // but it is unclear how the priorities should work, it may need to be a separate VM instruction
                    // so for now, we could emit a FeatureNotSupported CompileError
                    // but as this is a parser, we could emit a new AstNode representing a multigroup backref.
                    // Then when the analyzer sees it, it could return the error
                    if let Some(resolved_group) = self.resolve_target(target, Some(*ix)) {
                        if resolved_group > self.total_groups {
                            // Don't insert into the BitSet: an out-of-range number could allocate
                            // memory proportional to its raw value. Record the first occurrence so
                            // the analyzer can report it.
                            self.out_of_range_backref.get_or_insert(resolved_group);
                        } else {
                            self.backrefs.insert(resolved_group);
                        }
                        *expr = if let Some(relative_recursion_level) = *relative_recursion_level {
                            Expr::BackrefWithRelativeRecursionLevel {
                                group: resolved_group,
                                casei: *casei,
                                relative_level: relative_recursion_level,
                            }
                        } else {
                            Expr::Backref {
                                group: resolved_group,
                                casei: *casei,
                            }
                        };
                    } else {
                        // Distinguish: a name that exists but is forward → InvalidGroupNameBackref;
                        // anything else (unknown name, bad relative offset) → InvalidBackref.
                        let err = if let CaptureGroupTarget::ByName(name) = target {
                            if self.named_groups.contains_key(name.as_str()) {
                                ParseError::InvalidGroupNameBackref(name.clone())
                            } else {
                                ParseError::InvalidBackref
                            }
                        } else {
                            ParseError::InvalidBackref
                        };
                        return Err(Error::ParseError(*ix, err));
                    }
                }
                AstNode::SubroutineCall(target) => {
                    // TODO: if multiple groups with this name, don't resolve
                    // and instead just leave it as an AstNode for the analyzer to complain about
                    if let Some(resolved_group) = self.resolve_target(target, None) {
                        *expr = Expr::SubroutineCall(resolved_group);
                    }
                }
                AstNode::BackrefExistsCondition {
                    target,
                    relative_recursion_level,
                } => {
                    if let Some(resolved_group) = self.resolve_target(target, None) {
                        *expr = Expr::BackrefExistsCondition {
                            group: resolved_group,
                            relative_recursion_level: *relative_recursion_level,
                        };
                    } else {
                        return Err(Error::ParseError(*ix, ParseError::InvalidBackref));
                    }
                }
            }
        } else {
            match expr {
                Expr::Group(_) => {
                    self.next_group_index += 1;
                    self.curr_group = self.next_group_index - 1;
                }
                Expr::Backref { group, .. }
                | Expr::BackrefWithRelativeRecursionLevel { group, .. } => {
                    self.backrefs.insert(*group);
                }
                _ => {}
            }
            if !expr.is_leaf_node() {
                // recursively resolve in inner expressions
                for child in expr.children_iter_mut() {
                    self.resolve_capture_group_targets(child)?;
                }
            }
        }
        Ok(())
    }
}

// return (ix, value)
pub(crate) fn parse_usize(s: &str, ix: usize) -> Option<(usize, usize)> {
    let mut end = ix;
    while end < s.len() && s.as_bytes()[end].is_ascii_digit() {
        end += 1;
    }
    s[ix..end].parse::<usize>().ok().map(|val| (end, val))
}

#[derive(Debug, PartialEq)]
pub(crate) struct ParsedId<'a> {
    pub id: &'a str,
    pub relative: Option<isize>,
    pub skip: usize,
}

/// Attempts to parse an identifier, optionally followed by a relative number between the
/// specified opening and closing delimiters.  On success, returns
/// `Some((id, relative, skip))`, where `skip` is how much of the string was used.
pub(crate) fn parse_id<'a>(
    s: &'a str,
    open: &'_ str,
    close: &'_ str,
    allow_relative: bool,
) -> Option<ParsedId<'a>> {
    debug_assert!(!close.starts_with(is_id_char));

    if !s.starts_with(open) || s.len() <= open.len() + close.len() {
        return None;
    }

    let id_start = open.len();
    let mut iter = s[id_start..].char_indices().peekable();
    let after_id = iter.find(|(_, ch)| !is_id_char(*ch));

    let id_len = match after_id.map(|(i, _)| i) {
        Some(id_len) => id_len,
        None if close.is_empty() => s.len(),
        _ => 0,
    };

    let id_end = id_start + id_len;
    if id_len > 0 && s[id_end..].starts_with(close) {
        return Some(ParsedId {
            id: &s[id_start..id_end],
            relative: None,
            skip: id_end + close.len(),
        });
    } else if !allow_relative {
        return None;
    }
    let relative_sign = s.as_bytes()[id_end];
    if relative_sign == b'+' || relative_sign == b'-' {
        if let Some((end, relative_amount)) = parse_usize(s, id_end + 1) {
            if s[end..].starts_with(close) {
                if relative_amount == 0 && id_len == 0 {
                    return None;
                }
                let relative_amount_signed = if relative_sign == b'-' {
                    -(relative_amount as isize)
                } else {
                    relative_amount as isize
                };
                return Some(ParsedId {
                    id: &s[id_start..id_end],
                    relative: Some(relative_amount_signed),
                    skip: end + close.len(),
                });
            }
        }
    }
    None
}

/// Parse a group name that allows any characters except the closing delimiter.
/// The name must be non-empty. Returns `(name, total_bytes_consumed)` on success.
pub(crate) fn parse_unrestricted_name<'a>(
    s: &'a str,
    open: &str,
    close: &str,
) -> Option<(&'a str, usize)> {
    if !s.starts_with(open) || s.len() <= open.len() + close.len() {
        return None;
    }
    let after_open = &s[open.len()..];
    // Find the close delimiter
    let end = after_open.find(close)?;
    if end == 0 {
        return None; // empty name not allowed
    }
    let name = &after_open[..end];
    Some((name, open.len() + end + close.len()))
}

fn is_id_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn is_hex_digit(b: u8) -> bool {
    b.is_ascii_digit() || (b'a' <= (b | 32) && (b | 32) <= b'f')
}

pub(crate) fn make_literal(s: &str) -> Expr {
    make_literal_case_insensitive(s, false)
}

pub(crate) fn make_literal_case_insensitive(s: &str, case_insensitive: bool) -> Expr {
    Expr::Literal {
        val: String::from(s),
        casei: case_insensitive,
    }
}

pub(crate) fn make_ast_group(inner: Expr, name: Option<String>, ix: usize) -> Expr {
    Expr::AstNode(
        AstNode::AstGroup {
            name,
            inner: Box::new(inner),
        },
        ix,
    )
}

#[cfg(test)]
pub(crate) fn make_group(inner: Expr) -> Expr {
    Expr::Group(Arc::new(inner))
}

fn remap_unicode_property_if_necessary(
    property_name: &str,
    unicode_flag: bool,
    in_class: bool,
) -> String {
    let (mut neg, prop) = if let Some(p) = property_name.strip_prefix(r"\p{") {
        (false, p)
    } else if let Some(p) = property_name.strip_prefix(r"\P{") {
        (true, p)
    } else {
        return String::from(property_name);
    };
    if let Some(mut p) = prop.strip_suffix('}') {
        // Check if the property name starts with ^ for negation (Oniguruma syntax)
        // e.g., \p{^Emoji} should become \P{emoji}
        if let Some(stripped) = p.strip_prefix('^') {
            neg = !neg;
            p = stripped;
        }
        let apply_wrap = |inner: &str, in_c: bool, n: bool| {
            if in_c {
                if n {
                    format!("[^{}]", inner)
                } else {
                    inner.to_string()
                }
            } else if n {
                format!("[^{}]", inner)
            } else {
                format!("[{}]", inner)
            }
        };
        match (p.to_lowercase().as_ref(), unicode_flag, in_class, neg) {
            ("alnum", true, in_c, n) => apply_wrap(r"\p{alpha}\p{digit}", in_c, n),
            ("alnum", false, in_c, n) => apply_wrap(r"[:alnum:]", in_c, n),
            ("blank", true, in_c, n) => apply_wrap(r"\p{Zs}\x09", in_c, n),
            ("blank", false, in_c, n) => apply_wrap(r"\t ", in_c, n),
            ("cntrl", true, in_c, n) => apply_wrap(r"\x00-\x1F\x7F-\x9F", in_c, n),
            ("cntrl", false, in_c, n) => apply_wrap(r"[:cntrl:]", in_c, n),
            (r"word", true, _, false) => r"\w".to_string(),
            (r"word", true, _, true) => r"\W".to_string(),
            (r"word", false, false, false) => r"[_[:alnum:]]".to_string(),
            (r"word", false, false, true) => r"[^_[:alnum:]]".to_string(),
            (r"word", false, true, false) => r"_[:alnum:]".to_string(),
            (r"word", false, true, true) => r"[^_[:alnum:]]".to_string(),
            // graph - graphical/visible characters (excludes whitespace, control chars, etc)
            // For Unicode mode, we exclude White_Space and all Other category (\p{C})
            // Note: When used inside a character class, we return the full negated class
            // because we can't easily represent a negation as positive content
            (r"graph", true, _, false) => r"[^\p{White_Space}\p{C}]".to_string(),
            (r"graph", true, _, true) => r"[\p{White_Space}\p{C}]".to_string(),
            (r"graph", false, false, false) => r"[[:graph:]]".to_string(),
            (r"graph", false, false, true) => r"[^[:graph:]]".to_string(),
            (r"graph", false, true, false) => r"[:graph:]".to_string(),
            (r"graph", false, true, true) => r"[^[:graph:]]".to_string(),
            // print - printable characters (graph + space separator)
            // For Unicode mode: exclude Other category and non-space whitespace
            // Note: When used inside a character class, we return the full negated class
            (r"print", true, _, false) => r"[^\p{C}\t\n\v\f\r]".to_string(),
            (r"print", true, _, true) => r"[\p{C}\t\n\v\f\r]".to_string(),
            (r"print", false, false, false) => r"[[:print:]]".to_string(),
            (r"print", false, false, true) => r"[^[:print:]]".to_string(),
            (r"print", false, true, false) => r"[:print:]".to_string(),
            (r"print", false, true, true) => r"[^[:print:]]".to_string(),
            // cs - surrogates (only applies to UTF-16, never matches in UTF-8)
            ("cs", _, _, false) => r"\P{any}".to_string(),
            ("cs", _, _, true) => r"\p{any}".to_string(),
            // For other properties, convert to lowercase and reconstruct
            _ => {
                let lower = p.to_lowercase();
                if neg {
                    format!(r"\P{{{}}}", lower)
                } else {
                    format!(r"\p{{{}}}", lower)
                }
            }
        }
    } else {
        String::from(property_name)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/fancy-regex/unit/src/parse/tests_1799.rs"));
