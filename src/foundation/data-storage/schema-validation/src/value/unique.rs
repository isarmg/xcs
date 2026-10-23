use crate::value::cmp;
use ahash::{AHashSet, AHasher};
use json_codec::Value;
use std::{
    borrow::Borrow,
    hash::{Hash, Hasher},
};

// Based on implementation proposed by Sven Marnach:
// https://stackoverflow.com/questions/60882381/what-is-the-fastest-correct-way-to-detect-that-there-are-no-duplicates-in-a-json
pub(crate) struct HashedValue<'a>(&'a Value);

impl PartialEq for HashedValue<'_> {
    fn eq(&self, other: &Self) -> bool {
        cmp::equal(self.0, other.0)
    }
}

impl Eq for HashedValue<'_> {}

impl Hash for HashedValue<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.0 {
            Value::Null => state.write_u32(3_221_225_473), // chosen randomly
            Value::Bool(ref item) => item.hash(state),
            Value::Number(ref item) => {
                if let Some(number) = item.as_f64() {
                    number_bits(number).hash(state);
                } else if let Some(number) = item.as_u64() {
                    number.hash(state);
                } else if let Some(number) = item.as_i64() {
                    number.hash(state);
                }
            }
            Value::String(ref item) => item.hash(state),
            Value::Array(ref items) => {
                for item in items {
                    HashedValue(item).hash(state);
                }
            }
            Value::Object(ref items) => {
                let mut hash = 0;
                for (key, value) in items {
                    // We have no way of building a new hasher of type `H`, so we
                    // hardcode using the default hasher of a hash map.
                    let mut item_hasher = AHasher::default();
                    key.hash(&mut item_hasher);
                    HashedValue(value).hash(&mut item_hasher);
                    hash ^= item_hasher.finish();
                }
                state.write_u64(hash);
            }
        }
    }
}

// `0` and `-0.0` are equal numbers, so they must hash alike.
pub(crate) fn number_bits(number: f64) -> u64 {
    if number == 0.0 {
        0
    } else {
        number.to_bits()
    }
}

// Empirically calculated threshold after which the validator resorts to hashing.
// Calculated for an array of mixed types, large homogeneous arrays of primitive values might be
// processed faster with different thresholds, but this one gives a good baseline for the common
// case.
pub(crate) const ITEMS_SIZE_THRESHOLD: usize = 15;

// Generic over `Borrow<Value>` so both borrowed `serde_json` slices and the `Cow<Value>` handles
// materialized from other representations run the same duplicate-detection algorithm.
#[inline]
#[must_use]
pub fn is_unique<T: Borrow<Value>>(items: &[T]) -> bool {
    let size = items.len();
    if size <= 1 {
        // Empty arrays and one-element arrays always contain unique elements
        true
    } else if let [first, second] = items {
        !cmp::equal(first.borrow(), second.borrow())
    } else if let [first, second, third] = items {
        !cmp::equal(first.borrow(), second.borrow())
            && !cmp::equal(first.borrow(), third.borrow())
            && !cmp::equal(second.borrow(), third.borrow())
    } else if size <= ITEMS_SIZE_THRESHOLD {
        // If the array size is small enough we can compare all elements pairwise, which will
        // be faster than calculating hashes for each element, even if the algorithm is O(N^2)
        let mut idx = 0_usize;
        while idx < items.len() {
            let mut inner_idx = idx + 1;
            while inner_idx < items.len() {
                if cmp::equal(items[idx].borrow(), items[inner_idx].borrow()) {
                    return false;
                }
                inner_idx += 1;
            }
            idx += 1;
        }
        true
    } else {
        let mut seen = AHashSet::with_capacity(size);
        items
            .iter()
            .map(|item| HashedValue(item.borrow()))
            .all(move |x| seen.insert(x))
    }
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../../test/foundation/schema-validation/upstream/jsonschema-value/unit/src/unique/tests_111.rs"
));
