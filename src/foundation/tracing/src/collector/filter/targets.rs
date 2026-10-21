//! A [filter] that enables or disables spans and events based on their [target] and [level].
//!
//! See [`Targets`] for details.
//!
//! [target]: tracing::primitives::Metadata::target
//! [level]: tracing::primitives::Level
//! [filter]: crate::collector::layer#filtering-with-layers

use crate::collector::{
    filter::{
        directive::{DirectiveSet, ParseError, StaticDirective},
        LevelFilter,
    },
    layer,
};
use alloc::string::String;
use core::{
    fmt,
    iter::{Extend, FilterMap, FromIterator},
    slice,
    str::FromStr,
};
use tracing::primitives::{Interest, Level, Metadata, Subscriber};

/// A filter that enables or disables spans and events based on their [target]
/// and [level].
///
/// Targets are typically equal to the Rust module path of the code where the
/// span or event was recorded, although they may be overridden.
///
/// This type can be used for both [per-layer filtering][plf] (using its
/// [`Filter`] implementation) and [global filtering][global] (using its
/// [`Layer`] implementation).
///
/// See the [documentation on filtering with layers][filtering] for details.
///
/// # Filtering With `Targets`
///
/// A `Targets` filter consists of one or more [target] prefixes, paired with
/// [`LevelFilter`]s. If a span or event's [target] begins with one of those
/// prefixes, and its [level] is at or below the [`LevelFilter`] enabled for
/// that prefix, then the span or event will be enabled.
///
/// This is similar to the behavior implemented by the [`env_logger` crate] in
/// the `log` ecosystem.
///
/// The [`EnvFilter`] type also provided by this crate is very similar to `Targets`,
/// but is capable of a more sophisticated form of filtering where events may
/// also be enabled or disabled based on the span they are recorded in.
/// `Targets` can be thought of as a lighter-weight form of [`EnvFilter`] that
/// can be used instead when this dynamic filtering is not required.
///
/// # Examples
///
/// A `Targets` filter can be constructed by programmatically adding targets and
/// levels to enable:
///
/// ```
/// use tracing::collector::{filter, prelude::*};
/// use tracing::primitives::Level;
///
/// let filter = filter::Targets::new()
///     // Enable the `INFO` level for anything in `my_crate`
///     .with_target("my_crate", Level::INFO)
///     // Enable the `DEBUG` level for a specific module.
///     .with_target("my_crate::collector::interesting_module", Level::DEBUG);
///
/// // Build a new subscriber with the `fmt` layer using the `Targets`
/// // filter we constructed above.
/// tracing::collector::registry()
///     .with(tracing::collector::fmt::layer())
///     .with(filter)
///     .init();
/// ```
///
/// [`LevelFilter::OFF`] can be used to disable a particular target:
/// ```
/// use tracing::collector::filter::{Targets, LevelFilter};
/// use tracing::primitives::Level;
///
/// let filter = Targets::new()
///     .with_target("my_crate", Level::INFO)
///     // Disable all traces from `annoying_module`.
///     .with_target("my_crate::collector::annoying_module", LevelFilter::OFF);
/// # drop(filter);
/// ```
///
/// Alternatively, `Targets` implements [`std::str::FromStr`], allowing it to be
/// parsed from a comma-delimited list of `target=level` pairs. For example:
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use tracing::collector::filter;
/// use tracing::primitives::Level;
///
/// let filter = "my_crate=info,my_crate::collector::interesting_module=trace,other_crate=debug"
///     .parse::<filter::Targets>()?;
///
/// // The parsed filter is identical to a filter constructed using `with_target`:
/// assert_eq!(
///     filter,
///     filter::Targets::new()
///         .with_target("my_crate", Level::INFO)
///         .with_target("my_crate::collector::interesting_module", Level::TRACE)
///         .with_target("other_crate", Level::DEBUG)
/// );
/// # Ok(()) }
/// ```
///
/// This is particularly useful when the list of enabled targets is configurable
/// by the user at runtime.
///
/// The `Targets` filter can be used as a [per-layer filter][plf] *and* as a
/// [global filter][global]:
///
/// ```rust
/// use tracing::collector::{
///     fmt,
///     filter::{Targets, LevelFilter},
///     prelude::*,
/// };
/// use tracing::primitives::Level;
/// use std::{sync::Arc, fs::File};
/// # fn docs() -> Result<(), Box<dyn std::error::Error>> {
///
/// // A layer that logs events to stdout using the human-readable "pretty"
/// // format.
/// let stdout_log = fmt::layer().pretty();
///
/// // A layer that logs events to a file, using the JSON format.
/// let file = File::create("debug_log.json")?;
/// let debug_log = fmt::layer()
///     .with_writer(Arc::new(file))
///     .json();
///
/// tracing::collector::registry()
///     // Only log INFO and above to stdout, unless the span or event
///     // has the `my_crate::collector::cool_module` target prefix.
///     .with(stdout_log
///         .with_filter(
///             Targets::default()
///                 .with_target("my_crate::collector::cool_module", Level::DEBUG)
///                 .with_default(Level::INFO)
///        )
///     )
///     // Log everything enabled by the global filter to `debug_log.json`.
///     .with(debug_log)
///     // Configure a global filter for the whole subscriber stack. This will
///     // control what spans and events are recorded by both the `debug_log`
///     // and the `stdout_log` layers, and `stdout_log` will *additionally* be
///     // filtered by its per-layer filter.
///     .with(
///         Targets::default()
///             .with_target("my_crate", Level::TRACE)
///             .with_target("other_crate", Level::INFO)
///             .with_target("other_crate::collector::annoying_module", LevelFilter::OFF)
///             .with_target("third_crate", Level::DEBUG)
///     ).init();
/// # Ok(()) }
///```
///
/// [target]: tracing::primitives::Metadata::target
/// [level]: tracing::primitives::Level
/// [`Filter`]: crate::collector::layer::Filter
/// [`Layer`]: crate::collector::layer::Layer
/// [plf]: crate::collector::layer#per-layer-filtering
/// [global]: crate::collector::layer#global-filtering
/// [filtering]: crate::collector::layer#filtering-with-layers
/// [`env_logger` crate]: https://docs.rs/env_logger/0.9.0/env_logger/index.html#enabling-logging
/// [`EnvFilter`]: crate::collector::filter::EnvFilter
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Targets(DirectiveSet<StaticDirective>);

impl Targets {
    /// Returns a new `Targets` filter.
    ///
    /// This filter will enable no targets. Call [`with_target`] or [`with_targets`]
    /// to add enabled targets, and [`with_default`] to change the default level
    /// enabled for spans and events that didn't match any of the provided targets.
    ///
    /// [`with_target`]: Targets::with_target
    /// [`with_targets`]: Targets::with_targets
    /// [`with_default`]: Targets::with_default
    pub fn new() -> Self {
        Self::default()
    }

    /// Enables spans and events with [target]s starting with the provided target
    /// prefix if they are at or below the provided [`LevelFilter`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing::collector::filter;
    /// use tracing::primitives::Level;
    ///
    /// let filter = filter::Targets::new()
    ///     // Enable the `INFO` level for anything in `my_crate`
    ///     .with_target("my_crate", Level::INFO)
    ///     // Enable the `DEBUG` level for a specific module.
    ///     .with_target("my_crate::collector::interesting_module", Level::DEBUG);
    /// # drop(filter);
    /// ```
    ///
    /// [`LevelFilter::OFF`] can be used to disable a particular target:
    /// ```
    /// use tracing::collector::filter::{Targets, LevelFilter};
    /// use tracing::primitives::Level;
    ///
    /// let filter = Targets::new()
    ///     .with_target("my_crate", Level::INFO)
    ///     // Disable all traces from `annoying_module`.
    ///     .with_target("my_crate::collector::interesting_module", LevelFilter::OFF);
    /// # drop(filter);
    /// ```
    ///
    /// [target]: tracing::primitives::Metadata::target
    pub fn with_target(mut self, target: impl Into<String>, level: impl Into<LevelFilter>) -> Self {
        self.0.add(StaticDirective::new(
            Some(target.into()),
            Default::default(),
            level.into(),
        ));
        self
    }
    /// Adds [target]s from an iterator of [target]-[`LevelFilter`] pairs to this filter.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing::collector::filter;
    /// use tracing::primitives::Level;
    ///
    /// let filter = filter::Targets::new()
    ///     .with_targets(vec![
    ///         ("my_crate", Level::INFO),
    ///         ("my_crate::collector::some_module", Level::DEBUG),
    ///         ("my_crate::collector::other_module::cool_stuff", Level::TRACE),
    ///         ("other_crate", Level::WARN)
    ///     ]);
    /// # drop(filter);
    /// ```
    ///
    /// [`LevelFilter::OFF`] can be used to disable a particular target:
    /// ```
    /// use tracing::collector::filter::{Targets, LevelFilter};
    /// use tracing::primitives::Level;
    ///
    /// let filter = Targets::new()
    ///     .with_target("my_crate", Level::INFO)
    ///     // Disable all traces from `annoying_module`.
    ///     .with_target("my_crate::collector::interesting_module", LevelFilter::OFF);
    /// # drop(filter);
    /// ```
    ///
    /// [target]: tracing::primitives::Metadata::target
    pub fn with_targets<T, L>(mut self, targets: impl IntoIterator<Item = (T, L)>) -> Self
    where
        String: From<T>,
        LevelFilter: From<L>,
    {
        self.extend(targets);
        self
    }

    /// Sets the default level to enable for spans and events whose targets did
    /// not match any of the configured prefixes.
    ///
    /// By default, this is [`LevelFilter::OFF`]. This means that spans and
    /// events will only be enabled if they match one of the configured target
    /// prefixes. If this is changed to a different [`LevelFilter`], spans and
    /// events with targets that did not match any of the configured prefixes
    /// will be enabled if their level is at or below the provided level.
    pub fn with_default(mut self, level: impl Into<LevelFilter>) -> Self {
        self.0
            .add(StaticDirective::new(None, Default::default(), level.into()));
        self
    }

    /// Returns the default level for this filter, if one is set.
    ///
    /// The default level is used to filter any spans or events with targets
    /// that do not match any of the configured set of prefixes.
    ///
    /// The default level can be set for a filter either by using
    /// [`with_default`](Self::with_default) or when parsing from a filter string that includes a
    /// level without a target (e.g. `"trace"`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing::collector::filter::{LevelFilter, Targets};
    ///
    /// let filter = Targets::new().with_default(LevelFilter::INFO);
    /// assert_eq!(filter.default_level(), Some(LevelFilter::INFO));
    ///
    /// let filter: Targets = "info".parse().unwrap();
    /// assert_eq!(filter.default_level(), Some(LevelFilter::INFO));
    /// ```
    ///
    /// The default level is `None` if no default is set:
    ///
    /// ```
    /// use tracing::collector::filter::Targets;
    ///
    /// let filter = Targets::new();
    /// assert_eq!(filter.default_level(), None);
    ///
    /// let filter: Targets = "my_crate=info".parse().unwrap();
    /// assert_eq!(filter.default_level(), None);
    /// ```
    ///
    /// Note that an unset default level (`None`) behaves like [`LevelFilter::OFF`] when the filter is
    /// used, but it could also be set explicitly which may be useful to distinguish (such as when
    /// merging multiple `Targets`).
    ///
    /// ```
    /// use tracing::collector::filter::{LevelFilter, Targets};
    ///
    /// let filter = Targets::new().with_default(LevelFilter::OFF);
    /// assert_eq!(filter.default_level(), Some(LevelFilter::OFF));
    ///
    /// let filter: Targets = "off".parse().unwrap();
    /// assert_eq!(filter.default_level(), Some(LevelFilter::OFF));
    /// ```
    pub fn default_level(&self) -> Option<LevelFilter> {
        self.0.directives().find_map(|d| {
            if d.target.is_none() {
                Some(d.level)
            } else {
                None
            }
        })
    }

    /// Returns an iterator over the [target]-[`LevelFilter`] pairs in this filter.
    ///
    /// The order of iteration is undefined.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing::collector::filter::{Targets, LevelFilter};
    /// use tracing::primitives::Level;
    ///
    /// let filter = Targets::new()
    ///     .with_target("my_crate", Level::INFO)
    ///     .with_target("my_crate::collector::interesting_module", Level::DEBUG);
    ///
    /// let mut targets: Vec<_> = filter.iter().collect();
    /// targets.sort();
    ///
    /// assert_eq!(targets, vec![
    ///     ("my_crate", LevelFilter::INFO),
    ///     ("my_crate::collector::interesting_module", LevelFilter::DEBUG),
    /// ]);
    /// ```
    ///
    /// [target]: tracing::primitives::Metadata::target
    pub fn iter(&self) -> Iter<'_> {
        self.into_iter()
    }

    #[inline]
    fn interested(&self, metadata: &'static Metadata<'static>) -> Interest {
        if self.0.enabled(metadata) {
            Interest::always()
        } else {
            Interest::never()
        }
    }

    /// Returns whether a [target]-[`Level`] pair would be enabled
    /// by this `Targets`.
    ///
    /// This method can be used with [`module_path!`] from `std` as the target
    /// in order to emulate the behavior of the [`tracing::event!`] and [`tracing::span!`]
    /// macros.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing::collector::filter::{Targets, LevelFilter};
    /// use tracing::primitives::Level;
    ///
    /// let filter = Targets::new()
    ///     .with_target("my_crate", Level::INFO)
    ///     .with_target("my_crate::collector::interesting_module", Level::DEBUG);
    ///
    /// assert!(filter.would_enable("my_crate", &Level::INFO));
    /// assert!(!filter.would_enable("my_crate::collector::interesting_module", &Level::TRACE));
    /// ```
    ///
    /// [target]: tracing::primitives::Metadata::target
    /// [`module_path!`]: std::module_path!
    pub fn would_enable(&self, target: &str, level: &Level) -> bool {
        // "Correct" to call because `Targets` only produces `StaticDirective`'s with NO
        // fields
        self.0.target_enabled(target, level)
    }
}

impl<T, L> Extend<(T, L)> for Targets
where
    T: Into<String>,
    L: Into<LevelFilter>,
{
    fn extend<I: IntoIterator<Item = (T, L)>>(&mut self, iter: I) {
        let iter = iter.into_iter().map(|(target, level)| {
            StaticDirective::new(Some(target.into()), Default::default(), level.into())
        });
        self.0.extend(iter);
    }
}

impl<T, L> FromIterator<(T, L)> for Targets
where
    T: Into<String>,
    L: Into<LevelFilter>,
{
    fn from_iter<I: IntoIterator<Item = (T, L)>>(iter: I) -> Self {
        let mut this = Self::default();
        this.extend(iter);
        this
    }
}

impl FromStr for Targets {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.split(',')
            .map(StaticDirective::from_str)
            .collect::<Result<_, _>>()
            .map(Self)
    }
}

impl<S> layer::Layer<S> for Targets
where
    S: Subscriber,
{
    fn enabled(&self, metadata: &Metadata<'_>, _: layer::Context<'_, S>) -> bool {
        self.0.enabled(metadata)
    }

    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        self.interested(metadata)
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        Some(self.0.max_level)
    }
}

#[cfg(feature = "registry")]
#[cfg_attr(docsrs, doc(cfg(feature = "registry")))]
impl<S> layer::Filter<S> for Targets {
    fn enabled(&self, metadata: &Metadata<'_>, _: &layer::Context<'_, S>) -> bool {
        self.0.enabled(metadata)
    }

    fn callsite_enabled(&self, metadata: &'static Metadata<'static>) -> Interest {
        self.interested(metadata)
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        Some(self.0.max_level)
    }
}

impl IntoIterator for Targets {
    type Item = (String, LevelFilter);

    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter::new(self)
    }
}

impl<'a> IntoIterator for &'a Targets {
    type Item = (&'a str, LevelFilter);

    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self)
    }
}

impl fmt::Display for Targets {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut directives = self.0.directives();
        if let Some(directive) = directives.next() {
            write!(f, "{}", directive)?;
            for directive in directives {
                write!(f, ",{}", directive)?;
            }
        }

        Ok(())
    }
}

/// An owning iterator over the [target]-[level] pairs of a `Targets` filter.
///
/// This struct is created by the `IntoIterator` trait implementation of [`Targets`].
///
/// # Examples
///
/// Merge the targets from one `Targets` with another:
///
/// ```
/// use tracing::collector::filter::Targets;
/// use tracing::primitives::Level;
///
/// let mut filter = Targets::new().with_target("my_crate", Level::INFO);
/// let overrides = Targets::new().with_target("my_crate::collector::interesting_module", Level::DEBUG);
///
/// filter.extend(overrides);
/// # drop(filter);
/// ```
///
/// [target]: tracing::primitives::Metadata::target
/// [level]: tracing::primitives::Level
#[derive(Debug)]
pub struct IntoIter(
    #[allow(clippy::type_complexity)] // alias indirection would probably make this more confusing
    FilterMap<
        <DirectiveSet<StaticDirective> as IntoIterator>::IntoIter,
        fn(StaticDirective) -> Option<(String, LevelFilter)>,
    >,
);

impl IntoIter {
    fn new(targets: Targets) -> Self {
        Self(targets.0.into_iter().filter_map(|directive| {
            let level = directive.level;
            directive.target.map(|target| (target, level))
        }))
    }
}

impl Iterator for IntoIter {
    type Item = (String, LevelFilter);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

/// A borrowing iterator over the [target]-[level] pairs of a `Targets` filter.
///
/// This struct is created by [`iter`] method of [`Targets`], or from the `IntoIterator`
/// implementation for `&Targets`.
///
/// [target]: tracing::primitives::Metadata::target
/// [level]: tracing::primitives::Level
/// [`iter`]: Targets::iter
#[derive(Debug)]
pub struct Iter<'a>(
    FilterMap<
        slice::Iter<'a, StaticDirective>,
        fn(&'a StaticDirective) -> Option<(&'a str, LevelFilter)>,
    >,
);

impl<'a> Iter<'a> {
    fn new(targets: &'a Targets) -> Self {
        Self(targets.0.iter().filter_map(|directive| {
            directive
                .target
                .as_deref()
                .map(|target| (target, directive.level))
        }))
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a str, LevelFilter);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

#[cfg(test)]
include!(core::concat!(
    core::env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/tracing-family/collector/unit/src/filter/targets/tests_595.rs"
));
