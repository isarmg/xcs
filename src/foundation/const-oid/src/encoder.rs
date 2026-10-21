//! OID encoder with `const` support.

use crate::{
    Arc, Buffer, Error, ObjectIdentifier, Result,
    arcs::{ARC_MAX_FIRST, ARC_MAX_SECOND},
};

/// BER/DER encoder.
#[derive(Debug)]
pub(crate) struct Encoder<const MAX_SIZE: usize> {
    /// Current state.
    state: State,

    /// Bytes of the OID being BER-encoded in-progress.
    bytes: [u8; MAX_SIZE],

    /// Current position within the byte buffer.
    cursor: usize,
}

/// Current state of the encoder.
#[derive(Debug)]
enum State {
    /// Initial state - no arcs yet encoded.
    Initial,

    /// First arc has been supplied and stored as the wrapped [`Arc`].
    FirstArc(Arc),

    /// Encoding base 128 body of the OID.
    Body,
}

impl<const MAX_SIZE: usize> Encoder<MAX_SIZE> {
    /// Create a new encoder initialized to an empty default state.
    pub(crate) const fn new() -> Self {
        Self {
            state: State::Initial,
            bytes: [0u8; MAX_SIZE],
            cursor: 0,
        }
    }

    /// Extend an existing OID.
    pub(crate) const fn extend(oid: ObjectIdentifier<MAX_SIZE>) -> Self {
        Self {
            state: State::Body,
            bytes: oid.ber.bytes,
            cursor: oid.ber.length as usize,
        }
    }

    /// Encode an [`Arc`] as base 128 into the internal buffer.
    pub(crate) const fn arc(mut self, arc: Arc) -> Result<Self> {
        match self.state {
            State::Initial => {
                if arc > ARC_MAX_FIRST {
                    return Err(Error::ArcInvalid { arc });
                }

                self.state = State::FirstArc(arc);
                Ok(self)
            }
            State::FirstArc(first_arc) => {
                if arc > ARC_MAX_SECOND {
                    return Err(Error::ArcInvalid { arc });
                }

                self.state = State::Body;
                self.bytes[0] = checked_add!(
                    checked_mul!(checked_add!(ARC_MAX_SECOND, 1), first_arc),
                    arc
                ) as u8;
                self.cursor = 1;
                Ok(self)
            }
            State::Body => self.encode_base128(arc),
        }
    }

    /// Finish encoding an OID.
    pub(crate) const fn finish(self) -> Result<ObjectIdentifier<MAX_SIZE>> {
        if self.cursor == 0 {
            return Err(Error::Empty);
        }

        let ber = Buffer {
            bytes: self.bytes,
            length: self.cursor as u8,
        };

        Ok(ObjectIdentifier { ber })
    }

    /// Encode base 128.
    const fn encode_base128(mut self, arc: Arc) -> Result<Self> {
        let nbytes = base128_len(arc);
        let end_pos = checked_add!(self.cursor, nbytes);

        if end_pos > MAX_SIZE {
            return Err(Error::Length);
        }

        let mut i = 0;
        while i < nbytes {
            // TODO(tarcieri): use `?` when stable in `const fn`
            self.bytes[self.cursor] = match base128_byte(arc, i, nbytes) {
                Ok(byte) => byte,
                Err(e) => return Err(e),
            };
            self.cursor = checked_add!(self.cursor, 1);
            i = checked_add!(i, 1);
        }

        Ok(self)
    }
}

/// Compute the length of an arc when encoded in base 128.
const fn base128_len(arc: Arc) -> usize {
    match arc {
        0..=0x7f => 1,              // up to 7 bits
        0x80..=0x3fff => 2,         // up to 14 bits
        0x4000..=0x1fffff => 3,     // up to 21 bits
        0x200000..=0x0fffffff => 4, // up to 28 bits
        _ => 5,
    }
}

/// Compute the big endian base 128 encoding of the given [`Arc`] at the given byte.
const fn base128_byte(arc: Arc, pos: usize, total: usize) -> Result<u8> {
    debug_assert!(pos < total);
    let last_byte = checked_add!(pos, 1) == total;
    let mask = if last_byte { 0 } else { 0b10000000 };
    let shift = checked_mul!(checked_sub!(checked_sub!(total, pos), 1), 7);
    Ok(((arc >> shift) & 0b1111111) as u8 | mask)
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/const-oid/unit/src/encoder/tests_139.rs"));
