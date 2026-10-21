use crate::{
    compiler,
    error::ValidationError,
    keywords::CompilationResult,
    node::SchemaNode,
    paths::{LazyLocation, Location, RefTracker},
    types::{JsonType, JsonTypeSet},
    validator::{Validate, ValidationContext},
    Array, Json, LazyInstance, Node, JsonValueBackend,
};
use json_codec::{Map, Value};
use std::borrow::Cow;

pub(crate) struct AdditionalItemsObjectValidator<F: Json = JsonValueBackend> {
    node: SchemaNode<F>,
    items_count: usize,
}
impl AdditionalItemsObjectValidator {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(
        ctx: &compiler::Context<F>,
        schema: &'a Value,
        items_count: usize,
    ) -> CompilationResult<'a, F> {
        let node = compiler::compile(ctx, ctx.as_resource_ref(schema))?;
        Ok(Box::new(AdditionalItemsObjectValidator {
            node,
            items_count,
        }))
    }
}
impl<F: Json> Validate<F> for AdditionalItemsObjectValidator<F> {
    fn is_valid(&self, instance: &F::Node<'_>, ctx: &mut ValidationContext) -> bool {
        if let Some(array) = instance.as_array() {
            array
                .elements()
                .skip(self.items_count)
                .all(|item| self.node.is_valid(&item, ctx))
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
            for (idx, item) in array.elements().enumerate().skip(self.items_count) {
                self.node
                    .validate(&item, &location.push(idx), tracker, ctx)?;
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
        for (idx, item) in array.elements().enumerate().skip(self.items_count) {
            self.node
                .collect_errors(&item, &location.push(idx), tracker, ctx, errors);
        }
    }
}

pub(crate) struct AdditionalItemsBooleanValidator {
    items_count: usize,
    location: Location,
}
impl AdditionalItemsBooleanValidator {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(
        items_count: usize,
        location: Location,
    ) -> CompilationResult<'a, F> {
        Ok(Box::new(AdditionalItemsBooleanValidator {
            items_count,
            location,
        }))
    }
}
impl<F: Json> Validate<F> for AdditionalItemsBooleanValidator {
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(array) = instance.as_array() {
            if array.len() > self.items_count {
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
        if let Some(array) = instance.as_array() {
            if array.len() > self.items_count {
                return Err(ValidationError::additional_items(
                    self.location.clone(),
                    crate::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance.lazy_value(),
                    self.items_count,
                ));
            }
        }
        Ok(())
    }
}

#[inline]
pub(crate) fn compile<'a, F: Json>(
    ctx: &compiler::Context<F>,
    parent: &Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a, F>> {
    if let Some(items) = parent.get("items") {
        match items {
            Value::Array(items) => {
                let kctx = ctx.new_at_location("additionalItems");
                let items_count = items.len();
                match schema {
                    Value::Object(_) => Some(AdditionalItemsObjectValidator::compile(
                        &kctx,
                        schema,
                        items_count,
                    )),
                    Value::Bool(false) => Some(AdditionalItemsBooleanValidator::compile(
                        items_count,
                        kctx.location().clone(),
                    )),
                    Value::Bool(true) => None,
                    // Anything else is not a schema; fail the build like `additionalProperties`.
                    _ => {
                        let location = kctx.location().clone();
                        Some(Err(ValidationError::multiple_type_error(
                            location.clone(),
                            location,
                            Location::new(),
                            LazyInstance::Ready(Cow::Borrowed(schema)),
                            JsonTypeSet::from(JsonType::Object).insert(JsonType::Boolean),
                        )))
                    }
                }
            }
            // `additionalItems` describes the elements past an array-form `items` tuple. Any other
            // `items` value leaves no tail, so the keyword is ignored: a schema (object or boolean)
            // covers every element — including `items: false`, where the tail is empty rather than
            // forbidden — and an invalid `items` value is itself ignored. Non-arrays are not this
            // keyword's business at all.
            _ => None,
        }
    } else {
        None
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/jsonschema/unit/src/keywords/additional_items/tests_173.rs"));
