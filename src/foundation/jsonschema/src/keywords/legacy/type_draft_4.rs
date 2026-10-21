use crate::{
    compiler,
    error::ValidationError,
    keywords::{type_, CompilationResult},
    paths::{LazyLocation, Location, RefTracker},
    types::{JsonType, JsonTypeSet},
    validator::{Validate, ValidationContext},
    Json, LazyInstance, Node,
};
use json_codec::{json, Map, Value};
use std::{borrow::Cow, str::FromStr};

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
        match instance.json_type() {
            JsonType::Number => {
                let Some(n) = instance.as_number() else {
                    return false;
                };
                if is_integer(&n) {
                    self.types.contains(JsonType::Integer) || self.types.contains(JsonType::Number)
                } else {
                    self.types.contains(JsonType::Number)
                }
            }
            other => self.types.contains(other),
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
            Err(ValidationError::multiple_type_error(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                self.types,
            ))
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
}

// Draft 4: "a JSON number without a fraction or exponent part", so `1.0` and `1e2` are not
// integers here, unlike drafts 6+.
pub(crate) fn is_integer<N: jsonschema_value::JsonNumber>(num: &N) -> bool {
    num.is_written_as_integer()
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
        Ok(JsonType::Array) => type_::ArrayTypeValidator::compile(location),
        Ok(JsonType::Boolean) => type_::BooleanTypeValidator::compile(location),
        Ok(JsonType::Integer) => IntegerTypeValidator::compile(location),
        Ok(JsonType::Null) => type_::NullTypeValidator::compile(location),
        Ok(JsonType::Number) => type_::NumberTypeValidator::compile(location),
        Ok(JsonType::Object) => type_::ObjectTypeValidator::compile(location),
        Ok(JsonType::String) => type_::StringTypeValidator::compile(location),
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
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/jsonschema/unit/src/keywords/legacy/type_draft_4/tests_203.rs"));
