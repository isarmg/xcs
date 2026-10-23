use crate::validation::LazyInstance;
use std::{borrow::Cow, sync::Arc};

use crate::validation::{
    compiler,
    error::ValidationError,
    keywords::CompilationResult,
    options::PatternEngineOptions,
    paths::{LazyEvaluationPath, LazyLocation, Location, RefTracker},
    regex::{
        analyze_pattern, contains_ecma_whitespace, PatternOptimization, RegexEngine, RegexError,
        RegexFailureReason,
    },
    types::JsonType,
    validator::{Validate, ValidationContext},
    Json, Node,
};
use json_codec::{Map, Value};

/// Validator for patterns that are simple prefixes (optimized path).
pub(crate) struct PrefixPatternValidator {
    prefix: String,
    pattern: String,
    location: Location,
}

impl<F: Json> Validate<F> for PrefixPatternValidator {
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(item) = instance.as_string() {
            item.starts_with(&self.prefix)
        } else {
            true
        }
    }

    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if let Some(item) = instance.as_string() {
            if !item.starts_with(&self.prefix) {
                return Err(ValidationError::pattern(
                    self.location.clone(),
                    crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance.lazy_value(),
                    self.pattern.clone(),
                ));
            }
        }
        Ok(())
    }
}

/// Validator for patterns that are exact-match anchored patterns.
pub(crate) struct ExactPatternValidator {
    exact: String,
    pattern: String,
    location: Location,
}

impl<F: Json> Validate<F> for ExactPatternValidator {
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(item) = instance.as_string() {
            item.as_ref() == self.exact
        } else {
            true
        }
    }

    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if let Some(item) = instance.as_string() {
            if item.as_ref() != self.exact {
                return Err(ValidationError::pattern(
                    self.location.clone(),
                    crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance.lazy_value(),
                    self.pattern.clone(),
                ));
            }
        }
        Ok(())
    }
}

/// Validator for `^(a|b|c)$` alternation patterns (linear scan).
pub(crate) struct AlternationPatternValidator {
    alternatives: Vec<String>,
    pattern: String,
    location: Location,
}

impl<F: Json> Validate<F> for AlternationPatternValidator {
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(item) = instance.as_string() {
            self.alternatives
                .iter()
                .any(|a| a.as_str() == item.as_ref())
        } else {
            true
        }
    }

    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if let Some(item) = instance.as_string() {
            if !self
                .alternatives
                .iter()
                .any(|a| a.as_str() == item.as_ref())
            {
                return Err(ValidationError::pattern(
                    self.location.clone(),
                    crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance.lazy_value(),
                    self.pattern.clone(),
                ));
            }
        }
        Ok(())
    }
}

/// Validator for `^\S*$` — rejects any string containing ECMA-262 whitespace.
pub(crate) struct NoWhitespacePatternValidator {
    pattern: String,
    location: Location,
}

impl<F: Json> Validate<F> for NoWhitespacePatternValidator {
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(item) = instance.as_string() {
            !contains_ecma_whitespace(&item)
        } else {
            true
        }
    }

    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if let Some(item) = instance.as_string() {
            if contains_ecma_whitespace(&item) {
                return Err(ValidationError::pattern(
                    self.location.clone(),
                    crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance.lazy_value(),
                    self.pattern.clone(),
                ));
            }
        }
        Ok(())
    }
}

pub(crate) struct PatternValidator<R> {
    regex: Arc<R>,
    /// Original schema pattern, kept for error messages. The compiled `regex` stores the
    /// ECMA->Rust translated form (e.g. `\S` expanded into a verbose class), which is unreadable.
    pattern: String,
    location: Location,
}

impl<R: RegexEngine, F: Json> Validate<F> for PatternValidator<R> {
    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if let Some(item) = instance.as_string() {
            match self.regex.is_match(&item) {
                Ok(is_match) => {
                    if !is_match {
                        return Err(ValidationError::pattern(
                            self.location.clone(),
                            crate::validation::paths::capture_evaluation_path(
                                tracker,
                                &self.location,
                            ),
                            location.into(),
                            instance.lazy_value(),
                            self.pattern.clone(),
                        ));
                    }
                }
                Err(e) => {
                    let pattern = &self.pattern;
                    let tracker =
                        crate::validation::paths::capture_evaluation_path(tracker, &self.location);
                    return Err(match e.into_failure_reason() {
                        RegexFailureReason::FancyRegex(error) => ValidationError::backtrack_limit(
                            self.location.clone(),
                            tracker,
                            location.into(),
                            instance.lazy_value(),
                            error,
                        ),
                        RegexFailureReason::Panicked => ValidationError::regex_engine_failure(
                            self.location.clone(),
                            tracker,
                            location.into(),
                            instance.lazy_value(),
                            format!("Regex engine failed to evaluate pattern '{pattern}'"),
                        ),
                    });
                }
            }
        }
        Ok(())
    }

    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(item) = instance.as_string() {
            return self.regex.is_match(&item).unwrap_or(false);
        }
        true
    }
}

#[inline]
pub(crate) fn compile<'a, F: Json>(
    ctx: &compiler::Context<F>,
    _: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a, F>> {
    if let Value::String(item) = schema {
        // Try literal optimizations before compiling a full regex.
        match analyze_pattern(item) {
            Some(PatternOptimization::Exact(exact)) => {
                return Some(Ok(Box::new(ExactPatternValidator {
                    exact,
                    pattern: item.clone(),
                    location: ctx.location().join("pattern"),
                })));
            }
            Some(PatternOptimization::Prefix(prefix)) => {
                return Some(Ok(Box::new(PrefixPatternValidator {
                    prefix,
                    pattern: item.clone(),
                    location: ctx.location().join("pattern"),
                })));
            }
            Some(PatternOptimization::Alternation(alternatives)) => {
                return Some(Ok(Box::new(AlternationPatternValidator {
                    alternatives,
                    pattern: item.clone(),
                    location: ctx.location().join("pattern"),
                })));
            }
            Some(PatternOptimization::NoWhitespace) => {
                return Some(Ok(Box::new(NoWhitespacePatternValidator {
                    pattern: item.clone(),
                    location: ctx.location().join("pattern"),
                })));
            }
            None => {}
        }
        // Fall back to regex compilation
        match ctx.config().pattern_options() {
            PatternEngineOptions::FancyRegex { .. } => {
                let Ok(regex) = ctx.get_or_compile_regex(item) else {
                    return Some(Err(invalid_regex(ctx, schema)));
                };
                Some(Ok(Box::new(PatternValidator {
                    regex,
                    pattern: item.clone(),
                    location: ctx.location().join("pattern"),
                })))
            }
            PatternEngineOptions::Regex { .. } => {
                let Ok(regex) = ctx.get_or_compile_standard_regex(item) else {
                    return Some(Err(invalid_regex(ctx, schema)));
                };
                Some(Ok(Box::new(PatternValidator {
                    regex,
                    pattern: item.clone(),
                    location: ctx.location().join("pattern"),
                })))
            }
        }
    } else {
        let location = ctx.location().join("pattern");
        Some(Err(ValidationError::single_type_error(
            location.clone(),
            location,
            Location::new(),
            LazyInstance::Ready(Cow::Borrowed(schema)),
            JsonType::String,
        )))
    }
}

fn invalid_regex<'a, F: Json>(
    ctx: &compiler::Context<F>,
    schema: &'a Value,
) -> ValidationError<'a> {
    ValidationError::format(
        ctx.location().join("pattern"),
        LazyEvaluationPath::SameAsSchemaPath,
        Location::new(),
        LazyInstance::Ready(Cow::Borrowed(schema)),
        "regex",
    )
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../../test/foundation/schema-validation/upstream/jsonschema/unit/src/keywords/pattern/tests_325.rs"
));
