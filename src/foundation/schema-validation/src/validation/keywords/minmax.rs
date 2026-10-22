use crate::validation::{
    compiler,
    error::ValidationError,
    keywords::CompilationResult,
    numeric,
    paths::{LazyLocation, Location, RefTracker},
    types::JsonType,
    validator::{Validate, ValidationContext},
    Json, LazyInstance, Node,
};
use json_codec::{Map, Value};
use num_cmp::NumCmp;
use std::borrow::Cow;

macro_rules! define_numeric_keywords {
    ($($struct_name:ident => $fn_name:path => $error_fn_name:ident),* $(,)?) => {
        $(
            #[derive(Debug, Clone, PartialEq)]
            pub(crate) struct $struct_name<T> {
                pub(super) limit: T,
                limit_val: Value,
                location: Location,
            }

            impl<T> From<(T, Value, Location)> for $struct_name<T> {
                fn from((limit, limit_val, location): (T, Value, Location)) -> Self {
                    Self { limit, limit_val, location }
                }
            }

            impl<T, F> Validate<F> for $struct_name<T>
            where
                T: Copy + Send + Sync + ::numeric::traits::ToPrimitive,
                u64: NumCmp<T>,
                i64: NumCmp<T>,
                f64: NumCmp<T>,
                F: Json,
            {
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
                        Err(ValidationError::$error_fn_name(
                            self.location.clone(),
                            crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                            location.into(),
                            instance.lazy_value(),
                            self.limit_val.clone(),
                        ))
                    }
                }

                fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut ValidationContext) -> bool {
                    if let Some(item) = instance.as_number() {
                        $fn_name(&item, self.limit)
                    } else {
                        true
                    }
                }
            }
        )*
    };
}

define_numeric_keywords!(
    Minimum => numeric::ge => minimum,
    Maximum => numeric::le => maximum,
    ExclusiveMinimum => numeric::gt => exclusive_minimum,
    ExclusiveMaximum => numeric::lt => exclusive_maximum,
);

#[cfg(feature = "arbitrary-precision")]
pub(crate) mod bigint_validators {
    use crate::value::JsonNumber;

    use super::{
        numeric, Json, LazyLocation, Location, Node, RefTracker, Validate, ValidationContext,
        ValidationError, Value,
    };
    use crate::validation::numeric::bignum::{
        f64_ge_bigfrac, f64_ge_bigint, f64_gt_bigfrac, f64_gt_bigint, f64_le_bigfrac,
        f64_le_bigint, f64_lt_bigfrac, f64_lt_bigint, i64_ge_bigfrac, i64_ge_bigint,
        i64_gt_bigfrac, i64_gt_bigint, i64_le_bigfrac, i64_le_bigint, i64_lt_bigfrac,
        i64_lt_bigint, try_parse_bigfraction, u64_ge_bigfrac, u64_ge_bigint, u64_gt_bigfrac,
        u64_gt_bigint, u64_le_bigfrac, u64_le_bigint, u64_lt_bigfrac, u64_lt_bigint,
    };
    use ::numeric::bigint::BigInt;

    macro_rules! define_bigint_validator {
        ($struct_name:ident, $u64_cmp:path, $i64_cmp:path, $f64_cmp:path, $bigint_op:tt, $error_fn:ident, $infinity_fn:expr) => {
            #[derive(Debug, Clone, PartialEq)]
            pub(crate) struct $struct_name {
                pub(super) limit: BigInt,
                pub(super) limit_val: Value,
                pub(super) location: Location,
            }

            impl $struct_name {
                pub(crate) fn new(limit: BigInt, limit_val: Value, location: Location) -> Self {
                    Self { limit, limit_val, location }
                }
            }

            impl<F: Json> Validate<F> for $struct_name {
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
                        Err(ValidationError::$error_fn(
                            self.location.clone(),
                            crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                            location.into(),
                            instance.lazy_value(),
                            self.limit_val.clone(),
                        ))
                    }
                }

                fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut  ValidationContext) -> bool {
                    use fraction::BigFraction;
                    if let Some(item) = instance.as_number() {
                        // Try to parse instance as BigInt first
                        if let Some(instance_bigint) = numeric::bignum::try_parse_bigint(&item.to_number()) {
                            // Both are BigInt - direct comparison
                            instance_bigint $bigint_op self.limit
                        } else if let Some(v) = item.as_u64() {
                            $u64_cmp(v, &self.limit)
                        } else if let Some(v) = item.as_i64() {
                            $i64_cmp(v, &self.limit)
                        } else if let Some(v) = item.as_f64() {
                            $f64_cmp(v, &self.limit)
                        } else {
                            // Number doesn't fit in f64 (e.g., 1e1000)
                            // Since limit is BigInt, we need to compare with BigFraction
                            if let Some(instance_bigfrac) = numeric::bignum::try_parse_bigfraction(&item.to_number()) {
                                // Convert BigInt limit to BigFraction for comparison
                                // Use clone to avoid truncation through i128
                                let limit_frac = BigFraction::from(self.limit.clone());
                                instance_bigfrac $bigint_op limit_frac
                            } else {
                                // Can't parse as BigFraction - extremely large scientific notation
                                // (e.g., "1e10000" which exceeds BigFraction's parsing capability)
                                // These behave like positive/negative infinity for comparison purposes
                                let is_negative = item.as_str().starts_with('-');
                                $infinity_fn(is_negative)
                            }
                        }
                    } else {
                        true
                    }
                }
            }
        };
    }

    define_bigint_validator!(
        BigIntMinimum,
        u64_ge_bigint,
        i64_ge_bigint,
        f64_ge_bigint,
        >=,
        minimum,
        |is_negative: bool| !is_negative  // For >= and >, positive infinity passes
    );
    define_bigint_validator!(
        BigIntMaximum,
        u64_le_bigint,
        i64_le_bigint,
        f64_le_bigint,
        <=,
        maximum,
        |is_negative: bool| is_negative  // For <= and <, negative infinity passes
    );
    define_bigint_validator!(
        BigIntExclusiveMinimum,
        u64_gt_bigint,
        i64_gt_bigint,
        f64_gt_bigint,
        >,
        exclusive_minimum,
        |is_negative: bool| !is_negative  // For >= and >, positive infinity passes
    );
    define_bigint_validator!(
        BigIntExclusiveMaximum,
        u64_lt_bigint,
        i64_lt_bigint,
        f64_lt_bigint,
        <,
        exclusive_maximum,
        |is_negative: bool| is_negative  // For <= and <, negative infinity passes
    );

    // BigFraction validators for arbitrary precision floats
    use fraction::BigFraction;

    macro_rules! define_bigfrac_validator {
        ($struct_name:ident, $u64_cmp:path, $i64_cmp:path, $f64_cmp:path, $bigfrac_op:tt, $error_fn:ident) => {
            #[derive(Debug, Clone, PartialEq)]
            pub(crate) struct $struct_name {
                pub(super) limit: BigFraction,
                pub(super) limit_val: Value,
                pub(super) location: Location,
            }

            impl $struct_name {
                pub(crate) fn new(limit: BigFraction, limit_val: Value, location: Location) -> Self {
                    Self { limit, limit_val, location }
                }
            }

            impl<F: Json> Validate<F> for $struct_name {
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
                        Err(ValidationError::$error_fn(
                            self.location.clone(),
                            crate::validation::paths::capture_evaluation_path(tracker, &self.location),
                            location.into(),
                            instance.lazy_value(),
                            self.limit_val.clone(),
                        ))
                    }
                }

                fn is_valid(&self, instance: &F::Node<'_>, _ctx: &mut  ValidationContext) -> bool {
                    if let Some(item) = instance.as_number() {
                        // Try to parse instance as BigFraction for exact precision
                        if let Some(instance_bigfrac) = try_parse_bigfraction(&item.to_number()) {
                            // Both are BigFraction - direct comparison
                            instance_bigfrac $bigfrac_op self.limit
                        } else if let Some(v) = item.as_u64() {
                            $u64_cmp(v, &self.limit)
                        } else if let Some(v) = item.as_i64() {
                            $i64_cmp(v, &self.limit)
                        } else if let Some(instance_bigint) = numeric::bignum::try_parse_bigint(&item.to_number()) {
                            // An integer past i64 is exact as a BigInt, while f64 rounds it onto
                            // limits only a fractional digit separates it from.
                            BigFraction::from(instance_bigint) $bigfrac_op self.limit
                        } else if let Some(v) = item.as_f64() {
                            // Scientific notation or other f64-representable numbers
                            $f64_cmp(v, &self.limit)
                        } else {
                            // Extreme scientific notation beyond f64 range (e.g., 1e2000000)
                            // These are not supported for comparison - treat as always valid
                            // since we can't reliably compare them
                            true
                        }
                    } else {
                        true
                    }
                }
            }
        };
    }

    define_bigfrac_validator!(
        BigFracMinimum,
        u64_ge_bigfrac,
        i64_ge_bigfrac,
        f64_ge_bigfrac,
        >=,
        minimum
    );
    define_bigfrac_validator!(
        BigFracMaximum,
        u64_le_bigfrac,
        i64_le_bigfrac,
        f64_le_bigfrac,
        <=,
        maximum
    );
    define_bigfrac_validator!(
        BigFracExclusiveMinimum,
        u64_gt_bigfrac,
        i64_gt_bigfrac,
        f64_gt_bigfrac,
        >,
        exclusive_minimum
    );
    define_bigfrac_validator!(
        BigFracExclusiveMaximum,
        u64_lt_bigfrac,
        i64_lt_bigfrac,
        f64_lt_bigfrac,
        <,
        exclusive_maximum
    );
}

#[inline]
fn create_validator<T, V, F: Json>(
    ctx: &compiler::Context<F>,
    keyword: &str,
    limit: T,
    schema: &Value,
) -> CompilationResult<'static, F>
where
    V: From<(T, Value, Location)> + Validate<F> + 'static,
{
    let location = ctx.location().join(keyword);
    Ok(Box::new(V::from((limit, schema.clone(), location))))
}

fn number_type_error<'a, F: Json>(
    ctx: &compiler::Context<F>,
    keyword: &str,
    schema: &'a Value,
) -> CompilationResult<'a, F> {
    let location = ctx.location().join(keyword);
    Err(ValidationError::single_type_error(
        location.clone(),
        location,
        Location::new(),
        LazyInstance::Ready(Cow::Borrowed(schema)),
        JsonType::Number,
    ))
}

macro_rules! create_numeric_validator {
    ($validator_type:ident, $ctx:expr, $keyword:expr, $limit:expr, $schema:expr) => {
        if let Some(limit) = $limit.as_u64() {
            Some(create_validator::<_, $validator_type<u64>, _>(
                $ctx, $keyword, limit, $schema,
            ))
        } else if let Some(limit) = $limit.as_i64() {
            Some(create_validator::<_, $validator_type<i64>, _>(
                $ctx, $keyword, limit, $schema,
            ))
        } else {
            #[cfg(feature = "arbitrary-precision")]
            {
                if let Some(result) = create_bigint_validator($ctx, $keyword, $limit, $schema) {
                    return Some(result);
                }
            }
            // Handle numbers that don't fit in f64 (e.g., 1e10000)
            // These are extremely large scientific notation numbers
            if let Some(limit_f64) = $limit.as_f64() {
                Some(create_validator::<_, $validator_type<f64>, _>(
                    $ctx, $keyword, limit_f64, $schema,
                ))
            } else {
                #[cfg(feature = "arbitrary-precision")]
                {
                    // Number is too large for f64 and couldn't be parsed as BigInt/BigFraction
                    // (e.g., "1e10000" with huge exponent exceeding f64 range)
                    // Use actual infinity as the comparison limit based on sign
                    let is_negative = $limit.as_str().starts_with('-');
                    let infinity_limit = if is_negative {
                        f64::NEG_INFINITY
                    } else {
                        f64::INFINITY
                    };
                    Some(create_validator::<_, $validator_type<f64>, _>(
                        $ctx,
                        $keyword,
                        infinity_limit,
                        $schema,
                    ))
                }
                #[cfg(not(feature = "arbitrary-precision"))]
                {
                    unreachable!("as_f64() always returns Some without arbitrary-precision")
                }
            }
        }
    };
}

#[cfg(feature = "arbitrary-precision")]
fn create_bigint_validator<F: Json>(
    ctx: &compiler::Context<F>,
    keyword: &str,
    limit: &json_codec::Number,
    schema: &Value,
) -> Option<CompilationResult<'static, F>> {
    use bigint_validators::{
        BigFracExclusiveMaximum, BigFracExclusiveMinimum, BigFracMaximum, BigFracMinimum,
        BigIntExclusiveMaximum, BigIntExclusiveMinimum, BigIntMaximum, BigIntMinimum,
    };

    // Try BigInt first for large integers
    if let Some(bigint_limit) = numeric::bignum::try_parse_bigint(limit) {
        let location = ctx.location().join(keyword);
        let validator: Box<dyn Validate<F>> = match keyword {
            "minimum" => Box::new(BigIntMinimum::new(bigint_limit, schema.clone(), location)),
            "maximum" => Box::new(BigIntMaximum::new(bigint_limit, schema.clone(), location)),
            "exclusiveMinimum" => Box::new(BigIntExclusiveMinimum::new(
                bigint_limit,
                schema.clone(),
                location,
            )),
            "exclusiveMaximum" => Box::new(BigIntExclusiveMaximum::new(
                bigint_limit,
                schema.clone(),
                location,
            )),
            _ => return None,
        };
        return Some(Ok(validator));
    }

    // If not a BigInt, try BigFraction for exact decimal precision
    if let Some(bigfrac_limit) = numeric::bignum::try_parse_bigfraction(limit) {
        let location = ctx.location().join(keyword);
        let validator: Box<dyn Validate<F>> = match keyword {
            "minimum" => Box::new(BigFracMinimum::new(bigfrac_limit, schema.clone(), location)),
            "maximum" => Box::new(BigFracMaximum::new(bigfrac_limit, schema.clone(), location)),
            "exclusiveMinimum" => Box::new(BigFracExclusiveMinimum::new(
                bigfrac_limit,
                schema.clone(),
                location,
            )),
            "exclusiveMaximum" => Box::new(BigFracExclusiveMaximum::new(
                bigfrac_limit,
                schema.clone(),
                location,
            )),
            _ => return None,
        };
        return Some(Ok(validator));
    }

    None
}

#[inline]
pub(crate) fn compile_minimum<'a, F: Json>(
    ctx: &compiler::Context<F>,
    _: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a, F>> {
    match schema {
        Value::Number(limit) => create_numeric_validator!(Minimum, ctx, "minimum", limit, schema),
        _ => Some(number_type_error(ctx, "minimum", schema)),
    }
}

#[inline]
pub(crate) fn compile_maximum<'a, F: Json>(
    ctx: &compiler::Context<F>,
    _: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a, F>> {
    match schema {
        Value::Number(limit) => create_numeric_validator!(Maximum, ctx, "maximum", limit, schema),
        _ => Some(number_type_error(ctx, "maximum", schema)),
    }
}

#[inline]
pub(crate) fn compile_exclusive_minimum<'a, F: Json>(
    ctx: &compiler::Context<F>,
    _: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a, F>> {
    match schema {
        Value::Number(limit) => {
            create_numeric_validator!(ExclusiveMinimum, ctx, "exclusiveMinimum", limit, schema)
        }
        _ => Some(number_type_error(ctx, "exclusiveMinimum", schema)),
    }
}

#[inline]
pub(crate) fn compile_exclusive_maximum<'a, F: Json>(
    ctx: &compiler::Context<F>,
    _: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a, F>> {
    match schema {
        Value::Number(limit) => {
            create_numeric_validator!(ExclusiveMaximum, ctx, "exclusiveMaximum", limit, schema)
        }
        _ => Some(number_type_error(ctx, "exclusiveMaximum", schema)),
    }
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/schema-validation/upstream/jsonschema/unit/src/keywords/minmax/tests_498.rs"
));
