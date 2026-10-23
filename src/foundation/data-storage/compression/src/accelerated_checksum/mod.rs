//! Optional stable SIMD Adler-32. Linux amd64 selects SSE2/SSSE3/AVX2 safely; WASM uses scalar.

#[doc(hidden)]
pub mod hash;
#[doc(hidden)]
pub mod imp;

use imp::{get_imp, Adler32Imp};

/// An adler32 hash generator type.
#[derive(Clone)]
pub struct Adler32 {
  a: u16,
  b: u16,
  update: Adler32Imp,
}

impl Adler32 {
  /// Constructs a new `Adler32`.
  ///
  /// Potential overhead here due to runtime feature detection although in testing on 100k
  /// and 10k random byte arrays it was not really noticeable.
  ///
  /// # Examples
  /// ```rust
  /// use compression::accelerated_checksum::Adler32;
  ///
  /// let mut adler = Adler32::new();
  /// ```
  pub fn new() -> Self {
    Default::default()
  }

  /// Constructs a new `Adler32` using existing checksum.
  ///
  /// Potential overhead here due to runtime feature detection although in testing on 100k
  /// and 10k random byte arrays it was not really noticeable.
  ///
  /// # Examples
  /// ```rust
  /// use compression::accelerated_checksum::Adler32;
  ///
  /// let mut adler = Adler32::from_checksum(0xdeadbeaf);
  /// ```
  pub fn from_checksum(checksum: u32) -> Self {
    Self {
      a: checksum as u16,
      b: (checksum >> 16) as u16,
      update: get_imp(),
    }
  }

  /// Computes hash for supplied data and stores results in internal state.
  pub fn write(&mut self, data: &[u8]) {
    let (a, b) = (self.update)(self.a, self.b, data);

    self.a = a;
    self.b = b;
  }

  /// Returns the hash value for the values written so far.
  ///
  /// Despite its name, the method does not reset the hasher’s internal state. Additional
  /// writes will continue from the current value. If you need to start a fresh hash
  /// value, you will have to use `reset`.
  pub fn finish(&self) -> u32 {
    (u32::from(self.b) << 16) | u32::from(self.a)
  }

  /// Resets the internal state.
  pub fn reset(&mut self) {
    self.a = 1;
    self.b = 0;
  }
}

/// Compute Adler-32 hash on `Adler32Hash` type.
///
/// # Arguments
/// * `hash` - A Adler-32 hash-able type.
///
/// # Examples
/// ```rust
/// use compression::accelerated_checksum::adler32;
///
/// let hash = adler32(b"Adler-32");
/// println!("{}", hash); // 800813569
/// ```
pub fn adler32<H: Adler32Hash>(hash: &H) -> u32 {
  hash.hash()
}

/// A Adler-32 hash-able type.
pub trait Adler32Hash {
  /// Feeds this value into `Adler32`.
  fn hash(&self) -> u32;
}

impl Default for Adler32 {
  fn default() -> Self {
    Self {
      a: 1,
      b: 0,
      update: get_imp(),
    }
  }
}

#[cfg(feature = "std")]
pub mod read {
  //! Reader-based hashing.
  //!
  //! # Example
  //! ```rust
  //! use std::io::Cursor;
  //! use compression::accelerated_checksum::read::adler32;
  //!
  //! let mut reader = Cursor::new(b"Hello there");
  //! let hash = adler32(&mut reader).unwrap();
  //!
  //! println!("{}", hash) // 800813569
  //! ```
  use crate::accelerated_checksum::Adler32;
  use std::io::{Read, Result};

  /// Compute Adler-32 hash on reader until EOF.
  ///
  /// # Example
  /// ```rust
  /// use std::io::Cursor;
  /// use compression::accelerated_checksum::read::adler32;
  ///
  /// let mut reader = Cursor::new(b"Hello there");
  /// let hash = adler32(&mut reader).unwrap();
  ///
  /// println!("{}", hash) // 800813569
  /// ```
  pub fn adler32<R: Read>(reader: &mut R) -> Result<u32> {
    let mut hash = Adler32::new();
    let mut buf = [0; 4096];

    loop {
      match reader.read(&mut buf) {
        Ok(0) => return Ok(hash.finish()),
        Ok(n) => {
          hash.write(&buf[..n]);
        }
        Err(err) => return Err(err),
      }
    }
  }
}

#[cfg(feature = "std")]
pub mod bufread {
  //! BufRead-based hashing.
  //!
  //! Separate `BufRead` trait implemented to allow for custom buffer size optimization.
  //!
  //! # Example
  //! ```rust
  //! use std::io::{Cursor, BufReader};
  //! use compression::accelerated_checksum::bufread::adler32;
  //!
  //! let mut reader = Cursor::new(b"Hello there");
  //! let mut reader = BufReader::new(reader);
  //! let hash = adler32(&mut reader).unwrap();
  //!
  //! println!("{}", hash) // 800813569
  //! ```
  use crate::accelerated_checksum::Adler32;
  use std::io::{BufRead, ErrorKind, Result};

  /// Compute Adler-32 hash on buf reader until EOF.
  ///
  /// # Example
  /// ```rust
  /// use std::io::{Cursor, BufReader};
  /// use compression::accelerated_checksum::bufread::adler32;
  ///
  /// let mut reader = Cursor::new(b"Hello there");
  /// let mut reader = BufReader::new(reader);
  /// let hash = adler32(&mut reader).unwrap();
  ///
  /// println!("{}", hash) // 800813569
  /// ```
  pub fn adler32<R: BufRead>(reader: &mut R) -> Result<u32> {
    let mut hash = Adler32::new();

    loop {
      let consumed = match reader.fill_buf() {
        Ok(buf) => {
          if buf.is_empty() {
            return Ok(hash.finish());
          }

          hash.write(buf);
          buf.len()
        }
        Err(err) => match err.kind() {
          ErrorKind::Interrupted => continue,
          ErrorKind::UnexpectedEof => return Ok(hash.finish()),
          _ => return Err(err),
        },
      };

      reader.consume(consumed);
    }
  }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/compression/accelerated_checksum/unit/src/lib/tests_306.rs"));
