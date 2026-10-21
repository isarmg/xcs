//! The implementation for Version 7 UUIDs.
//!
//! Note that you need to enable the `v7` Cargo feature
//! in order to use this module.

use core::cmp;

use crate::{rng, timestamp::Timestamp, Builder, Uuid};

impl Uuid {
    /// Create a new version 7 UUID using the current time value.
    ///
    /// This method is a convenient alternative to [`Uuid::new_v7`] that uses the current system time
    /// as the source timestamp. All UUIDs generated through this method by the same process are
    /// guaranteed to be ordered by their creation.
    #[cfg(feature = "std")]
    pub fn now_v7() -> Self {
        Self::new_v7(Timestamp::now(
            crate::timestamp::context::shared_context_v7(),
        ))
    }

    /// Create a new version 7 UUID using a time value and random bytes.
    ///
    /// When the `std` feature is enabled, you can also use [`Uuid::now_v7`].
    ///
    /// Note that usage of this method requires the `v7` feature of this crate
    /// to be enabled.
    ///
    /// Also see [`Uuid::now_v7`] for a convenient way to generate version 7
    /// UUIDs using the current system time.
    ///
    /// # Counter treatment
    ///
    /// This method accepts a [`Timestamp`] which may include a counter value.
    /// The 74 most significant bits of the counter value are retained when
    /// constructing the UUID, and the rest is filled with random data. Avoid
    /// using a counter wider than 74 bits.
    ///
    /// # Examples
    ///
    /// A v7 UUID can be created from a unix [`Timestamp`] plus a 128 bit
    /// random number. When supplied as such, the data will be combined
    /// to ensure uniqueness and sortability at millisecond granularity.
    ///
    /// ```rust
    /// # use uuid::{Uuid, Timestamp, NoContext};
    /// let ts = Timestamp::from_unix(NoContext, 1497624119, 1234);
    ///
    /// let uuid = Uuid::new_v7(ts);
    ///
    /// assert!(
    ///     uuid.hyphenated().to_string().starts_with("015cb15a-86d8-7")
    /// );
    /// ```
    ///
    /// A v7 UUID can also be created with a counter to ensure batches of
    /// UUIDs created together remain sortable:
    ///
    /// ```rust
    /// # use uuid::{Uuid, Timestamp, ContextV7};
    /// let context = ContextV7::new();
    /// let uuid1 = Uuid::new_v7(Timestamp::from_unix(&context, 1497624119, 1234));
    /// let uuid2 = Uuid::new_v7(Timestamp::from_unix(&context, 1497624119, 1234));
    ///
    /// assert!(uuid1 < uuid2);
    /// ```
    ///
    /// # References
    ///
    /// * [UUID Version 7 in RFC 9562](https://www.ietf.org/rfc/rfc9562.html#section-5.7)
    pub fn new_v7(ts: Timestamp) -> Self {
        let (secs, nanos) = ts.to_unix();
        let millis = secs
            .saturating_mul(1000)
            .saturating_add(nanos as u64 / 1_000_000);

        let (mut counter, counter_bits) = ts.counter();

        // `Builder::from_unix_timestamp_millis` takes the top 80 bits of this value,
        // so the counter is placed directly below the version nibble and shifted
        // around the variant:
        //
        // bit 127                                                       bit 48
        // | ver (4) |    rand_a (12)    | var (2) |       rand_b (62)        | ...
        //           |<- counter <= 12 ->|
        //           |<---- counter > 12: shifted by 2 over the variant ---->|
        const RAND_A_BITS: u32 = 12;
        const PAYLOAD_BITS: u32 = RAND_A_BITS + 62;

        // Retain the most significant bits of a counter wider than the payload
        let mut counter_bits = cmp::min(counter_bits as u32, 128);
        if counter_bits > PAYLOAD_BITS {
            counter >>= counter_bits - PAYLOAD_BITS;
            counter_bits = PAYLOAD_BITS;
        }

        // Shift the counter around the variant field
        if counter_bits > RAND_A_BITS {
            let mask = u128::MAX << (counter_bits - RAND_A_BITS);
            counter = (counter & !mask) | ((counter & mask) << 2);
            counter_bits += 2;
        }

        let counter_and_random = if counter_bits == 0 {
            rng::u128()
        } else {
            let shift = 124 - counter_bits;

            (rng::u128() & (u128::MAX >> (128 - shift))) | (counter << shift)
        };

        Builder::from_unix_timestamp_millis(
            millis,
            &counter_and_random.to_be_bytes()[..10].try_into().unwrap(),
        )
        .into_uuid()
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/uuid/unit/src/v7/tests_121.rs"));
