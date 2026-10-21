pub(crate) use crate::filter::directive::{FilterVec, ParseError, StaticDirective};
use crate::filter::{
    directive::{DirectiveSet, Match},
    env::{field, FieldMap},
    level::LevelFilter,
};
use alloc::{borrow::ToOwned, string::String, vec::Vec};
use std::{cmp::Ordering, fmt, iter::FromIterator, str::FromStr};
use tracing_core::{span, Level, Metadata};

/// A single filtering directive.
// TODO(eliza): add a builder for programmatically constructing directives?
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(docsrs, doc(cfg(feature = "env-filter")))]
pub struct Directive {
    in_span: Option<String>,
    fields: Vec<field::Match>,
    pub(crate) target: Option<String>,
    pub(crate) level: LevelFilter,
}

/// A set of dynamic filtering directives.
pub(super) type Dynamics = DirectiveSet<Directive>;

/// A set of static filtering directives.
pub(super) type Statics = DirectiveSet<StaticDirective>;

pub(crate) type CallsiteMatcher = MatchSet<field::CallsiteMatch>;
pub(crate) type SpanMatcher = MatchSet<field::SpanMatch>;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MatchSet<T> {
    field_matches: FilterVec<T>,
    base_level: LevelFilter,
}

impl Directive {
    pub(super) fn has_name(&self) -> bool {
        self.in_span.is_some()
    }

    pub(super) fn has_fields(&self) -> bool {
        !self.fields.is_empty()
    }

    pub(super) fn to_static(&self) -> Option<StaticDirective> {
        if !self.is_static() {
            return None;
        }

        // TODO(eliza): these strings are all immutable; we should consider
        // `Arc`ing them to make this more efficient...
        let field_names = self.fields.iter().map(field::Match::name).collect();

        Some(StaticDirective::new(
            self.target.clone(),
            field_names,
            self.level,
        ))
    }

    fn is_static(&self) -> bool {
        !self.has_name() && !self.fields.iter().any(field::Match::has_value)
    }

    pub(super) fn is_dynamic(&self) -> bool {
        self.has_name() || self.has_fields()
    }

    pub(crate) fn field_matcher(&self, meta: &Metadata<'_>) -> Option<field::CallsiteMatch> {
        let fieldset = meta.fields();
        let fields = self
            .fields
            .iter()
            .filter_map(
                |field::Match {
                     ref name,
                     ref value,
                 }| {
                    if let Some(field) = fieldset.field(name) {
                        let value = value.as_ref().cloned()?;
                        Some(Ok((field, value)))
                    } else {
                        Some(Err(()))
                    }
                },
            )
            .collect::<Result<FieldMap<_>, ()>>()
            .ok()?;
        Some(field::CallsiteMatch {
            fields,
            level: self.level,
        })
    }

    pub(super) fn make_tables(
        directives: impl IntoIterator<Item = Directive>,
    ) -> (Dynamics, Statics) {
        // TODO(eliza): this could be made more efficient...
        let (dyns, stats): (Vec<Directive>, Vec<Directive>) =
            directives.into_iter().partition(Directive::is_dynamic);
        let statics = stats
            .into_iter()
            .filter_map(|d| d.to_static())
            .chain(dyns.iter().filter_map(Directive::to_static))
            .collect();
        (Dynamics::from_iter(dyns), statics)
    }

    pub(super) fn deregexify(&mut self) {
        for field in &mut self.fields {
            field.value = match field.value.take() {
                Some(field::ValueMatch::Pat(pat)) => {
                    Some(field::ValueMatch::Debug(pat.into_debug_match()))
                }
                x => x,
            }
        }
    }

    pub(super) fn parse(from: &str, regex: bool) -> Result<Self, ParseError> {
        let mut cur = Self {
            level: LevelFilter::TRACE,
            target: None,
            in_span: None,
            fields: Vec::new(),
        };

        #[derive(Debug)]
        enum ParseState {
            Start,
            LevelOrTarget { start: usize },
            Span { span_start: usize },
            Field { field_start: usize },
            Fields,
            Target,
            Level { level_start: usize },
            Complete,
        }

        use ParseState::*;
        let mut state = Start;
        for (i, c) in from.trim().char_indices() {
            state = match (state, c) {
                (Start, '[') => Span { span_start: i + 1 },
                (Start, c) if !['-', ':', '_'].contains(&c) && !c.is_alphanumeric() => {
                    return Err(ParseError::new())
                }
                (Start, _) => LevelOrTarget { start: i },
                (LevelOrTarget { start }, '=') => {
                    cur.target = Some(from[start..i].to_owned());
                    Level { level_start: i + 1 }
                }
                (LevelOrTarget { start }, '[') => {
                    cur.target = Some(from[start..i].to_owned());
                    Span { span_start: i + 1 }
                }
                (LevelOrTarget { start }, ',') => {
                    let (level, target) = match &from[start..] {
                        "" => (LevelFilter::TRACE, None),
                        level_or_target => match LevelFilter::from_str(level_or_target) {
                            Ok(level) => (level, None),
                            Err(_) => (LevelFilter::TRACE, Some(level_or_target.to_owned())),
                        },
                    };

                    cur.level = level;
                    cur.target = target;
                    Complete
                }
                (state @ LevelOrTarget { .. }, _) => state,
                (Target, '=') => Level { level_start: i + 1 },
                (Span { span_start }, ']') => {
                    cur.in_span = Some(from[span_start..i].to_owned());
                    Target
                }
                (Span { span_start }, '{') => {
                    cur.in_span = match &from[span_start..i] {
                        "" => None,
                        _ => Some(from[span_start..i].to_owned()),
                    };
                    Field { field_start: i + 1 }
                }
                (state @ Span { .. }, _) => state,
                (Field { field_start }, '}') => {
                    cur.fields.push(match &from[field_start..i] {
                        "" => return Err(ParseError::new()),
                        field => field::Match::parse(field, regex)?,
                    });
                    Fields
                }
                (Field { field_start }, ',') => {
                    cur.fields.push(match &from[field_start..i] {
                        "" => return Err(ParseError::new()),
                        field => field::Match::parse(field, regex)?,
                    });
                    Field { field_start: i + 1 }
                }
                (state @ Field { .. }, _) => state,
                (Fields, ']') => Target,
                (Level { level_start }, ',') => {
                    cur.level = match &from[level_start..i] {
                        "" => LevelFilter::TRACE,
                        level => LevelFilter::from_str(level)?,
                    };
                    Complete
                }
                (state @ Level { .. }, _) => state,
                _ => return Err(ParseError::new()),
            };
        }

        match state {
            LevelOrTarget { start } => {
                let (level, target) = match &from[start..] {
                    "" => (LevelFilter::TRACE, None),
                    level_or_target => match LevelFilter::from_str(level_or_target) {
                        Ok(level) => (level, None),
                        // Setting the target without the level enables every level for that target
                        Err(_) => (LevelFilter::TRACE, Some(level_or_target.to_owned())),
                    },
                };

                cur.level = level;
                cur.target = target;
            }
            Level { level_start } => {
                cur.level = match &from[level_start..] {
                    "" => LevelFilter::TRACE,
                    level => LevelFilter::from_str(level)?,
                };
            }
            Target | Complete => {}
            _ => return Err(ParseError::new()),
        };

        Ok(cur)
    }
}

impl Match for Directive {
    fn cares_about(&self, meta: &Metadata<'_>) -> bool {
        // Does this directive have a target filter, and does it match the
        // metadata's target?
        if let Some(ref target) = self.target {
            if !meta.target().starts_with(&target[..]) {
                return false;
            }
        }

        // Do we have a name filter, and does it match the metadata's name?
        // TODO(eliza): put name globbing here?
        if let Some(ref name) = self.in_span {
            if name != meta.name() {
                return false;
            }
        }

        // Does the metadata define all the fields that this directive cares about?
        let actual_fields = meta.fields();
        for expected_field in &self.fields {
            // Does the actual field set (from the metadata) contain this field?
            if actual_fields.field(&expected_field.name).is_none() {
                return false;
            }
        }

        true
    }

    fn level(&self) -> &LevelFilter {
        &self.level
    }
}

impl FromStr for Directive {
    type Err = ParseError;
    fn from_str(from: &str) -> Result<Self, Self::Err> {
        Directive::parse(from, true)
    }
}

impl Default for Directive {
    fn default() -> Self {
        Directive {
            level: LevelFilter::OFF,
            target: None,
            in_span: None,
            fields: Vec::new(),
        }
    }
}

impl PartialOrd for Directive {
    fn partial_cmp(&self, other: &Directive) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Directive {
    fn cmp(&self, other: &Directive) -> Ordering {
        // We attempt to order directives by how "specific" they are. This
        // ensures that we try the most specific directives first when
        // attempting to match a piece of metadata.

        // First, we compare based on whether a target is specified, and the
        // lengths of those targets if both have targets.
        let ordering = self
            .target
            .as_ref()
            .map(String::len)
            .cmp(&other.target.as_ref().map(String::len))
            // Next compare based on the presence of span names.
            .then_with(|| self.in_span.is_some().cmp(&other.in_span.is_some()))
            // Then we compare how many fields are defined by each
            // directive.
            .then_with(|| self.fields.len().cmp(&other.fields.len()))
            // Finally, we fall back to lexicographical ordering if the directives are
            // equally specific. Although this is no longer semantically important,
            // we need to define a total ordering to determine the directive's place
            // in the BTreeMap.
            .then_with(|| {
                self.target
                    .cmp(&other.target)
                    .then_with(|| self.in_span.cmp(&other.in_span))
                    .then_with(|| self.fields[..].cmp(&other.fields[..]))
            })
            .reverse();

        #[cfg(debug_assertions)]
        {
            if ordering == Ordering::Equal {
                debug_assert_eq!(
                    self.target, other.target,
                    "invariant violated: Ordering::Equal must imply a.target == b.target"
                );
                debug_assert_eq!(
                    self.in_span, other.in_span,
                    "invariant violated: Ordering::Equal must imply a.in_span == b.in_span"
                );
                debug_assert_eq!(
                    self.fields, other.fields,
                    "invariant violated: Ordering::Equal must imply a.fields == b.fields"
                );
            }
        }

        ordering
    }
}

impl fmt::Display for Directive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut wrote_any = false;
        if let Some(ref target) = self.target {
            fmt::Display::fmt(target, f)?;
            wrote_any = true;
        }

        if self.in_span.is_some() || !self.fields.is_empty() {
            f.write_str("[")?;

            if let Some(ref span) = self.in_span {
                fmt::Display::fmt(span, f)?;
            }

            let mut fields = self.fields.iter();
            if let Some(field) = fields.next() {
                write!(f, "{{{}", field)?;
                for field in fields {
                    write!(f, ",{}", field)?;
                }
                f.write_str("}")?;
            }

            f.write_str("]")?;
            wrote_any = true;
        }

        if wrote_any {
            f.write_str("=")?;
        }

        fmt::Display::fmt(&self.level, f)
    }
}

impl From<LevelFilter> for Directive {
    fn from(level: LevelFilter) -> Self {
        Self {
            level,
            ..Self::default()
        }
    }
}

impl From<Level> for Directive {
    fn from(level: Level) -> Self {
        LevelFilter::from_level(level).into()
    }
}

// === impl Dynamics ===

impl Dynamics {
    pub(crate) fn matcher(&self, metadata: &Metadata<'_>) -> Option<CallsiteMatcher> {
        let mut base_level = None;
        let field_matches = self
            .directives_for(metadata)
            .filter_map(|d| {
                if let Some(f) = d.field_matcher(metadata) {
                    return Some(f);
                }
                match base_level {
                    Some(ref b) if d.level > *b => base_level = Some(d.level),
                    None => base_level = Some(d.level),
                    _ => {}
                }
                None
            })
            .collect();

        if let Some(base_level) = base_level {
            Some(CallsiteMatcher {
                field_matches,
                base_level,
            })
        } else if !field_matches.is_empty() {
            Some(CallsiteMatcher {
                field_matches,
                base_level: base_level.unwrap_or(LevelFilter::OFF),
            })
        } else {
            None
        }
    }

    pub(crate) fn has_value_filters(&self) -> bool {
        self.directives()
            .any(|d| d.fields.iter().any(|f| f.value.is_some()))
    }
}

// ===== impl DynamicMatch =====

impl CallsiteMatcher {
    /// Create a new `SpanMatch` for a given instance of the matched callsite.
    pub(crate) fn to_span_match(&self, attrs: &span::Attributes<'_>) -> SpanMatcher {
        let field_matches = self
            .field_matches
            .iter()
            .map(|m| {
                let m = m.to_span_match();
                attrs.record(&mut m.visitor());
                m
            })
            .collect();
        SpanMatcher {
            field_matches,
            base_level: self.base_level,
        }
    }
}

impl SpanMatcher {
    /// Returns the level currently enabled for this callsite.
    pub(crate) fn level(&self) -> LevelFilter {
        self.field_matches
            .iter()
            .filter_map(field::SpanMatch::filter)
            .max()
            .unwrap_or(self.base_level)
    }

    pub(crate) fn record_update(&self, record: &span::Record<'_>) {
        for m in &self.field_matches {
            record.record(&mut m.visitor())
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tracing-subscriber/unit/src/filter/env/directive/test_482.rs"));
