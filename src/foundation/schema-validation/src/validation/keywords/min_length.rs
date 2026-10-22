use crate::reference::Uri;
use crate::validation::{
    compiler,
    error::ValidationError,
    evaluation::{absorbed_error_node, ChildList, ErrorDescription},
    keywords::{
        helpers::{fail_on_non_positive_integer, size_limit},
        CompilationResult,
    },
    paths::{LazyLocation, Location, RefTracker},
    validator::{evaluate_assertion, EvaluationResult, Validate, ValidationContext},
    Json, Node,
};
use json_codec::{Map, Value};
use std::sync::Arc;

pub(crate) struct MinLengthValidator {
    limit: u64,
    location: Location,
}

impl<F: Json> Validate<F> for MinLengthValidator {
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(length) = instance.string_length() {
            if length < self.limit {
                return false;
            }
        }
        true
    }

    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if let Some(length) = instance.string_length() {
            if length < self.limit {
                return Err(ValidationError::min_length(
                    self.location.clone(),
                    crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance.lazy_value(),
                    self.limit,
                ));
            }
        }
        Ok(())
    }

    fn evaluate(
        &self,
        instance: &F::Node<'_>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        evaluate_assertion::<F, _>(self, instance, location, tracker, ctx)
    }
}

/// `minLength` and `maxLength` together. A length is a count of code points, so measuring it
/// walks the whole string; two validators walk it twice.
pub(crate) struct LengthRangeValidator {
    minimum: u64,
    maximum: u64,
    min_location: Location,
    max_location: Location,
    max_absolute_location: Option<Arc<Uri<String>>>,
}

impl LengthRangeValidator {
    fn min_error<'i, F: Json>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
    ) -> ValidationError<'i> {
        ValidationError::min_length(
            self.min_location.clone(),
            crate::validation::paths::capture_evaluation_path(tracker, &self.min_location),
            location.into(),
            instance.lazy_value(),
            self.minimum,
        )
    }

    fn max_error<'i, F: Json>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
    ) -> ValidationError<'i> {
        ValidationError::max_length(
            self.max_location.clone(),
            crate::validation::paths::capture_evaluation_path(tracker, &self.max_location),
            location.into(),
            instance.lazy_value(),
            self.maximum,
        )
        .with_absolute_keyword_location(self.max_absolute_location.clone())
    }
}

impl<F: Json> Validate<F> for LengthRangeValidator {
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(length) = instance.string_length() {
            if length < self.minimum || length > self.maximum {
                return false;
            }
        }
        true
    }

    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if let Some(length) = instance.string_length() {
            // `minLength` sorts first
            if length < self.minimum {
                return Err(self.min_error::<F>(instance, location, tracker));
            }
            if length > self.maximum {
                return Err(self.max_error::<F>(instance, location, tracker));
            }
        }
        Ok(())
    }

    fn collect_errors<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
        errors: &mut Vec<ValidationError<'i>>,
    ) {
        if let Some(length) = instance.string_length() {
            // `minLength` above `maxLength` fails both at once, as two validators would
            if length < self.minimum {
                errors.push(self.min_error::<F>(instance, location, tracker));
            }
            if length > self.maximum {
                errors.push(self.max_error::<F>(instance, location, tracker));
            }
        }
    }

    fn evaluate(
        &self,
        instance: &F::Node<'_>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        let Some(length) = instance.string_length() else {
            return EvaluationResult::valid_empty();
        };
        // This node sits at the `minLength` location, so only `maxLength` needs its own node.
        let mut children = ChildList::default();
        if length > self.maximum {
            let error = ErrorDescription::from_validation_error(
                &self.max_error::<F>(instance, location, tracker),
            );
            let child = absorbed_error_node(
                location,
                tracker,
                &self.max_location,
                self.max_absolute_location.as_ref(),
                error,
                ctx,
            );
            children.push(&mut ctx.arena, child);
        }
        let mut result = EvaluationResult::from_children(children);
        if length < self.minimum {
            result.mark_errored(ErrorDescription::from_validation_error(
                &self.min_error::<F>(instance, location, tracker),
            ));
        }
        result
    }
}

#[inline]
pub(crate) fn compile<'a, F: Json>(
    ctx: &compiler::Context<F>,
    parent: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a, F>> {
    let location = ctx.location().join("minLength");
    let Some(minimum) = size_limit(ctx, schema) else {
        return Some(Err(fail_on_non_positive_integer(schema, location)));
    };
    // `max_length::compile` steps aside when it sees this pair
    let maximum = parent
        .get("maxLength")
        .filter(|_| !ctx.is_keyword_overridden("maxLength"))
        .and_then(|max| size_limit(ctx, max));
    if let Some(maximum) = maximum {
        let max_location = ctx.location().join("maxLength");
        return Some(Ok(Box::new(LengthRangeValidator {
            minimum,
            maximum,
            max_absolute_location: ctx.absolute_location(&max_location),
            min_location: location,
            max_location,
        })));
    }
    Some(Ok(Box::new(MinLengthValidator {
        limit: minimum,
        location,
    })))
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/schema-validation/upstream/jsonschema/unit/src/keywords/min_length/tests_222.rs"
));
