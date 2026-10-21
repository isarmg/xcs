// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! External iterators for generic mathematics
//!
//! ## Compatibility
//!
//! The `num-iter` crate is tested for rustc 1.31 and greater.


use core::ops::{Add, Bound, RangeBounds, Sub};
use crate::integer::Integer;
use crate::traits::{CheckedAdd, One, ToPrimitive, Zero};

/// An iterator over the range [start, stop)
#[derive(Clone)]
pub struct Range<A> {
    state: A,
    stop: A,
    one: A,
}

/// Returns an iterator over the given range [start, stop) (that is, starting
/// at start (inclusive), and ending at stop (exclusive)).
///
/// # Example
///
/// ```rust
/// let array = [0, 1, 2, 3, 4];
///
/// for i in numeric::iter::range(0, 5) {
///     println!("{}", i);
///     assert_eq!(i,  array[i]);
/// }
/// ```
#[inline]
pub fn range<A>(start: A, stop: A) -> Range<A>
where
    A: Add<A, Output = A> + PartialOrd + Clone + One,
{
    Range {
        state: start,
        stop,
        one: One::one(),
    }
}

#[inline]
fn unsigned<T: ToPrimitive>(x: &T) -> Option<u128> {
    match x.to_u128() {
        Some(u) => Some(u),
        None => Some(x.to_i128()? as u128),
    }
}

impl<A> RangeBounds<A> for Range<A> {
    fn start_bound(&self) -> Bound<&A> {
        Bound::Included(&self.state)
    }

    fn end_bound(&self) -> Bound<&A> {
        Bound::Excluded(&self.stop)
    }
}

// FIXME: rust-lang/rust#10414: Unfortunate type bound
impl<A> Iterator for Range<A>
where
    A: Add<A, Output = A> + PartialOrd + Clone + ToPrimitive,
{
    type Item = A;

    #[inline]
    fn next(&mut self) -> Option<A> {
        if self.state < self.stop {
            let result = self.state.clone();
            self.state = self.state.clone() + self.one.clone();
            Some(result)
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        // Check for empty ranges first.
        if self.state >= self.stop {
            return (0, Some(0));
        }

        // Try to cast both ends to the largest unsigned primitive.
        // Note that negative values will wrap to a large positive.
        if let Some(a) = unsigned(&self.state) {
            if let Some(b) = unsigned(&self.stop) {
                // We've lost signs, but we already know state < stop, so
                // a `wrapping_sub` will give the correct unsigned delta.
                return match b.wrapping_sub(a).to_usize() {
                    Some(len) => (len, Some(len)),
                    None => (usize::MAX, None),
                };
            }
        }

        // Standard fallback for unbounded/unrepresentable bounds
        (0, None)
    }
}

/// `Integer` is required to ensure the range will be the same regardless of
/// the direction it is consumed.
impl<A> DoubleEndedIterator for Range<A>
where
    A: Integer + Clone + ToPrimitive,
{
    #[inline]
    fn next_back(&mut self) -> Option<A> {
        if self.stop > self.state {
            self.stop.dec();
            Some(self.stop.clone())
        } else {
            None
        }
    }
}

/// An iterator over the range [start, stop]
#[derive(Clone)]
pub struct RangeInclusive<A> {
    range: Range<A>,
    done: bool,
}

/// Return an iterator over the range [start, stop]
#[inline]
pub fn range_inclusive<A>(start: A, stop: A) -> RangeInclusive<A>
where
    A: Add<A, Output = A> + PartialOrd + Clone + One,
{
    RangeInclusive {
        range: range(start, stop),
        done: false,
    }
}

impl<A> RangeBounds<A> for RangeInclusive<A> {
    fn start_bound(&self) -> Bound<&A> {
        Bound::Included(&self.range.state)
    }

    fn end_bound(&self) -> Bound<&A> {
        Bound::Included(&self.range.stop)
    }
}

impl<A> Iterator for RangeInclusive<A>
where
    A: Add<A, Output = A> + PartialOrd + Clone + ToPrimitive,
{
    type Item = A;

    #[inline]
    fn next(&mut self) -> Option<A> {
        match self.range.next() {
            Some(x) => Some(x),
            None => {
                if !self.done && self.range.state == self.range.stop {
                    self.done = true;
                    Some(self.range.stop.clone())
                } else {
                    None
                }
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lo, hi) = self.range.size_hint();
        if self.done {
            (lo, hi)
        } else {
            let lo = lo.saturating_add(1);
            let hi = match hi {
                Some(x) => x.checked_add(1),
                None => None,
            };
            (lo, hi)
        }
    }
}

impl<A> DoubleEndedIterator for RangeInclusive<A>
where
    A: Sub<A, Output = A> + Integer + Clone + ToPrimitive,
{
    #[inline]
    fn next_back(&mut self) -> Option<A> {
        if self.range.stop > self.range.state {
            let result = self.range.stop.clone();
            self.range.stop.dec();
            Some(result)
        } else if !self.done && self.range.state == self.range.stop {
            self.done = true;
            Some(self.range.stop.clone())
        } else {
            None
        }
    }
}

/// An iterator over the range [start, stop) by `step`. It handles overflow by stopping.
#[derive(Clone)]
pub struct RangeStep<A> {
    state: A,
    stop: A,
    step: A,
    rev: bool,
}

/// Return an iterator over the range [start, stop) by `step`. It handles overflow by stopping.
#[inline]
pub fn range_step<A>(start: A, stop: A, step: A) -> RangeStep<A>
where
    A: CheckedAdd + PartialOrd + Clone + Zero,
{
    let rev = step < Zero::zero();
    RangeStep {
        state: start,
        stop,
        step,
        rev,
    }
}

impl<A> Iterator for RangeStep<A>
where
    A: CheckedAdd + PartialOrd + Clone,
{
    type Item = A;

    #[inline]
    fn next(&mut self) -> Option<A> {
        if (self.rev && self.state > self.stop) || (!self.rev && self.state < self.stop) {
            let result = self.state.clone();
            match self.state.checked_add(&self.step) {
                Some(x) => self.state = x,
                None => self.state = self.stop.clone(),
            }
            Some(result)
        } else {
            None
        }
    }
}

/// An iterator over the range [start, stop] by `step`. It handles overflow by stopping.
#[derive(Clone)]
pub struct RangeStepInclusive<A> {
    state: A,
    stop: A,
    step: A,
    rev: bool,
    done: bool,
}

/// Return an iterator over the range [start, stop] by `step`. It handles overflow by stopping.
#[inline]
pub fn range_step_inclusive<A>(start: A, stop: A, step: A) -> RangeStepInclusive<A>
where
    A: CheckedAdd + PartialOrd + Clone + Zero,
{
    let rev = step < Zero::zero();
    RangeStepInclusive {
        state: start,
        stop,
        step,
        rev,
        done: false,
    }
}

impl<A> Iterator for RangeStepInclusive<A>
where
    A: CheckedAdd + PartialOrd + Clone + PartialEq,
{
    type Item = A;

    #[inline]
    fn next(&mut self) -> Option<A> {
        if !self.done
            && ((self.rev && self.state >= self.stop) || (!self.rev && self.state <= self.stop))
        {
            let result = self.state.clone();
            match self.state.checked_add(&self.step) {
                Some(x) => self.state = x,
                None => self.done = true,
            }
            Some(result)
        } else {
            None
        }
    }
}

/// An iterator over the infinite range starting at `start`
#[derive(Clone)]
pub struct RangeFrom<A> {
    state: A,
    one: A,
}

/// Return an iterator over the infinite range starting at `start` and continuing forever.
///
/// *Note*: Currently, the `Iterator` implementation is not checked for overflow.
/// If you use a finite-sized integer type and the integer overflows,
/// it might panic in debug mode or wrap around in release mode.
/// **This behavior is not guaranteed and may change at any time.**
#[inline]
pub fn range_from<A>(start: A) -> RangeFrom<A>
where
    A: Add<A, Output = A> + Clone + One,
{
    RangeFrom {
        state: start,
        one: One::one(),
    }
}

impl<A> RangeBounds<A> for RangeFrom<A> {
    fn start_bound(&self) -> Bound<&A> {
        Bound::Included(&self.state)
    }

    fn end_bound(&self) -> Bound<&A> {
        Bound::Unbounded
    }
}

impl<A> Iterator for RangeFrom<A>
where
    A: Add<A, Output = A> + Clone,
{
    type Item = A;

    #[inline]
    fn next(&mut self) -> Option<A> {
        let result = self.state.clone();
        self.state = self.state.clone() + self.one.clone();
        Some(result)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (usize::MAX, None)
    }
}

/// An iterator over the infinite range starting at `start` by `step`
#[derive(Clone)]
pub struct RangeStepFrom<A> {
    state: A,
    step: A,
}

/// Return an iterator over the infinite range starting at `start` and continuing forever by `step`.
///
/// *Note*: Currently, the `Iterator` implementation is not checked for overflow.
/// If you use a finite-sized integer type and the integer overflows,
/// it might panic in debug mode or wrap around in release mode.
/// **This behavior is not guaranteed and may change at any time.**
#[inline]
pub fn range_step_from<A>(start: A, step: A) -> RangeStepFrom<A>
where
    A: Add<A, Output = A> + Clone,
{
    RangeStepFrom { state: start, step }
}

impl<A> Iterator for RangeStepFrom<A>
where
    A: Add<A, Output = A> + Clone,
{
    type Item = A;

    #[inline]
    fn next(&mut self) -> Option<A> {
        let result = self.state.clone();
        self.state = self.state.clone() + self.step.clone();
        Some(result)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (usize::MAX, None)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/numeric/iter/unit/src/lib/tests_407.rs"));
