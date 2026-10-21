// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

use crate::codepointtrie::CodePointMapRange;

/// This is an iterator that coalesces adjacent ranges in an iterator over code
/// point ranges
pub(crate) struct RangeListIteratorCoalescer<I, T> {
    iter: I,
    peek: Option<CodePointMapRange<T>>,
}

impl<I, T: Eq> RangeListIteratorCoalescer<I, T>
where
    I: Iterator<Item = CodePointMapRange<T>>,
{
    pub fn new(iter: I) -> Self {
        Self { iter, peek: None }
    }
}

impl<I, T: Eq> Iterator for RangeListIteratorCoalescer<I, T>
where
    I: Iterator<Item = CodePointMapRange<T>>,
{
    type Item = CodePointMapRange<T>;

    fn next(&mut self) -> Option<Self::Item> {
        // Get the initial range we're working with: either a leftover
        // range from last time, or the next range
        let mut ret = if let Some(peek) = self.peek.take() {
            peek
        } else {
            // Exit early when there are no ranges
            self.iter.next()?
        };

        // Keep pulling ranges
        #[expect(clippy::while_let_on_iterator)]
        // can't move the iterator, also we want it to be explicit that we're not draining the iterator
        while let Some(next) = self.iter.next() {
            if *next.range.start() == ret.range.end() + 1 && next.value == ret.value {
                // Range has no gap, coalesce
                ret.range = *ret.range.start()..=*next.range.end();
            } else {
                // Range has a gap, return what we have so far, update
                // peek
                self.peek = Some(next);
                return Some(ret);
            }
        }

        // Ran out of elements, exit
        Some(ret)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/icu_collections/unit/src/iterator_utils/tests_59.rs"));
