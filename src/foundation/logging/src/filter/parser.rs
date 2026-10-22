use alloc::{borrow::ToOwned, format, string::String, vec::Vec};
use core::fmt::{Display, Formatter};

use log::LevelFilter;

use super::Directive;
use super::FilterOp;

#[derive(Default, Debug)]
pub(crate) struct ParseResult {
    pub(crate) directives: Vec<Directive>,
    pub(crate) filter: Option<FilterOp>,
    pub(crate) errors: Vec<String>,
}

impl ParseResult {
    fn add_directive(&mut self, directive: Directive) {
        self.directives.push(directive);
    }

    fn set_filter(&mut self, filter: FilterOp) {
        self.filter = Some(filter);
    }

    fn add_error(&mut self, message: String) {
        self.errors.push(message);
    }

    pub(crate) fn ok(self) -> Result<(Vec<Directive>, Option<FilterOp>), ParseError> {
        let Self {
            directives,
            filter,
            errors,
        } = self;
        if let Some(error) = errors.into_iter().next() {
            Err(ParseError { details: error })
        } else {
            Ok((directives, filter))
        }
    }
}

/// Error during logger directive parsing process.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParseError {
    details: String,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "error parsing logger filter: {}", self.details)
    }
}

impl std::error::Error for ParseError {}

/// Parse a logging specification string (e.g: `crate1,crate2::mod3,crate3::x=error/foo`)
/// and return a vector with log directives.
pub(crate) fn parse_spec(spec: &str) -> ParseResult {
    let mut result = ParseResult::default();

    let mut parts = spec.split('/');
    let mods = parts.next();
    let filter = parts.next();
    if parts.next().is_some() {
        result.add_error(format!("invalid logging spec '{spec}' (too many '/'s)"));
        return result;
    }
    if let Some(m) = mods {
        for s in m.split(',').map(|ss| ss.trim()) {
            if s.is_empty() {
                continue;
            }
            let mut parts = s.split('=');
            let (log_level, name) =
                match (parts.next(), parts.next().map(|s| s.trim()), parts.next()) {
                    (Some(part0), None, None) => {
                        // if the single argument is a log-level string or number,
                        // treat that as a global fallback
                        match part0.parse() {
                            Ok(num) => (num, None),
                            Err(_) => (LevelFilter::max(), Some(part0)),
                        }
                    }
                    (Some(part0), Some(""), None) => (LevelFilter::max(), Some(part0)),
                    (Some(part0), Some(part1), None) => {
                        if let Ok(num) = part1.parse() {
                            (num, Some(part0))
                        } else {
                            result.add_error(format!("invalid logging spec '{part1}'"));
                            continue;
                        }
                    }
                    _ => {
                        result.add_error(format!("invalid logging spec '{s}'"));
                        continue;
                    }
                };

            result.add_directive(Directive {
                name: name.map(|s| s.to_owned()),
                level: log_level,
            });
        }
    }

    if let Some(filter) = filter {
        match FilterOp::new(filter) {
            Ok(filter_op) => result.set_filter(filter_op),
            Err(err) => result.add_error(format!("invalid regex filter - {err}")),
        }
    }

    result
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/logging/filter/unit/src/parser/tests_118.rs"
));
