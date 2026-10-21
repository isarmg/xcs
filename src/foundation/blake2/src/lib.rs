//! An implementation of the [BLAKE2][1] hash functions.
//!
//! # Usage
//!
//! [`Blake2b512`] and [`Blake2s256`] can be used in the following way:
//!
//! ```rust
//! use blake2::{Blake2b512, Blake2s256, Digest};
//!
//! // create a Blake2b512 object
//! let mut hasher = Blake2b512::new();
//!
//! // write input message
//! hasher.update(b"hello world");
//!
//! // read hash digest and consume hasher
//! let res = hasher.finalize();
//! assert_eq!(res[..], [0x02, 0x1c, 0xed, 0x87, 0x99, 0x29, 0x6c, 0xec, 0xa5, 0x57, 0x83, 0x2a, 0xb9, 0x41, 0xa5, 0x0b, 0x4a, 0x11, 0xf8, 0x34, 0x78, 0xcf, 0x14, 0x1f, 0x51, 0xf9, 0x33, 0xf6, 0x53, 0xab, 0x9f, 0xbc, 0xc0, 0x5a, 0x03, 0x7c, 0xdd, 0xbe, 0xd0, 0x6e, 0x30, 0x9b, 0xf3, 0x34, 0x94, 0x2c, 0x4e, 0x58, 0xcd, 0xf1, 0xa4, 0x6e, 0x23, 0x79, 0x11, 0xcc, 0xd7, 0xfc, 0xf9, 0x78, 0x7c, 0xbc, 0x7f, 0xd0][..]);
//!
//! // same example for Blake2s256:
//! let mut hasher = Blake2s256::new();
//! hasher.update(b"hello world");
//! let res = hasher.finalize();
//! assert_eq!(res[..], [0x9a, 0xec, 0x68, 0x06, 0x79, 0x45, 0x61, 0x10, 0x7e, 0x59, 0x4b, 0x1f, 0x6a, 0x8a, 0x6b, 0x0c, 0x92, 0xa0, 0xcb, 0xa9, 0xac, 0xf5, 0xe5, 0xe9, 0x3c, 0xca, 0x06, 0xf7, 0x81, 0x81, 0x3b, 0x0b][..]);
//! ```
//!
//! Also see [RustCrypto/hashes](https://github.com/RustCrypto/hashes) readme.
//!
//! ## Variable output size
//!
//! This implementation supports run and compile time variable sizes.
//!
//! Run time variable output example:
//! ```rust
//! use blake2::Blake2bVar;
//! use blake2::{cryptographic_core::Update, VariableOutput};
//!
//! let mut hasher = Blake2bVar::new(10).unwrap();
//! hasher.update(b"my_input");
//! let mut buf = [0u8; 10];
//! hasher.finalize_variable(&mut buf).unwrap();
//! assert_eq!(buf, [0x2c, 0xc5, 0x5c, 0x84, 0xe4, 0x16, 0x92, 0x4e, 0x64, 0x00]);
//! ```
//!
//! Compile time variable output example:
//! ```rust
//! use blake2::{Blake2b, Digest, cryptographic_core::consts::U10};
//!
//! type Blake2b80 = Blake2b<U10>;
//!
//! let mut hasher = Blake2b80::new();
//! hasher.update(b"my_input");
//! let res = hasher.finalize();
//! assert_eq!(res[..], [0x2c, 0xc5, 0x5c, 0x84, 0xe4, 0x16, 0x92, 0x4e, 0x64, 0x00][..]);
//! ```
//!
//! # Acknowledgment
//! Based on the [blake2-rfc][2] crate.
//!
//! [1]: https://en.wikipedia.org/wiki/BLAKE_(hash_function)#BLAKE2
//! [2]: https://github.com/cesarb/blake2-rfc

#![no_std]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/RustCrypto/media/6ee8e381/logo.svg",
    html_favicon_url = "https://raw.githubusercontent.com/RustCrypto/media/6ee8e381/logo.svg"
)]
#![warn(missing_docs, rust_2018_idioms)]
#![cfg_attr(feature = "simd", feature(platform_intrinsics, repr_simd))]
#![cfg_attr(feature = "simd", allow(incomplete_features))]

#[cfg(feature = "std")]
extern crate std;
#[cfg(feature = "alloc")]
extern crate alloc;

pub use cryptographic_core::{self, Digest};

use core::{convert::TryInto, fmt, marker::PhantomData, ops::Div};
use cryptographic_core::{
    buffer::{Lazy, LazyBuffer},
    consts::{U128, U32, U4, U64},
    block_api::{
        AlgorithmName, Block, BlockSizeUser, Buffer, BufferKindUser,
        CtOutWrapper, OutputSizeUser, TruncSide, UpdateCore,
        VariableOutputCore,
    },
    common_types::{InvalidLength, Key, KeyInit, KeySizeUser},
    array::{ArraySize, Array},
    typenum::{IsLessOrEqual, LeEq, NonZero, Unsigned},
    FixedOutput, HashMarker, InvalidOutputSize, MacMarker, Output, Update,
};
#[cfg(feature = "reset")]
use cryptographic_core::{FixedOutputReset, Reset};

mod buffered;
mod variable;
pub use buffered::BufferedHash;
pub use variable::{RuntimeOutputHash, VariableOutput, VariableOutputReset, InvalidBufferSize};

mod as_bytes;
mod consts;

mod simd;

#[macro_use]
mod macros;

use as_bytes::AsBytes;
use consts::{BLAKE2B_IV, BLAKE2S_IV};
use simd::{u32x4, u64x4, Vector4};

blake2_impl!(
    Blake2bVarCore,
    "Blake2b",
    u64,
    u64x4,
    U64,
    U128,
    32,
    24,
    16,
    63,
    BLAKE2B_IV,
    "Blake2b instance with a variable output.",
    "Blake2b instance with a fixed output.",
);

/// BLAKE2b which allows to choose output size at runtime.
pub type Blake2bVar = RuntimeOutputHash<Blake2bVarCore>;
/// Core hasher state of BLAKE2b generic over output size.
pub type Blake2bCore<OutSize> = CtOutWrapper<Blake2bVarCore, OutSize>;
/// BLAKE2b generic over output size.
pub type Blake2b<OutSize> = BufferedHash<Blake2bCore<OutSize>>;
/// BLAKE2b-512 hasher state.
pub type Blake2b512 = Blake2b<U64>;

blake2_mac_impl!(Blake2bMac, Blake2bVarCore, U64, "Blake2b MAC function");

/// BLAKE2b-512 MAC state.
pub type Blake2bMac512 = Blake2bMac<U64>;

blake2_impl!(
    Blake2sVarCore,
    "Blake2s",
    u32,
    u32x4,
    U32,
    U64,
    16,
    12,
    8,
    7,
    BLAKE2S_IV,
    "Blake2s instance with a variable output.",
    "Blake2s instance with a fixed output.",
);

/// BLAKE2s which allows to choose output size at runtime.
pub type Blake2sVar = RuntimeOutputHash<Blake2sVarCore>;
/// Core hasher state of BLAKE2s generic over output size.
pub type Blake2sCore<OutSize> = CtOutWrapper<Blake2sVarCore, OutSize>;
/// BLAKE2s generic over output size.
pub type Blake2s<OutSize> = BufferedHash<Blake2sCore<OutSize>>;
/// BLAKE2s-256 hasher state.
pub type Blake2s256 = Blake2s<U32>;

blake2_mac_impl!(Blake2sMac, Blake2sVarCore, U32, "Blake2s MAC function");

/// BLAKE2s-256 MAC state.
pub type Blake2sMac256 = Blake2sMac<U32>;
