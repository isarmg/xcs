use crate::validation::{
    compiler,
    error::ValidationError,
    keywords::{CompilationResult, NotNullable, Nullability, Nullable},
    paths::{LazyLocation, Location, RefTracker},
    properties::{KeyHead, PropertyName},
    types::{JsonType, JsonTypeSet},
    validator::{Validate, ValidationContext},
    Json, LazyInstance, Node,
};
use crate::value::JsonNumber;
use ahash::AHashSet;
use json_codec::{Map, Value};
use std::{borrow::Cow, marker::PhantomData};

const STRING_ENUM_THRESHOLD: usize = 10;

#[derive(Debug)]
pub(crate) struct EnumValidator {
    options: Value,
    // Types that occur in items
    types: JsonTypeSet,
    items: Vec<Value>,
    location: Location,
}

impl EnumValidator {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(
        schema: &'a Value,
        items: &'a [Value],
        location: Location,
    ) -> CompilationResult<'a, F> {
        let mut types = JsonTypeSet::empty();
        for item in items {
            types = types.insert(JsonType::from(item));
        }
        Ok(Box::new(EnumValidator {
            options: schema.clone(),
            items: items.to_vec(),
            types,
            location,
        }))
    }
}

impl<F: Json> Validate<F> for EnumValidator {
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
            Err(ValidationError::enumeration(
                self.location.clone(),
                crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                &self.options,
            ))
        }
    }

    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        // If the input value type is not in the types present among the enum options, then there
        // is no reason to compare it against all items - we know that
        // there are no items with such type at all
        if self.types.contains_value_type::<F>(instance) {
            self.items.iter().any(|item| instance.equals_value(item))
        } else {
            false
        }
    }
}

#[derive(Debug)]
pub(crate) struct SingleValueEnumValidator {
    value: Value,
    options: Value,
    location: Location,
}

impl SingleValueEnumValidator {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(
        schema: &'a Value,
        value: &'a Value,
        location: Location,
    ) -> CompilationResult<'a, F> {
        Ok(Box::new(SingleValueEnumValidator {
            options: schema.clone(),
            value: value.clone(),
            location,
        }))
    }
}

impl<F: Json> Validate<F> for SingleValueEnumValidator {
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
            Err(ValidationError::enumeration(
                self.location.clone(),
                crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                &self.options,
            ))
        }
    }

    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        instance.equals_value(&self.value)
    }
}

#[derive(Debug)]
pub(crate) struct SmallStringEnumValidator<N> {
    options: Value,
    items: Vec<PropertyName>,
    location: Location,
    nullability: PhantomData<N>,
}

impl<N: Nullability> SmallStringEnumValidator<N> {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(
        schema: &'a Value,
        items: &'a [Value],
        location: Location,
    ) -> CompilationResult<'a, F> {
        Ok(Box::new(SmallStringEnumValidator::<N> {
            options: schema.clone(),
            items: items
                .iter()
                .filter_map(Value::as_str)
                .map(|item| PropertyName::new(item.to_owned()))
                .collect(),
            location,
            nullability: PhantomData,
        }))
    }
}

impl<F: Json, N: Nullability> Validate<F> for SmallStringEnumValidator<N> {
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
            Err(ValidationError::enumeration(
                self.location.clone(),
                crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                &self.options,
            ))
        }
    }

    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(s) = instance.as_string() {
            let s = s.as_ref();
            let head = KeyHead::of(s);
            self.items.iter().any(|item| item.matches(head, s))
        } else {
            N::ACCEPTS_NULL && instance.is_null()
        }
    }
}

#[derive(Debug)]
pub(crate) struct BigStringEnumValidator<N> {
    options: Value,
    items: AHashSet<Box<str>>,
    location: Location,
    nullability: PhantomData<N>,
}

impl<N: Nullability> BigStringEnumValidator<N> {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(
        schema: &'a Value,
        items: &'a [Value],
        location: Location,
    ) -> CompilationResult<'a, F> {
        Ok(Box::new(BigStringEnumValidator::<N> {
            options: schema.clone(),
            items: items
                .iter()
                .filter_map(Value::as_str)
                .map(Into::into)
                .collect(),
            location,
            nullability: PhantomData,
        }))
    }
}

impl<F: Json, N: Nullability> Validate<F> for BigStringEnumValidator<N> {
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
            Err(ValidationError::enumeration(
                self.location.clone(),
                crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                &self.options,
            ))
        }
    }

    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(s) = instance.as_string() {
            self.items.contains(s.as_ref())
        } else {
            N::ACCEPTS_NULL && instance.is_null()
        }
    }
}

#[derive(Debug)]
pub(crate) struct IntegerEnumValidator<N> {
    options: Value,
    items: Vec<Value>,
    /// Sorted and deduplicated; answers for instances written as an `i64`.
    integers: Vec<i64>,
    location: Location,
    nullability: PhantomData<N>,
}

impl<N: Nullability> IntegerEnumValidator<N> {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(
        schema: &'a Value,
        items: &'a [Value],
        location: Location,
    ) -> CompilationResult<'a, F> {
        let mut integers: Vec<i64> = items.iter().filter_map(Value::as_i64).collect();
        integers.sort_unstable();
        integers.dedup();
        Ok(Box::new(IntegerEnumValidator::<N> {
            options: schema.clone(),
            items: items.iter().filter(|v| !v.is_null()).cloned().collect(),
            integers,
            location,
            nullability: PhantomData,
        }))
    }
}

impl<F: Json, N: Nullability> Validate<F> for IntegerEnumValidator<N> {
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
            Err(ValidationError::enumeration(
                self.location.clone(),
                crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                &self.options,
            ))
        }
    }

    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(number) = instance.as_number() {
            if let Some(value) = number.as_i64() {
                self.integers.binary_search(&value).is_ok()
            } else {
                // `2.0`, `1e2`, past `i64`: numeric equality across written forms and precisions
                self.items.iter().any(|item| instance.equals_value(item))
            }
        } else {
            N::ACCEPTS_NULL && instance.is_null()
        }
    }
}

#[inline]
pub(crate) fn compile<'a, F: Json>(
    ctx: &compiler::Context<F>,
    _: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a, F>> {
    if let Value::Array(items) = schema {
        let location = ctx.location().join("enum");
        if items.len() == 1 {
            let value = items.iter().next().expect("Vec is not empty");
            Some(SingleValueEnumValidator::compile(schema, value, location))
        } else if items
            .iter()
            .all(|v| matches!(v, Value::String(_) | Value::Null))
        {
            let small = items.len() <= STRING_ENUM_THRESHOLD;
            match (small, items.iter().any(Value::is_null)) {
                (true, true) => Some(SmallStringEnumValidator::<Nullable>::compile(
                    schema, items, location,
                )),
                (true, false) => Some(SmallStringEnumValidator::<NotNullable>::compile(
                    schema, items, location,
                )),
                (false, true) => Some(BigStringEnumValidator::<Nullable>::compile(
                    schema, items, location,
                )),
                (false, false) => Some(BigStringEnumValidator::<NotNullable>::compile(
                    schema, items, location,
                )),
            }
        } else if items.iter().all(|v| v.is_null() || v.as_i64().is_some()) {
            if items.iter().any(Value::is_null) {
                Some(IntegerEnumValidator::<Nullable>::compile(
                    schema, items, location,
                ))
            } else {
                Some(IntegerEnumValidator::<NotNullable>::compile(
                    schema, items, location,
                ))
            }
        } else {
            Some(EnumValidator::compile(schema, items, location))
        }
    } else {
        let location = ctx.location().join("enum");
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
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/schema-validation/upstream/jsonschema/unit/src/keywords/enum_/tests_366.rs"
));
