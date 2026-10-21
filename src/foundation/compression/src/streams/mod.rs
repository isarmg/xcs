//! DEFLATE, zlib, and gzip streaming I/O using the maintained Rust codec.
//! Read wrappers may read ahead; use bufread wrappers to preserve trailing input.

#![allow(trivial_numeric_casts)]
#![cfg_attr(test, deny(warnings))]




extern crate std;




use std::{error, io};

use alloc::vec::Vec;

pub use crate::streams::crc::{Crc, CrcReader, CrcWriter};
pub use crate::streams::gz::GzBuilder;
pub use crate::streams::gz::GzHeader;
pub use crate::streams::mem::{Compress, CompressError, Decompress, DecompressError, Status};
pub use crate::streams::mem::{FlushCompress, FlushDecompress};

mod bufreader;
mod crc;
mod deflate;
mod ffi;
mod gz;
mod mem;
mod zio;
mod zlib;

/// Types which operate over [`Read`] streams, both encoders and decoders for
/// various formats.
///
/// Note that the `read` decoder types may read past the end of the compressed
/// data while decoding. If the caller requires subsequent reads to start
/// immediately following the compressed data  wrap the `Read` type in a
/// [`BufReader`] and use the `BufReader` with the equivalent decoder from the
/// `bufread` module and also for the subsequent reads.
///
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
/// [`BufReader`]: https://doc.rust-lang.org/std/io/struct.BufReader.html
pub mod read {
    pub use crate::streams::deflate::read::DeflateDecoder;
    pub use crate::streams::deflate::read::DeflateEncoder;
    pub use crate::streams::gz::read::GzDecoder;
    pub use crate::streams::gz::read::GzEncoder;
    pub use crate::streams::gz::read::MultiGzDecoder;
    pub use crate::streams::zlib::read::ZlibDecoder;
    pub use crate::streams::zlib::read::ZlibEncoder;
}

/// Types which operate over [`Write`] streams, both encoders and decoders for
/// various formats.
///
/// [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
pub mod write {
    pub use crate::streams::deflate::write::DeflateDecoder;
    pub use crate::streams::deflate::write::DeflateEncoder;
    pub use crate::streams::gz::write::GzDecoder;
    pub use crate::streams::gz::write::GzEncoder;
    pub use crate::streams::gz::write::MultiGzDecoder;
    pub use crate::streams::zlib::write::ZlibDecoder;
    pub use crate::streams::zlib::write::ZlibEncoder;
}

/// Types which operate over [`BufRead`] streams, both encoders and decoders for
/// various formats.
///
/// [`BufRead`]: https://doc.rust-lang.org/std/io/trait.BufRead.html
pub mod bufread {
    pub use crate::streams::deflate::bufread::DeflateDecoder;
    pub use crate::streams::deflate::bufread::DeflateEncoder;
    pub use crate::streams::gz::bufread::GzDecoder;
    pub use crate::streams::gz::bufread::GzEncoder;
    pub use crate::streams::gz::bufread::MultiGzDecoder;
    pub use crate::streams::zlib::bufread::ZlibDecoder;
    pub use crate::streams::zlib::bufread::ZlibEncoder;
}

fn _assert_send_sync() {
    fn _assert_send_sync<T: Send + Sync>() {}

    _assert_send_sync::<read::DeflateEncoder<&[u8]>>();
    _assert_send_sync::<read::DeflateDecoder<&[u8]>>();
    _assert_send_sync::<read::ZlibEncoder<&[u8]>>();
    _assert_send_sync::<read::ZlibDecoder<&[u8]>>();
    _assert_send_sync::<read::GzEncoder<&[u8]>>();
    _assert_send_sync::<read::GzDecoder<&[u8]>>();
    _assert_send_sync::<read::MultiGzDecoder<&[u8]>>();
    _assert_send_sync::<write::DeflateEncoder<Vec<u8>>>();
    _assert_send_sync::<write::DeflateDecoder<Vec<u8>>>();
    _assert_send_sync::<write::ZlibEncoder<Vec<u8>>>();
    _assert_send_sync::<write::ZlibDecoder<Vec<u8>>>();
    _assert_send_sync::<write::GzEncoder<Vec<u8>>>();
    _assert_send_sync::<write::GzDecoder<Vec<u8>>>();
}

/// When compressing data, the compression level can be specified by a value in
/// this struct.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Compression(u32);

impl Compression {
    /// Creates a new description of the compression level with an explicitly
    /// specified integer.
    ///
    /// The integer here is typically on a scale of 0-9 where 0 means "no
    /// compression" and 9 means "take as long as you'd like".
    pub const fn new(level: u32) -> Compression {
        Compression(level)
    }

    /// No compression is to be performed, this may actually inflate data
    /// slightly when encoding.
    pub const fn none() -> Compression {
        Compression(0)
    }

    /// Optimize for the best speed of encoding.
    pub const fn fast() -> Compression {
        Compression(1)
    }

    /// Optimize for the size of data being encoded.
    pub const fn best() -> Compression {
        Compression(9)
    }

    /// Returns an integer representing the compression level, typically on a
    /// scale of 0-9. See [`new`](Self::new) for details about compression levels.
    pub fn level(&self) -> u32 {
        self.0
    }
}

impl Default for Compression {
    fn default() -> Compression {
        Compression(6)
    }
}
