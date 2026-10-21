#![cfg(test)]
#![allow(unused_assignments)]

// These tests are primarily targeting "abusive" producers that will
// try to drive the "collect consumer" incorrectly. These should
// result in panics.

use super::collect_with_consumer;
use crate::iter::plumbing::*;
use rayon_core::join;

use std::fmt;
use std::panic;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::Result as ThreadResult;

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon/functions/src/iter/collect/test/produce_too_many_items_17.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon/functions/src/iter/collect/test/produce_fewer_items_32.rs"));

// Complete is not called by the consumer. Hence,the collection vector is not fully initialized.
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon/functions/src/iter/collect/test/left_produces_items_with_no_complete_47.rs"));

// Complete is not called by the right consumer. Hence,the
// collection vector is not fully initialized.
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon/functions/src/iter/collect/test/right_produces_items_with_no_complete_63.rs"));

// Complete is not called by the consumer. Hence,the collection vector is not fully initialized.
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon/functions/src/iter/collect/test/produces_items_with_no_complete_78.rs"));

// The left consumer produces too many items while the right
// consumer produces correct number.
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon/functions/src/iter/collect/test/left_produces_too_many_items_98.rs"));

// The right consumer produces too many items while the left
// consumer produces correct number.
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon/functions/src/iter/collect/test/right_produces_too_many_items_115.rs"));

// The left consumer produces fewer items while the right
// consumer produces correct number.
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon/functions/src/iter/collect/test/left_produces_fewer_items_132.rs"));

// The left and right consumer produce the correct number but
// only left result is returned
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon/functions/src/iter/collect/test/only_left_result_151.rs"));

// The left and right consumer produce the correct number but
// only right result is returned
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon/functions/src/iter/collect/test/only_right_result_169.rs"));

// The left and right consumer produce the correct number but reduce
// in the wrong order.
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon/functions/src/iter/collect/test/reducer_does_not_preserve_order_186.rs"));

// The right consumer produces fewer items while the left
// consumer produces correct number.
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon/functions/src/iter/collect/test/right_produces_fewer_items_205.rs"));

// The left consumer panics and the right stops short, like `panic_fuse()`.
// We should get the left panic without finishing `collect_with_consumer`.
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon/functions/src/iter/collect/test/left_panics_224.rs"));

// The right consumer panics and the left stops short, like `panic_fuse()`.
// We should get the right panic without finishing `collect_with_consumer`.
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon/functions/src/iter/collect/test/right_panics_250.rs"));

// The left consumer produces fewer items while the right
// consumer produces correct number; check that created elements are dropped
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon/functions/src/iter/collect/test/left_produces_fewer_items_drops_276.rs"));

/// This counter can create elements, and then count and verify
/// the number of which have actually been dropped again.
#[derive(Default)]
struct DropCounter {
    created: AtomicUsize,
    dropped: AtomicUsize,
}

struct Element<'a>(&'a AtomicUsize);

impl DropCounter {
    fn created(&self) -> usize {
        self.created.load(Ordering::SeqCst)
    }

    fn dropped(&self) -> usize {
        self.dropped.load(Ordering::SeqCst)
    }

    fn element(&self) -> Element<'_> {
        self.created.fetch_add(1, Ordering::SeqCst);
        Element(&self.dropped)
    }

    fn assert_drop_count(&self) {
        assert_eq!(
            self.created(),
            self.dropped(),
            "Expected {} dropped elements, but found {}",
            self.created(),
            self.dropped()
        );
    }
}

impl<'a> Drop for Element<'a> {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

/// Assert that the result from catch_unwind is a panic that contains expected message
fn assert_is_panic_with_message<T>(result: &ThreadResult<T>, expected: &str)
where
    T: fmt::Debug,
{
    match result {
        Ok(value) => {
            panic!("assertion failure: Expected panic, got successful {value:?}");
        }
        Err(error) => {
            let message_str = error.downcast_ref::<&'static str>().cloned();
            let message_string = error.downcast_ref::<String>().map(String::as_str);
            if let Some(message) = message_str.or(message_string) {
                if !message.contains(expected) {
                    panic!(
                        "assertion failure: Expected {expected:?}, but found panic with {message:?}"
                    );
                }
            // assertion passes
            } else {
                panic!(
                    "assertion failure: Expected {expected:?}, but found panic with unknown value"
                );
            }
        }
    }
}
