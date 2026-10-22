use crate::validation::{
    compiler,
    error::ValidationError,
    keywords::CompilationResult,
    paths::Location,
    validator::{Validate, ValidationContext},
    Array, Json, Node,
};
use json_codec::{Map, Number, Value};

use crate::validation::paths::{LazyLocation, RefTracker};

struct ConstArrayValidator {
    value: Vec<Value>,
    location: Location,
}
impl ConstArrayValidator {
    #[inline]
    pub(crate) fn compile<F: Json>(
        value: &[Value],
        location: Location,
    ) -> CompilationResult<'_, F> {
        Ok(Box::new(ConstArrayValidator {
            value: value.to_vec(),
            location,
        }))
    }
}
impl<F: Json> Validate<F> for ConstArrayValidator {
    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if Validate::<F>::is_valid(self, instance, ctx) {
            Ok(())
        } else {
            Err(ValidationError::constant_array(
                self.location.clone(),
                crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                &self.value,
            ))
        }
    }

    #[inline]
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(items) = instance.as_array() {
            items.len() == self.value.len()
                && items
                    .elements()
                    .zip(&self.value)
                    .all(|(item, expected)| item.equals_value(expected))
        } else {
            false
        }
    }
}

struct ConstBooleanValidator {
    value: bool,
    location: Location,
}
impl ConstBooleanValidator {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(
        value: bool,
        location: Location,
    ) -> CompilationResult<'a, F> {
        Ok(Box::new(ConstBooleanValidator { value, location }))
    }
}
impl<F: Json> Validate<F> for ConstBooleanValidator {
    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if Validate::<F>::is_valid(self, instance, ctx) {
            Ok(())
        } else {
            Err(ValidationError::constant_boolean(
                self.location.clone(),
                crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                self.value,
            ))
        }
    }

    #[inline]
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        instance.as_boolean() == Some(self.value)
    }
}

struct ConstNullValidator {
    location: Location,
}
impl ConstNullValidator {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(location: Location) -> CompilationResult<'a, F> {
        Ok(Box::new(ConstNullValidator { location }))
    }
}
impl<F: Json> Validate<F> for ConstNullValidator {
    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if Validate::<F>::is_valid(self, instance, ctx) {
            Ok(())
        } else {
            Err(ValidationError::constant_null(
                self.location.clone(),
                crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
            ))
        }
    }
    #[inline]
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        instance.is_null()
    }
}

struct ConstNumberValidator {
    // This is saved in order to ensure that the error message is not altered by precision loss
    original_value: Number,
    location: Location,
}

impl ConstNumberValidator {
    #[inline]
    pub(crate) fn compile<F: Json>(
        original_value: &Number,
        location: Location,
    ) -> CompilationResult<'_, F> {
        Ok(Box::new(ConstNumberValidator {
            original_value: original_value.clone(),
            location,
        }))
    }
}

impl<F: Json> Validate<F> for ConstNumberValidator {
    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if Validate::<F>::is_valid(self, instance, ctx) {
            Ok(())
        } else {
            Err(ValidationError::constant_number(
                self.location.clone(),
                crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                &self.original_value,
            ))
        }
    }

    #[inline]
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(item) = instance.as_number() {
            crate::validation::cmp::equal_numbers(&item, &self.original_value)
        } else {
            false
        }
    }
}

pub(crate) struct ConstObjectValidator {
    value: Value,
    location: Location,
}

impl ConstObjectValidator {
    #[inline]
    pub(crate) fn compile<F: Json>(
        value: &Map<String, Value>,
        location: Location,
    ) -> CompilationResult<'_, F> {
        Ok(Box::new(ConstObjectValidator {
            value: Value::Object(value.clone()),
            location,
        }))
    }
}

impl<F: Json> Validate<F> for ConstObjectValidator {
    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if Validate::<F>::is_valid(self, instance, ctx) {
            Ok(())
        } else {
            Err(ValidationError::constant_object(
                self.location.clone(),
                crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                &self.value,
            ))
        }
    }

    #[inline]
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        instance.equals_value(&self.value)
    }
}

pub(crate) struct ConstStringValidator {
    value: String,
    location: Location,
}

impl ConstStringValidator {
    #[inline]
    pub(crate) fn compile<F: Json>(value: &str, location: Location) -> CompilationResult<'_, F> {
        Ok(Box::new(ConstStringValidator {
            value: value.to_string(),
            location,
        }))
    }
}

impl<F: Json> Validate<F> for ConstStringValidator {
    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if Validate::<F>::is_valid(self, instance, ctx) {
            Ok(())
        } else {
            Err(ValidationError::constant_string(
                self.location.clone(),
                crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                &self.value,
            ))
        }
    }

    #[inline]
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(item) = instance.as_string() {
            self.value == item.as_ref()
        } else {
            false
        }
    }
}

#[inline]
pub(crate) fn compile<'a, F: Json>(
    ctx: &compiler::Context<F>,
    _: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a, F>> {
    let location = ctx.location().join("const");
    match schema {
        Value::Array(items) => Some(ConstArrayValidator::compile(items, location)),
        Value::Bool(item) => Some(ConstBooleanValidator::compile(*item, location)),
        Value::Null => Some(ConstNullValidator::compile(location)),
        Value::Number(item) => Some(ConstNumberValidator::compile(item, location)),
        Value::Object(map) => Some(ConstObjectValidator::compile(map, location)),
        Value::String(string) => Some(ConstStringValidator::compile(string, location)),
    }
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/schema-validation/upstream/jsonschema/unit/src/keywords/const_/tests_296.rs"
));
