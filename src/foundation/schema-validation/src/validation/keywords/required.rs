use crate::validation::LazyInstance;
use std::borrow::Cow;

use crate::reference::Vocabulary;
use crate::validation::{
    compiler,
    error::ValidationError,
    keywords::CompilationResult,
    paths::{LazyLocation, Location, RefTracker},
    properties::{KeyHead, PropertyName, HASHMAP_THRESHOLD},
    types::JsonType,
    validator::{Validate, ValidationContext},
    Json, JsonValueBackend, Node, Object,
};
use json_codec::{Map, Value};

/// Longest `required` array scanned in one pass; beyond it the comparisons outgrow the lookups.
const MAX_SCANNED_REQUIRED: usize = 16;

pub(crate) struct RequiredValidator<F: Json = JsonValueBackend> {
    required: Vec<(PropertyName, F::PreparedKey)>,
    location: Location,
}

impl RequiredValidator {
    #[inline]
    pub(crate) fn compile<F: Json>(
        items: &[Value],
        location: Location,
    ) -> CompilationResult<'_, F> {
        let mut required = Vec::with_capacity(items.len());
        for item in items {
            match item {
                Value::String(string) => {
                    required.push((PropertyName::new(string.clone()), F::prepare_key(string)));
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
        Ok(Box::new(RequiredValidator { required, location }))
    }
}

impl<F: Json> RequiredValidator<F> {
    #[inline]
    fn target(&self) -> u32 {
        (1_u32 << self.required.len()) - 1
    }

    /// One members pass marking each required name found, or `None` to use lookups instead.
    #[inline]
    fn scanned<'a, O: Object<'a, F>>(&self, object: &O) -> Option<u32> {
        let count = self.required.len();
        if count > MAX_SCANNED_REQUIRED || object.len() > F::KEYS_PER_LOOKUP * count {
            return None;
        }
        let target = self.target();
        let mut found = 0_u32;
        for (name, _) in object.members() {
            let name: &str = name.as_ref();
            let head = KeyHead::of(name);
            for (index, (required, _)) in self.required.iter().enumerate() {
                if required.matches(head, name) {
                    found |= 1 << index;
                    break;
                }
            }
            if found == target {
                break;
            }
        }
        Some(found)
    }
}

impl<F: Json> Validate<F> for RequiredValidator<F> {
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(object) = instance.as_object() {
            if object.len() < self.required.len() {
                return false;
            }
            let count = self.required.len();
            if count <= MAX_SCANNED_REQUIRED && object.len() <= F::KEYS_PER_LOOKUP * count {
                let target = (1_u32 << count) - 1;
                let mut found = 0_u32;
                for (name, _) in object.members() {
                    let name: &str = name.as_ref();
                    let head = KeyHead::of(name);
                    for (index, (required, _)) in self.required.iter().enumerate() {
                        if required.matches(head, name) {
                            found |= 1 << index;
                            break;
                        }
                    }
                    if found == target {
                        break;
                    }
                }
                return found == target;
            }
            self.required
                .iter()
                .all(|(_, key)| object.get(key).is_some())
        } else {
            true
        }
    }

    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if let Some(object) = instance.as_object() {
            let missing = match self.scanned(&object) {
                Some(found) if found == self.target() => None,
                Some(found) => Some(&self.required[found.trailing_ones() as usize].0),
                None => self
                    .required
                    .iter()
                    .find(|(_, key)| object.get(key).is_none())
                    .map(|(property_name, _)| property_name),
            };
            if let Some(property_name) = missing {
                return Err(ValidationError::required(
                    self.location.clone(),
                    crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance.lazy_value(),
                    Value::String(property_name.as_str().to_owned()),
                ));
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
        if let Some(object) = instance.as_object() {
            let eval_path =
                crate::validation::paths::capture_evaluation_path(tracker, &self.location);
            let found = self.scanned(&object);
            for (index, (property_name, key)) in self.required.iter().enumerate() {
                let present = match found {
                    Some(found) => found & (1 << index) != 0,
                    None => object.get(key).is_some(),
                };
                if !present {
                    errors.push(ValidationError::required(
                        self.location.clone(),
                        eval_path.clone(),
                        location.into(),
                        instance.lazy_value(),
                        Value::String(property_name.as_str().to_owned()),
                    ));
                }
            }
        }
    }
}

pub(crate) struct SingleItemRequiredValidator<F: Json = JsonValueBackend> {
    value: String,
    key: F::PreparedKey,
    location: Location,
}

impl SingleItemRequiredValidator {
    #[inline]
    pub(crate) fn compile<F: Json>(value: &str, location: Location) -> CompilationResult<'_, F> {
        Ok(Box::new(SingleItemRequiredValidator {
            value: value.to_string(),
            key: F::prepare_key(value),
            location,
        }))
    }
}

impl<F: Json> Validate<F> for SingleItemRequiredValidator<F> {
    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if !self.is_valid(instance, ctx) {
            return Err(ValidationError::required(
                self.location.clone(),
                crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                Value::String(self.value.clone()),
            ));
        }
        Ok(())
    }

    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(object) = instance.as_object() {
            if object.is_empty() {
                return false;
            }
            object.get(&self.key).is_some()
        } else {
            true
        }
    }
}

/// Specialized validator for exactly 2 required properties.
/// Uses fixed-size array and unrolled checks to avoid Vec/iterator overhead.
pub(crate) struct Required2Validator<F: Json = JsonValueBackend> {
    first: String,
    first_key: F::PreparedKey,
    second: String,
    second_key: F::PreparedKey,
    location: Location,
}

impl Required2Validator {
    #[inline]
    pub(crate) fn compile<F: Json>(
        first: String,
        second: String,
        location: Location,
    ) -> CompilationResult<'static, F> {
        Ok(Box::new(Required2Validator {
            first_key: F::prepare_key(&first),
            second_key: F::prepare_key(&second),
            first,
            second,
            location,
        }))
    }
}

impl<F: Json> Validate<F> for Required2Validator<F> {
    #[inline]
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(object) = instance.as_object() {
            object.len() >= 2
                && object.get(&self.first_key).is_some()
                && object.get(&self.second_key).is_some()
        } else {
            true
        }
    }

    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if let Some(object) = instance.as_object() {
            if object.get(&self.first_key).is_none() {
                return Err(ValidationError::required(
                    self.location.clone(),
                    crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance.lazy_value(),
                    Value::String(self.first.as_str().to_owned()),
                ));
            }
            if object.get(&self.second_key).is_none() {
                return Err(ValidationError::required(
                    self.location.clone(),
                    crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance.lazy_value(),
                    Value::String(self.second.as_str().to_owned()),
                ));
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
        if let Some(object) = instance.as_object() {
            let eval_path =
                crate::validation::paths::capture_evaluation_path(tracker, &self.location);
            if object.get(&self.first_key).is_none() {
                errors.push(ValidationError::required(
                    self.location.clone(),
                    eval_path.clone(),
                    location.into(),
                    instance.lazy_value(),
                    Value::String(self.first.as_str().to_owned()),
                ));
            }
            if object.get(&self.second_key).is_none() {
                errors.push(ValidationError::required(
                    self.location.clone(),
                    eval_path,
                    location.into(),
                    instance.lazy_value(),
                    Value::String(self.second.as_str().to_owned()),
                ));
            }
        }
    }
}

/// Specialized validator for exactly 3 required properties.
/// Uses fixed-size fields and unrolled checks to avoid Vec/iterator overhead.
pub(crate) struct Required3Validator<F: Json = JsonValueBackend> {
    first: PropertyName,
    first_key: F::PreparedKey,
    second: PropertyName,
    second_key: F::PreparedKey,
    third: PropertyName,
    third_key: F::PreparedKey,
    location: Location,
}

impl Required3Validator {
    #[inline]
    pub(crate) fn compile<F: Json>(
        first: String,
        second: String,
        third: String,
        location: Location,
    ) -> CompilationResult<'static, F> {
        Ok(Box::new(Required3Validator {
            first_key: F::prepare_key(&first),
            second_key: F::prepare_key(&second),
            third_key: F::prepare_key(&third),
            first: PropertyName::new(first),
            second: PropertyName::new(second),
            third: PropertyName::new(third),
            location,
        }))
    }
}

impl<F: Json> Required3Validator<F> {
    // `str` equality rejects on length; an ordered lookup must `memcmp` every probe.
    #[inline]
    fn found<'a, O: Object<'a, F>>(&self, object: &O) -> u8 {
        if object.len() > F::KEYS_PER_LOOKUP * 3 {
            return u8::from(object.get(&self.first_key).is_some())
                | (u8::from(object.get(&self.second_key).is_some()) << 1)
                | (u8::from(object.get(&self.third_key).is_some()) << 2);
        }
        let mut found = 0_u8;
        for (name, _) in object.members() {
            let name: &str = name.as_ref();
            let head = KeyHead::of(name);
            if self.first.matches(head, name) {
                found |= 0b001;
            } else if self.second.matches(head, name) {
                found |= 0b010;
            } else if self.third.matches(head, name) {
                found |= 0b100;
            }
            if found == 0b111 {
                break;
            }
        }
        found
    }
}

impl<F: Json> Validate<F> for Required3Validator<F> {
    #[inline]
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(object) = instance.as_object() {
            if object.len() < 3 {
                return false;
            }
            if object.len() > F::KEYS_PER_LOOKUP * 3 {
                return object.get(&self.first_key).is_some()
                    && object.get(&self.second_key).is_some()
                    && object.get(&self.third_key).is_some();
            }
            self.found(&object) == 0b111
        } else {
            true
        }
    }

    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if let Some(object) = instance.as_object() {
            let found = self.found(&object);
            if found != 0b111 {
                let missing = if found & 0b001 == 0 {
                    &self.first
                } else if found & 0b010 == 0 {
                    &self.second
                } else {
                    &self.third
                };
                return Err(ValidationError::required(
                    self.location.clone(),
                    crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance.lazy_value(),
                    Value::String(missing.as_str().to_owned()),
                ));
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
        if let Some(object) = instance.as_object() {
            let eval_path =
                crate::validation::paths::capture_evaluation_path(tracker, &self.location);
            let found = self.found(&object);
            if found & 0b001 == 0 {
                errors.push(ValidationError::required(
                    self.location.clone(),
                    eval_path.clone(),
                    location.into(),
                    instance.lazy_value(),
                    Value::String(self.first.as_str().to_owned()),
                ));
            }
            if found & 0b010 == 0 {
                errors.push(ValidationError::required(
                    self.location.clone(),
                    eval_path.clone(),
                    location.into(),
                    instance.lazy_value(),
                    Value::String(self.second.as_str().to_owned()),
                ));
            }
            if found & 0b100 == 0 {
                errors.push(ValidationError::required(
                    self.location.clone(),
                    eval_path,
                    location.into(),
                    instance.lazy_value(),
                    Value::String(self.third.as_str().to_owned()),
                ));
            }
        }
    }
}

#[inline]
pub(crate) fn compile<'a, F: Json>(
    ctx: &compiler::Context<F>,
    parent: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a, F>> {
    // Fused validators compile under the applicator vocabulary only
    if let Some(items) = schema
        .as_array()
        .filter(|_| ctx.has_vocabulary(&Vocabulary::Applicator))
    {
        let has_properties = parent.contains_key("properties");
        let has_pattern_properties = parent.contains_key("patternProperties");
        let additional_props_false =
            matches!(parent.get("additionalProperties"), Some(Value::Bool(false)));

        // Case 1: properties + additionalProperties: false + required: [1 item], no patternProperties
        // Handled by AdditionalPropertiesNotEmptyFalseWithRequired1Validator
        if items.len() == 1
            && additional_props_false
            && has_properties
            && !has_pattern_properties
            && !ctx.is_keyword_overridden("additionalProperties")
        {
            return None;
        }

        // Case 2: properties + required: [2 items], no additionalProperties, no patternProperties
        // Handled by SmallPropertiesWithRequired2Validator — only when the map is below the
        // threshold; above it BigPropertiesValidator is used and does not include required checks.
        // Must NOT skip when additionalProperties is a schema object: properties::compile returns
        // None in that case (AdditionalPropertiesNotEmptyValidator takes over), so
        // SmallPropertiesWithRequired2Validator is never created — required would be silently dropped.
        let additional_props_is_schema =
            matches!(parent.get("additionalProperties"), Some(Value::Object(_)));
        let properties_below_threshold = parent
            .get("properties")
            .and_then(Value::as_object)
            .is_some_and(|m| m.len() < HASHMAP_THRESHOLD);
        if items.len() == 2
            && has_properties
            && !ctx.is_keyword_overridden("properties")
            && properties_below_threshold
            && !additional_props_false
            && !additional_props_is_schema
            && !has_pattern_properties
        {
            return None;
        }
    }
    let location = ctx.location().join("required");
    compile_with_path(schema, location)
}

#[inline]
pub(crate) fn compile_with_path<F: Json>(
    schema: &Value,
    location: Location,
) -> Option<CompilationResult<'_, F>> {
    // IMPORTANT: If this function will ever return `None`, adjust `dependencies.rs` accordingly
    match schema {
        Value::Array(items) => match items.len() {
            1 => {
                let item = &items[0];
                if let Value::String(item) = item {
                    Some(SingleItemRequiredValidator::compile(item, location))
                } else {
                    Some(Err(ValidationError::single_type_error(
                        location.clone(),
                        location,
                        Location::new(),
                        LazyInstance::Ready(Cow::Borrowed(item)),
                        JsonType::String,
                    )))
                }
            }
            2 => {
                let (first, second) = (&items[0], &items[1]);
                match (first, second) {
                    (Value::String(first), Value::String(second)) => Some(
                        Required2Validator::compile(first.clone(), second.clone(), location),
                    ),
                    (Value::String(_), other) | (other, _) => {
                        Some(Err(ValidationError::single_type_error(
                            location.clone(),
                            location,
                            Location::new(),
                            LazyInstance::Ready(Cow::Borrowed(other)),
                            JsonType::String,
                        )))
                    }
                }
            }
            3 => {
                let (first, second, third) = (&items[0], &items[1], &items[2]);
                match (first, second, third) {
                    (Value::String(first), Value::String(second), Value::String(third)) => {
                        Some(Required3Validator::compile(
                            first.clone(),
                            second.clone(),
                            third.clone(),
                            location,
                        ))
                    }
                    (Value::String(_), Value::String(_), other)
                    | (Value::String(_), other, _)
                    | (other, _, _) => Some(Err(ValidationError::single_type_error(
                        location.clone(),
                        location,
                        Location::new(),
                        LazyInstance::Ready(Cow::Borrowed(other)),
                        JsonType::String,
                    ))),
                }
            }
            _ => Some(RequiredValidator::compile(items, location)),
        },
        _ => Some(Err(ValidationError::single_type_error(
            location.clone(),
            location,
            Location::new(),
            LazyInstance::Ready(Cow::Borrowed(schema)),
            JsonType::Array,
        ))),
    }
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/schema-validation/upstream/jsonschema/unit/src/keywords/required/tests_602.rs"
));
