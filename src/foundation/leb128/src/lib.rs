//! Read and write DWARF's "Little Endian Base 128" (LEB128) variable length
//! integer encoding.
//!
//! The implementation is a direct translation of the psuedocode in the DWARF 4
//! standard's appendix C.
//!
//! Read and write signed integers:
//!
//! ```
//! use leb128;
//!
//! let mut buf = [0; 1024];
//!
//! // Write to anything that implements `std::io::Write`.
//! {
//!     let mut writable = &mut buf[..];
//!     leb128::write::signed(&mut writable, -12345).expect("Should write number");
//! }
//!
//! // Read from anything that implements `std::io::Read`.
//! let mut readable = &buf[..];
//! let val = leb128::read::signed(&mut readable).expect("Should read number");
//! assert_eq!(val, -12345);
//! ```
//!
//! Or read and write unsigned integers:
//!
//! ```
//! use leb128;
//!
//! let mut buf = [0; 1024];
//!
//! {
//!     let mut writable = &mut buf[..];
//!     leb128::write::unsigned(&mut writable, 98765).expect("Should write number");
//! }
//!
//! let mut readable = &buf[..];
//! let val = leb128::read::unsigned(&mut readable).expect("Should read number");
//! assert_eq!(val, 98765);
//! ```

#![deny(missing_docs)]

#[doc(hidden)]
pub const CONTINUATION_BIT: u8 = 1 << 7;
#[doc(hidden)]
pub const SIGN_BIT: u8 = 1 << 6;

#[doc(hidden)]
#[inline]
pub fn low_bits_of_byte(byte: u8) -> u8 {
    byte & !CONTINUATION_BIT
}

#[doc(hidden)]
#[inline]
pub fn low_bits_of_u64(val: u64) -> u8 {
    let byte = val & (u8::MAX as u64);
    low_bits_of_byte(byte as u8)
}

/// A module for reading LEB128-encoded signed and unsigned integers.
pub mod read {
    use super::{low_bits_of_byte, CONTINUATION_BIT, SIGN_BIT};
    use std::fmt;
    use std::io;

    /// An error type for reading LEB128-encoded values.
    #[derive(Debug)]
    pub enum Error {
        /// There was an underlying IO error.
        IoError(io::Error),
        /// The number being read is larger than can be represented.
        Overflow,
    }

    impl From<io::Error> for Error {
        fn from(e: io::Error) -> Self {
            Error::IoError(e)
        }
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
            match *self {
                Error::IoError(ref e) => e.fmt(f),
                Error::Overflow => {
                    write!(f, "The number being read is larger than can be represented")
                }
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match *self {
                Error::IoError(ref e) => Some(e),
                Error::Overflow => None,
            }
        }
    }

    /// Read an unsigned LEB128-encoded number from the `std::io::Read` stream
    /// `r`.
    ///
    /// On success, return the number.
    pub fn unsigned<R>(r: &mut R) -> Result<u64, Error>
    where
        R: ?Sized + io::Read,
    {
        let mut result = 0;
        let mut shift = 0;

        loop {
            let mut buf = [0];
            r.read_exact(&mut buf)?;

            if shift == 63 && buf[0] != 0x00 && buf[0] != 0x01 {
                while buf[0] & CONTINUATION_BIT != 0 {
                    r.read_exact(&mut buf)?;
                }
                return Err(Error::Overflow);
            }

            let low_bits = low_bits_of_byte(buf[0]) as u64;
            result |= low_bits << shift;

            if buf[0] & CONTINUATION_BIT == 0 {
                return Ok(result);
            }

            shift += 7;
        }
    }

    /// Read a signed LEB128-encoded number from the `std::io::Read` stream `r`.
    ///
    /// On success, return the number.
    pub fn signed<R>(r: &mut R) -> Result<i64, Error>
    where
        R: ?Sized + io::Read,
    {
        let mut result = 0;
        let mut shift = 0;
        let size = 64;
        let mut byte;

        loop {
            let mut buf = [0];
            r.read_exact(&mut buf)?;

            byte = buf[0];
            if shift == 63 && byte != 0x00 && byte != 0x7f {
                while buf[0] & CONTINUATION_BIT != 0 {
                    r.read_exact(&mut buf)?;
                }
                return Err(Error::Overflow);
            }

            let low_bits = low_bits_of_byte(byte) as i64;
            result |= low_bits << shift;
            shift += 7;

            if byte & CONTINUATION_BIT == 0 {
                break;
            }
        }

        if shift < size && (SIGN_BIT & byte) == SIGN_BIT {
            // Sign extend the result.
            result |= !0 << shift;
        }

        Ok(result)
    }
}

/// A module for writing LEB128-encoded signed and unsigned integers.
pub mod write {
    use super::{low_bits_of_u64, CONTINUATION_BIT};
    use std::io;

    /// Write `val` to the `std::io::Write` stream `w` as an unsigned LEB128 value.
    ///
    /// On success, return the number of bytes written to `w`.
    pub fn unsigned<W>(w: &mut W, mut val: u64) -> Result<usize, io::Error>
    where
        W: ?Sized + io::Write,
    {
        let mut bytes_written = 0;
        loop {
            let mut byte = low_bits_of_u64(val);
            val >>= 7;
            if val != 0 {
                // More bytes to come, so set the continuation bit.
                byte |= CONTINUATION_BIT;
            }

            let buf = [byte];
            w.write_all(&buf)?;
            bytes_written += 1;

            if val == 0 {
                return Ok(bytes_written);
            }
        }
    }

    /// Return the number of bytes needed to encode `value` as unsigned LEB128.
    pub fn unsigned_len(mut val: u64) -> usize {
        let mut bytes_needed = 0;
        loop {
            bytes_needed += 1;
            val >>= 7;
            if val == 0 {
                return bytes_needed;
            }
        }
    }

    /// Write `val` to the `std::io::Write` stream `w` as a signed LEB128 value.
    ///
    /// On success, return the number of bytes written to `w`.
    pub fn signed<W>(w: &mut W, mut val: i64) -> Result<usize, io::Error>
    where
        W: ?Sized + io::Write,
    {
        let mut bytes_written = 0;
        loop {
            let mut byte = val as u8;
            // Keep the sign bit for testing
            val >>= 6;
            let done = val == 0 || val == -1;
            if done {
                byte &= !CONTINUATION_BIT;
            } else {
                // Remove the sign bit
                val >>= 1;
                // More bytes to come, so set the continuation bit.
                byte |= CONTINUATION_BIT;
            }

            let buf = [byte];
            w.write_all(&buf)?;
            bytes_written += 1;

            if done {
                return Ok(bytes_written);
            }
        }
    }

    /// Return the number of bytes needed to encode `value` a signed LEB128.
    pub fn signed_len(mut val: i64) -> usize {
        let mut bytes_needed = 0;
        loop {
            bytes_needed += 1;
            // Keep the sign bit for testing
            val >>= 6;
            if val == 0 || val == -1 {
                return bytes_needed;
            }
            val >>= 1;
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/leb128/unit/src/lib/tests_269.rs"));
