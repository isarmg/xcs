/*!
This module provides a regular expression printer for `Hir`.
*/

use core::fmt;

use crate::{
    hir::{
        self,
        visitor::{self, Visitor},
        Hir, HirKind,
    },
    is_meta_character,
};

/// A builder for constructing a printer.
///
/// Note that since a printer doesn't have any configuration knobs, this type
/// remains unexported.
#[derive(Clone, Debug)]
struct PrinterBuilder {
    _priv: (),
}

impl Default for PrinterBuilder {
    fn default() -> PrinterBuilder {
        PrinterBuilder::new()
    }
}

impl PrinterBuilder {
    fn new() -> PrinterBuilder {
        PrinterBuilder { _priv: () }
    }

    fn build(&self) -> Printer {
        Printer { _priv: () }
    }
}

/// A printer for a regular expression's high-level intermediate
/// representation.
///
/// A printer converts a high-level intermediate representation (HIR) to a
/// regular expression pattern string. This particular printer uses constant
/// stack space and heap space proportional to the size of the HIR.
///
/// Since this printer is only using the HIR, the pattern it prints will likely
/// not resemble the original pattern at all. For example, a pattern like
/// `\pL` will have its entire class written out.
///
/// The purpose of this printer is to provide a means to mutate an HIR and then
/// build a regular expression from the result of that mutation. (A regex
/// library could provide a constructor from this HIR explicitly, but that
/// creates an unnecessary public coupling between the regex library and this
/// specific HIR representation.)
#[derive(Debug)]
pub struct Printer {
    _priv: (),
}

impl Printer {
    /// Create a new printer.
    pub fn new() -> Printer {
        PrinterBuilder::new().build()
    }

    /// Print the given `Ast` to the given writer. The writer must implement
    /// `fmt::Write`. Typical implementations of `fmt::Write` that can be used
    /// here are a `fmt::Formatter` (which is available in `fmt::Display`
    /// implementations) or a `&mut String`.
    pub fn print<W: fmt::Write>(&mut self, hir: &Hir, wtr: W) -> fmt::Result {
        visitor::visit(hir, Writer { wtr })
    }
}

#[derive(Debug)]
struct Writer<W> {
    wtr: W,
}

impl<W: fmt::Write> Visitor for Writer<W> {
    type Output = ();
    type Err = fmt::Error;

    fn finish(self) -> fmt::Result {
        Ok(())
    }

    fn visit_pre(&mut self, hir: &Hir) -> fmt::Result {
        match *hir.kind() {
            HirKind::Empty => {
                // Technically an empty sub-expression could be "printed" by
                // just ignoring it, but in practice, you could have a
                // repetition operator attached to an empty expression, and you
                // really need something in the concrete syntax to make that
                // work as you'd expect.
                self.wtr.write_str(r"(?:)")?;
            }
            // Repetition operators are strictly suffix oriented.
            HirKind::Repetition(_) => {}
            HirKind::Literal(hir::Literal(ref bytes)) => {
                // See the comment on the 'Concat' and 'Alternation' case below
                // for why we put parens here. Literals are, conceptually,
                // a special case of concatenation where each element is a
                // character. The HIR flattens this into a Box<[u8]>, but we
                // still need to treat it like a concatenation for correct
                // printing. As a special case, we don't write parens if there
                // is only one character. One character means there is no
                // concat so we don't need parens. Adding parens would still be
                // correct, but we drop them here because it tends to create
                // rather noisy regexes even in simple cases.
                let result = core::str::from_utf8(bytes);
                let len = result.map_or(bytes.len(), |s| s.chars().count());
                if len > 1 {
                    self.wtr.write_str(r"(?:")?;
                }
                match result {
                    Ok(string) => {
                        for c in string.chars() {
                            self.write_literal_char(c)?;
                        }
                    }
                    Err(_) => {
                        for &b in bytes.iter() {
                            self.write_literal_byte(b)?;
                        }
                    }
                }
                if len > 1 {
                    self.wtr.write_str(r")")?;
                }
            }
            HirKind::Class(hir::Class::Unicode(ref cls)) => {
                if cls.ranges().is_empty() {
                    return self.wtr.write_str("[a&&b]");
                }
                self.wtr.write_str("[")?;
                for range in cls.iter() {
                    if range.start() == range.end() {
                        self.write_literal_char(range.start())?;
                    } else if u32::from(range.start()) + 1
                        == u32::from(range.end())
                    {
                        self.write_literal_char(range.start())?;
                        self.write_literal_char(range.end())?;
                    } else {
                        self.write_literal_char(range.start())?;
                        self.wtr.write_str("-")?;
                        self.write_literal_char(range.end())?;
                    }
                }
                self.wtr.write_str("]")?;
            }
            HirKind::Class(hir::Class::Bytes(ref cls)) => {
                if cls.ranges().is_empty() {
                    return self.wtr.write_str("[a&&b]");
                }
                self.wtr.write_str("(?-u:[")?;
                for range in cls.iter() {
                    if range.start() == range.end() {
                        self.write_literal_class_byte(range.start())?;
                    } else if range.start() + 1 == range.end() {
                        self.write_literal_class_byte(range.start())?;
                        self.write_literal_class_byte(range.end())?;
                    } else {
                        self.write_literal_class_byte(range.start())?;
                        self.wtr.write_str("-")?;
                        self.write_literal_class_byte(range.end())?;
                    }
                }
                self.wtr.write_str("])")?;
            }
            HirKind::Look(ref look) => match *look {
                hir::Look::Start => {
                    self.wtr.write_str(r"\A")?;
                }
                hir::Look::End => {
                    self.wtr.write_str(r"\z")?;
                }
                hir::Look::StartLF => {
                    self.wtr.write_str("(?m:^)")?;
                }
                hir::Look::EndLF => {
                    self.wtr.write_str("(?m:$)")?;
                }
                hir::Look::StartCRLF => {
                    self.wtr.write_str("(?mR:^)")?;
                }
                hir::Look::EndCRLF => {
                    self.wtr.write_str("(?mR:$)")?;
                }
                hir::Look::WordAscii => {
                    self.wtr.write_str(r"(?-u:\b)")?;
                }
                hir::Look::WordAsciiNegate => {
                    self.wtr.write_str(r"(?-u:\B)")?;
                }
                hir::Look::WordUnicode => {
                    self.wtr.write_str(r"\b")?;
                }
                hir::Look::WordUnicodeNegate => {
                    self.wtr.write_str(r"\B")?;
                }
                hir::Look::WordStartAscii => {
                    self.wtr.write_str(r"(?-u:\b{start})")?;
                }
                hir::Look::WordEndAscii => {
                    self.wtr.write_str(r"(?-u:\b{end})")?;
                }
                hir::Look::WordStartUnicode => {
                    self.wtr.write_str(r"\b{start}")?;
                }
                hir::Look::WordEndUnicode => {
                    self.wtr.write_str(r"\b{end}")?;
                }
                hir::Look::WordStartHalfAscii => {
                    self.wtr.write_str(r"(?-u:\b{start-half})")?;
                }
                hir::Look::WordEndHalfAscii => {
                    self.wtr.write_str(r"(?-u:\b{end-half})")?;
                }
                hir::Look::WordStartHalfUnicode => {
                    self.wtr.write_str(r"\b{start-half}")?;
                }
                hir::Look::WordEndHalfUnicode => {
                    self.wtr.write_str(r"\b{end-half}")?;
                }
            },
            HirKind::Capture(hir::Capture { ref name, .. }) => {
                self.wtr.write_str("(")?;
                if let Some(ref name) = *name {
                    write!(self.wtr, "?P<{name}>")?;
                }
            }
            // Why do this? Wrapping concats and alts in non-capturing groups
            // is not *always* necessary, but is sometimes necessary. For
            // example, 'concat(a, alt(b, c))' should be written as 'a(?:b|c)'
            // and not 'ab|c'. The former is clearly the intended meaning, but
            // the latter is actually 'alt(concat(a, b), c)'.
            //
            // It would be possible to only group these things in cases where
            // it's strictly necessary, but it requires knowing the parent
            // expression. And since this technique is simpler and always
            // correct, we take this route. More to the point, it is a non-goal
            // of an HIR printer to show a nice easy-to-read regex. Indeed,
            // its construction forbids it from doing so. Therefore, inserting
            // extra groups where they aren't necessary is perfectly okay.
            HirKind::Concat(_) | HirKind::Alternation(_) => {
                self.wtr.write_str(r"(?:")?;
            }
        }
        Ok(())
    }

    fn visit_post(&mut self, hir: &Hir) -> fmt::Result {
        match *hir.kind() {
            // Handled during visit_pre
            HirKind::Empty
            | HirKind::Literal(_)
            | HirKind::Class(_)
            | HirKind::Look(_) => {}
            HirKind::Repetition(ref x) => {
                match (x.min, x.max) {
                    (0, Some(1)) => {
                        self.wtr.write_str("?")?;
                    }
                    (0, None) => {
                        self.wtr.write_str("*")?;
                    }
                    (1, None) => {
                        self.wtr.write_str("+")?;
                    }
                    (1, Some(1)) => {
                        // 'a{1}' and 'a{1}?' are exactly equivalent to 'a'.
                        return Ok(());
                    }
                    (m, None) => {
                        write!(self.wtr, "{{{m},}}")?;
                    }
                    (m, Some(n)) if m == n => {
                        write!(self.wtr, "{{{m}}}")?;
                        // a{m} and a{m}? are always exactly equivalent.
                        return Ok(());
                    }
                    (m, Some(n)) => {
                        write!(self.wtr, "{{{m},{n}}}")?;
                    }
                }
                if !x.greedy {
                    self.wtr.write_str("?")?;
                }
            }
            HirKind::Capture(_)
            | HirKind::Concat(_)
            | HirKind::Alternation(_) => {
                self.wtr.write_str(r")")?;
            }
        }
        Ok(())
    }

    fn visit_alternation_in(&mut self) -> fmt::Result {
        self.wtr.write_str("|")
    }
}

impl<W: fmt::Write> Writer<W> {
    fn write_literal_char(&mut self, c: char) -> fmt::Result {
        if is_meta_character(c) {
            self.wtr.write_str("\\")?;
        }
        self.wtr.write_char(c)
    }

    fn write_literal_byte(&mut self, b: u8) -> fmt::Result {
        if b <= 0x7F && !b.is_ascii_control() && !b.is_ascii_whitespace() {
            self.write_literal_char(char::try_from(b).unwrap())
        } else {
            write!(self.wtr, "(?-u:\\x{b:02X})")
        }
    }

    fn write_literal_class_byte(&mut self, b: u8) -> fmt::Result {
        if b <= 0x7F && !b.is_ascii_control() && !b.is_ascii_whitespace() {
            self.write_literal_char(char::try_from(b).unwrap())
        } else {
            write!(self.wtr, "\\x{b:02X}")
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/regex-syntax/unit/src/hir/print/tests_333.rs"));
