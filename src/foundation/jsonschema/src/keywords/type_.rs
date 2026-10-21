use crate::{
    compiler,
    error::ValidationError,
    evaluation::ErrorDescription,
    keywords::CompilationResult,
    paths::Location,
    types::{JsonType, JsonTypeSet},
    validator::{EvaluationResult, Validate, ValidationContext},
    Json, LazyInstance, Node,
};
use serde_json::{json, Map, Value};
use std::{borrow::Cow, str::FromStr};

use crate::paths::{LazyLocation, RefTracker};

pub(crate) struct MultipleTypesValidator {
    types: JsonTypeSet,
    location: Location,
}

impl MultipleTypesValidator {
    #[inline]
    pub(crate) fn compile<F: Json>(
        items: &[Value],
        location: Location,
    ) -> CompilationResult<'_, F> {
        let mut types = JsonTypeSet::empty();
        for item in items {
            match item {
                Value::String(string) => {
                    if let Ok(ty) = JsonType::from_str(string.as_str()) {
                        types = types.insert(ty);
                    } else {
                        return Err(ValidationError::enumeration(
                            location.clone(),
                            location,
                            Location::new(),
                            LazyInstance::Ready(Cow::Borrowed(item)),
                            &json!([
                                "array", "boolean", "integer", "null", "number", "object", "string"
                            ]),
                        ));
                    }
                }
                _ => {
                    return Err(ValidationError::single_type_error(
                        location.clone(),
                        location,
                        Location::new(),
                        LazyInstance::Ready(Cow::Borrowed(item)),
                        JsonType::String,
                    ))
                }
            }
        }
        Ok(Box::new(MultipleTypesValidator { types, location }))
    }
}

impl<F: Json> Validate<F> for MultipleTypesValidator {
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        self.types.contains_value_type::<F>(instance)
    }
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
            Err(ValidationError::multiple_type_error(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                self.types,
            ))
        }
    }
    fn evaluate(
        &self,
        instance: &F::Node<'_>,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if Validate::<F>::is_valid(self, instance, ctx) {
            EvaluationResult::valid_empty()
        } else {
            let message = format!("{} is not of types {:?}", instance.to_value(), self.types);
            EvaluationResult::invalid_empty(vec![ErrorDescription::new("type", message)])
        }
    }
}

pub(crate) struct NullTypeValidator {
    location: Location,
}

impl NullTypeValidator {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(location: Location) -> CompilationResult<'a, F> {
        Ok(Box::new(NullTypeValidator { location }))
    }
}

impl<F: Json> Validate<F> for NullTypeValidator {
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        instance.is_null()
    }
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
            Err(ValidationError::single_type_error(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                JsonType::Null,
            ))
        }
    }
    fn evaluate(
        &self,
        instance: &F::Node<'_>,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if Validate::<F>::is_valid(self, instance, ctx) {
            EvaluationResult::valid_empty()
        } else {
            EvaluationResult::invalid_empty(vec![ErrorDescription::new(
                "type",
                format!(r#"{} is not of type "null""#, instance.to_value()),
            )])
        }
    }
}

pub(crate) struct BooleanTypeValidator {
    location: Location,
}

impl BooleanTypeValidator {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(location: Location) -> CompilationResult<'a, F> {
        Ok(Box::new(BooleanTypeValidator { location }))
    }
}

impl<F: Json> Validate<F> for BooleanTypeValidator {
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        instance.as_boolean().is_some()
    }
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
            Err(ValidationError::single_type_error(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                JsonType::Boolean,
            ))
        }
    }
    fn evaluate(
        &self,
        instance: &F::Node<'_>,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if Validate::<F>::is_valid(self, instance, ctx) {
            EvaluationResult::valid_empty()
        } else {
            EvaluationResult::invalid_empty(vec![ErrorDescription::new(
                "type",
                format!(r#"{} is not of type "boolean""#, instance.to_value()),
            )])
        }
    }
}

pub(crate) struct StringTypeValidator {
    location: Location,
}

impl StringTypeValidator {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(location: Location) -> CompilationResult<'a, F> {
        Ok(Box::new(StringTypeValidator { location }))
    }
}

impl<F: Json> Validate<F> for StringTypeValidator {
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        instance.is_string()
    }

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
            Err(ValidationError::single_type_error(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                JsonType::String,
            ))
        }
    }
    fn evaluate(
        &self,
        instance: &F::Node<'_>,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if Validate::<F>::is_valid(self, instance, ctx) {
            EvaluationResult::valid_empty()
        } else {
            EvaluationResult::invalid_empty(vec![ErrorDescription::new(
                "type",
                format!(r#"{} is not of type "string""#, instance.to_value()),
            )])
        }
    }
}

pub(crate) struct ArrayTypeValidator {
    location: Location,
}

impl ArrayTypeValidator {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(location: Location) -> CompilationResult<'a, F> {
        Ok(Box::new(ArrayTypeValidator { location }))
    }
}

impl<F: Json> Validate<F> for ArrayTypeValidator {
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        instance.as_array().is_some()
    }

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
            Err(ValidationError::single_type_error(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                JsonType::Array,
            ))
        }
    }
    fn evaluate(
        &self,
        instance: &F::Node<'_>,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if Validate::<F>::is_valid(self, instance, ctx) {
            EvaluationResult::valid_empty()
        } else {
            EvaluationResult::invalid_empty(vec![ErrorDescription::new(
                "type",
                format!(r#"{} is not of type "array""#, instance.to_value()),
            )])
        }
    }
}

pub(crate) struct ObjectTypeValidator {
    location: Location,
}

impl ObjectTypeValidator {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(location: Location) -> CompilationResult<'a, F> {
        Ok(Box::new(ObjectTypeValidator { location }))
    }
}

impl<F: Json> Validate<F> for ObjectTypeValidator {
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        instance.as_object().is_some()
    }
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
            Err(ValidationError::single_type_error(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                JsonType::Object,
            ))
        }
    }
    fn evaluate(
        &self,
        instance: &F::Node<'_>,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if Validate::<F>::is_valid(self, instance, ctx) {
            EvaluationResult::valid_empty()
        } else {
            EvaluationResult::invalid_empty(vec![ErrorDescription::new(
                "type",
                format!(r#"{} is not of type "object""#, instance.to_value()),
            )])
        }
    }
}

pub(crate) struct NumberTypeValidator {
    location: Location,
}

impl NumberTypeValidator {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(location: Location) -> CompilationResult<'a, F> {
        Ok(Box::new(NumberTypeValidator { location }))
    }
}

impl<F: Json> Validate<F> for NumberTypeValidator {
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        instance.is_number()
    }
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
            Err(ValidationError::single_type_error(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                JsonType::Number,
            ))
        }
    }
    fn evaluate(
        &self,
        instance: &F::Node<'_>,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if Validate::<F>::is_valid(self, instance, ctx) {
            EvaluationResult::valid_empty()
        } else {
            EvaluationResult::invalid_empty(vec![ErrorDescription::new(
                "type",
                format!(r#"{} is not of type "number""#, instance.to_value()),
            )])
        }
    }
}

pub(crate) struct IntegerTypeValidator {
    location: Location,
}

impl IntegerTypeValidator {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(location: Location) -> CompilationResult<'a, F> {
        Ok(Box::new(IntegerTypeValidator { location }))
    }
}

impl<F: Json> Validate<F> for IntegerTypeValidator {
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(num) = instance.as_number() {
            is_integer(&num)
        } else {
            false
        }
    }
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
            Err(ValidationError::single_type_error(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                JsonType::Integer,
            ))
        }
    }
    fn evaluate(
        &self,
        instance: &F::Node<'_>,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if Validate::<F>::is_valid(self, instance, ctx) {
            EvaluationResult::valid_empty()
        } else {
            EvaluationResult::invalid_empty(vec![ErrorDescription::new(
                "type",
                format!(r#"{} is not of type "integer""#, instance.to_value()),
            )])
        }
    }
}

pub(crate) fn is_integer<N: jsonschema_value::JsonNumber>(num: &N) -> bool {
    num.is_integer()
}

#[inline]
pub(crate) fn compile<'a, F: Json>(
    ctx: &compiler::Context<F>,
    _parent: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a, F>> {
    let location = ctx.location().join("type");
    match schema {
        Value::String(item) => Some(compile_single_type(item.as_str(), location, schema)),
        Value::Array(items) => {
            if items.len() == 1 {
                let item = &items[0];
                if let Value::String(ty) = item {
                    Some(compile_single_type(ty.as_str(), location, item))
                } else {
                    Some(Err(ValidationError::single_type_error(
                        location.clone(),
                        location,
                        Location::new(),
                        LazyInstance::Ready(Cow::Borrowed(item)),
                        JsonType::String,
                    )))
                }
            } else {
                Some(MultipleTypesValidator::compile(items, location))
            }
        }
        _ => {
            let location = ctx.location().join("type");
            Some(Err(ValidationError::multiple_type_error(
                location.clone(),
                location,
                Location::new(),
                LazyInstance::Ready(Cow::Borrowed(schema)),
                JsonTypeSet::from(JsonType::String).insert(JsonType::Array),
            )))
        }
    }
}

fn compile_single_type<'a, F: Json>(
    item: &str,
    location: Location,
    instance: &'a Value,
) -> CompilationResult<'a, F> {
    match JsonType::from_str(item) {
        Ok(JsonType::Array) => ArrayTypeValidator::compile(location),
        Ok(JsonType::Boolean) => BooleanTypeValidator::compile(location),
        Ok(JsonType::Integer) => IntegerTypeValidator::compile(location),
        Ok(JsonType::Null) => NullTypeValidator::compile(location),
        Ok(JsonType::Number) => NumberTypeValidator::compile(location),
        Ok(JsonType::Object) => ObjectTypeValidator::compile(location),
        Ok(JsonType::String) => StringTypeValidator::compile(location),
        Err(()) => Err(ValidationError::compile_error(
            location.clone(),
            location,
            Location::new(),
            LazyInstance::Ready(Cow::Borrowed(instance)),
            "Unexpected type",
        )),
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/jsonschema/unit/src/keywords/type_/tests_536.rs"));
