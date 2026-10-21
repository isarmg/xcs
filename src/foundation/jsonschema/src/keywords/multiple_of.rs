use crate::{
    compiler,
    error::ValidationError,
    keywords::CompilationResult,
    numeric,
    paths::{LazyLocation, Location, RefTracker},
    types::JsonType,
    validator::{Validate, ValidationContext},
    Json, LazyInstance, Node,
};
#[cfg(feature = "arbitrary-precision")]
use jsonschema_value::JsonNumber;
use serde_json::{Map, Value};
use std::borrow::Cow;

pub(crate) struct MultipleOfFloatValidator {
    multiple_of: f64,
    #[cfg(feature = "arbitrary-precision")]
    original_value: serde_json::Number,
    location: Location,
}

impl MultipleOfFloatValidator {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(
        multiple_of: f64,
        #[cfg(feature = "arbitrary-precision")] original_value: &serde_json::Number,
        location: Location,
    ) -> CompilationResult<'a, F> {
        Ok(Box::new(MultipleOfFloatValidator {
            multiple_of,
            #[cfg(feature = "arbitrary-precision")]
            original_value: original_value.clone(),
            location,
        }))
    }
}

impl<F: Json> Validate<F> for MultipleOfFloatValidator {
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(item) = instance.as_number() {
            numeric::is_multiple_of_float(&item, self.multiple_of)
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
        if !Validate::<F>::is_valid(self, instance, ctx) {
            #[cfg(feature = "arbitrary-precision")]
            {
                return Err(ValidationError::multiple_of(
                    self.location.clone(),
                    crate::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance.lazy_value(),
                    Value::Number(self.original_value.clone()),
                ));
            }
            #[cfg(not(feature = "arbitrary-precision"))]
            {
                return Err(ValidationError::multiple_of(
                    self.location.clone(),
                    crate::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance.lazy_value(),
                    self.multiple_of,
                ));
            }
        }
        Ok(())
    }
}

pub(crate) struct MultipleOfIntegerValidator {
    multiple_of: f64,
    #[cfg(feature = "arbitrary-precision")]
    original_value: serde_json::Number,
    location: Location,
}

impl MultipleOfIntegerValidator {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(
        multiple_of: f64,
        #[cfg(feature = "arbitrary-precision")] original_value: &serde_json::Number,
        location: Location,
    ) -> CompilationResult<'a, F> {
        Ok(Box::new(MultipleOfIntegerValidator {
            multiple_of,
            #[cfg(feature = "arbitrary-precision")]
            original_value: original_value.clone(),
            location,
        }))
    }
}

impl<F: Json> Validate<F> for MultipleOfIntegerValidator {
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        if let Some(item) = instance.as_number() {
            numeric::is_multiple_of_integer(&item, self.multiple_of)
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
        if !Validate::<F>::is_valid(self, instance, ctx) {
            #[cfg(feature = "arbitrary-precision")]
            {
                return Err(ValidationError::multiple_of(
                    self.location.clone(),
                    crate::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance.lazy_value(),
                    Value::Number(self.original_value.clone()),
                ));
            }
            #[cfg(not(feature = "arbitrary-precision"))]
            {
                return Err(ValidationError::multiple_of(
                    self.location.clone(),
                    crate::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance.lazy_value(),
                    self.multiple_of,
                ));
            }
        }
        Ok(())
    }
}

#[cfg(feature = "arbitrary-precision")]
pub(crate) struct MultipleOfBigIntValidator {
    multiple_of: num_bigint::BigInt,
    original_value: serde_json::Number,
    location: Location,
}

#[cfg(feature = "arbitrary-precision")]
impl MultipleOfBigIntValidator {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(
        multiple_of: num_bigint::BigInt,
        original_value: &serde_json::Number,
        location: Location,
    ) -> CompilationResult<'a, F> {
        Ok(Box::new(MultipleOfBigIntValidator {
            multiple_of,
            original_value: original_value.clone(),
            location,
        }))
    }
}

#[cfg(feature = "arbitrary-precision")]
impl<F: Json> Validate<F> for MultipleOfBigIntValidator {
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        use num_bigint::BigInt;
        use num_traits::One;
        if let Some(item) = instance.as_number() {
            if let Some(instance_bigint) = numeric::bignum::try_parse_bigint(&item.to_number()) {
                numeric::bignum::is_multiple_of_bigint(&instance_bigint, &self.multiple_of)
            } else if let Some(v) = item.as_u64() {
                let v_bigint = BigInt::from(v);
                numeric::bignum::is_multiple_of_bigint(&v_bigint, &self.multiple_of)
            } else if let Some(v) = item.as_i64() {
                let v_bigint = BigInt::from(v);
                numeric::bignum::is_multiple_of_bigint(&v_bigint, &self.multiple_of)
            } else if let Some(v) = item.as_f64() {
                if v.fract() != 0.0 {
                    return false;
                }
                // JSON numbers that originate as f64 are only exactly representable if their
                // absolute value is below 2^53. Within that "safe integer" window we can round-trip
                // to i64 without losing bits, so convert and reuse the bigint path.
                #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
                if v.is_finite() && v.abs() < (1u64 << 53) as f64 {
                    let v_bigint = BigInt::from(v as i64);
                    numeric::bignum::is_multiple_of_bigint(&v_bigint, &self.multiple_of)
                } else {
                    // 2^53 and above can no longer be recovered exactly from the binary64 mantissa,
                    // so we bail out instead of pretending we know whether the bigint divisor divides
                    // such an approximation.
                    false
                }
            } else {
                // as_f64() returning None means the literal overflowed binary64 (e.g. 1e10000) or was
                // written in plain decimal form with more precision than f64 can carry. We attempt to
                // parse it as an exact BigFraction and only accept it when the denominator is 1 so we
                // can safely convert it to BigInt before running the modulo check below.
                if let Some(instance_bigfrac) =
                    numeric::bignum::try_parse_bigfraction(&item.to_number())
                {
                    if instance_bigfrac.denom().is_none_or(One::is_one) {
                        if let Some(numer) = instance_bigfrac.numer() {
                            let instance_bigint = BigInt::from(numer.clone());
                            return numeric::bignum::is_multiple_of_bigint(
                                &instance_bigint,
                                &self.multiple_of,
                            );
                        }
                    }
                }
                // If we made it here we encountered scientific notation that can’t be normalized, or
                // a decimal with a fractional part. Either way it cannot be an integer multiple.
                false
            }
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
        if !Validate::<F>::is_valid(self, instance, ctx) {
            return Err(ValidationError::multiple_of(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                Value::Number(self.original_value.clone()),
            ));
        }
        Ok(())
    }
}

#[cfg(feature = "arbitrary-precision")]
pub(crate) struct MultipleOfBigFracValidator {
    multiple_of: fraction::BigFraction,
    original_value: serde_json::Number,
    location: Location,
}

#[cfg(feature = "arbitrary-precision")]
impl MultipleOfBigFracValidator {
    #[inline]
    pub(crate) fn compile<'a, F: Json>(
        multiple_of: fraction::BigFraction,
        original_value: &serde_json::Number,
        location: Location,
    ) -> CompilationResult<'a, F> {
        Ok(Box::new(MultipleOfBigFracValidator {
            multiple_of,
            original_value: original_value.clone(),
            location,
        }))
    }
}

#[cfg(feature = "arbitrary-precision")]
impl<F: Json> Validate<F> for MultipleOfBigFracValidator {
    fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
        use num_traits::ToPrimitive;
        if let Some(item) = instance.as_number() {
            // Try to parse instance as BigFraction for exact precision
            if let Some(instance_bigfrac) =
                numeric::bignum::try_parse_bigfraction(&item.to_number())
            {
                numeric::bignum::is_multiple_of_bigfrac(&instance_bigfrac, &self.multiple_of)
            } else if let Some(instance_bigint) =
                numeric::bignum::try_parse_bigint(&item.to_number())
            {
                let value_frac = fraction::BigFraction::from(instance_bigint);
                numeric::bignum::is_multiple_of_bigfrac(&value_frac, &self.multiple_of)
            } else if let Some(v) = item.as_u64() {
                let v_frac = fraction::BigFraction::from(v);
                numeric::bignum::is_multiple_of_bigfrac(&v_frac, &self.multiple_of)
            } else if let Some(v) = item.as_i64() {
                let v_frac = fraction::BigFraction::from(v);
                numeric::bignum::is_multiple_of_bigfrac(&v_frac, &self.multiple_of)
            } else {
                // Fall back to the regular float-based validation since precision is already lost
                let multiple_f64 = self.multiple_of.to_f64().unwrap_or(f64::INFINITY);
                numeric::is_multiple_of_float(&item, multiple_f64)
            }
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
        if !Validate::<F>::is_valid(self, instance, ctx) {
            return Err(ValidationError::multiple_of(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance.lazy_value(),
                Value::Number(self.original_value.clone()),
            ));
        }
        Ok(())
    }
}

#[inline]
pub(crate) fn compile<'a, F: Json>(
    ctx: &compiler::Context<F>,
    _: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a, F>> {
    if let Value::Number(multiple_of) = schema {
        let location = ctx.location().join("multipleOf");

        #[cfg(feature = "arbitrary-precision")]
        {
            // Try BigInt first for large integers
            if let Some(bigint_multiple) = numeric::bignum::try_parse_bigint(multiple_of) {
                return Some(MultipleOfBigIntValidator::compile(
                    bigint_multiple,
                    multiple_of,
                    location,
                ));
            }
            // Then try BigFraction for exact decimal precision
            if let Some(bigfrac_multiple) = numeric::bignum::try_parse_bigfraction(multiple_of) {
                return Some(MultipleOfBigFracValidator::compile(
                    bigfrac_multiple,
                    multiple_of,
                    location,
                ));
            }
        }

        // If not BigInt or BigFraction, try to represent as f64
        // Note: Extreme scientific notation (e.g., 1e309, 1e400) cannot fit in f64
        if let Some(f64_value) = multiple_of.as_f64() {
            #[cfg(feature = "arbitrary-precision")]
            {
                if f64_value.fract() == 0. {
                    Some(MultipleOfIntegerValidator::compile(
                        f64_value,
                        multiple_of,
                        location,
                    ))
                } else {
                    Some(MultipleOfFloatValidator::compile(
                        f64_value,
                        multiple_of,
                        location,
                    ))
                }
            }
            #[cfg(not(feature = "arbitrary-precision"))]
            {
                if f64_value.fract() == 0. {
                    Some(MultipleOfIntegerValidator::compile(f64_value, location))
                } else {
                    Some(MultipleOfFloatValidator::compile(f64_value, location))
                }
            }
        } else {
            // Extreme numbers beyond f64 range (e.g., 1e309) - not supported
            // Skip this keyword - all instances will be considered valid
            None
        }
    } else {
        let location = ctx.location().join("multipleOf");
        Some(Err(ValidationError::single_type_error(
            location.clone(),
            location,
            Location::new(),
            LazyInstance::Ready(Cow::Borrowed(schema)),
            JsonType::Number,
        )))
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/jsonschema/unit/src/keywords/multiple_of/tests_394.rs"));
