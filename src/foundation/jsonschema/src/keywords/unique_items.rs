use crate::{
    compiler,
    error::ValidationError,
    keywords::CompilationResult,
    paths::{LazyLocation, Location, RefTracker},
    validator::{Validate, ValidationContext},
    Array, Json, Node,
};
use json_codec::{Map, Value};

fn has_unique_items<F: Json>(instance: &F::Node<'_>) -> bool {
    let Some(array) = instance.as_array() else {
        return true;
    };
    array.is_unique()
}

pub(crate) struct UniqueItemsValidator {
    location: Location,
}

impl UniqueItemsValidator {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(location: Location) -> CompilationResult<'a, F> {
        Ok(Box::new(UniqueItemsValidator { location }))
    }
}

impl<F: Json> Validate<F> for UniqueItemsValidator {
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        has_unique_items::<F>(instance)
    }

    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if has_unique_items::<F>(instance) {
            Ok(())
        } else {
            Err(ValidationError::unique_items(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
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
    if let Value::Bool(value) = schema {
        if *value {
            let location = ctx.location().join("uniqueItems");
            Some(UniqueItemsValidator::compile(location))
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/jsonschema/unit/src/keywords/unique_items/tests_72.rs"));
