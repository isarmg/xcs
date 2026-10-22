use crate::validation::{
    compiler,
    error::ValidationError,
    keywords::CompilationResult,
    node::SchemaNode,
    paths::{LazyLocation, RefTracker},
    validator::{Validate, ValidationContext},
    Json, JsonValueBackend, Node,
};
use json_codec::{Map, Value};

pub(crate) struct NotValidator<F: Json> {
    // needed only for error representation
    original: Value,
    node: SchemaNode<F>,
}

impl NotValidator<JsonValueBackend> {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(
        ctx: &compiler::Context<F>,
        schema: &'a Value,
    ) -> CompilationResult<'a, F> {
        let ctx = ctx.new_at_location("not");
        Ok(Box::new(NotValidator {
            original: schema.clone(),
            node: compiler::compile(&ctx, ctx.as_resource_ref(schema))?,
        }))
    }
}

impl<F: Json> Validate<F> for NotValidator<F> {
    fn is_valid(&self, instance: &F::Node<'_>, ctx: &mut ValidationContext) -> bool {
        !self.node.is_valid(instance, ctx)
    }

    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if self.is_valid(instance, ctx) {
            Ok(())
        } else {
            Err(ValidationError::not(
                self.node.location().clone(),
                crate::validation::paths::capture_evaluation_path(tracker, self.node.location()),
                location.into(),
                instance.lazy_value(),
                self.original.clone(),
            ))
        }
    }
}

#[inline]
pub(crate) fn compile<'a, F: Json>(
    ctx: &compiler::Context<F>,
    _: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a, F>> {
    Some(NotValidator::compile(ctx, schema))
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/schema-validation/upstream/jsonschema/unit/src/keywords/not/tests_67.rs"
));
