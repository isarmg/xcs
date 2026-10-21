use crate::{
    compiler,
    error::ValidationError,
    evaluation::{Annotations, ChildList},
    node::SchemaNode,
    paths::{LazyLocation, Location, RefTracker},
    types::JsonType,
    validator::{EvaluationResult, Validate, ValidationContext},
    Array, Json, LazyInstance, Node, JsonValueBackend,
};
use json_codec::{Map, Value};
use std::borrow::Cow;

use super::CompilationResult;

pub(crate) struct PrefixItemsValidator<F: Json = JsonValueBackend> {
    schemas: Vec<SchemaNode<F>>,
}

impl PrefixItemsValidator {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(
        ctx: &compiler::Context<F>,
        items: &'a [Value],
    ) -> CompilationResult<'a, F> {
        let ctx = ctx.new_at_location("prefixItems");
        let mut schemas = Vec::with_capacity(items.len());
        for (idx, item) in items.iter().enumerate() {
            let ctx = ctx.new_at_location(idx);
            let validators = compiler::compile(&ctx, ctx.as_resource_ref(item))?;
            schemas.push(validators);
        }
        Ok(Box::new(PrefixItemsValidator { schemas }))
    }
}

impl<F: Json> Validate<F> for PrefixItemsValidator<F> {
    fn is_valid(&self, instance: &F::Node<'_>, ctx: &mut ValidationContext) -> bool {
        if let Some(array) = instance.as_array() {
            for (schema, item) in self.schemas.iter().zip(array.elements()) {
                if !schema.is_valid(&item, ctx) {
                    return false;
                }
            }
            true
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
            for (idx, (schema, item)) in self.schemas.iter().zip(array.elements()).enumerate() {
                schema.validate(&item, &location.push(idx), tracker, ctx)?;
            }
        }
        Ok(())
    }

    fn collect_errors<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
        errors: &mut Vec<ValidationError<'i>>,
    ) {
        let Some(array) = instance.as_array() else {
            return;
        };
        for (idx, (schema, item)) in self.schemas.iter().zip(array.elements()).enumerate() {
            schema.collect_errors(&item, &location.push(idx), tracker, ctx, errors);
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
            if array.len() != 0 {
                let mut children = ChildList::default();
                let mut max_index_applied = 0usize;
                for (idx, (schema_node, item)) in
                    self.schemas.iter().zip(array.elements()).enumerate()
                {
                    let child = schema_node.evaluate_instance_below(
                        &item,
                        &location.push(idx),
                        tracker,
                        ctx,
                    );
                    children.push(&mut ctx.arena, child);
                    max_index_applied = idx;
                }
                let annotation = if children.len() == array.len() {
                    Value::Bool(true)
                } else {
                    Value::from(max_index_applied)
                };
                let mut result = EvaluationResult::from_children(children);
                result.annotate(Annotations::new(annotation));
                return result;
            }
        }
        EvaluationResult::valid_empty()
    }
}

#[inline]
pub(crate) fn compile<'a, F: Json>(
    ctx: &compiler::Context<F>,
    _: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a, F>> {
    if let Value::Array(items) = schema {
        Some(PrefixItemsValidator::compile(ctx, items))
    } else {
        let location = ctx.location().join("prefixItems");
        Some(Err(ValidationError::single_type_error(
            location.clone(),
            location,
            Location::new(),
            LazyInstance::Ready(Cow::Borrowed(schema)),
            JsonType::Array,
        )))
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/jsonschema/unit/src/keywords/prefix_items/tests_139.rs"));
