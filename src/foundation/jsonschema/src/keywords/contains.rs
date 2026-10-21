use crate::{
    compiler,
    error::ValidationError,
    evaluation::{Annotations, ChildList, ErrorDescription},
    keywords::CompilationResult,
    node::SchemaNode,
    paths::{LazyLocation, Location, RefTracker},
    validator::{EvaluationResult, Validate, ValidationContext},
    Array, Draft, Json, Node, SerdeJson,
};
use referencing::{Uri, Vocabulary};
use serde_json::{Map, Value};
use std::sync::Arc;

use super::helpers::map_get_u64;

/// The keyword a `contains` failure is reported at.
struct Site {
    location: Location,
    absolute_location: Option<Arc<Uri<String>>>,
}

impl Site {
    fn new<F: Json>(ctx: &compiler::Context<F>, keyword: &str) -> Self {
        let location = ctx.location().join(keyword);
        Site {
            absolute_location: ctx.absolute_location(&location),
            location,
        }
    }

    #[cold]
    #[inline(never)]
    fn error<'i, F: Json>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
    ) -> ValidationError<'i> {
        ValidationError::contains(
            self.location.clone(),
            crate::paths::capture_evaluation_path(tracker, &self.location),
            location.into(),
            instance.lazy_value(),
        )
        .with_absolute_keyword_location(self.absolute_location.clone())
    }
}

pub(crate) struct ContainsValidator<F: Json = SerdeJson> {
    node: SchemaNode<F>,
}

impl ContainsValidator {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(
        ctx: &compiler::Context<F>,
        schema: &'a Value,
    ) -> CompilationResult<'a, F> {
        let ctx = ctx.new_at_location("contains");
        Ok(Box::new(ContainsValidator {
            node: compiler::compile(&ctx, ctx.as_resource_ref(schema))?,
        }))
    }
}

impl<F: Json> Validate<F> for ContainsValidator<F> {
    fn is_valid(&self, instance: &F::Node<'_>, ctx: &mut ValidationContext) -> bool {
        if let Some(array) = instance.as_array() {
            array.elements().any(|item| self.node.is_valid(&item, ctx))
        } else {
            true
        }
    }

    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if let Some(array) = instance.as_array() {
            if array.elements().any(|item| self.node.is_valid(&item, ctx)) {
                return Ok(());
            }
            let loc = self.node.location();
            Err(ValidationError::contains(
                loc.clone(),
                crate::paths::capture_evaluation_path(tracker, loc),
                location.into(),
                instance.lazy_value(),
            ))
        } else {
            Ok(())
        }
    }

    fn evaluate(
        &self,
        instance: &F::Node<'_>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if let Some(array) = instance.as_array() {
            let mut results = ChildList::default();
            let mut indices = Vec::with_capacity(array.len());
            for (idx, item) in array.elements().enumerate() {
                let path = location.push(idx);
                let result = self
                    .node
                    .evaluate_instance_below(&item, &path, tracker, ctx);
                if result.valid {
                    indices.push(idx);
                    results.push(&mut ctx.arena, result);
                }
            }
            if indices.is_empty() {
                let loc = self.node.location();
                let eval_path = crate::paths::capture_evaluation_path(tracker, loc);
                EvaluationResult::Invalid {
                    errors: vec![ErrorDescription::from_validation_error(
                        &ValidationError::contains(
                            loc.clone(),
                            eval_path,
                            location.into(),
                            instance.lazy_value(),
                        ),
                    )],
                    children: ChildList::default(),
                    annotations: None,
                }
            } else {
                EvaluationResult::Valid {
                    annotations: Some(Annotations::new(Value::from(indices))),
                    children: results,
                }
            }
        } else {
            let mut result = EvaluationResult::valid_empty();
            result.annotate(Annotations::new(Value::Array(Vec::new())));
            result
        }
    }
}

/// `minContains` validation. Used only if there is no `maxContains` present.
///
/// Docs: <https://json-schema.org/draft/2019-09/json-schema-validation.html#rfc.section.6.4.5>
pub(crate) struct MinContainsValidator<F: Json = SerdeJson> {
    node: SchemaNode<F>,
    min_contains: u64,
    min: Site,
}

impl MinContainsValidator {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(
        ctx: &compiler::Context<F>,
        schema: &'a Value,
        min_contains: u64,
    ) -> CompilationResult<'a, F> {
        let min = Site::new(ctx, "minContains");
        let ctx = ctx.new_at_location("minContains");
        Ok(Box::new(MinContainsValidator {
            node: compiler::compile(&ctx, ctx.as_resource_ref(schema))?,
            min_contains,
            min,
        }))
    }
}

impl<F: Json> Validate<F> for MinContainsValidator<F> {
    fn is_valid(&self, instance: &F::Node<'_>, ctx: &mut ValidationContext) -> bool {
        if let Some(array) = instance.as_array() {
            let mut matches = 0;
            for item in array.elements() {
                if self
                    .node
                    .validators()
                    .all(|validator| validator.is_valid(&item, ctx))
                {
                    matches += 1;
                    if matches >= self.min_contains {
                        return true;
                    }
                }
            }
            self.min_contains == 0
        } else {
            true
        }
    }

    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if let Some(array) = instance.as_array() {
            let mut matches = 0;
            for item in array.elements() {
                if self
                    .node
                    .validators()
                    .all(|validator| validator.is_valid(&item, ctx))
                {
                    matches += 1;
                    if matches >= self.min_contains {
                        return Ok(());
                    }
                }
            }
            if self.min_contains > 0 {
                Err(self.min.error::<F>(instance, location, tracker))
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }
}

/// `maxContains` validation. Used only if there is no `minContains` present.
///
/// Docs: <https://json-schema.org/draft/2019-09/json-schema-validation.html#rfc.section.6.4.4>
pub(crate) struct MaxContainsValidator<F: Json = SerdeJson> {
    node: SchemaNode<F>,
    max_contains: u64,
    max: Site,
}

impl MaxContainsValidator {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(
        ctx: &compiler::Context<F>,
        schema: &'a Value,
        max_contains: u64,
    ) -> CompilationResult<'a, F> {
        let max = Site::new(ctx, "maxContains");
        let ctx = ctx.new_at_location("maxContains");
        Ok(Box::new(MaxContainsValidator {
            node: compiler::compile(&ctx, ctx.as_resource_ref(schema))?,
            max_contains,
            max,
        }))
    }
}

impl<F: Json> Validate<F> for MaxContainsValidator<F> {
    fn is_valid(&self, instance: &F::Node<'_>, ctx: &mut ValidationContext) -> bool {
        if let Some(array) = instance.as_array() {
            let mut matches = 0;
            for item in array.elements() {
                if self
                    .node
                    .validators()
                    .all(|validator| validator.is_valid(&item, ctx))
                {
                    matches += 1;
                    if matches > self.max_contains {
                        return false;
                    }
                }
            }
            matches != 0
        } else {
            true
        }
    }

    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if let Some(array) = instance.as_array() {
            let mut matches = 0;
            for item in array.elements() {
                if self
                    .node
                    .validators()
                    .all(|validator| validator.is_valid(&item, ctx))
                {
                    matches += 1;
                    if matches > self.max_contains {
                        return Err(self.max.error::<F>(instance, location, tracker));
                    }
                }
            }
            if matches > 0 {
                Ok(())
            } else {
                Err(self.max.error::<F>(instance, location, tracker))
            }
        } else {
            Ok(())
        }
    }
}

/// `maxContains` & `minContains` validation combined.
///
/// Docs:
///   `maxContains` - <https://json-schema.org/draft/2019-09/json-schema-validation.html#rfc.section.6.4.4>
///   `minContains` - <https://json-schema.org/draft/2019-09/json-schema-validation.html#rfc.section.6.4.5>
pub(crate) struct MinMaxContainsValidator<F: Json = SerdeJson> {
    node: SchemaNode<F>,
    min_contains: u64,
    max_contains: u64,
    // Both bounds report against their own keyword, which the shared subschema location cannot name.
    min: Site,
    max: Site,
}

impl MinMaxContainsValidator {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(
        ctx: &compiler::Context<F>,
        schema: &'a Value,
        min_contains: u64,
        max_contains: u64,
    ) -> CompilationResult<'a, F> {
        let min = Site::new(ctx, "minContains");
        let max = Site::new(ctx, "maxContains");
        let ctx = ctx.new_at_location("contains");
        Ok(Box::new(MinMaxContainsValidator {
            node: compiler::compile(&ctx, ctx.as_resource_ref(schema))?,
            min_contains,
            max_contains,
            min,
            max,
        }))
    }
}

impl<F: Json> Validate<F> for MinMaxContainsValidator<F> {
    fn is_valid(&self, instance: &F::Node<'_>, ctx: &mut ValidationContext) -> bool {
        if let Some(array) = instance.as_array() {
            let mut matches = 0;
            for item in array.elements() {
                if self
                    .node
                    .validators()
                    .all(|validator| validator.is_valid(&item, ctx))
                {
                    matches += 1;
                    if matches > self.max_contains {
                        return false;
                    }
                }
            }
            matches <= self.max_contains && matches >= self.min_contains
        } else {
            true
        }
    }

    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if let Some(array) = instance.as_array() {
            let mut matches = 0;
            for item in array.elements() {
                if self
                    .node
                    .validators()
                    .all(|validator| validator.is_valid(&item, ctx))
                {
                    matches += 1;
                    if matches > self.max_contains {
                        return Err(self.max.error::<F>(instance, location, tracker));
                    }
                }
            }
            if matches < self.min_contains {
                Err(self.min.error::<F>(instance, location, tracker))
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }
}

#[inline]
pub(crate) fn compile<'a, F: Json>(
    ctx: &compiler::Context<F>,
    parent: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a, F>> {
    match ctx.draft() {
        Draft::Draft4 | Draft::Draft6 | Draft::Draft7 => {
            Some(ContainsValidator::compile(ctx, schema))
        }
        Draft::Draft201909 | Draft::Draft202012 => compile_contains(ctx, parent, schema),
        _ => None,
    }
}

#[inline]
fn compile_contains<'a, F: Json>(
    ctx: &compiler::Context<F>,
    parent: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a, F>> {
    // `minContains` and `maxContains` belong to the validation vocabulary
    if !ctx.has_vocabulary(&Vocabulary::Validation) {
        return Some(ContainsValidator::compile(ctx, schema));
    }
    let min_contains = match map_get_u64(parent, ctx, "minContains").transpose() {
        Ok(n) => n,
        Err(err) => return Some(Err(err)),
    };
    let max_contains = match map_get_u64(parent, ctx, "maxContains").transpose() {
        Ok(n) => n,
        Err(err) => return Some(Err(err)),
    };

    match (min_contains, max_contains) {
        (Some(min), Some(max)) => Some(MinMaxContainsValidator::compile(ctx, schema, min, max)),
        (Some(min), None) => Some(MinContainsValidator::compile(ctx, schema, min)),
        (None, Some(max)) => Some(MaxContainsValidator::compile(ctx, schema, max)),
        (None, None) => Some(ContainsValidator::compile(ctx, schema)),
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/jsonschema/unit/src/keywords/contains/tests_439.rs"));
