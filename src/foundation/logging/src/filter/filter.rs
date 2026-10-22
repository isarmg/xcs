use alloc::{borrow::ToOwned, string::ToString, vec::Vec};
use core::{fmt, mem};

use log::{LevelFilter, Metadata, Record};

use super::enabled;
use super::parse_spec;
use super::parser::ParseResult;
use super::Directive;
use super::FilterOp;
use super::ParseError;

/// A builder for a log filter.
///
/// It can be used to parse a set of directives from a string before building
/// a [`Filter`] instance.
///
/// ## Example
///
/// ```
/// # use std::env;
/// use logging::filter::Builder;
///
/// let mut builder = Builder::new();
///
/// // Parse a logging filter from an environment variable.
/// if let Ok(rust_log) = env::var("RUST_LOG") {
///     builder.parse(&rust_log);
/// }
///
/// let filter = builder.build();
/// ```
pub struct Builder {
    directives: Vec<Directive>,
    filter: Option<FilterOp>,
    built: bool,
}

impl Builder {
    /// Initializes the filter builder with defaults.
    pub fn new() -> Builder {
        Builder {
            directives: Vec::new(),
            filter: None,
            built: false,
        }
    }

    /// Initializes the filter builder from an environment.
    pub fn from_env(env: &str) -> Builder {
        let mut builder = Builder::new();

        if let Ok(s) = std::env::var(env) {
            builder.parse(&s);
        }

        builder
    }

    /// Insert the directive replacing any directive with the same name.
    fn insert_directive(&mut self, mut directive: Directive) {
        if let Some(pos) = self
            .directives
            .iter()
            .position(|d| d.name == directive.name)
        {
            mem::swap(&mut self.directives[pos], &mut directive);
        } else {
            self.directives.push(directive);
        }
    }

    /// Adds a directive to the filter for a specific module.
    pub fn filter_module(&mut self, module: &str, level: LevelFilter) -> &mut Self {
        self.filter(Some(module), level)
    }

    /// Adds a directive to the filter for all modules.
    pub fn filter_level(&mut self, level: LevelFilter) -> &mut Self {
        self.filter(None, level)
    }

    /// Adds a directive to the filter.
    ///
    /// The given module (if any) will log at most the specified level provided.
    /// If no module is provided then the filter will apply to all log messages.
    pub fn filter(&mut self, module: Option<&str>, level: LevelFilter) -> &mut Self {
        self.insert_directive(Directive {
            name: module.map(|s| s.to_owned()),
            level,
        });
        self
    }

    /// Parses the directives string.
    ///
    /// See the [Enabling Logging] section for more details.
    ///
    /// [Enabling Logging]: ../index.html#enabling-logging
    pub fn parse(&mut self, filters: &str) -> &mut Self {
        #![allow(clippy::print_stderr)] // compatibility

        let ParseResult {
            directives,
            filter,
            errors,
        } = parse_spec(filters);

        for error in errors {
            eprintln!("warning: {error}, ignoring it");
        }

        self.filter = filter;

        for directive in directives {
            self.insert_directive(directive);
        }
        self
    }

    /// Parses the directive string, returning an error if the given directive string is invalid.
    ///
    /// See the [Enabling Logging] section for more details.
    ///
    /// [Enabling Logging]: ../index.html#enabling-logging
    pub fn try_parse(&mut self, filters: &str) -> Result<&mut Self, ParseError> {
        let (directives, filter) = parse_spec(filters).ok()?;

        self.filter = filter;

        for directive in directives {
            self.insert_directive(directive);
        }
        Ok(self)
    }

    /// Build a log filter.
    pub fn build(&mut self) -> Filter {
        assert!(!self.built, "attempt to re-use consumed builder");
        self.built = true;

        let mut directives = Vec::new();
        if self.directives.is_empty() {
            // Adds the default filter if none exist
            directives.push(Directive {
                name: None,
                level: LevelFilter::Error,
            });
        } else {
            // Consume directives.
            directives = mem::take(&mut self.directives);
            // Sort the directives by length of their name, this allows a
            // little more efficient lookup at runtime.
            directives.sort_by(|a, b| {
                let alen = a.name.as_ref().map(|a| a.len()).unwrap_or(0);
                let blen = b.name.as_ref().map(|b| b.len()).unwrap_or(0);
                alen.cmp(&blen)
            });
        }

        Filter {
            directives: mem::take(&mut directives),
            filter: mem::take(&mut self.filter),
        }
    }
}

impl Default for Builder {
    fn default() -> Self {
        Builder::new()
    }
}

impl fmt::Debug for Builder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.built {
            f.debug_struct("Filter").field("built", &true).finish()
        } else {
            f.debug_struct("Filter")
                .field("filter", &self.filter)
                .field("directives", &self.directives)
                .finish()
        }
    }
}

/// A log filter.
///
/// This struct can be used to determine whether or not a log record
/// should be written to the output.
/// Use the [`Builder`] type to parse and construct a `Filter`.
///
/// [`Builder`]: struct.Builder.html
#[derive(Clone)]
pub struct Filter {
    directives: Vec<Directive>,
    filter: Option<FilterOp>,
}

impl Filter {
    /// Returns the maximum `LevelFilter` that this filter instance is
    /// configured to output.
    ///
    /// # Example
    ///
    /// ```rust
    /// use log::LevelFilter;
    /// use logging::filter::Builder;
    ///
    /// let mut builder = Builder::new();
    /// builder.filter(Some("module1"), LevelFilter::Info);
    /// builder.filter(Some("module2"), LevelFilter::Error);
    ///
    /// let filter = builder.build();
    /// assert_eq!(filter.filter(), LevelFilter::Info);
    /// ```
    pub fn filter(&self) -> LevelFilter {
        self.directives
            .iter()
            .map(|d| d.level)
            .max()
            .unwrap_or(LevelFilter::Off)
    }

    /// Checks if this record matches the configured filter.
    pub fn matches(&self, record: &Record<'_>) -> bool {
        if !self.enabled(record.metadata()) {
            return false;
        }

        if let Some(filter) = self.filter.as_ref() {
            if !filter.is_match(&record.args().to_string()) {
                return false;
            }
        }

        true
    }

    /// Determines if a log message with the specified metadata would be logged.
    pub fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        let level = metadata.level();
        let target = metadata.target();

        enabled(&self.directives, level, target)
    }
}

impl fmt::Debug for Filter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Filter")
            .field("filter", &self.filter)
            .field("directives", &self.directives)
            .finish()
    }
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/logging/filter/unit/src/filter/tests_262.rs"
));
