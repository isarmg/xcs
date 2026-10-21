//! Lossless integer division
//! - The algorithm uses stack only, no introduced heap allocations for calculation
//!   (although underlying integer type implementation may perform those)
//! - Linear in the number of produced digits: O(n), where n is the count of
//!   integral digits plus the requested fractional precision. Note that for
//!   arbitrary-precision integer types (e.g. `num::BigInt`/`num::BigUint`)
//!   each per-digit step is itself not O(1), so the overall cost is
//!   O(n · cost_of_integer_op).
//! - Abstract from a particular integer implementation, may be used on primitive types
//!   (as i32 or u32) as well as complex ones (num::BigInt, num::BigUint)
//!
//! Thus can be efficiently used on any integer type implementing a bunch of required traits
//! (which all primitive ints and num::bigint implement out of the box).
//! Although in that case the underlying math will be using heap.

use error::DivisionError;
use generic::GenericInteger;

use std::cmp::{Eq, Ordering, PartialEq};
use std::fmt::Write;

/// Division state encapsulates remainder and divisor
#[derive(Clone, Debug)]
pub struct DivisionState<I> {
    pub remainder: I,
    pub divisor: I,
}

impl<I> DivisionState<I> {
    pub fn new(remainder: I, divisor: I) -> Self {
        DivisionState { remainder, divisor }
    }
}

impl<I> PartialEq for DivisionState<I>
where
    I: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.remainder.eq(&other.remainder) && self.divisor.eq(&other.divisor)
    }
}

impl<I> Eq for DivisionState<I> where I: Eq {}

/// Divide two numbers and produce every single digit of the whole part of the resulting number
///
/// WARNING: Negative numbers as arguments are not supported.
///
/// Returns remainder of the division
/// If the consumer returns `Ok(true)` keeps on calculation
/// If the consumer returns `Ok(false)` stops calculation and returns the remainder
/// If the consumer returns `Err(_)` the calculation will be stopped and the error will be passed as the result value
///
/// # Examples
///
/// ```
/// use fraction::division::divide_integral;
///
/// let mut result: [u8; 2] = [0; 2];
/// let mut ptr: usize = 0;
///
/// let state = divide_integral(30, 2, |d| {
///     result[ptr] = d;
///     ptr += 1;
///     Ok(true)
/// }).unwrap();
///
/// assert_eq!([1, 5], result);
/// assert_eq!(state.remainder, 0);
/// assert_eq!(state.divisor, 2);
/// ```
///
/// ```
/// use fraction::division::divide_integral;
///
/// let mut result: u8 = 0;
///
/// let state = divide_integral(30, 2, |d| {
///     result = d;
///     Ok(false)
/// }).unwrap();
///
/// assert_eq!(result, 1);
/// assert_eq!(state.remainder, 10);
/// assert_eq!(state.divisor, 2);
/// ```
pub fn divide_integral<I, Consumer>(
    mut dividend: I,
    divisor: I,
    mut consumer: Consumer,
) -> Result<DivisionState<I>, DivisionError>
where
    Consumer: FnMut(u8) -> Result<bool, DivisionError>,
    I: Clone + GenericInteger,
{
    if divisor.is_zero() {
        return Err(DivisionError::DivisionByZero);
    }

    /* === Figuring out the number size === */
    let mut ptr: I = match dividend.cmp(&divisor) {
        Ordering::Equal => {
            consumer(1u8)?;
            return Ok(DivisionState::new(I::zero(), divisor));
        }
        Ordering::Less => {
            consumer(0u8)?;
            return Ok(DivisionState::new(dividend, divisor));
        }
        Ordering::Greater => {
            let mut ptr: I = I::_1();

            loop {
                if ptr > dividend {
                    if I::_1r().map_or_else(|| ptr > I::_1(), |_1| ptr > *_1) {
                        I::_10r().map(|_10| ptr /= _10).or_else(|| {
                            ptr /= I::_10();
                            None
                        });
                    }
                    break;
                }

                ptr = match I::_10r()
                    .map_or_else(|| ptr.checked_mul(&I::_10()), |_10| ptr.checked_mul(_10))
                {
                    Some(n) => n,
                    None => break,
                };
            }

            ptr
        }
    };

    let mut passed_leading_zeroes: bool = false;
    loop {
        let digit = dividend.div_floor(&ptr).div_floor(&divisor);

        if I::_0r().map_or_else(|| digit > I::_0(), |_0| digit > *_0) {
            passed_leading_zeroes = true;
            dividend -= digit.clone() * &divisor * &ptr;
        }

        if passed_leading_zeroes {
            let d: Option<u8> = digit.to_u8();
            match d {
                Some(n) => {
                    if !consumer(n)? {
                        return Ok(DivisionState::new(dividend, divisor));
                    }
                }
                None => unreachable!(),
            };
        }

        if I::_1r().map_or_else(|| ptr == I::_1(), |_1| ptr == *_1) {
            break;
        }

        I::_10r().map(|_10| ptr /= _10).or_else(|| {
            ptr /= I::_10();
            None
        });
    }

    Ok(DivisionState::new(dividend, divisor))
}

/// Produces the fractional part of the decimal from a rest part left after division
///
/// WARNING: Negative numbers as arguments are not supported.
///
/// Returns remainder of the division
/// If the consumer returns `Ok(Ok(state))` keeps on calculation
/// If the consumer returns `Ok(Err(state))` stops calculation and returns the remainder
/// If the consumer returns `Err(_)` the calculation will be stopped and the error will be passed as the result value
///
/// # Examples
///
/// ```
/// use fraction::division::divide_rem;
///
/// let mut result: [u8; 2] = [0; 2];
/// let mut ptr: usize = 0;
///
/// let state = divide_rem(3, 4, |state, digit| {
///     result[ptr] = digit;
///     ptr += 1;
///     Ok(Ok(state))
/// }).unwrap();
///
/// assert_eq!(state.remainder, 0);
/// assert_eq!(state.divisor, 4);
/// assert_eq!(result, [7, 5]);  // 3/4 == 0.75
/// ```
#[inline]
pub fn divide_rem<I, Consumer>(
    dividend: I,
    divisor: I,
    consumer: Consumer,
) -> Result<DivisionState<I>, DivisionError>
where
    Consumer: FnMut(
        DivisionState<I>,
        u8,
    ) -> Result<Result<DivisionState<I>, DivisionState<I>>, DivisionError>,
    I: Clone + GenericInteger,
{
    divide_rem_resume(DivisionState::new(dividend, divisor), consumer)
}

/// [divide_rem] co-routine implementation  
/// Performs the division, changes the state and returns it
///
/// # Examples
///
/// ```
/// use fraction::division::{DivisionState, divide_rem_resume};
///
/// let mut state = Some(DivisionState::new(1, 3));
/// let mut precision = 5;
///
/// let mut result: Vec<u8> = Vec::new();
///
/// loop {
///     if precision == 0 { break }
///
///     state = Some(
///         divide_rem_resume(state.take().unwrap(), |s, digit| {
///             precision -= 1;
///             result.push(digit);
///
///             Ok(Err(s))
///         }).unwrap()
///     );
///
///     // perform some other operations
/// }
///
/// assert_eq!(result, vec![3, 3, 3, 3, 3]);
/// ```
pub fn divide_rem_resume<I, Consumer>(
    mut state: DivisionState<I>,
    mut consumer: Consumer,
) -> Result<DivisionState<I>, DivisionError>
where
    Consumer: FnMut(
        DivisionState<I>,
        u8,
    ) -> Result<Result<DivisionState<I>, DivisionState<I>>, DivisionError>,
    I: Clone + GenericInteger,
{
    loop {
        if state.remainder.is_zero() {
            break;
        }

        let digit = I::_10r()
            .map(|_10| {
                let (rem, digit) = state.remainder.checked_mul(_10).map_or_else(
                    || {
                        let (reduced_divisor, reduced_divisor_rem) = state.divisor.div_rem(_10);

                        let mut digit = state.remainder.div_floor(&reduced_divisor);
                        let mut remainder = state.remainder.clone();

                        remainder -= digit.clone() * &reduced_divisor;

                        let mut red_div_rem_diff =
                            (reduced_divisor_rem.clone() * &digit).div_rem(_10);

                        loop {
                            if red_div_rem_diff.0 > remainder {
                                digit -= I::one();
                                remainder += &reduced_divisor;
                                red_div_rem_diff =
                                    (reduced_divisor_rem.clone() * &digit).div_rem(_10);
                            } else {
                                break;
                            }
                        }

                        remainder -= red_div_rem_diff.0;
                        remainder *= _10;

                        if red_div_rem_diff.1 > remainder {
                            digit -= I::one();
                            remainder += &state.divisor;
                        }

                        remainder -= red_div_rem_diff.1;

                        (remainder, digit.to_u8())
                    },
                    |mut remainder| {
                        let digit = remainder.div_floor(&state.divisor);
                        remainder -= digit.clone() * &state.divisor;

                        (remainder, digit.to_u8())
                    },
                );

                state.remainder = rem;
                digit
            })
            .or_else(|| {
                let ten = I::_10();

                let (rem, digit) = state.remainder.checked_mul(&ten).map_or_else(
                    || {
                        let (reduced_divisor, reduced_divisor_rem) = state.divisor.div_rem(&ten);

                        let mut digit = state.remainder.div_floor(&reduced_divisor);
                        let mut remainder = state.remainder.clone();

                        remainder -= digit.clone() * &reduced_divisor;

                        let mut red_div_rem_diff =
                            (reduced_divisor_rem.clone() * &digit).div_rem(&ten);

                        loop {
                            if red_div_rem_diff.0 > remainder {
                                digit -= I::one();
                                remainder += &reduced_divisor;
                                red_div_rem_diff =
                                    (reduced_divisor_rem.clone() * &digit).div_rem(&ten);
                            } else {
                                break;
                            }
                        }

                        remainder -= red_div_rem_diff.0;
                        remainder *= &ten;

                        if red_div_rem_diff.1 > remainder {
                            digit -= I::one();
                            remainder += &state.divisor;
                        }

                        remainder -= red_div_rem_diff.1;

                        (remainder, digit.to_u8())
                    },
                    |mut remainder: I| {
                        let digit = remainder.div_floor(&state.divisor);
                        remainder -= digit.clone() * &state.divisor;

                        (remainder, digit.to_u8())
                    },
                );

                state.remainder = rem;
                Some(digit)
            })
            .unwrap();

        match digit {
            Some(n) => {
                state = match consumer(state, n)? {
                    Ok(state) => state,
                    Err(s) => return Ok(s),
                };
            }
            None => unreachable!(),
        };
    }

    Ok(state)
}

/// Calculate the max possible length of division in characters (including floating point)
/// This may be useful for string/vector pre-allocations
///
/// WARNING: Negative numbers as arguments are not supported.
///
/// # Examples
/// ```
/// use fraction::division::division_result_max_char_length;
///
/// assert_eq!(division_result_max_char_length(&1, 0), 1);
/// assert_eq!(division_result_max_char_length(&10, 0), 2);
/// assert_eq!(division_result_max_char_length(&10, 1), 4);
/// assert_eq!(division_result_max_char_length(&100, 2), 6);
/// assert_eq!(division_result_max_char_length(&900, 2), 6);
/// ```
pub fn division_result_max_char_length<I>(dividend: &I, precision: usize) -> usize
where
    I: Clone + GenericInteger,
{
    let mut ptr: I = I::_1();
    let mut len: usize = 0;

    loop {
        len += 1;

        ptr = match I::_10r().map_or_else(|| ptr.checked_mul(&I::_10()), |_10| ptr.checked_mul(_10))
        {
            Some(n) => n,
            None => break,
        };

        if ptr > *dividend {
            break;
        }
    }

    len + precision + usize::from(precision > 0)
}

/// Divide a fraction into a [`String`]
///
/// WARNING: Negative numbers as arguments are not supported.
///
/// - Makes only one allocation for the resulting string
/// - Does not round the last digit
///
/// Calculates the resulting string length first, allocates it,
/// then makes the division and puts the result into the preallocated
/// string.
///
/// # Examples
///
/// ```
/// use fraction::division::divide_to_string;
///
/// assert_eq! (divide_to_string(2, 4, 2, false).unwrap(), "0.5");
/// assert_eq! (divide_to_string(2, 4, 2, true).unwrap(), "0.50");
/// assert_eq! (divide_to_string(5, 7, 16, false).unwrap(), "0.7142857142857142");
/// assert_eq! (divide_to_string(1, 3, 3, false).unwrap(), "0.333");
/// assert_eq! (divide_to_string(1, 3, 3, true).unwrap(), "0.333");
/// ```
///
/// [`String`]: https://doc.rust-lang.org/std/string/struct.String.html
/// [`divide_integral`]: ./fn.divide_integral.html
/// [`divide_rem`]: ./fn.divide_rem.html
#[inline]
pub fn divide_to_string<I>(
    dividend: I,
    divisor: I,
    precision: usize,
    trail_zeroes: bool,
) -> Result<String, DivisionError>
where
    I: Clone + GenericInteger,
{
    let mut result = String::with_capacity(division_result_max_char_length(&dividend, precision));

    divide_to_writeable(&mut result, dividend, divisor, precision, trail_zeroes)?;

    Ok(result)
}

/// Divide a fraction into a [`Vec<u8>`] of ASCII(utf8) chars
///
/// WARNING: Negative numbers as arguments are not supported.
///
/// - Makes only one allocation for the resulting vector
/// - Does not round the last digit
///
/// Calculates the resulting vector length first, allocates it,
/// then makes the division and puts the result into the preallocated
/// vector.
/// Uses [`divide_integral`] and [`divide_rem`] functions internally.
///
/// # Examples
///
/// ```
/// use fraction::division::divide_to_ascii_vec;
///
/// assert_eq! (divide_to_ascii_vec(2, 4, 2, false).unwrap(), vec![48, 46, 53]);  // "0.5" in ascii
/// assert_eq! (divide_to_ascii_vec(2, 4, 2, true).unwrap(), vec![48, 46, 53, 48]);  // "0.50" in ascii
/// assert_eq! (divide_to_ascii_vec(5, 7, 16, false).unwrap(), vec![48, 46, 55, 49, 52, 50, 56, 53, 55, 49, 52, 50, 56, 53, 55, 49, 52, 50]);  // "0.7142857142857142" in ascii
/// assert_eq! (divide_to_ascii_vec(1, 3, 3, false).unwrap(), vec![48, 46, 51, 51, 51]);  // "0.333" in ascii
/// ```
///
/// [`Vec<u8>`]: https://doc.rust-lang.org/std/vec/struct.Vec.html
/// [`divide_integral`]: ./fn.divide_integral.html
/// [`divide_rem`]: ./fn.divide_rem.html
#[inline]
pub fn divide_to_ascii_vec<I>(
    dividend: I,
    divisor: I,
    precision: usize,
    trail_zeroes: bool,
) -> Result<Vec<u8>, DivisionError>
where
    I: Clone + GenericInteger,
{
    const ZERO: u8 = 48u8;
    const DOT: u8 = 46u8;

    let mut result = Vec::with_capacity(division_result_max_char_length(&dividend, precision));

    divide_to_callback(dividend, divisor, precision, trail_zeroes, |digit| {
        if digit == 10u8 {
            result.push(DOT);
        } else {
            result.push(ZERO + digit);
        }
        Ok(true)
    })?;

    Ok(result)
}

/// Divide a fraction into a writeable target implementing [`std::fmt::Write`]  
/// Returns the remainder of the division
///
/// - No allocations
/// - Does not round the last digit
///
/// Makes the division and puts the result into the formatter.
/// Uses [`divide_integral`] and [`divide_rem`] functions internally.
///
/// WARNING: Negative numbers as arguments are not supported.
///
/// # Examples
///
/// ```
/// use fraction::division::{ divide_to_writeable, division_result_max_char_length };
///
/// let num = 7;
/// let denom = 4;
///
/// let mut string = String::with_capacity(division_result_max_char_length(&num, 2));
///
/// divide_to_writeable(&mut string, num, denom, 2, false).ok();
///
/// assert_eq!(string, "1.75");
/// ```
///
/// ```
/// use fraction::division::divide_to_writeable;
///
/// use std::fmt;
///
/// struct Foo(i32, i32);
///
/// impl fmt::Display for Foo {
///     fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
///         let precision = formatter.precision().unwrap_or(2);
///         match divide_to_writeable(formatter, self.0, self.1, precision, false) {
///             Err(_) => Err(fmt::Error),
///             _ => Ok(())
///         }
///     }
/// }
///
/// assert_eq! (format!("{}", Foo(1, 2)), "0.5");
/// assert_eq! (format!("{:.3}", Foo(1, 3)), "0.333");
/// assert_eq! (format!("{:.16}", Foo(5, 7)), "0.7142857142857142");
/// ```
///
/// [`std::fmt::Write`]: https://doc.rust-lang.org/std/fmt/trait.Write.html
/// [`divide_integral`]: ./fn.divide_integral.html
/// [`divide_rem`]: ./fn.divide_rem.html
#[inline]
pub fn divide_to_writeable<I>(
    writeable: &mut dyn Write,
    dividend: I,
    divisor: I,
    precision: usize,
    trail_zeroes: bool,
) -> Result<I, DivisionError>
where
    I: Clone + GenericInteger,
{
    divide_to_callback(dividend, divisor, precision, trail_zeroes, |digit| {
        write_digit(writeable, digit)
    })
}

/// Calculate the division result and pass every character into the callback  
/// Returns the remainder of the division
///
/// - Makes no allocations
///
/// Uses [`divide_integral`] and [`divide_rem`] functions internally.
///
/// WARNING: Negative numbers as arguments are not supported.
///
/// # Callback
///
/// Callback receives every digit of the result as a u8 value.
/// The floating point character (dot) is passed to the callback as `10u8`.
/// If the callback returns `Ok(true)`, the function keeps on calculation
/// If the callback returns `Ok(false)`, the function stops calculation and returns the remainder
/// If the callback returns `Err(_)` the calculation will be stopped and the error will propagate as the result value
///
/// # Examples
///
/// ```
/// use fraction::division::divide_to_callback;
///
/// let mut result = Vec::new();
///
/// // calculate 7/4, which is 1.75
/// divide_to_callback(7, 4, 2, false, |d| { result.push(d as i32); Ok(true) }).ok();
///
/// assert_eq!(&result, &[1, 10, 7, 5]);
/// ```
pub fn divide_to_callback<I, C>(
    dividend: I,
    divisor: I,
    mut precision: usize,
    trail_zeroes: bool,
    mut callback: C,
) -> Result<I, DivisionError>
where
    C: FnMut(u8) -> Result<bool, DivisionError>,
    I: Clone + GenericInteger,
{
    let mut keep_going = true;

    let mut div_state = divide_integral(dividend, divisor, |digit: u8| match callback(digit) {
        result @ Ok(false) => {
            keep_going = false;
            result
        }
        result => result,
    })?;

    if !keep_going {
        return Ok(div_state.remainder);
    }

    if precision > 0 {
        let mut dot = false;
        let mut trailing_zeroes = 0;

        if !div_state.remainder.is_zero() {
            div_state = divide_rem(
                div_state.remainder,
                div_state.divisor,
                |state, digit: u8| {
                    precision -= 1;

                    if digit == 0 {
                        trailing_zeroes += 1;
                    } else {
                        if !dot {
                            dot = true;

                            match callback(10u8) {
                                result @ Ok(false) => {
                                    keep_going = false;
                                    result
                                }
                                result => result,
                            }?;

                            if !keep_going {
                                return Ok(Err(state));
                            }
                        }

                        while trailing_zeroes > 0 {
                            trailing_zeroes -= 1;

                            match callback(0u8) {
                                result @ Ok(false) => {
                                    keep_going = false;
                                    result
                                }
                                result => result,
                            }?;

                            if !keep_going {
                                return Ok(Err(state));
                            }
                        }

                        match callback(digit) {
                            result @ Ok(false) => {
                                keep_going = false;
                                result
                            }
                            result => result,
                        }?;

                        if !keep_going {
                            return Ok(Err(state));
                        }
                    }

                    if precision > 0 {
                        Ok(Ok(state))
                    } else {
                        Ok(Err(state))
                    }
                },
            )?;
        }

        if keep_going && trail_zeroes {
            if !dot {
                match callback(10u8) {
                    result @ Ok(false) => {
                        keep_going = false;
                        result
                    }
                    result => result,
                }?;

                if !keep_going {
                    return Ok(div_state.remainder);
                }
            }

            while trailing_zeroes > 0 {
                trailing_zeroes -= 1;

                match callback(0u8) {
                    result @ Ok(false) => {
                        keep_going = false;
                        result
                    }
                    result => result,
                }?;

                if !keep_going {
                    return Ok(div_state.remainder);
                }
            }

            while precision > 0 {
                precision -= 1;

                match callback(0u8) {
                    result @ Ok(false) => {
                        keep_going = false;
                        result
                    }
                    result => result,
                }?;

                if !keep_going {
                    return Ok(div_state.remainder);
                }
            }
        }
    }

    Ok(div_state.remainder)
}

/// A helper function to use in conjunction with [divide_to_callback]
///
/// divide_to_callback passes digits to its callback, this function can
/// be used to write digits (and dots) into the writeable buffer after
/// your callback performs its side-effects.
///
/// # Examples
///
/// ```
/// use fraction::division::{divide_to_callback, write_digit};
///
/// let mut result = String::new();
/// let mut length = 0;
///
/// // calculate 7/4, which is 1.75
/// divide_to_callback(7, 4, 2, false, |d| { length += 1; write_digit(&mut result, d) }).ok();
///
/// assert_eq!(&result, "1.75");
/// assert_eq!(length, result.len());
/// ```
pub fn write_digit(writeable: &mut dyn Write, digit: u8) -> Result<bool, DivisionError> {
    if digit == 10u8 {
        match writeable.write_char('.') {
            Ok(_) => Ok(true),
            Err(e) => Err(DivisionError::from(e)),
        }
    } else {
        match writeable.write_fmt(format_args!("{}", digit)) {
            Ok(_) => Ok(true),
            Err(e) => Err(DivisionError::from(e)),
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/fraction/unit/src/division/tests_783.rs"));
