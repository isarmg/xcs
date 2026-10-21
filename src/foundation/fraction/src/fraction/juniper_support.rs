//! Juniper values conversion for GenericFraction
//! The format is a case-sensitive text representation of the sign and numbers.
//! Finite and infinity inputs require an explicit outer `+` or `-` sign; signs
//! inside numeric components are rejected. The `NaN` special value is the
//! exception because it has no sign. Supported forms are:
//! * Special cases: `NaN`, `+inf`, `-inf`
//! * Zero: `+0`, `-0`
//! * Whole numbers: `+1`, `-2`
//! * Fractions: `+1/2`, `-3/4`

use super::{generic_fraction::parse_magnitude, GenericFraction, Sign};
use generic::GenericInteger;
use juniper::{ParseScalarResult, ParseScalarValue, Value};

impl<__S, T> ::juniper::GraphQLValueAsync<__S> for GenericFraction<T>
where
    Self: Sync,
    Self::TypeInfo: Sync,
    Self::Context: Sync,
    T: Clone + GenericInteger,
    __S: ::juniper::ScalarValue + Send + Sync,
{
    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        selection_set: Option<&'a [::juniper::Selection<__S>]>,
        executor: &'a ::juniper::Executor<Self::Context, __S>,
    ) -> ::juniper::BoxFuture<'a, ::juniper::ExecutionResult<__S>> {
        use juniper::futures::future;
        let v = ::juniper::GraphQLValue::resolve(self, info, selection_set, executor);
        Box::pin(future::ready(v))
    }
}
impl<S, T> ::juniper::marker::IsInputType<S> for GenericFraction<T>
where
    S: ::juniper::ScalarValue,
    T: Clone + GenericInteger,
{
}
impl<S, T> ::juniper::marker::IsOutputType<S> for GenericFraction<T>
where
    S: ::juniper::ScalarValue,
    T: Clone + GenericInteger,
{
}
impl<S, T> ::juniper::GraphQLType<S> for GenericFraction<T>
where
    S: ::juniper::ScalarValue,
    T: Clone + GenericInteger,
{
    fn name(_: &Self::TypeInfo) -> Option<&'static str> {
        Some("Fraction")
    }
    fn meta<'r>(
        info: &Self::TypeInfo,
        registry: &mut ::juniper::Registry<'r, S>,
    ) -> ::juniper::meta::MetaType<'r, S>
    where
        S: 'r,
    {
        registry
            .build_scalar_type::<Self>(info)
            .description("Fraction")
            .into_meta()
    }
}
impl<S, T> ::juniper::GraphQLValue<S> for GenericFraction<T>
where
    S: ::juniper::ScalarValue,
    T: Clone + GenericInteger,
{
    type Context = ();
    type TypeInfo = ();
    fn type_name<'__i>(&self, info: &'__i Self::TypeInfo) -> Option<&'__i str> {
        <Self as ::juniper::GraphQLType<S>>::name(info)
    }
    fn resolve(
        &self,
        _info: &(),
        _selection: Option<&[::juniper::Selection<S>]>,
        _executor: &::juniper::Executor<Self::Context, S>,
    ) -> ::juniper::ExecutionResult<S> {
        Ok(Value::scalar(self.to_string()))
    }
}
impl<S, T> ::juniper::ToInputValue<S> for GenericFraction<T>
where
    S: ::juniper::ScalarValue,
    T: Clone + GenericInteger,
{
    fn to_input_value(&self) -> ::juniper::InputValue<S> {
        ::juniper::ToInputValue::to_input_value(&format!("{:+}", self))
    }
}
impl<S, T> ::juniper::FromInputValue<S> for GenericFraction<T>
where
    S: ::juniper::ScalarValue,
    T: Clone + GenericInteger,
{
    fn from_input_value(value: &::juniper::InputValue<S>) -> Option<Self> {
        {
            let val = value.as_string_value()?;
            if val.len() < 2 {
                None
            } else if val == "NaN" {
                Some(GenericFraction::nan())
            } else if val == "-inf" {
                Some(GenericFraction::neg_infinity())
            } else if val == "+inf" {
                Some(GenericFraction::infinity())
            } else {
                let sign = match val.as_bytes()[0] {
                    43 => Sign::Plus,
                    45 => Sign::Minus,
                    _ => return None,
                };
                let (denom, split): (T, usize) = if let Some(idx) = val.find('/') {
                    let (_, denom) = val.split_at(idx + 1);
                    match parse_magnitude::<T>(denom, 10) {
                        Ok(val) => (val, idx),
                        Err(_) => return None,
                    }
                } else {
                    (T::one(), val.len())
                };
                let (snum, _) = val.split_at(split);
                let (_, num) = snum.split_at(1);
                match parse_magnitude::<T>(num, 10) {
                    Ok(num) => match sign {
                        Sign::Plus => Some(GenericFraction::new(num, denom)),
                        Sign::Minus => Some(GenericFraction::new_neg(num, denom)),
                    },
                    Err(_) => None,
                }
            }
        }
    }
}
impl<S, T> ::juniper::ParseScalarValue<S> for GenericFraction<T>
where
    S: ::juniper::ScalarValue,
    T: Clone + GenericInteger,
{
    fn from_str(value: ::juniper::parser::ScalarToken<'_>) -> ParseScalarResult<'_, S> {
        {
            <String as ParseScalarValue<S>>::from_str(value)
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/fraction/unit/src/fraction/juniper_support/tests_151.rs"));
